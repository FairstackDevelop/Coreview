use std::time::{Duration, Instant};
use windows::core::PCWSTR;
use windows::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW, PdhRemoveCounter, PDH_FMT_COUNTERVALUE_ITEM_W,
    PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA,
};

pub struct Wildcard {
    query: PDH_HQUERY,
    path: PCWSTR,
    counter: Option<PDH_HCOUNTER>,
    added: Instant,
}

unsafe impl Send for Wildcard {}

impl Wildcard {
    pub fn new(path: PCWSTR) -> Option<Wildcard> {
        unsafe {
            let mut query = PDH_HQUERY::default();
            if PdhOpenQueryW(PCWSTR::null(), 0, &mut query) != 0 {
                return None;
            }
            Some(Wildcard { query, path, counter: None, added: Instant::now() - Duration::from_secs(60) })
        }
    }

    fn refresh(&mut self) {
        unsafe {
            if let Some(c) = self.counter.take() {
                PdhRemoveCounter(c);
            }
            let mut counter = PDH_HCOUNTER::default();
            if PdhAddEnglishCounterW(self.query, self.path, 0, &mut counter) == 0 {
                self.counter = Some(counter);
            }
            self.added = Instant::now();
            PdhCollectQueryData(self.query);
        }
        std::thread::sleep(Duration::from_millis(400));
    }

    pub fn read(&mut self) -> Vec<(String, f64)> {
        if self.counter.is_none() && self.added.elapsed() < Duration::from_secs(10) {
            return Vec::new();
        }
        if self.counter.is_none() || self.added.elapsed() > Duration::from_secs(6) {
            self.refresh();
        }
        let Some(counter) = self.counter else { return Vec::new() };
        unsafe {
            PdhCollectQueryData(self.query);
            let (mut size, mut count) = (0u32, 0u32);
            if PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, None) != PDH_MORE_DATA || size == 0 {
                return Vec::new();
            }
            let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
            let ptr = buffer.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
            if PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, Some(ptr)) != 0 {
                return Vec::new();
            }
            std::slice::from_raw_parts(ptr, count as usize)
                .iter()
                .filter(|item| item.FmtValue.CStatus == 0)
                .filter_map(|item| Some((item.szName.to_string().ok()?, item.FmtValue.Anonymous.doubleValue)))
                .collect()
        }
    }
}

pub fn gpu_load(items: &[(String, f64)]) -> Option<f64> {
    let mut per_adapter: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    for (name, value) in items.iter().filter(|(n, _)| n.contains("engtype_3D")) {
        let luid = name.split("_phys").next().and_then(|s| s.split("luid_").nth(1)).unwrap_or("0").to_string();
        *per_adapter.entry(luid).or_default() += value;
    }
    per_adapter.values().cloned().reduce(f64::max).map(|v| v.clamp(0.0, 100.0))
}

pub fn package_watts(items: &[(String, f64)]) -> Option<f64> {
    items
        .iter()
        .filter(|(n, _)| n.to_uppercase().ends_with("_PKG"))
        .map(|(_, v)| v / 1000.0)
        .filter(|w| *w > 0.0 && *w < 1000.0)
        .reduce(|a, b| a + b)
}
