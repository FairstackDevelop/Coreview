use async_trait::async_trait;
use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use axum_server::{tls_rustls::RustlsConfig, Handle};
use std::net::SocketAddr as Addr;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::Manager;

pub const DEFAULT_PORT: u16 = 47821;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub enabled: bool,
    pub port: u16,
    pub allow_control: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { enabled: false, port: DEFAULT_PORT, allow_control: false }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Device {
    id: String,
    name: String,
    token_hash: String,
    created: u64,
    last_seen: u64,
    control: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    id: String,
    name: String,
    created: u64,
    last_seen: u64,
    control: bool,
}

#[derive(Serialize, Clone)]
pub struct Event {
    t: u64,
    text: String,
}

#[async_trait]
pub trait Provider: Send + Sync + 'static {
    async fn info(&self) -> Value;
    async fn live(&self) -> Value;
    async fn processes(&self) -> Value;
    async fn history(&self, from: u64, to: u64, points: usize) -> Value;
    async fn command(&self, action: &str, pid: Option<u32>) -> Result<String, String>;
}

struct Pairing {
    code: String,
    expires: Instant,
}

pub struct Shared {
    provider: Arc<dyn Provider>,
    dir: PathBuf,
    config: Mutex<Config>,
    devices: Mutex<Vec<Device>>,
    pairing: Mutex<Option<Pairing>>,
    fails: Mutex<(u32, Instant)>,
    events: Mutex<VecDeque<Event>>,
    fingerprint: Mutex<String>,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn random_hex(bytes: usize) -> String {
    let mut out = String::new();
    for _ in 0..bytes {
        out.push_str(&format!("{:02x}", rand::random::<u8>()));
    }
    out
}

fn hash(text: &str) -> String {
    hex::encode(Sha256::digest(text.as_bytes()))
}

impl Shared {
    pub fn new(provider: Arc<dyn Provider>, dir: PathBuf) -> Arc<Shared> {
        let _ = std::fs::create_dir_all(&dir);
        let config = std::fs::read_to_string(dir.join("config.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        let devices = std::fs::read_to_string(dir.join("devices.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        Arc::new(Shared {
            provider,
            dir,
            config: Mutex::new(config),
            devices: Mutex::new(devices),
            pairing: Mutex::new(None),
            fails: Mutex::new((0, Instant::now())),
            events: Mutex::new(VecDeque::new()),
            fingerprint: Mutex::new(String::new()),
        })
    }

    pub fn create_pairing(&self) -> String {
        let code = format!("{:08}", rand::random::<u32>() % 100_000_000);
        *self.pairing.lock().unwrap() = Some(Pairing { code: code.clone(), expires: Instant::now() + Duration::from_secs(300) });
        code
    }

    pub fn fingerprint(&self) -> String {
        self.fingerprint.lock().unwrap().clone()
    }

    pub fn allow_control(&self, allow: bool) {
        self.config.lock().unwrap().allow_control = allow;
    }

    pub fn allow_device(&self, control: bool) {
        for d in self.devices.lock().unwrap().iter_mut() {
            d.control = control;
        }
    }

    fn save_config(&self) {
        if let Ok(text) = serde_json::to_string_pretty(&*self.config.lock().unwrap()) {
            let _ = std::fs::write(self.dir.join("config.json"), text);
        }
    }

    fn save_devices(&self) {
        if let Ok(text) = serde_json::to_string_pretty(&*self.devices.lock().unwrap()) {
            let _ = std::fs::write(self.dir.join("devices.json"), text);
        }
    }

    fn log(&self, text: impl Into<String>) {
        let mut events = self.events.lock().unwrap();
        events.push_front(Event { t: now_ms(), text: text.into() });
        events.truncate(100);
    }

    fn certificate(&self) -> Result<(String, String, String), String> {
        let (cert, key, fp) = (self.dir.join("cert.pem"), self.dir.join("key.pem"), self.dir.join("fingerprint.txt"));
        if let (Ok(c), Ok(k), Ok(f)) = (std::fs::read_to_string(&cert), std::fs::read_to_string(&key), std::fs::read_to_string(&fp)) {
            return Ok((c, k, f.trim().to_string()));
        }
        let generated = rcgen::generate_simple_self_signed(vec!["coreview.local".to_string()]).map_err(|e| e.to_string())?;
        let fingerprint = hex::encode(Sha256::digest(generated.cert.der().as_ref()));
        let (c, k) = (generated.cert.pem(), generated.signing_key.serialize_pem());
        std::fs::write(&cert, &c).map_err(|e| e.to_string())?;
        std::fs::write(&key, &k).map_err(|e| e.to_string())?;
        std::fs::write(&fp, &fingerprint).map_err(|e| e.to_string())?;
        Ok((c, k, fingerprint))
    }
}

type ApiError = (StatusCode, String);

fn authorize(s: &Shared, headers: &HeaderMap) -> Result<Device, ApiError> {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or((StatusCode::UNAUTHORIZED, "missing token".to_string()))?;
    let wanted = hash(token);
    let mut devices = s.devices.lock().unwrap();
    let device = devices.iter_mut().find(|d| d.token_hash == wanted).ok_or((StatusCode::UNAUTHORIZED, "unknown device".to_string()))?;
    device.last_seen = now_ms();
    Ok(device.clone())
}

#[derive(Deserialize)]
struct PairBody {
    code: String,
    name: String,
}

async fn pair(State(s): State<Arc<Shared>>, Json(body): Json<PairBody>) -> Result<Json<Value>, ApiError> {
    {
        let mut fails = s.fails.lock().unwrap();
        if fails.1.elapsed() > Duration::from_secs(60) {
            *fails = (0, Instant::now());
        }
        if fails.0 >= 5 {
            return Err((StatusCode::TOO_MANY_REQUESTS, "too many attempts".into()));
        }
    }
    let valid = {
        let mut pairing = s.pairing.lock().unwrap();
        match pairing.as_ref() {
            Some(p) if p.expires > Instant::now() && p.code == body.code => {
                *pairing = None;
                true
            }
            _ => false,
        }
    };
    if !valid {
        let mut fails = s.fails.lock().unwrap();
        fails.0 += 1;
        fails.1 = Instant::now();
        s.log("pairing failed");
        return Err((StatusCode::FORBIDDEN, "invalid or expired code".into()));
    }
    let token = random_hex(32);
    let name: String = body.name.chars().take(60).collect();
    let device = Device { id: random_hex(6), name: name.clone(), token_hash: hash(&token), created: now_ms(), last_seen: now_ms(), control: false };
    let id = device.id.clone();
    s.devices.lock().unwrap().push(device);
    s.save_devices();
    s.log(format!("paired: {name}"));
    Ok(Json(json!({ "token": token, "deviceId": id, "control": false })))
}

async fn info(State(s): State<Arc<Shared>>, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    let device = authorize(&s, &headers)?;
    let mut value = s.provider.info().await;
    if let Some(map) = value.as_object_mut() {
        map.insert("control".into(), json!(device.control && s.config.lock().unwrap().allow_control));
    }
    Ok(Json(value))
}

async fn live(State(s): State<Arc<Shared>>, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    authorize(&s, &headers)?;
    Ok(Json(s.provider.live().await))
}

async fn processes(State(s): State<Arc<Shared>>, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    authorize(&s, &headers)?;
    Ok(Json(s.provider.processes().await))
}

#[derive(Deserialize)]
struct HistoryQuery {
    from: u64,
    to: u64,
    points: Option<usize>,
}

async fn history(State(s): State<Arc<Shared>>, headers: HeaderMap, Query(q): Query<HistoryQuery>) -> Result<Json<Value>, ApiError> {
    authorize(&s, &headers)?;
    Ok(Json(s.provider.history(q.from, q.to, q.points.unwrap_or(300).clamp(10, 1000)).await))
}

#[derive(Deserialize)]
struct CommandBody {
    action: String,
    pid: Option<u32>,
}

async fn command(State(s): State<Arc<Shared>>, headers: HeaderMap, Json(body): Json<CommandBody>) -> Result<Json<Value>, ApiError> {
    let device = authorize(&s, &headers)?;
    if !s.config.lock().unwrap().allow_control || !device.control {
        s.log(format!("command denied ({}): {}", device.name, body.action));
        return Err((StatusCode::FORBIDDEN, "remote commands are not allowed".into()));
    }
    s.log(format!("command ({}): {}", device.name, body.action));
    match s.provider.command(&body.action, body.pid).await {
        Ok(text) => Ok(Json(json!({ "ok": true, "message": text }))),
        Err(e) => Err((StatusCode::BAD_REQUEST, e)),
    }
}

pub fn router(shared: Arc<Shared>) -> Router {
    Router::new()
        .route("/v1/pair", post(pair))
        .route("/v1/info", get(info))
        .route("/v1/live", get(live))
        .route("/v1/processes", get(processes))
        .route("/v1/history", get(history))
        .route("/v1/command", post(command))
        .with_state(shared)
}

pub async fn serve(shared: Arc<Shared>, port: u16, handle: Handle<Addr>) -> Result<(), String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let (cert, key, fingerprint) = shared.certificate()?;
    *shared.fingerprint.lock().unwrap() = fingerprint;
    let tls = RustlsConfig::from_pem(cert.into_bytes(), key.into_bytes()).await.map_err(|e| e.to_string())?;
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    axum_server::bind_rustls(addr, tls).handle(handle).serve(router(shared).into_make_service()).await.map_err(|e| e.to_string())
}

pub struct AgentState {
    shared: Arc<Shared>,
    handle: Mutex<Option<Handle<Addr>>>,
}

struct AppProvider {
    app: tauri::AppHandle,
}

#[async_trait]
impl Provider for AppProvider {
    async fn info(&self) -> Value {
        json!({
            "host": sysinfo::System::host_name().unwrap_or_default(),
            "os": std::env::consts::OS,
            "agent": env!("CARGO_PKG_VERSION"),
            "hardware": crate::hw::hardware_info().await,
        })
    }

    async fn live(&self) -> Value {
        let stats = crate::live::live_stats(self.app.state()).await.ok();
        json!({
            "t": now_ms(),
            "stats": stats,
            "power": crate::power::power_stats(self.app.clone()).await,
            "smc": crate::smc::smc_sensors(self.app.clone()).await,
            "gpus": crate::winsensors::gpu_stats(self.app.clone()).await,
        })
    }

    async fn processes(&self) -> Value {
        json!(crate::live::processes(self.app.state()).await.unwrap_or_default())
    }

    async fn history(&self, from: u64, to: u64, points: usize) -> Value {
        match crate::history::history_query(self.app.clone(), from, to, points).await {
            Ok(data) => json!(data),
            Err(_) => json!({ "points": [], "marks": [] }),
        }
    }

    async fn command(&self, action: &str, pid: Option<u32>) -> Result<String, String> {
        match action {
            "kill" => {
                let pid = pid.ok_or("pid required")?;
                let done = crate::live::kill_process(pid, self.app.state()).await?;
                if done { Ok("process ended".into()) } else { Err("could not end the process".into()) }
            }
            "sleep" | "restart" | "shutdown" | "lock" => tauri::async_runtime::spawn_blocking({
                let action = action.to_string();
                move || power_action(&action)
            })
            .await
            .map_err(|e| e.to_string())?,
            _ => Err("unknown action".into()),
        }
    }
}

fn power_action(action: &str) -> Result<String, String> {
    use crate::util::run_res;
    let result = if cfg!(windows) {
        match action {
            "shutdown" => run_res("shutdown", &["/s", "/t", "15"]),
            "restart" => run_res("shutdown", &["/r", "/t", "15"]),
            "sleep" => run_res("rundll32.exe", &["powrprof.dll,SetSuspendState", "0,1,0"]),
            _ => run_res("rundll32.exe", &["user32.dll,LockWorkStation"]),
        }
    } else if cfg!(target_os = "macos") {
        match action {
            "shutdown" => run_res("osascript", &["-e", "tell application \"System Events\" to shut down"]),
            "restart" => run_res("osascript", &["-e", "tell application \"System Events\" to restart"]),
            "sleep" => run_res("pmset", &["sleepnow"]),
            _ => run_res("pmset", &["displaysleepnow"]),
        }
    } else {
        Err("not supported on this system".into())
    };
    result.map(|_| format!("{action} started"))
}

fn private_addresses() -> Vec<String> {
    let mut list: Vec<String> = local_ip_address::list_afinet_netifas()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(_, ip)| match ip {
            std::net::IpAddr::V4(v4) if v4.is_private() && !v4.is_loopback() => Some(v4.to_string()),
            _ => None,
        })
        .collect();
    list.sort();
    list.dedup();
    list
}

fn encode(text: &str) -> String {
    text.bytes()
        .map(|b| if b.is_ascii_alphanumeric() || b"-_.,".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect()
}

impl AgentState {
    pub fn new(app: tauri::AppHandle) -> AgentState {
        let dir = app.path().app_data_dir().unwrap_or_else(|_| std::env::temp_dir()).join("agent");
        AgentState { shared: Shared::new(Arc::new(AppProvider { app }), dir), handle: Mutex::new(None) }
    }

    fn start(&self) {
        let mut slot = self.handle.lock().unwrap();
        if slot.is_some() {
            return;
        }
        let handle = Handle::new();
        *slot = Some(handle.clone());
        let shared = self.shared.clone();
        let port = shared.config.lock().unwrap().port;
        tauri::async_runtime::spawn(async move {
            shared.log(format!("agent started on port {port}"));
            if let Err(e) = serve(shared.clone(), port, handle).await {
                shared.log(format!("agent error: {e}"));
            }
        });
    }

    fn stop(&self) {
        if let Some(handle) = self.handle.lock().unwrap().take() {
            handle.graceful_shutdown(Some(Duration::from_secs(2)));
            self.shared.log("agent stopped");
        }
    }

    pub fn autostart(&self) {
        if self.shared.config.lock().unwrap().enabled {
            self.start();
        }
    }

    pub fn shutdown(&self) {
        if let Some(handle) = self.handle.lock().unwrap().take() {
            handle.shutdown();
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatus {
    enabled: bool,
    running: bool,
    port: u16,
    allow_control: bool,
    fingerprint: String,
    addresses: Vec<String>,
    devices: Vec<DeviceInfo>,
    events: Vec<Event>,
}

fn status_of(state: &AgentState) -> AgentStatus {
    let shared = &state.shared;
    let config = shared.config.lock().unwrap().clone();
    let fingerprint = {
        let known = shared.fingerprint.lock().unwrap().clone();
        if known.is_empty() { std::fs::read_to_string(shared.dir.join("fingerprint.txt")).unwrap_or_default().trim().to_string() } else { known }
    };
    AgentStatus {
        enabled: config.enabled,
        running: state.handle.lock().unwrap().is_some(),
        port: config.port,
        allow_control: config.allow_control,
        fingerprint,
        addresses: private_addresses(),
        devices: shared
            .devices
            .lock()
            .unwrap()
            .iter()
            .map(|d| DeviceInfo { id: d.id.clone(), name: d.name.clone(), created: d.created, last_seen: d.last_seen, control: d.control })
            .collect(),
        events: shared.events.lock().unwrap().iter().cloned().collect(),
    }
}

#[tauri::command]
pub fn agent_status(state: tauri::State<AgentState>) -> AgentStatus {
    status_of(&state)
}

#[tauri::command]
pub fn agent_set_enabled(state: tauri::State<AgentState>, enabled: bool) -> AgentStatus {
    state.shared.config.lock().unwrap().enabled = enabled;
    state.shared.save_config();
    if enabled {
        state.start();
    } else {
        state.stop();
    }
    status_of(&state)
}

#[tauri::command]
pub fn agent_set_options(state: tauri::State<AgentState>, port: u16, allow_control: bool) -> AgentStatus {
    let restart = {
        let mut config = state.shared.config.lock().unwrap();
        let changed = config.port != port.max(1024);
        config.port = port.max(1024);
        config.allow_control = allow_control;
        changed && config.enabled
    };
    state.shared.save_config();
    if restart {
        state.stop();
        state.start();
    }
    status_of(&state)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingInfo {
    link: String,
    code: String,
    expires_in: u64,
}

#[tauri::command]
pub fn agent_new_pairing(state: tauri::State<AgentState>) -> Result<PairingInfo, String> {
    let status = status_of(&state);
    if !status.running {
        return Err("agent is not running".into());
    }
    let code = state.shared.create_pairing();
    let host = sysinfo::System::host_name().unwrap_or_default();
    let link = format!("coreview://pair?h={}&p={}&f={}&c={}&n={}", status.addresses.join(","), status.port, status.fingerprint, code, encode(&host));
    Ok(PairingInfo { link, code, expires_in: 300 })
}

#[tauri::command]
pub fn agent_revoke(state: tauri::State<AgentState>, id: String) -> AgentStatus {
    state.shared.devices.lock().unwrap().retain(|d| d.id != id);
    state.shared.save_devices();
    state.shared.log("device removed");
    status_of(&state)
}

#[tauri::command]
pub fn agent_set_device_control(state: tauri::State<AgentState>, id: String, control: bool) -> AgentStatus {
    if let Some(d) = state.shared.devices.lock().unwrap().iter_mut().find(|d| d.id == id) {
        d.control = control;
    }
    state.shared.save_devices();
    status_of(&state)
}

#[cfg(test)]
mod tests;
