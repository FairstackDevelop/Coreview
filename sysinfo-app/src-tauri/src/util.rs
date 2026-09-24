use serde_json::Value;
use std::process::Command;

pub fn run_res(cmd: &str, args: &[&str]) -> Result<String, String> {
    let mut c = Command::new(cmd);
    c.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    let out = c.output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn run(cmd: &str, args: &[&str]) -> Option<String> {
    run_res(cmd, args).ok()
}

pub fn ps_run(script: &str) -> Result<String, String> {
    run_res("powershell", &["-NoProfile", "-NonInteractive", "-Command", script])
}

pub fn json_cmd(cmd: &str, args: &[&str]) -> Option<Value> {
    serde_json::from_str(&run(cmd, args)?).ok()
}

pub fn ps(script: &str) -> Vec<Value> {
    let full = format!("{script} | ConvertTo-Json -Compress -Depth 4");
    let Some(v) = json_cmd("powershell", &["-NoProfile", "-NonInteractive", "-Command", &full]) else {
        return Vec::new();
    };
    match v {
        Value::Array(a) => a,
        Value::Null => Vec::new(),
        other => vec![other],
    }
}

pub fn s(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(x)) => x.trim().to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

pub fn n(v: &Value, key: &str) -> u64 {
    match v.get(key) {
        Some(Value::Number(x)) => x.as_u64().unwrap_or(0),
        Some(Value::String(x)) => x.trim().parse().unwrap_or(0),
        _ => 0,
    }
}

pub fn ioreg_num(text: &str, key: &str) -> Option<u64> {
    let needle = format!("\"{key}\" = ");
    text.lines()
        .find_map(|l| l.trim().strip_prefix(&needle))
        .and_then(|v| v.trim().parse().ok())
}

pub fn walk<'a>(v: &'a Value, out: &mut Vec<&'a Value>, pred: &dyn Fn(&Value) -> bool) {
    match v {
        Value::Object(m) => {
            if pred(v) {
                out.push(v);
            }
            for x in m.values() {
                walk(x, out, pred);
            }
        }
        Value::Array(a) => a.iter().for_each(|x| walk(x, out, pred)),
        _ => {}
    }
}
