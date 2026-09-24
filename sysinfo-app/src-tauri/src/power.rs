use crate::util::*;
use serde::Serialize;

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PowerStats {
    watts: Option<f64>,
    battery_watts: Option<f64>,
    adapter_watts: Option<f64>,
    adapter_rated: Option<u64>,
    on_ac: bool,
    percent: Option<u64>,
    charging: bool,
    minutes_remaining: Option<u64>,
    poll_ms: u64,
    source: String,
}

fn nested_num(text: &str, key: &str) -> Option<i64> {
    let needle = format!("\"{key}\"=");
    let start = text.find(&needle)? + needle.len();
    let rest = &text[start..];
    let end = rest.find(|c: char| !(c.is_ascii_digit() || c == '-')).unwrap_or(rest.len());
    let s = &rest[..end];
    s.parse::<i64>().ok().or_else(|| s.parse::<u64>().ok().map(|v| v as i64))
}

fn mac_power() -> Option<PowerStats> {
    let io = run("ioreg", &["-rn", "AppleSmartBattery"])?;
    let volt = ioreg_num(&io, "Voltage")? as f64;
    let amp = ioreg_num(&io, "InstantAmperage").or_else(|| ioreg_num(&io, "Amperage"))? as i64 as f64;
    let battery_w = amp * volt / 1_000_000.0;
    let on_ac = io.contains("\"ExternalConnected\" = Yes");
    let system = nested_num(&io, "SystemLoad").filter(|v| *v > 0).map(|v| v as f64 / 1000.0);
    let max = ioreg_num(&io, "MaxCapacity").filter(|m| *m > 0);
    Some(PowerStats {
        watts: system.or_else(|| (!on_ac).then_some(-battery_w).filter(|w| *w > 0.0)),
        battery_watts: Some(battery_w),
        adapter_watts: nested_num(&io, "SystemPowerIn").filter(|v| *v > 0).map(|v| v as f64 / 1000.0),
        adapter_rated: nested_num(&io, "Watts").filter(|v| *v > 0).map(|v| v as u64),
        on_ac,
        percent: ioreg_num(&io, "CurrentCapacity").zip(max).map(|(c, m)| (c * 100 / m).min(100)),
        charging: io.contains("\"IsCharging\" = Yes"),
        minutes_remaining: ioreg_num(&io, "TimeRemaining").filter(|m| *m < 60_000),
        poll_ms: 2000,
        source: "AppleSmartBattery".into(),
    })
}

fn win_power() -> Option<PowerStats> {
    let status = ps("Get-CimInstance -Namespace root/wmi -ClassName BatteryStatus | Select-Object DischargeRate,ChargeRate,Charging,Discharging,PowerOnline").into_iter().next()?;
    let battery = ps("Get-CimInstance Win32_Battery | Select-Object EstimatedChargeRemaining,EstimatedRunTime").into_iter().next();
    let discharge = n(&status, "DischargeRate") as f64 / 1000.0;
    let charge = n(&status, "ChargeRate") as f64 / 1000.0;
    let flag = |k: &str| status.get(k).and_then(|v| v.as_bool()).unwrap_or(false);
    Some(PowerStats {
        watts: (discharge > 0.0).then_some(discharge),
        battery_watts: Some(if charge > 0.0 { charge } else { -discharge }),
        on_ac: flag("PowerOnline"),
        charging: flag("Charging"),
        percent: battery.as_ref().map(|b| n(b, "EstimatedChargeRemaining")),
        minutes_remaining: battery.as_ref().map(|b| n(b, "EstimatedRunTime")).filter(|m| *m < 60_000),
        poll_ms: 5000,
        source: "BatteryStatus".into(),
        ..Default::default()
    })
}

#[tauri::command]
pub async fn power_stats() -> Option<PowerStats> {
    tauri::async_runtime::spawn_blocking(|| {
        if cfg!(target_os = "macos") {
            mac_power()
        } else if cfg!(windows) {
            win_power()
        } else {
            None
        }
    })
    .await
    .ok()
    .flatten()
}
