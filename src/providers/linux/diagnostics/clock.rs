//! Clock / NTP sync via timedatectl (fixed args).

use std::process::Stdio;

use tokio::process::Command;

use crate::error::AppError;
use crate::model::ClockSyncSnapshot;
use crate::sanitize::sanitize_text;

pub(crate) async fn probe_clock() -> Result<Option<ClockSyncSnapshot>, AppError> {
    // Prefer machine-readable `timedatectl show` over human `status` text.
    let show = Command::new("timedatectl")
        .args(["show", "--property=NTPSynchronized", "--value"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;
    if let Ok(output) = show {
        if output.status.success() {
            let value = String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_ascii_lowercase();
            let ntp = match value.as_str() {
                "yes" | "true" | "1" => Some(true),
                "no" | "false" | "0" => Some(false),
                _ => None,
            };
            return Ok(Some(ClockSyncSnapshot {
                ntp_synchronized: ntp,
                system_time_status: sanitize_text(&format!("NTPSynchronized={value}")),
            }));
        }
    }

    let output = Command::new("timedatectl")
        .arg("status")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;
    let Ok(output) = output else {
        return Ok(None);
    };
    if !output.status.success() {
        return Ok(None);
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut ntp = None;
    for line in text.lines() {
        let l = line.trim().to_ascii_lowercase();
        if l.contains("system clock synchronized") || l.contains("ntp synchronized") {
            if l.ends_with("yes") {
                ntp = Some(true);
            } else if l.ends_with("no") {
                ntp = Some(false);
            }
        }
    }
    Ok(Some(ClockSyncSnapshot {
        ntp_synchronized: ntp,
        system_time_status: sanitize_text(text.lines().next().unwrap_or("timedatectl status")),
    }))
}
