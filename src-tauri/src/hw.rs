use crate::util::*;
use serde::Serialize;
use serde_json::Value;
use sysinfo::{Disks, Networks, System};

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OsInfo {
    name: String,
    version: String,
    kernel: String,
    arch: String,
    hostname: String,
    uptime_secs: u64,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
    brand: String,
    vendor: String,
    physical_cores: usize,
    logical_cores: usize,
    base_mhz: u64,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MemModule {
    size: String,
    kind: String,
    speed: String,
    manufacturer: String,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInfo {
    total: u64,
    swap_total: u64,
    modules: Vec<MemModule>,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BoardInfo {
    manufacturer: String,
    model: String,
    serial: String,
    firmware: String,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    name: String,
    vendor: String,
    vram: String,
    cores: String,
    driver: String,
    displays: Vec<String>,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DiskInfo {
    name: String,
    mount: String,
    fs: String,
    kind: String,
    total: u64,
    available: u64,
    removable: bool,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PhysDisk {
    name: String,
    media: String,
    size: u64,
    health: String,
    protocol: String,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetIface {
    name: String,
    mac: String,
    ips: Vec<String>,
    mtu: u64,
    state: String,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BatteryInfo {
    percent: u64,
    charging: bool,
    plugged: bool,
    cycle_count: Option<u64>,
    health_percent: Option<u64>,
    design_mah: Option<u64>,
    max_mah: Option<u64>,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HardwareInfo {
    os: OsInfo,
    cpu: CpuInfo,
    memory: MemoryInfo,
    board: BoardInfo,
    gpus: Vec<GpuInfo>,
    disks: Vec<DiskInfo>,
    physical_disks: Vec<PhysDisk>,
    network: Vec<NetIface>,
    battery: Option<BatteryInfo>,
}

fn mac_profiler(kind: &str) -> Value {
    json_cmd("system_profiler", &[kind, "-json"]).unwrap_or(Value::Null)
}

fn first(v: &Value, key: &str) -> Value {
    v.get(key)
        .and_then(|a| a.as_array())
        .and_then(|a| a.first())
        .cloned()
        .unwrap_or(Value::Null)
}

fn os_info() -> OsInfo {
    OsInfo {
        name: System::name().unwrap_or_default(),
        version: System::long_os_version().or_else(System::os_version).unwrap_or_default(),
        kernel: System::kernel_version().unwrap_or_default(),
        arch: System::cpu_arch(),
        hostname: System::host_name().unwrap_or_default(),
        uptime_secs: System::uptime(),
    }
}

fn board_info() -> BoardInfo {
    if cfg!(target_os = "macos") {
        let hw = first(&mac_profiler("SPHardwareDataType"), "SPHardwareDataType");
        return BoardInfo {
            manufacturer: "Apple".into(),
            model: format!("{} ({})", s(&hw, "machine_name"), s(&hw, "machine_model")),
            serial: s(&hw, "serial_number"),
            firmware: s(&hw, "boot_rom_version"),
        };
    }
    if cfg!(windows) {
        let b = ps("Get-CimInstance Win32_BaseBoard | Select Manufacturer,Product,SerialNumber")
            .into_iter()
            .next()
            .unwrap_or(Value::Null);
        let bios = ps("Get-CimInstance Win32_BIOS | Select SMBIOSBIOSVersion,Manufacturer")
            .into_iter()
            .next()
            .unwrap_or(Value::Null);
        return BoardInfo {
            manufacturer: s(&b, "Manufacturer"),
            model: s(&b, "Product"),
            serial: s(&b, "SerialNumber"),
            firmware: format!("{} {}", s(&bios, "Manufacturer"), s(&bios, "SMBIOSBIOSVersion")),
        };
    }
    let m = sysinfo::Motherboard::new();
    BoardInfo {
        manufacturer: m.as_ref().and_then(|m| m.vendor_name()).unwrap_or_default(),
        model: m.as_ref().and_then(|m| m.name()).unwrap_or_default(),
        serial: m.as_ref().and_then(|m| m.serial_number()).unwrap_or_default(),
        firmware: m.as_ref().and_then(|m| m.version()).unwrap_or_default(),
    }
}

fn memory_modules() -> Vec<MemModule> {
    if cfg!(target_os = "macos") {
        let m = first(&mac_profiler("SPMemoryDataType"), "SPMemoryDataType");
        return vec![MemModule {
            size: s(&m, "SPMemoryDataType"),
            kind: s(&m, "dimm_type"),
            speed: s(&m, "dimm_speed"),
            manufacturer: s(&m, "dimm_manufacturer"),
        }];
    }
    if cfg!(windows) {
        return ps("Get-CimInstance Win32_PhysicalMemory | Select Capacity,Speed,Manufacturer,SMBIOSMemoryType")
            .iter()
            .map(|m| MemModule {
                size: format!("{} GB", n(m, "Capacity") / 1_073_741_824),
                kind: match n(m, "SMBIOSMemoryType") {
                    26 => "DDR4".into(),
                    34 => "DDR5".into(),
                    24 => "DDR3".into(),
                    _ => "DDR".into(),
                },
                speed: format!("{} MHz", n(m, "Speed")),
                manufacturer: s(m, "Manufacturer"),
            })
            .collect();
    }
    Vec::new()
}

fn gpus() -> Vec<GpuInfo> {
    if cfg!(target_os = "macos") {
        let v = mac_profiler("SPDisplaysDataType");
        return v
            .get("SPDisplaysDataType")
            .and_then(|a| a.as_array())
            .map(|a| {
                a.iter()
                    .map(|g| GpuInfo {
                        name: s(g, "sppci_model"),
                        vendor: s(g, "spdisplays_vendor").replace("sppci_vendor_", ""),
                        vram: s(g, "spdisplays_vram"),
                        cores: s(g, "sppci_cores"),
                        driver: String::new(),
                        displays: g
                            .get("spdisplays_ndrvs")
                            .and_then(|d| d.as_array())
                            .map(|d| {
                                d.iter()
                                    .map(|x| format!("{} — {}", s(x, "_name"), s(x, "_spdisplays_resolution")))
                                    .collect()
                            })
                            .unwrap_or_default(),
                    })
                    .collect()
            })
            .unwrap_or_default();
    }
    if cfg!(windows) {
        return ps("Get-CimInstance Win32_VideoController | Select Name,AdapterCompatibility,AdapterRAM,DriverVersion,CurrentHorizontalResolution,CurrentVerticalResolution,CurrentRefreshRate")
            .iter()
            .map(|g| GpuInfo {
                name: s(g, "Name"),
                vendor: s(g, "AdapterCompatibility"),
                vram: match n(g, "AdapterRAM") {
                    0 => String::new(),
                    b => format!("{} MB", b / 1_048_576),
                },
                cores: String::new(),
                driver: s(g, "DriverVersion"),
                displays: vec![format!(
                    "{}×{} @ {} Hz",
                    n(g, "CurrentHorizontalResolution"),
                    n(g, "CurrentVerticalResolution"),
                    n(g, "CurrentRefreshRate")
                )],
            })
            .collect();
    }
    Vec::new()
}

fn physical_disks() -> Vec<PhysDisk> {
    if cfg!(target_os = "macos") {
        let mut out = Vec::new();
        for kind in ["SPNVMeDataType", "SPSerialATADataType"] {
            let v = mac_profiler(kind);
            let mut found = Vec::new();
            walk(&v, &mut found, &|x| x.get("smart_status").is_some());
            for d in found {
                out.push(PhysDisk {
                    name: s(d, "device_name").is_empty().then(|| s(d, "_name")).unwrap_or_else(|| s(d, "device_name")),
                    media: if kind == "SPNVMeDataType" { "NVMe SSD".into() } else { s(d, "spsata_medium_type") },
                    size: n(d, "size_in_bytes"),
                    health: s(d, "smart_status"),
                    protocol: if kind == "SPNVMeDataType" { "NVMe".into() } else { "SATA".into() },
                });
            }
        }
        return out;
    }
    if cfg!(windows) {
        return ps("Get-PhysicalDisk | Select FriendlyName,MediaType,Size,HealthStatus,BusType")
            .iter()
            .map(|d| PhysDisk {
                name: s(d, "FriendlyName"),
                media: s(d, "MediaType"),
                size: n(d, "Size"),
                health: s(d, "HealthStatus"),
                protocol: s(d, "BusType"),
            })
            .collect();
    }
    Vec::new()
}

fn logical_disks() -> Vec<DiskInfo> {
    Disks::new_with_refreshed_list()
        .list()
        .iter()
        .map(|d| DiskInfo {
            name: d.name().to_string_lossy().into_owned(),
            mount: d.mount_point().to_string_lossy().into_owned(),
            fs: d.file_system().to_string_lossy().into_owned(),
            kind: d.kind().to_string(),
            total: d.total_space(),
            available: d.available_space(),
            removable: d.is_removable(),
        })
        .collect()
}

fn interfaces() -> Vec<NetIface> {
    let mut list: Vec<NetIface> = Networks::new_with_refreshed_list()
        .list()
        .iter()
        .map(|(name, d)| NetIface {
            name: name.clone(),
            mac: d.mac_address().to_string(),
            ips: d.ip_networks().iter().map(|i| i.to_string()).collect(),
            mtu: d.mtu(),
            state: format!("{:?}", d.operational_state()),
        })
        .collect();
    list.sort_by(|a, b| a.name.cmp(&b.name));
    list
}

fn battery() -> Option<BatteryInfo> {
    if cfg!(target_os = "macos") {
        let batt = run("pmset", &["-g", "batt"])?;
        if !batt.contains("InternalBattery") {
            return None;
        }
        let percent = batt
            .split('%')
            .next()
            .and_then(|h| h.rsplit(|c: char| !c.is_ascii_digit()).next())
            .and_then(|d| d.parse().ok())
            .unwrap_or(0);
        let lower = batt.to_lowercase();
        let io = run("ioreg", &["-rn", "AppleSmartBattery"]).unwrap_or_default();
        let design = ioreg_num(&io, "DesignCapacity");
        let max = ioreg_num(&io, "AppleRawMaxCapacity").or_else(|| ioreg_num(&io, "NominalChargeCapacity"));
        return Some(BatteryInfo {
            percent,
            charging: lower.contains("; charging") || lower.contains("finishing charge"),
            plugged: lower.contains("ac power"),
            cycle_count: ioreg_num(&io, "CycleCount"),
            health_percent: design.zip(max).filter(|(d, _)| *d > 0).map(|(d, m)| (m * 100 / d).min(100)),
            design_mah: design,
            max_mah: max,
        });
    }
    if cfg!(windows) {
        let b = ps("Get-CimInstance Win32_Battery | Select EstimatedChargeRemaining,BatteryStatus")
            .into_iter()
            .next()?;
        let status = n(&b, "BatteryStatus");
        return Some(BatteryInfo {
            percent: n(&b, "EstimatedChargeRemaining"),
            charging: status == 6 || status == 7 || status == 8 || status == 9,
            plugged: status != 1,
            ..Default::default()
        });
    }
    None
}

#[tauri::command]
pub async fn hardware_info() -> HardwareInfo {
    tauri::async_runtime::spawn_blocking(|| {
        let mut sys = System::new();
        sys.refresh_cpu_all();
        sys.refresh_memory();
        let cpus = sys.cpus();
        let cpu = CpuInfo {
            brand: cpus.first().map(|c| c.brand().to_string()).unwrap_or_default(),
            vendor: cpus.first().map(|c| c.vendor_id().to_string()).unwrap_or_default(),
            physical_cores: System::physical_core_count().unwrap_or(cpus.len()),
            logical_cores: cpus.len(),
            base_mhz: cpus.first().map(|c| c.frequency()).unwrap_or(0),
        };
        HardwareInfo {
            os: os_info(),
            cpu,
            memory: MemoryInfo {
                total: sys.total_memory(),
                swap_total: sys.total_swap(),
                modules: memory_modules(),
            },
            board: board_info(),
            gpus: gpus(),
            disks: logical_disks(),
            physical_disks: physical_disks(),
            network: interfaces(),
            battery: battery(),
        }
    })
    .await
    .unwrap_or_default()
}
