use async_trait::async_trait;
use axum_server::Handle;
use serde_json::{json, Value};
use std::sync::Arc;
use sysinfo_app_lib::agent::{serve, Provider, Shared};

struct Demo;

#[async_trait]
impl Provider for Demo {
    async fn info(&self) -> Value {
        json!({ "host": "demo-pc", "os": "test", "agent": "demo", "hardware": { "cpu": { "brand": "Demo CPU" } } })
    }
    async fn live(&self) -> Value {
        json!({ "t": 1, "stats": { "cpuTotal": 42.0 }, "power": null, "smc": null, "gpus": [] })
    }
    async fn processes(&self) -> Value {
        json!([{ "pid": 1, "name": "demo", "cpu": 1.0, "memory": 10 }])
    }
    async fn history(&self, from: u64, _to: u64, _points: usize) -> Value {
        json!({ "points": [{ "t": from, "cpu": 5.0 }], "marks": [] })
    }
    async fn command(&self, action: &str, _pid: Option<u32>) -> Result<String, String> {
        Ok(format!("demo did {action}"))
    }
}

#[tokio::main]
async fn main() {
    let port: u16 = std::env::args().nth(1).and_then(|p| p.parse().ok()).unwrap_or(47999);
    let dir = std::env::temp_dir().join(format!("coreview-demo-{port}"));
    let _ = std::fs::remove_dir_all(&dir);
    let shared = Shared::new(Arc::new(Demo), dir);
    let s = shared.clone();
    tokio::spawn(async move {
        let _ = serve(s, port, Handle::new()).await;
    });
    while shared.fingerprint().is_empty() {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    shared.allow_control(true);
    let code = shared.create_pairing();
    println!("LINK coreview://pair?h=127.0.0.1&p={port}&f={}&c={code}&n=demo-pc", shared.fingerprint());
    tokio::time::sleep(std::time::Duration::from_secs(120)).await;
}
