//! Clock / NTP sync via timedatectl (fixed args).

use crate::actions::trusted::{run_trusted_optional, ExecutionPolicy, TrustedCommand};
use crate::error::AppError;
use crate::model::ClockSyncSnapshot;
use crate::sanitize::sanitize_text;

pub(crate) async fn probe_clock() -> Result<Option<ClockSyncSnapshot>, AppError> {
    // Prefer machine-readable `timedatectl show` over human `status` text.
    if let Ok(Some(output)) = run_trusted_optional(
        TrustedCommand::Timedatectl,
        &["show", "--property=NTPSynchronized", "--value"],
        ExecutionPolicy::timedatectl(),
    )
    .await
    {
        if output.success {
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

    let Some(output) = run_trusted_optional(
        TrustedCommand::Timedatectl,
        &["status"],
        ExecutionPolicy::timedatectl(),
    )
    .await?
    else {
        return Ok(None);
    };
    if !output.success {
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
