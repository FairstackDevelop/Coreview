use crate::util::*;
use serde::Serialize;

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PowerStats {
    watts: Option<f64>,
    battery_watts: Option<f64>,
    adapter_watts: Option<f64>,
    adapter_rated: Option<u64>,
    on_ac: bool,
    percent: Option<u64>,
    charging: bool,
    minutes_remaining: Option<u64>,
    cpu_watts: Option<f64>,
    gpu_watts: Option<f64>,
    ane_watts: Option<f64>,
    dram_watts: Option<f64>,
    battery_volts: Option<f64>,
    battery_amps: Option<f64>,
    gpu_load: Option<f64>,
    poll_ms: u64,
    source: String,
}

#[cfg(target_os = "macos")]
fn nested_num(text: &str, key: &str) -> Option<i64> {
    let needle = format!("\"{key}\"=");
    let start = text.find(&needle)? + needle.len();
    let rest = &text[start..];
    let end = rest.find(|c: char| !(c.is_ascii_digit() || c == '-')).unwrap_or(rest.len());
    let s = &rest[..end];
    s.parse::<i64>().ok().or_else(|| s.parse::<u64>().ok().map(|v| v as i64))
}

#[cfg(target_os = "macos")]
fn mac_power() -> Option<PowerStats> {
    let io = run("ioreg", &["-rn", "AppleSmartBattery"])?;
    let volt = ioreg_num(&io, "Voltage")? as f64;
    let amp = ioreg_num(&io, "InstantAmperage").or_else(|| ioreg_num(&io, "Amperage"))? as i64 as f64;
    let battery_w = amp * volt / 1_000_000.0;
    let on_ac = io.contains("\"ExternalConnected\" = Yes");
    let system = nested_num(&io, "SystemLoad").filter(|v| *v > 0).map(|v| v as f64 / 1000.0);
    let max = ioreg_num(&io, "MaxCapacity").filter(|m| *m > 0);
    let parts = crate::ioreport::breakdown();
    let gpu_load = run("ioreg", &["-rc", "IOAccelerator", "-d", "1"]).and_then(|s| nested_num(&s, "Device Utilization %")).map(|v| v as f64);
    Some(PowerStats {
        cpu_watts: parts.cpu,
        gpu_watts: parts.gpu,
        ane_watts: parts.ane,
        dram_watts: parts.dram,
        battery_volts: Some(volt / 1000.0),
        battery_amps: Some(amp / 1000.0),
        gpu_load,
        watts: system.or_else(|| (!on_ac).then_some(-battery_w).filter(|w| *w > 0.0)),
        battery_watts: Some(battery_w),
        adapter_watts: nested_num(&io, "SystemPowerIn").filter(|v| *v > 0).map(|v| v as f64 / 1000.0),
        adapter_rated: nested_num(&io, "Watts").filter(|v| *v > 0).map(|v| v as u64),
        on_ac,
        percent: ioreg_num(&io, "CurrentCapacity").zip(max).map(|(c, m)| (c * 100 / m).min(100)),
        charging: io.contains("\"IsCharging\" = Yes"),
        minutes_remaining: ioreg_num(&io, "TimeRemaining").filter(|m| *m < 60_000),
        poll_ms: 2000,
        source: "AppleSmartBattery".into(),
    })
}

#[cfg(windows)]
static BATTERY: std::sync::Mutex<Option<(std::time::Instant, Option<serde_json::Value>, Option<serde_json::Value>)>> = std::sync::Mutex::new(None);

#[cfg(windows)]
fn battery_rows() -> (Option<serde_json::Value>, Option<serde_json::Value>) {
    let mut cache = BATTERY.lock().unwrap();
    if let Some((at, status, battery)) = cache.as_ref() {
        if at.elapsed().as_secs() < 15 {
            return (status.clone(), battery.clone());
        }
    }
    let status = ps("Get-CimInstance -Namespace root/wmi -ClassName BatteryStatus | Select-Object DischargeRate,ChargeRate,Charging,Discharging,PowerOnline").into_iter().next();
    let battery = ps("Get-CimInstance Win32_Battery | Select-Object EstimatedChargeRemaining,EstimatedRunTime").into_iter().next();
    *cache = Some((std::time::Instant::now(), status.clone(), battery.clone()));
    (status, battery)
}

#[cfg(windows)]
fn win_power() -> Option<PowerStats> {
    let (status, battery) = battery_rows();
    let extra = crate::winsensors::to_power();
    if status.is_none() && battery.is_none() && extra.cpu.is_none() && extra.gpu.is_none() && extra.gpu_load.is_none() {
        return None;
    }
    let discharge = status.as_ref().map(|s| n(s, "DischargeRate") as f64 / 1000.0).unwrap_or(0.0);
    let charge = status.as_ref().map(|s| n(s, "ChargeRate") as f64 / 1000.0).unwrap_or(0.0);
    let flag = |k: &str| status.as_ref().and_then(|s| s.get(k)).and_then(|v| v.as_bool()).unwrap_or(false);
    Some(PowerStats {
        cpu_watts: extra.cpu,
        gpu_watts: extra.gpu,
        gpu_load: extra.gpu_load,
        watts: (discharge > 0.0).then_some(discharge),
        battery_watts: status.as_ref().map(|_| if charge > 0.0 { charge } else { -discharge }),
        on_ac: flag("PowerOnline"),
        charging: flag("Charging"),
        percent: battery.as_ref().map(|b| n(b, "EstimatedChargeRemaining")),
        minutes_remaining: battery.as_ref().map(|b| n(b, "EstimatedRunTime")).filter(|m| *m < 60_000),
        poll_ms: 2000,
        source: "LibreHardwareMonitor".into(),
        ..Default::default()
    })
}

#[cfg(target_os = "macos")]
fn platform_power(_app: &tauri::AppHandle) -> Option<PowerStats> {
    mac_power()
}

#[cfg(windows)]
fn platform_power(app: &tauri::AppHandle) -> Option<PowerStats> {
    crate::winsensors::ensure_started(app);
    win_power()
}

#[cfg(not(any(target_os = "macos", windows)))]
fn platform_power(_app: &tauri::AppHandle) -> Option<PowerStats> {
    None
}

#[tauri::command]
pub async fn power_stats(app: tauri::AppHandle) -> Option<PowerStats> {
    tauri::async_runtime::spawn_blocking(move || platform_power(&app)).await.ok().flatten()
}
