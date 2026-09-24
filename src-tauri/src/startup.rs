use crate::util::*;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StartupItem {
    id: String,
    name: String,
    command: String,
    location: String,
    enabled: bool,
    can_toggle: bool,
    can_remove: bool,
}

const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved";

const WIN_LIST: &str = r#"
$ap='Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved';
$out=@();
$reg=@(
 @{p='HKCU:\Software\Microsoft\Windows\CurrentVersion\Run';a="HKCU:\$ap\Run";s='user'},
 @{p='HKLM:\Software\Microsoft\Windows\CurrentVersion\Run';a="HKLM:\$ap\Run";s='machine'},
 @{p='HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run';a="HKLM:\$ap\Run32";s='machine32'});
foreach($r in $reg){ if(Test-Path $r.p){ $k=Get-Item $r.p; foreach($n in $k.GetValueNames()){ if($n){ $en=$true; try{$b=(Get-ItemProperty $r.a -ErrorAction Stop).$n; if($b -and ($b[0] -band 1)){$en=$false}}catch{}; $out+=[pscustomobject]@{kind='registry';scope=$r.s;name=$n;command=[string]$k.GetValue($n);enabled=$en} } } } };
$fol=@(@{p=[Environment]::GetFolderPath('Startup');a="HKCU:\$ap\StartupFolder";s='user'},@{p=[Environment]::GetFolderPath('CommonStartup');a="HKLM:\$ap\StartupFolder";s='machine'});
foreach($f in $fol){ if($f.p -and (Test-Path $f.p)){ foreach($i in Get-ChildItem -LiteralPath $f.p -File -Force){ if($i.Name -ne 'desktop.ini'){ $en=$true; try{$b=(Get-ItemProperty $f.a -ErrorAction Stop).($i.Name); if($b -and ($b[0] -band 1)){$en=$false}}catch{}; $out+=[pscustomobject]@{kind='folder';scope=$f.s;name=$i.Name;command=$i.FullName;enabled=$en} } } } };
@($out)
"#;

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_default())
}

fn uid() -> String {
    run("id", &["-u"]).map(|s| s.trim().to_string()).unwrap_or_default()
}

fn esc_ps(s: &str) -> String {
    s.replace('\'', "''")
}

fn esc_as(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn run_key(scope: &str) -> &'static str {
    match scope {
        "user" => r"HKCU:\Software\Microsoft\Windows\CurrentVersion\Run",
        "machine" => r"HKLM:\Software\Microsoft\Windows\CurrentVersion\Run",
        _ => r"HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run",
    }
}

fn approved_key(kind: &str, scope: &str) -> String {
    let hive = if scope == "user" { "HKCU" } else { "HKLM" };
    let leaf = match (kind, scope) {
        ("folder", _) => "StartupFolder",
        (_, "machine32") => "Run32",
        _ => "Run",
    };
    format!(r"{hive}:\{APPROVED}\{leaf}")
}

fn mac_disabled() -> HashMap<String, bool> {
    let mut map = HashMap::new();
    let Some(text) = run("launchctl", &["print-disabled", &format!("gui/{}", uid())]) else {
        return map;
    };
    for line in text.lines() {
        if let Some((k, v)) = line.split_once("=>") {
            let label = k.trim().trim_matches('"').to_string();
            let v = v.trim().trim_end_matches(',');
            map.insert(label, v == "disabled" || v == "true");
        }
    }
    map
}

fn plist_label(path: &Path) -> Option<String> {
    run("plutil", &["-extract", "Label", "raw", "-o", "-", &path.to_string_lossy()]).map(|s| s.trim().to_string())
}

fn plist_command(path: &Path) -> String {
    let p = path.to_string_lossy();
    if let Some(Value::Array(a)) = json_cmd("plutil", &["-extract", "ProgramArguments", "json", "-o", "-", &p]) {
        return a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" ");
    }
    run("plutil", &["-extract", "Program", "raw", "-o", "-", &p]).map(|s| s.trim().to_string()).unwrap_or_default()
}

fn mac_dir(dir: PathBuf, prefix: &str, location: &str, editable: bool, disabled: &HashMap<String, bool>, out: &mut Vec<StartupItem>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().map(|x| x != "plist").unwrap_or(true) {
            continue;
        }
        let name = plist_label(&p).unwrap_or_else(|| p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default());
        out.push(StartupItem {
            id: format!("{prefix}|{}", p.to_string_lossy()),
            enabled: !disabled.get(&name).copied().unwrap_or(false),
            command: plist_command(&p),
            name,
            location: location.into(),
            can_toggle: editable,
            can_remove: editable,
        });
    }
}

fn list_items() -> Vec<StartupItem> {
    let mut out = Vec::new();
    if cfg!(target_os = "macos") {
        let disabled = mac_disabled();
        mac_dir(home().join("Library/LaunchAgents"), "agent", "userAgents", true, &disabled, &mut out);
        mac_dir(PathBuf::from("/Library/LaunchAgents"), "global", "globalAgents", false, &disabled, &mut out);
        mac_dir(PathBuf::from("/Library/LaunchDaemons"), "global", "daemons", false, &disabled, &mut out);
        let script = "tell application \"System Events\"\nset o to \"\"\nrepeat with i in every login item\nset o to o & (name of i) & \"||\" & (path of i) & linefeed\nend repeat\nreturn o\nend tell";
        if let Some(text) = run("osascript", &["-e", script]) {
            for line in text.lines() {
                if let Some((name, path)) = line.split_once("||") {
                    out.push(StartupItem {
                        id: format!("login|{name}"),
                        name: name.into(),
                        command: path.into(),
                        location: "loginItems".into(),
                        enabled: true,
                        can_toggle: false,
                        can_remove: true,
                    });
                }
            }
        }
    } else if cfg!(windows) {
        for v in ps(WIN_LIST.trim()) {
            let (kind, scope, name) = (s(&v, "kind"), s(&v, "scope"), s(&v, "name"));
            let location = match (kind.as_str(), scope.as_str()) {
                ("folder", _) => "startupFolder",
                (_, "user") => "regUser",
                _ => "regMachine",
            };
            out.push(StartupItem {
                id: format!("{kind}|{scope}|{name}"),
                command: s(&v, "command"),
                enabled: v.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true),
                can_toggle: true,
                can_remove: true,
                location: location.into(),
                name,
            });
        }
    } else if let Ok(rd) = fs::read_dir(home().join(".config/autostart")) {
        for e in rd.flatten() {
            out.push(StartupItem {
                id: format!("autostart|{}", e.path().to_string_lossy()),
                name: e.file_name().to_string_lossy().into_owned(),
                command: e.path().to_string_lossy().into_owned(),
                location: "autostart".into(),
                enabled: true,
                can_toggle: false,
                can_remove: false,
            });
        }
    }
    out
}

fn find(id: &str) -> Result<StartupItem, String> {
    list_items().into_iter().find(|i| i.id == id).ok_or_else(|| "item not found".to_string())
}

#[tauri::command]
pub async fn startup_items() -> Vec<StartupItem> {
    tauri::async_runtime::spawn_blocking(list_items).await.unwrap_or_default()
}

#[tauri::command]
pub async fn startup_set_enabled(id: String, enabled: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let item = find(&id)?;
        if !item.can_toggle {
            return Err("read-only".into());
        }
        let mut parts = id.splitn(3, '|');
        let (kind, a, b) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""), parts.next().unwrap_or(""));
        if kind == "agent" {
            let target = format!("gui/{}/{}", uid(), item.name);
            run_res("launchctl", &[if enabled { "enable" } else { "disable" }, &target])?;
            if !enabled {
                let _ = run_res("launchctl", &["bootout", &target]);
            }
            return Ok(());
        }
        let script = format!(
            "$a='{}'; if(!(Test-Path $a)){{New-Item $a -Force | Out-Null}}; $b=New-Object byte[] 12; $b[0]={}; Set-ItemProperty -Path $a -Name '{}' -Value $b -Type Binary",
            esc_ps(&approved_key(kind, a)),
            if enabled { 2 } else { 3 },
            esc_ps(b)
        );
        ps_run(&script).map(|_| ())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn startup_remove(id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let item = find(&id)?;
        if !item.can_remove {
            return Err("read-only".into());
        }
        let mut parts = id.splitn(3, '|');
        let (kind, a, b) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""), parts.next().unwrap_or(""));
        match kind {
            "agent" => {
                let path = PathBuf::from(a);
                let _ = run_res("launchctl", &["bootout", &format!("gui/{}/{}", uid(), item.name)]);
                let trash = home().join(".Trash");
                let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                let file = path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
                fs::rename(&path, trash.join(format!("{stamp}-{file}"))).map_err(|e| e.to_string())
            }
            "login" => {
                let script = format!("tell application \"System Events\" to delete login item \"{}\"", esc_as(a));
                run_res("osascript", &["-e", &script]).map(|_| ())
            }
            "registry" => {
                let script = format!(
                    "Remove-ItemProperty -Path '{}' -Name '{}' -ErrorAction Stop; Remove-ItemProperty -Path '{}' -Name '{}' -ErrorAction SilentlyContinue",
                    run_key(a),
                    esc_ps(b),
                    esc_ps(&approved_key(kind, a)),
                    esc_ps(b)
                );
                ps_run(&script).map(|_| ())
            }
            "folder" => ps_run(&format!("Remove-Item -LiteralPath '{}' -Force", esc_ps(&item.command))).map(|_| ()),
            _ => Err("unsupported".into()),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn startup_add(path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let p = Path::new(&path);
        if !p.exists() {
            return Err("file not found".into());
        }
        let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        if cfg!(target_os = "macos") {
            let script = format!(
                "tell application \"System Events\" to make login item at end with properties {{path:\"{}\", hidden:false}}",
                esc_as(&path)
            );
            run_res("osascript", &["-e", &script]).map(|_| ())
        } else if cfg!(windows) {
            ps_run(&format!(
                "Set-ItemProperty -Path '{}' -Name '{}' -Value '\"{}\"'",
                run_key("user"),
                esc_ps(&stem),
                esc_ps(&path)
            ))
            .map(|_| ())
        } else {
            Err("unsupported".into())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
