use core_foundation_sys::array::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef};
use core_foundation_sys::base::{CFRelease, CFTypeRef};
use core_foundation_sys::dictionary::{CFDictionaryGetValue, CFDictionaryRef, CFMutableDictionaryRef};
use core_foundation_sys::string::{kCFStringEncodingUTF8, CFStringCreateWithCString, CFStringGetCString, CFStringRef};
use std::ffi::{c_void, CString};
use std::sync::Mutex;
use std::time::Instant;

#[link(name = "IOReport", kind = "dylib")]
extern "C" {
    fn IOReportCopyChannelsInGroup(group: CFStringRef, subgroup: CFStringRef, a: u64, b: u64, c: u64) -> CFMutableDictionaryRef;
    fn IOReportCreateSubscription(a: *const c_void, desired: CFMutableDictionaryRef, subbed: *mut CFMutableDictionaryRef, channel_id: u64, b: CFTypeRef) -> *const c_void;
    fn IOReportCreateSamples(sub: *const c_void, subbed: CFMutableDictionaryRef, c: CFTypeRef) -> CFDictionaryRef;
    fn IOReportCreateSamplesDelta(a: CFDictionaryRef, b: CFDictionaryRef, c: CFTypeRef) -> CFDictionaryRef;
    fn IOReportChannelGetChannelName(item: CFDictionaryRef) -> CFStringRef;
    fn IOReportChannelGetUnitLabel(item: CFDictionaryRef) -> CFStringRef;
    fn IOReportSimpleGetIntegerValue(item: CFDictionaryRef, index: i32) -> i64;
}

#[derive(Default, Clone, Copy)]
pub struct Breakdown {
    pub cpu: Option<f64>,
    pub gpu: Option<f64>,
    pub ane: Option<f64>,
    pub dram: Option<f64>,
}

struct State {
    sub: *const c_void,
    subbed: CFMutableDictionaryRef,
    prev: CFDictionaryRef,
    at: Instant,
}

unsafe impl Send for State {}

static STATE: Mutex<Option<State>> = Mutex::new(None);

unsafe fn cfstr(s: &str) -> CFStringRef {
    let c = CString::new(s).unwrap();
    CFStringCreateWithCString(std::ptr::null(), c.as_ptr(), kCFStringEncodingUTF8)
}

unsafe fn rust_string(s: CFStringRef) -> String {
    if s.is_null() {
        return String::new();
    }
    let mut buf = [0i8; 128];
    if CFStringGetCString(s, buf.as_mut_ptr(), 128, kCFStringEncodingUTF8) == 0 {
        return String::new();
    }
    std::ffi::CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned()
}

fn joules(value: i64, unit: &str) -> f64 {
    let scale = match unit.trim() {
        "mJ" => 1e-3,
        "uJ" => 1e-6,
        "nJ" => 1e-9,
        _ => return 0.0,
    };
    value as f64 * scale
}

pub fn channels(delta_secs: f64) -> Vec<(String, f64)> {
    let mut guard = STATE.lock().unwrap();
    unsafe {
        if guard.is_none() {
            let group = cfstr("Energy Model");
            let channels = IOReportCopyChannelsInGroup(group, std::ptr::null(), 0, 0, 0);
            CFRelease(group as CFTypeRef);
            if channels.is_null() {
                return Vec::new();
            }
            let mut subbed: CFMutableDictionaryRef = std::ptr::null_mut();
            let sub = IOReportCreateSubscription(std::ptr::null(), channels, &mut subbed, 0, std::ptr::null());
            if sub.is_null() || subbed.is_null() {
                return Vec::new();
            }
            let prev = IOReportCreateSamples(sub, subbed, std::ptr::null());
            *guard = Some(State { sub, subbed, prev, at: Instant::now() });
            return Vec::new();
        }
        let st = guard.as_mut().unwrap();
        let cur = IOReportCreateSamples(st.sub, st.subbed, std::ptr::null());
        if cur.is_null() || st.prev.is_null() {
            return Vec::new();
        }
        let elapsed = if delta_secs > 0.0 { delta_secs } else { st.at.elapsed().as_secs_f64() }.max(0.05);
        let delta = IOReportCreateSamplesDelta(st.prev, cur, std::ptr::null());
        CFRelease(st.prev as CFTypeRef);
        st.prev = cur;
        st.at = Instant::now();
        if delta.is_null() {
            return Vec::new();
        }
        let key = cfstr("IOReportChannels");
        let array = CFDictionaryGetValue(delta, key as *const c_void) as CFArrayRef;
        CFRelease(key as CFTypeRef);
        let mut out = Vec::new();
        if !array.is_null() {
            for i in 0..CFArrayGetCount(array) {
                let item = CFArrayGetValueAtIndex(array, i) as CFDictionaryRef;
                let name = rust_string(IOReportChannelGetChannelName(item));
                let unit = rust_string(IOReportChannelGetUnitLabel(item));
                let value = IOReportSimpleGetIntegerValue(item, 0);
                out.push((name, joules(value, &unit) / elapsed));
            }
        }
        CFRelease(delta as CFTypeRef);
        out
    }
}

pub fn breakdown() -> Breakdown {
    let list = channels(0.0);
    if list.is_empty() {
        return Breakdown::default();
    }
    let sum = |pred: &dyn Fn(&str) -> bool| {
        let v: Vec<f64> = list.iter().filter(|(n, _)| pred(n)).map(|(_, w)| *w).collect();
        (!v.is_empty()).then(|| v.iter().sum())
    };
    Breakdown {
        cpu: sum(&|n| n == "CPU Energy"),
        gpu: sum(&|n| n == "GPU Energy"),
        ane: sum(&|n| n == "ANE"),
        dram: sum(&|n| n == "DRAM"),
    }
}
