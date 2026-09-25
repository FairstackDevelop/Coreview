use serde::Serialize;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;
use sysinfo::{Components, Disks, Networks, Pid, ProcessesToUpdate, System};

pub struct Shared {
    pub sys: Mutex<System>,
    nets: Mutex<Networks>,
    disks: Mutex<Disks>,
    comps: Mutex<Components>,
    last: Mutex<Instant>,
    #[cfg(windows)]
    perf: Mutex<Option<crate::cpuperf::CpuPerf>>,
}

#[derive(Clone)]
pub struct AppState(Arc<OnceLock<Shared>>);

impl AppState {
    pub fn new() -> Self {
        Self(Arc::new(OnceLock::new()))
    }

    pub fn get(&self) -> &Shared {
        self.0.get_or_init(Shared::new)
    }
}

impl Shared {
    fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_all();
        sys.refresh_memory();
        Self {
            sys: Mutex::new(sys),
            nets: Mutex::new(Networks::new_with_refreshed_list()),
            disks: Mutex::new(Disks::new_with_refreshed_list()),
            comps: Mutex::new(Components::new_with_refreshed_list()),
            last: Mutex::new(Instant::now()),
            #[cfg(windows)]
            perf: Mutex::new(crate::cpuperf::CpuPerf::new()),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Temp {
    label: String,
    celsius: f32,
    max: Option<f32>,
    critical: Option<f32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskUsageLive {
    mount: String,
    total: u64,
    available: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveStats {
    timestamp: u64,
    cpu_total: f32,
    cpu_cores: Vec<f32>,
    cpu_mhz: u64,
    mem_used: u64,
    mem_total: u64,
    swap_used: u64,
    swap_total: u64,
    temps: Vec<Temp>,
    net_rx: f64,
    net_tx: f64,
    disk_read: f64,
    disk_write: f64,
    disks: Vec<DiskUsageLive>,
    load: [f64; 3],
}

fn collect_live(state: &Shared) -> LiveStats {
    let elapsed = {
        let mut last = state.last.lock().unwrap();
        let e = last.elapsed().as_secs_f64().max(0.05);
        *last = Instant::now();
        e
    };

    let mut sys = state.sys.lock().unwrap();
    sys.refresh_cpu_all();
    sys.refresh_memory();

    let mut nets = state.nets.lock().unwrap();
    nets.refresh(true);
    let (rx, tx) = nets
        .list()
        .values()
        .fold((0u64, 0u64), |(r, t), n| (r + n.received(), t + n.transmitted()));

    let mut disks = state.disks.lock().unwrap();
    disks.refresh(true);
    let (dr, dw) = disks.list().iter().fold((0u64, 0u64), |(r, w), d| {
        let u = d.usage();
        (r + u.read_bytes, w + u.written_bytes)
    });

    let mut comps = state.comps.lock().unwrap();
    comps.refresh(true);
    let temps = comps
        .list()
        .iter()
        .filter(|c| !c.label().to_lowercase().contains("tcal"))
        .filter_map(|c| {
            c.temperature().filter(|t| t.is_finite() && *t > 0.0).map(|t| Temp {
                label: c.label().to_string(),
                celsius: t,
                max: c.max().filter(|m| m.is_finite()),
                critical: c.critical().filter(|m| m.is_finite()),
            })
        })
        .collect();

    #[cfg(windows)]
    let (util_override, mhz_override) = state.perf.lock().unwrap().as_ref().map(|p| p.read()).unwrap_or((None, None));
    #[cfg(not(windows))]
    let (util_override, mhz_override): (Option<f64>, Option<f64>) = (None, None);

    let cpus = sys.cpus();
    let mhz = if cpus.is_empty() { 0 } else { cpus.iter().map(|c| c.frequency()).sum::<u64>() / cpus.len() as u64 };
    let la = System::load_average();

    LiveStats {
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        cpu_total: util_override.map(|u| u.clamp(0.0, 100.0) as f32).unwrap_or_else(|| sys.global_cpu_usage()),
        cpu_cores: cpus.iter().map(|c| c.cpu_usage()).collect(),
        cpu_mhz: mhz_override.filter(|m| *m > 0.0).map(|m| m as u64).unwrap_or(mhz),
        mem_used: sys.used_memory(),
        mem_total: sys.total_memory(),
        swap_used: sys.used_swap(),
        swap_total: sys.total_swap(),
        temps,
        net_rx: rx as f64 / elapsed,
        net_tx: tx as f64 / elapsed,
        disk_read: dr as f64 / elapsed,
        disk_write: dw as f64 / elapsed,
        disks: disks
            .list()
            .iter()
            .map(|d| DiskUsageLive {
                mount: d.mount_point().to_string_lossy().into_owned(),
                total: d.total_space(),
                available: d.available_space(),
            })
            .collect(),
        load: [la.one, la.five, la.fifteen],
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcInfo {
    pid: u32,
    name: String,
    cpu: f32,
    memory: u64,
    status: String,
    run_time: u64,
    exe: String,
}

fn collect_processes(state: &Shared) -> Vec<ProcInfo> {
    let mut sys = state.sys.lock().unwrap();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    let cores = sys.cpus().len().max(1) as f32;
    let mut list: Vec<ProcInfo> = sys
        .processes()
        .values()
        .map(|p| ProcInfo {
            pid: p.pid().as_u32(),
            name: p.name().to_string_lossy().into_owned(),
            cpu: p.cpu_usage() / cores,
            memory: p.memory(),
            status: p.status().to_string(),
            run_time: p.run_time(),
            exe: p.exe().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default(),
        })
        .collect();
    list.sort_by(|a, b| b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal).then(b.memory.cmp(&a.memory)));
    list.truncate(300);
    list
}

fn terminate(pid: u32, state: &Shared) -> bool {
    let sys = state.sys.lock().unwrap();
    sys.process(Pid::from_u32(pid)).map(|p| p.kill()).unwrap_or(false)
}

#[tauri::command]
pub async fn live_stats(state: tauri::State<'_, AppState>) -> Result<LiveStats, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || collect_live(state.get()))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn processes(state: tauri::State<'_, AppState>) -> Result<Vec<ProcInfo>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || collect_processes(state.get()))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn kill_process(pid: u32, state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || terminate(pid, state.get()))
        .await
        .map_err(|e| e.to_string())
}
