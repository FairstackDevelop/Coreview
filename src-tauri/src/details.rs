use crate::util::*;
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize, Default)]
pub struct Node {
    name: String,
    props: Vec<(String, String)>,
    children: Vec<Node>,
}

const NAME_KEYS: [&str; 10] = [
    "_name", "name", "Name", "Caption", "DisplayName", "DeviceName", "FriendlyName", "device_name", "sppci_model", "Description",
];

fn scalar(v: &Value) -> Option<String> {
    match v {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn label_of(v: &Value, fallback: &str) -> String {
    NAME_KEYS
        .iter()
        .find_map(|k| v.get(*k).and_then(scalar))
        .unwrap_or_else(|| fallback.to_string())
}

fn to_node(name: String, v: &Value) -> Node {
    let mut node = Node { name, ..Default::default() };
    let Value::Object(map) = v else {
        if let Some(s) = scalar(v) {
            node.props.push(("value".into(), s));
        }
        return node;
    };
    for (k, val) in map {
        if NAME_KEYS.contains(&k.as_str()) && node.name == scalar(val).unwrap_or_default() {
            continue;
        }
        match val {
            Value::Object(_) => node.children.push(to_node(k.clone(), val)),
            Value::Array(a) if a.iter().any(|x| x.is_object()) => {
                for (i, x) in a.iter().enumerate() {
                    node.children.push(to_node(label_of(x, &format!("{k} {}", i + 1)), x));
                }
            }
            Value::Array(a) => {
                let joined = a.iter().filter_map(scalar).collect::<Vec<_>>().join(", ");
                if !joined.is_empty() {
                    node.props.push((k.clone(), joined));
                }
            }
            other => {
                if let Some(s) = scalar(other) {
                    node.props.push((k.clone(), s));
                }
            }
        }
    }
    node
}

fn objects(values: &[Value], fallback: &str) -> Vec<Node> {
    values.iter().enumerate().map(|(i, v)| to_node(label_of(v, &format!("{fallback} {}", i + 1)), v)).collect()
}

fn section(name: &str, children: Vec<Node>) -> Option<Node> {
    (!children.is_empty()).then(|| Node { name: name.into(), props: Vec::new(), children })
}

pub fn categories() -> Vec<&'static str> {
    if cfg!(target_os = "macos") {
        vec!["computer", "cpu", "memory", "graphics", "storage", "network", "wifi", "usb", "bluetooth", "audio", "power", "printers", "cameras", "software", "drivers", "services", "env"]
    } else if cfg!(windows) {
        vec!["computer", "cpu", "memory", "graphics", "storage", "network", "wifi", "usb", "bluetooth", "audio", "power", "printers", "cameras", "pci", "sensors", "software", "drivers", "services", "env"]
    } else {
        vec!["computer", "cpu", "env"]
    }
}

fn mac_types(id: &str) -> &'static [&'static str] {
    match id {
        "computer" => &["SPHardwareDataType", "SPSoftwareDataType"],
        "memory" => &["SPMemoryDataType"],
        "graphics" => &["SPDisplaysDataType"],
        "storage" => &["SPStorageDataType", "SPNVMeDataType", "SPSerialATADataType"],
        "network" => &["SPNetworkDataType", "SPEthernetDataType"],
        "wifi" => &["SPAirPortDataType"],
        "usb" => &["SPUSBDataType", "SPUSBHostDataType", "SPThunderboltDataType"],
        "bluetooth" => &["SPBluetoothDataType"],
        "audio" => &["SPAudioDataType"],
        "power" => &["SPPowerDataType"],
        "printers" => &["SPPrintersDataType"],
        "cameras" => &["SPCameraDataType"],
        "software" => &["SPApplicationsDataType"],
        "drivers" => &["SPExtensionsDataType"],
        _ => &[],
    }
}

fn mac_detail(id: &str) -> Vec<Node> {
    match id {
        "cpu" => {
            let text = run("sysctl", &["-a"]).unwrap_or_default();
            let mut props: Vec<(String, String)> = text
                .lines()
                .filter(|l| l.starts_with("hw.") || l.starts_with("machdep.cpu."))
                .filter_map(|l| l.split_once(": "))
                .map(|(k, v)| (k.to_string(), v.trim().to_string()))
                .filter(|(_, v)| !v.is_empty())
                .collect();
            props.sort();
            vec![Node { name: "CPU".into(), props, children: Vec::new() }]
        }
        "services" => {
            let text = run("launchctl", &["list"]).unwrap_or_default();
            let props = text
                .lines()
                .skip(1)
                .filter_map(|l| {
                    let f: Vec<&str> = l.split_whitespace().collect();
                    (f.len() == 3).then(|| (f[2].to_string(), if f[0] == "-" { format!("— ({})", f[1]) } else { format!("PID {}", f[0]) }))
                })
                .collect();
            vec![Node { name: "launchd".into(), props, children: Vec::new() }]
        }
        "env" => env_nodes(),
        _ => mac_types(id)
            .iter()
            .filter_map(|ty| {
                let v = json_cmd("system_profiler", &[ty, "-json"])?;
                let arr = v.get(*ty)?.as_array()?.clone();
                section(ty.trim_start_matches("SP").trim_end_matches("DataType"), objects(&arr, "Item"))
            })
            .collect(),
    }
}

fn env_nodes() -> Vec<Node> {
    let mut props: Vec<(String, String)> = std::env::vars().collect();
    props.sort();
    vec![Node { name: "Environment".into(), props, children: Vec::new() }]
}

const CIM_SKIP: &str = "-ExcludeProperty CimClass,CimInstanceProperties,CimSystemProperties";

fn cim(class: &str, filter: &str) -> Vec<Value> {
    ps(&format!("Get-CimInstance {class} {filter} | Select-Object * {CIM_SKIP}"))
}

fn pnp(filter: &str) -> Vec<Value> {
    ps(&format!("Get-PnpDevice {filter} | Select-Object * {CIM_SKIP}"))
}

fn win_detail(id: &str) -> Vec<Node> {
    let sets: Vec<(&str, Vec<Value>)> = match id {
        "computer" => vec![
            ("Computer system", cim("Win32_ComputerSystem", "")),
            ("Operating system", cim("Win32_OperatingSystem", "")),
            ("BIOS", cim("Win32_BIOS", "")),
            ("Base board", cim("Win32_BaseBoard", "")),
            ("Enclosure", cim("Win32_SystemEnclosure", "")),
        ],
        "cpu" => vec![("Processor", cim("Win32_Processor", "")), ("Cache", cim("Win32_CacheMemory", ""))],
        "memory" => vec![("Memory array", cim("Win32_PhysicalMemoryArray", "")), ("Modules", cim("Win32_PhysicalMemory", ""))],
        "graphics" => vec![("Video controllers", cim("Win32_VideoController", "")), ("Monitors", cim("Win32_DesktopMonitor", ""))],
        "storage" => vec![
            ("Disk drives", cim("Win32_DiskDrive", "")),
            ("Physical disks", ps(&format!("Get-PhysicalDisk | Select-Object * {CIM_SKIP}"))),
            ("Logical disks", cim("Win32_LogicalDisk", "")),
            ("Optical drives", cim("Win32_CDROMDrive", "")),
        ],
        "network" => vec![
            ("Adapters", cim("Win32_NetworkAdapter", "-Filter 'PhysicalAdapter=True'")),
            ("Configuration", cim("Win32_NetworkAdapterConfiguration", "-Filter 'IPEnabled=True'")),
        ],
        "wifi" => {
            let text = run("netsh", &["wlan", "show", "interfaces"]).unwrap_or_default();
            let props = text
                .lines()
                .filter_map(|l| l.split_once(" : "))
                .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
                .collect();
            return vec![Node { name: "Wi-Fi".into(), props, children: Vec::new() }];
        }
        "usb" => vec![("USB controllers", cim("Win32_USBController", "")), ("USB hubs", cim("Win32_USBHub", ""))],
        "bluetooth" => vec![("Bluetooth", pnp("-Class Bluetooth"))],
        "audio" => vec![("Sound devices", cim("Win32_SoundDevice", ""))],
        "power" => vec![("Batteries", cim("Win32_Battery", "")), ("Portable batteries", cim("Win32_PortableBattery", ""))],
        "printers" => vec![("Printers", cim("Win32_Printer", ""))],
        "cameras" => vec![("Cameras", pnp("-Class Camera,Image"))],
        "pci" => vec![("PCI devices", ps("Get-PnpDevice -PresentOnly | Where-Object InstanceId -like 'PCI*' | Select-Object Class,FriendlyName,Manufacturer,Status,InstanceId"))],
        "software" => vec![(
            "Installed programs",
            ps(r"Get-ItemProperty 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*','HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*' -ErrorAction SilentlyContinue | Where-Object DisplayName | Select-Object DisplayName,DisplayVersion,Publisher,InstallDate,InstallLocation,EstimatedSize | Sort-Object DisplayName"),
        )],
        "drivers" => vec![(
            "Signed drivers",
            ps("Get-CimInstance Win32_PnPSignedDriver | Where-Object DeviceName | Select-Object DeviceName,Manufacturer,DriverVersion,DriverDate,DriverProviderName,InfName | Sort-Object DeviceName"),
        )],
        "services" => vec![("Services", ps("Get-CimInstance Win32_Service | Select-Object Name,DisplayName,State,StartMode,PathName,StartName | Sort-Object DisplayName"))],
        "sensors" => {
            return crate::winsensors::raw_sensors()
                .into_iter()
                .map(|(name, props)| Node { name, props, children: Vec::new() })
                .collect();
        }
        "env" => return env_nodes(),
        _ => Vec::new(),
    };
    sets.into_iter().filter_map(|(name, values)| section(name, objects(&values, "Item"))).collect()
}

#[tauri::command]
pub fn detail_categories() -> Vec<&'static str> {
    categories()
}

#[tauri::command]
pub async fn detail_data(category: String) -> Result<Vec<Node>, String> {
    if !categories().contains(&category.as_str()) {
        return Err("unknown category".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        if cfg!(target_os = "macos") {
            mac_detail(&category)
        } else if cfg!(windows) {
            win_detail(&category)
        } else if category == "env" {
            env_nodes()
        } else {
            Vec::new()
        }
    })
    .await
    .map_err(|e| e.to_string())
}
