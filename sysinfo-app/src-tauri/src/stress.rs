use serde::Serialize;
use std::hint::black_box;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed, Ordering::SeqCst};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use sysinfo::System;

#[derive(Default)]
struct Inner {
    kind: String,
    threads: usize,
    duration: u64,
    started: Option<Instant>,
    ended: Option<Instant>,
    reason: String,
    last_ops: u64,
    last_at: Option<Instant>,
    rate: f64,
}

#[derive(Clone)]
pub struct StressState {
    inner: Arc<Mutex<Inner>>,
    stop: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
    ops: Arc<AtomicU64>,
    disk_w: Arc<AtomicU64>,
    disk_r: Arc<AtomicU64>,
}

impl StressState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner::default())),
            stop: Arc::new(AtomicBool::new(false)),
            running: Arc::new(AtomicBool::new(false)),
            ops: Arc::new(AtomicU64::new(0)),
            disk_w: Arc::new(AtomicU64::new(0)),
            disk_r: Arc::new(AtomicU64::new(0)),
        }
    }
}

fn xorshift(x: &mut u64) -> u64 {
    *x ^= *x << 13;
    *x ^= *x >> 7;
    *x ^= *x << 17;
    *x
}

fn cpu_worker(stop: Arc<AtomicBool>, ops: Arc<AtomicU64>, seed: u64) {
    let mut x = seed | 1;
    let mut f = 1.000001f64;
    let mut acc = 0f64;
    while !stop.load(Relaxed) {
        for _ in 0..200_000 {
            let r = xorshift(&mut x).wrapping_mul(0x2545_F491_4F6C_DD1D);
            f = (f * 1.000_000_1 + (r as f64) * 1e-19).sqrt() + f.sin() * 0.5 + 1.0;
            acc += f;
        }
        black_box(acc);
        ops.fetch_add(200_000, Relaxed);
    }
}

fn mem_worker(stop: Arc<AtomicBool>, ops: Arc<AtomicU64>, words: usize, seed: u64) {
    let mut buf = vec![0u64; words];
    for (i, v) in buf.iter_mut().enumerate() {
        *v = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
    let mut x = seed | 1;
    while !stop.load(Relaxed) {
        for _ in 0..1_000_000 {
            let i = (xorshift(&mut x) as usize) % words;
            buf[i] = buf[i].wrapping_add(x) ^ buf[(i * 7 + 1) % words];
        }
        black_box(buf.iter().fold(0u64, |a, v| a.wrapping_add(*v)));
        ops.fetch_add(1_000_000 + words as u64, Relaxed);
    }
}

#[cfg(target_os = "macos")]
fn no_cache(f: &std::fs::File) {
    use std::os::unix::io::AsRawFd;
    unsafe {
        libc::fcntl(f.as_raw_fd(), libc::F_NOCACHE, 1);
    }
}

#[cfg(not(target_os = "macos"))]
fn no_cache(_: &std::fs::File) {}

fn disk_worker(stop: Arc<AtomicBool>, ops: Arc<AtomicU64>, w: Arc<AtomicU64>, r: Arc<AtomicU64>) {
    const CHUNK: usize = 8 * 1024 * 1024;
    const CHUNKS: usize = 64;
    let path = std::env::temp_dir().join(format!("coreview-stress-{}.tmp", std::process::id()));
    let mut x = 0x1234_5678_9ABC_DEF1u64;
    let chunk: Vec<u8> = (0..CHUNK).map(|_| xorshift(&mut x) as u8).collect();
    let mb = |bytes: u64, t: Instant| ((bytes as f64 / 1_048_576.0) / t.elapsed().as_secs_f64().max(0.001) * 100.0) as u64;

    'run: while !stop.load(Relaxed) {
        let Ok(mut file) = std::fs::OpenOptions::new().write(true).create(true).truncate(true).open(&path) else { break };
        no_cache(&file);
        let t = Instant::now();
        let mut written = 0u64;
        for _ in 0..CHUNKS {
            if stop.load(Relaxed) || file.write_all(&chunk).is_err() {
                break 'run;
            }
            written += CHUNK as u64;
            ops.fetch_add(CHUNK as u64 / 1_048_576, Relaxed);
        }
        let _ = file.sync_all();
        w.store(mb(written, t), Relaxed);
        drop(file);

        let Ok(mut file) = std::fs::File::open(&path) else { break };
        no_cache(&file);
        let mut buf = vec![0u8; CHUNK];
        let t = Instant::now();
        let mut read = 0u64;
        while let Ok(n) = file.read(&mut buf) {
            if n == 0 || stop.load(Relaxed) {
                break;
            }
            read += n as u64;
            ops.fetch_add(n as u64 / 1_048_576, Relaxed);
        }
        if read > 0 {
            r.store(mb(read, t), Relaxed);
        }
    }
    let _ = std::fs::remove_file(&path);
}

#[tauri::command]
pub fn stress_start(state: tauri::State<StressState>, kind: String, threads: usize, seconds: u64) -> Result<(), String> {
    if !["cpu", "memory", "disk", "all"].contains(&kind.as_str()) {
        return Err("unknown test".into());
    }
    if state.running.swap(true, SeqCst) {
        return Err("a test is already running".into());
    }
    let threads = threads.clamp(1, 256);
    let seconds = seconds.clamp(5, 7200);
    state.stop.store(false, SeqCst);
    state.ops.store(0, SeqCst);
    state.disk_w.store(0, SeqCst);
    state.disk_r.store(0, SeqCst);
    {
        let mut i = state.inner.lock().unwrap();
        *i = Inner { kind: kind.clone(), threads, duration: seconds, started: Some(Instant::now()), ..Default::default() };
    }

    let mut sys = System::new();
    sys.refresh_memory();
    let mem_threads = if kind == "all" { 2 } else { threads };
    let words = ((sys.total_memory() / 4).min(2 << 30) as usize / mem_threads / 8).max(1 << 20);
    let s = state.inner().clone();

    thread::spawn(move || {
        let mut handles: Vec<JoinHandle<()>> = Vec::new();
        if kind == "cpu" || kind == "all" {
            for i in 0..threads {
                let (st, op) = (s.stop.clone(), s.ops.clone());
                handles.push(thread::spawn(move || cpu_worker(st, op, 0x9E37 + i as u64 * 7919)));
            }
        }
        if kind == "memory" || kind == "all" {
            for i in 0..mem_threads {
                let (st, op) = (s.stop.clone(), s.ops.clone());
                handles.push(thread::spawn(move || mem_worker(st, op, words, 0xABCD + i as u64 * 104_729)));
            }
        }
        if kind == "disk" {
            let (st, op, w, r) = (s.stop.clone(), s.ops.clone(), s.disk_w.clone(), s.disk_r.clone());
            handles.push(thread::spawn(move || disk_worker(st, op, w, r)));
        }

        let started = Instant::now();
        while !s.stop.load(SeqCst) && started.elapsed().as_secs() < seconds {
            thread::sleep(Duration::from_millis(200));
        }
        let natural = !s.stop.load(SeqCst);
        s.stop.store(true, SeqCst);
        for h in handles {
            let _ = h.join();
        }
        let mut i = s.inner.lock().unwrap();
        if i.reason.is_empty() {
            i.reason = if natural { "finished".into() } else { "stopped".into() };
        }
        i.ended = Some(Instant::now());
        s.running.store(false, SeqCst);
    });
    Ok(())
}

#[tauri::command]
pub fn stress_stop(state: tauri::State<StressState>, reason: Option<String>) {
    if state.running.load(SeqCst) {
        let mut i = state.inner.lock().unwrap();
        if i.reason.is_empty() {
            i.reason = reason.filter(|r| r == "overheat").unwrap_or_else(|| "stopped".into());
        }
        state.stop.store(true, SeqCst);
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StressStatus {
    running: bool,
    kind: String,
    threads: usize,
    elapsed_secs: f64,
    duration_secs: u64,
    ops: u64,
    ops_per_sec: f64,
    reason: String,
    disk_write_mbs: f64,
    disk_read_mbs: f64,
}

#[tauri::command]
pub fn stress_status(state: tauri::State<StressState>) -> StressStatus {
    let running = state.running.load(SeqCst);
    let ops = state.ops.load(Relaxed);
    let mut i = state.inner.lock().unwrap();
    let now = Instant::now();
    if running {
        match i.last_at {
            Some(t) if now.duration_since(t).as_secs_f64() < 0.8 => {}
            Some(t) => {
                i.rate = ops.saturating_sub(i.last_ops) as f64 / now.duration_since(t).as_secs_f64();
                i.last_ops = ops;
                i.last_at = Some(now);
            }
            None => {
                i.last_ops = ops;
                i.last_at = Some(now);
            }
        }
    } else {
        i.rate = 0.0;
    }
    let elapsed = i.started.map(|s| i.ended.unwrap_or(now).duration_since(s).as_secs_f64()).unwrap_or(0.0);
    StressStatus {
        running,
        kind: i.kind.clone(),
        threads: i.threads,
        elapsed_secs: elapsed,
        duration_secs: i.duration,
        ops,
        ops_per_sec: i.rate,
        reason: i.reason.clone(),
        disk_write_mbs: state.disk_w.load(Relaxed) as f64 / 100.0,
        disk_read_mbs: state.disk_r.load(Relaxed) as f64 / 100.0,
    }
}
