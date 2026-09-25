use windows::core::w;
use windows::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCollectQueryData, PdhGetFormattedCounterValue, PdhOpenQueryW, PDH_FMT_COUNTERVALUE, PDH_FMT_DOUBLE,
    PDH_HCOUNTER, PDH_HQUERY,
};

pub struct CpuPerf {
    query: PDH_HQUERY,
    utility: PDH_HCOUNTER,
    performance: PDH_HCOUNTER,
    frequency: PDH_HCOUNTER,
}

fn value(counter: PDH_HCOUNTER) -> Option<f64> {
    let mut v = PDH_FMT_COUNTERVALUE::default();
    let rc = unsafe { PdhGetFormattedCounterValue(counter, PDH_FMT_DOUBLE, None, &mut v) };
    (rc == 0 && v.CStatus == 0).then(|| unsafe { v.Anonymous.doubleValue })
}

impl CpuPerf {
    pub fn new() -> Option<CpuPerf> {
        unsafe {
            let mut query = PDH_HQUERY::default();
            if PdhOpenQueryW(windows::core::PCWSTR::null(), 0, &mut query) != 0 {
                return None;
            }
            let add = |path: windows::core::PCWSTR| {
                let mut counter = PDH_HCOUNTER::default();
                (PdhAddEnglishCounterW(query, path, 0, &mut counter) == 0).then_some(counter)
            };
            let utility = add(w!("\\Processor Information(_Total)\\% Processor Utility"))?;
            let performance = add(w!("\\Processor Information(_Total)\\% Processor Performance"))?;
            let frequency = add(w!("\\Processor Information(_Total)\\Processor Frequency"))?;
            PdhCollectQueryData(query);
            Some(CpuPerf { query, utility, performance, frequency })
        }
    }

    pub fn read(&self) -> (Option<f64>, Option<f64>) {
        unsafe {
            PdhCollectQueryData(self.query);
        }
        let mhz = value(self.frequency).zip(value(self.performance)).map(|(f, p)| f * p / 100.0);
        (value(self.utility), mhz)
    }
}

unsafe impl Send for CpuPerf {}
