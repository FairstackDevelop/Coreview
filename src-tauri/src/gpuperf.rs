use std::collections::HashMap;
use std::time::{Duration, Instant};
use windows::core::{w, PCWSTR};
use windows::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW, PdhRemoveCounter, PDH_FMT_COUNTERVALUE_ITEM_W,
    PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA,
};

pub struct GpuEngines {
    query: PDH_HQUERY,
    counter: Option<PDH_HCOUNTER>,
    added: Instant,
}

unsafe impl Send for GpuEngines {}

impl GpuEngines {
    pub fn new() -> Option<GpuEngines> {
        unsafe {
            let mut query = PDH_HQUERY::default();
            if PdhOpenQueryW(PCWSTR::null(), 0, &mut query) != 0 {
                return None;
            }
            Some(GpuEngines { query, counter: None, added: Instant::now() - Duration::from_secs(60) })
        }
    }

    fn refresh(&mut self) {
        unsafe {
            if let Some(c) = self.counter.take() {
                PdhRemoveCounter(c);
            }
            let mut counter = PDH_HCOUNTER::default();
            if PdhAddEnglishCounterW(self.query, w!("\\GPU Engine(*)\\Utilization Percentage"), 0, &mut counter) == 0 {
                self.counter = Some(counter);
            }
            self.added = Instant::now();
            PdhCollectQueryData(self.query);
        }
        std::thread::sleep(Duration::from_millis(400));
    }

    pub fn read(&mut self) -> Option<f64> {
        if self.counter.is_none() || self.added.elapsed() > Duration::from_secs(6) {
            self.refresh();
        }
        unsafe {
            PdhCollectQueryData(self.query);
            let counter = self.counter?;
            let (mut size, mut count) = (0u32, 0u32);
            if PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, None) != PDH_MORE_DATA || size == 0 {
                return None;
            }
            let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
            let ptr = buffer.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
            if PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, Some(ptr)) != 0 {
                return None;
            }
            let items = std::slice::from_raw_parts(ptr, count as usize);
            let mut per_adapter: HashMap<String, f64> = HashMap::new();
            for item in items {
                if item.FmtValue.CStatus != 0 {
                    continue;
                }
                let Ok(name) = item.szName.to_string() else { continue };
                if !name.contains("engtype_3D") {
                    continue;
                }
                let luid = name.split("_phys").next().and_then(|s| s.split("luid_").nth(1)).unwrap_or("0").to_string();
                *per_adapter.entry(luid).or_default() += item.FmtValue.Anonymous.doubleValue;
            }
            per_adapter.values().cloned().reduce(f64::max).map(|v| v.clamp(0.0, 100.0))
        }
    }
}
