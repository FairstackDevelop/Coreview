use serde::Serialize;

#[derive(Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Fan {
    pub id: u32,
    pub rpm: f64,
    pub min: f64,
    pub max: f64,
    pub name: String,
}

#[derive(Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SmcTemp {
    pub key: String,
    pub group: String,
    pub kind: String,
    pub index: Option<u32>,
    pub celsius: f64,
    pub name: String,
    pub hw: String,
}

#[derive(Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SmcSensors {
    pub fans: Vec<Fan>,
    pub temps: Vec<SmcTemp>,
    pub supported: bool,
}

#[cfg(target_os = "macos")]
mod mac {
    use super::*;
    use std::ffi::{c_char, c_void, CString};

    #[link(name = "IOKit", kind = "framework")]
    extern "C" {
        fn IOServiceMatching(name: *const c_char) -> *mut c_void;
        fn IOServiceGetMatchingService(main_port: u32, matching: *mut c_void) -> u32;
        fn IOServiceOpen(service: u32, task: u32, kind: u32, conn: *mut u32) -> i32;
        fn IOServiceClose(conn: u32) -> i32;
        fn IOObjectRelease(obj: u32) -> i32;
        static mach_task_self_: u32;
        fn IOConnectCallStructMethod(conn: u32, selector: u32, input: *const c_void, input_size: usize, output: *mut c_void, output_size: *mut usize) -> i32;
    }

    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct Version {
        major: u8,
        minor: u8,
        build: u8,
        reserved: u8,
        release: u16,
    }

    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct PLimit {
        version: u16,
        length: u16,
        cpu: u32,
        gpu: u32,
        mem: u32,
    }

    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct KeyInfo {
        size: u32,
        kind: u32,
        attributes: u8,
    }

    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct KeyData {
        key: u32,
        vers: Version,
        plimit: PLimit,
        info: KeyInfo,
        result: u8,
        status: u8,
        cmd: u8,
        data32: u32,
        bytes: [u8; 32],
    }

    const READ_BYTES: u8 = 5;
    const READ_INDEX: u8 = 8;
    const READ_INFO: u8 = 9;

    pub struct Smc(u32);

    impl Smc {
        pub fn open() -> Option<Smc> {
            unsafe {
                let name = CString::new("AppleSMC").ok()?;
                let service = IOServiceGetMatchingService(0, IOServiceMatching(name.as_ptr()));
                if service == 0 {
                    return None;
                }
                let mut conn = 0u32;
                let rc = IOServiceOpen(service, mach_task_self_, 0, &mut conn);
                IOObjectRelease(service);
                (rc == 0).then_some(Smc(conn))
            }
        }

        fn call(&self, input: &KeyData) -> Option<KeyData> {
            let mut out = KeyData::default();
            let mut size = std::mem::size_of::<KeyData>();
            let rc = unsafe {
                IOConnectCallStructMethod(self.0, 2, input as *const _ as *const c_void, std::mem::size_of::<KeyData>(), &mut out as *mut _ as *mut c_void, &mut size)
            };
            (rc == 0 && out.result == 0).then_some(out)
        }

        fn key_code(key: &str) -> u32 {
            key.bytes().fold(0u32, |a, b| (a << 8) | b as u32)
        }

        fn code_key(code: u32) -> String {
            code.to_be_bytes().iter().map(|b| *b as char).collect()
        }

        fn key_at(&self, index: u32) -> Option<String> {
            let out = self.call(&KeyData { cmd: READ_INDEX, data32: index, ..Default::default() })?;
            Some(Self::code_key(out.key))
        }

        fn read(&self, key: &str) -> Option<(String, Vec<u8>)> {
            let code = Self::key_code(key);
            let info = self.call(&KeyData { key: code, cmd: READ_INFO, ..Default::default() })?.info;
            let out = self.call(&KeyData { key: code, cmd: READ_BYTES, info, ..Default::default() })?;
            Some((Self::code_key(info.kind), out.bytes[..(info.size as usize).min(32)].to_vec()))
        }

        pub fn number(&self, key: &str) -> Option<f64> {
            let (kind, b) = self.read(key)?;
            decode(&kind, &b)
        }

        pub fn count(&self) -> u32 {
            self.number("#KEY").unwrap_or(0.0) as u32
        }

        pub fn keys(&self) -> Vec<String> {
            (0..self.count()).filter_map(|i| self.key_at(i)).collect()
        }
    }

    impl Drop for Smc {
        fn drop(&mut self) {
            unsafe {
                IOServiceClose(self.0);
            }
        }
    }

    fn decode(kind: &str, b: &[u8]) -> Option<f64> {
        Some(match (kind, b.len()) {
            ("flt ", 4) => f32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64,
            ("sp78", 2) => i16::from_be_bytes([b[0], b[1]]) as f64 / 256.0,
            ("fpe2", 2) => u16::from_be_bytes([b[0], b[1]]) as f64 / 4.0,
            ("ui8 ", 1) => b[0] as f64,
            ("ui16", 2) => u16::from_be_bytes([b[0], b[1]]) as f64,
            ("ui32", 4) => u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as f64,
            _ => return None,
        })
    }

    fn classify(key: &str) -> (&'static str, &'static str) {
        let k = key.as_bytes();
        let digit = |i: usize| k.get(i).map(|c| c.is_ascii_digit()).unwrap_or(false);
        match key {
            "TCAD" | "TCAG" | "TCXC" | "TCXc" | "TCGC" => ("cpu", "package"),
            "TC0P" | "TC1P" => ("cpu", "proximity"),
            "TC0H" | "TC1H" | "Th0H" | "Th1H" | "Th2H" => ("cpu", "heatsink"),
            "TC0D" | "TC0E" | "TC0F" | "TC1D" => ("cpu", "diode"),
            "TG0P" | "TG1P" => ("gpu", "proximity"),
            "TG0H" | "TG1H" => ("gpu", "heatsink"),
            "TG0D" | "TG1D" | "TGDD" | "TGDE" => ("gpu", "diode"),
            "TM0P" | "TM1P" | "TM0S" | "TM1S" | "TM2S" | "TM3S" | "TM0V" => ("memory", "proximity"),
            "TN0P" | "TN0D" | "TN0H" | "TN1P" | "TP0P" | "TPCD" | "TP0D" | "TP1P" | "TP0T" => ("board", "chipset"),
            "TW0P" => ("board", "wireless"),
            "TH0P" | "TH1P" | "TH2P" | "TH0A" | "TH0B" | "TH0O" | "TH0F" => ("storage", "proximity"),
            "TL0P" | "TL1P" => ("ambient", "lcd"),
            "Ts0P" | "Ts1P" | "Ts0S" | "Ts1S" => ("ambient", "palmrest"),
            "TA0P" | "TA1P" | "TA2P" | "Ta0P" | "TA0V" | "TA1V" => ("ambient", "ambient"),
            "TaLP" | "TaRF" | "TaLC" | "TaRC" | "TaLT" | "TaRT" => ("ambient", "airflow"),
            _ if key.starts_with("TB") && digit(2) => ("battery", ""),
            _ if key.starts_with("TC") && digit(2) && k.get(3) == Some(&b'C') => ("cpu", "core"),
            _ if key.starts_with("Tp") => ("cpu", "perf"),
            _ if key.starts_with("Te") => ("cpu", "eff"),
            _ if key.starts_with("Tg") => ("gpu", "diode"),
            _ if key.starts_with("Tm") => ("memory", "proximity"),
            _ => ("other", ""),
        }
    }

    pub fn read_all() -> SmcSensors {
        let Some(smc) = Smc::open() else { return SmcSensors::default() };
        let mut out = SmcSensors { supported: true, ..Default::default() };
        let fans = smc.number("FNum").unwrap_or(0.0) as u32;
        for i in 0..fans.min(8) {
            let rpm = smc.number(&format!("F{i}Ac")).unwrap_or(0.0);
            out.fans.push(Fan {
                id: i,
                rpm,
                min: smc.number(&format!("F{i}Mn")).unwrap_or(0.0),
                max: smc.number(&format!("F{i}Mx")).unwrap_or(0.0),
                ..Default::default()
            });
        }
        let mut keys: Vec<String> = smc.keys().into_iter().filter(|k| k.starts_with('T')).collect();
        keys.sort();
        for key in keys {
            let Some(v) = smc.number(&key).filter(|v| *v > 1.0 && *v < 130.0) else { continue };
            let (group, kind) = classify(&key);
            out.temps.push(SmcTemp { key, group: group.into(), kind: kind.into(), celsius: v, ..Default::default() });
        }
        for kind in ["perf", "eff", "diode", "core"] {
            for (n, t) in out.temps.iter_mut().filter(|t| t.kind == kind).enumerate() {
                t.index = Some(n as u32 + 1);
            }
        }
        out
    }
}

pub fn read(app: &tauri::AppHandle) -> SmcSensors {
    #[cfg(target_os = "macos")]
    {
        let _ = app;
        mac::read_all()
    }
    #[cfg(not(target_os = "macos"))]
    {
        crate::winsensors::ensure_started(app);
        crate::winsensors::to_smc()
    }
}

#[tauri::command]
pub async fn smc_sensors(app: tauri::AppHandle) -> SmcSensors {
    tauri::async_runtime::spawn_blocking(move || read(&app)).await.unwrap_or_default()
}
