use super::*;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::CryptoProvider;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, Error, SignatureScheme};

#[derive(Debug)]
struct Pinned {
    fingerprint: String,
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for Pinned {
    fn verify_server_cert(&self, end_entity: &CertificateDer<'_>, _: &[CertificateDer<'_>], _: &ServerName<'_>, _: &[u8], _: UnixTime) -> Result<ServerCertVerified, Error> {
        if hex::encode(Sha256::digest(end_entity.as_ref())).eq_ignore_ascii_case(&self.fingerprint) {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(Error::General("fingerprint mismatch".into()))
        }
    }

    fn verify_tls12_signature(&self, message: &[u8], cert: &CertificateDer<'_>, dss: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn verify_tls13_signature(&self, message: &[u8], cert: &CertificateDer<'_>, dss: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider.signature_verification_algorithms.supported_schemes()
    }
}

fn client(fingerprint: &str) -> reqwest::Client {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .unwrap()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(Pinned { fingerprint: fingerprint.to_string(), provider }))
        .with_no_client_auth();
    reqwest::Client::builder().use_preconfigured_tls(config).build().unwrap()
}

struct Mock;

#[async_trait]
impl Provider for Mock {
    async fn info(&self) -> Value {
        json!({ "host": "test-pc" })
    }
    async fn live(&self) -> Value {
        json!({ "stats": { "cpuTotal": 12.5 } })
    }
    async fn processes(&self) -> Value {
        json!([])
    }
    async fn history(&self, from: u64, _to: u64, _points: usize) -> Value {
        json!({ "points": [{ "t": from }], "marks": [] })
    }
    async fn command(&self, action: &str, _pid: Option<u32>) -> Result<String, String> {
        Ok(format!("did {action}"))
    }
}

async fn start() -> (Arc<Shared>, u16, String) {
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let dir = std::env::temp_dir().join(format!("coreview-agent-test-{}", random_hex(4)));
    let shared = Shared::new(Arc::new(Mock), dir);
    let (s, handle) = (shared.clone(), Handle::new());
    tokio::spawn(async move {
        let _ = serve(s, port, handle).await;
    });
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let fp = shared.fingerprint.lock().unwrap().clone();
        if !fp.is_empty() && std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return (shared, port, fp);
        }
    }
    panic!("server did not start");
}

fn url(port: u16, path: &str) -> String {
    format!("https://127.0.0.1:{port}{path}")
}

async fn pair_with(shared: &Arc<Shared>, c: &reqwest::Client, port: u16, code: &str) -> reqwest::Response {
    *shared.pairing.lock().unwrap() = Some(Pairing { code: "12345678".into(), expires: Instant::now() + Duration::from_secs(60) });
    c.post(url(port, "/v1/pair")).json(&json!({ "code": code, "name": "phone" })).send().await.unwrap()
}

#[tokio::test]
async fn pairing_auth_and_commands() {
    let (shared, port, fp) = start().await;
    let c = client(&fp);

    assert_eq!(c.get(url(port, "/v1/info")).send().await.unwrap().status(), 401);

    let bad = pair_with(&shared, &c, port, "00000000").await;
    assert_eq!(bad.status(), 403);

    let ok = pair_with(&shared, &c, port, "12345678").await;
    assert_eq!(ok.status(), 200);
    let body: Value = ok.json().await.unwrap();
    let token = body["token"].as_str().unwrap().to_string();
    assert_eq!(token.len(), 64);

    let reuse = c.post(url(port, "/v1/pair")).json(&json!({ "code": "12345678", "name": "again" })).send().await.unwrap();
    assert_eq!(reuse.status(), 403);

    let info: Value = c.get(url(port, "/v1/info")).bearer_auth(&token).send().await.unwrap().json().await.unwrap();
    assert_eq!(info["host"], "test-pc");
    assert_eq!(info["control"], false);

    let live: Value = c.get(url(port, "/v1/live")).bearer_auth(&token).send().await.unwrap().json().await.unwrap();
    assert_eq!(live["stats"]["cpuTotal"], 12.5);

    let hist: Value = c.get(url(port, "/v1/history?from=100&to=200")).bearer_auth(&token).send().await.unwrap().json().await.unwrap();
    assert_eq!(hist["points"][0]["t"], 100);

    let denied = c.post(url(port, "/v1/command")).bearer_auth(&token).json(&json!({ "action": "sleep" })).send().await.unwrap();
    assert_eq!(denied.status(), 403);

    shared.config.lock().unwrap().allow_control = true;
    let still = c.post(url(port, "/v1/command")).bearer_auth(&token).json(&json!({ "action": "sleep" })).send().await.unwrap();
    assert_eq!(still.status(), 403);

    shared.devices.lock().unwrap()[0].control = true;
    let allowed = c.post(url(port, "/v1/command")).bearer_auth(&token).json(&json!({ "action": "sleep" })).send().await.unwrap();
    assert_eq!(allowed.status(), 200);

    let wrong = c.get(url(port, "/v1/info")).bearer_auth("nope").send().await.unwrap();
    assert_eq!(wrong.status(), 401);

    shared.devices.lock().unwrap().clear();
    let revoked = c.get(url(port, "/v1/info")).bearer_auth(&token).send().await.unwrap();
    assert_eq!(revoked.status(), 401);
}

#[tokio::test]
async fn wrong_fingerprint_is_rejected_and_attempts_are_limited() {
    let (shared, port, fp) = start().await;
    let evil = client(&"0".repeat(64));
    assert!(evil.get(url(port, "/v1/info")).send().await.is_err());

    let c = client(&fp);
    for _ in 0..5 {
        assert_eq!(pair_with(&shared, &c, port, "99999999").await.status(), 403);
    }
    assert_eq!(pair_with(&shared, &c, port, "12345678").await.status(), 429);
}
