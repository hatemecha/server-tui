//! Boot id + previous-boot journal summary.

use std::fs;

use crate::error::AppError;
use crate::model::PreviousBootSummary;
use crate::providers::linux::diagnostics::journal::{run_journalctl, JournalQuery};
use crate::sanitize::sanitize_cell;

pub(crate) async fn probe_boot_ids(
) -> Result<(Option<String>, Option<PreviousBootSummary>), AppError> {
    let boot = fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .ok()
        .map(|s| sanitize_cell(s.trim()));

    let prev = match run_journalctl(&JournalQuery::previous_boot_tail(40)).await {
        Ok(text) => {
            let lower = text.to_ascii_lowercase();
            let clean = lower.contains("reached target shutdown")
                || lower.contains("powering off")
                || lower.contains("reboot: restarting system")
                || lower.contains("systemd-shutdown");
            // Unclean hint only — never claim kernel panic.
            let unclean_hint = !text.trim().is_empty() && !clean;
            Some(PreviousBootSummary {
                boot_id: "previous".into(),
                unclean_hint,
                note: if unclean_hint {
                    "Previous boot journal ended without an obvious clean shutdown marker".into()
                } else {
                    "Previous boot journal looks consistent with a normal shutdown".into()
                },
            })
        }
        Err(_) => None,
    };

    Ok((boot, prev))
}
