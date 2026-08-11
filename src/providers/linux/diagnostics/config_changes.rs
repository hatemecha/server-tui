//! Recent /etc metadata change listing (info-level).

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::error::AppError;
use crate::model::EtcMetaChange;
use crate::sanitize::sanitize_path_display;

pub(crate) fn probe_etc_meta() -> Result<Vec<EtcMetaChange>, AppError> {
    let etc = Path::new("/etc");
    if !etc.is_dir() {
        return Ok(Vec::new());
    }
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(7 * 24 * 3600))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(etc) else {
        return Ok(out);
    };
    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let Ok(mtime) = meta.modified() else {
            continue;
        };
        if mtime < cutoff {
            continue;
        }
        let kind = if meta.is_dir() { "dir" } else { "file" };
        out.push(EtcMetaChange {
            path: sanitize_path_display(&entry.path()),
            kind: kind.into(),
        });
        if out.len() >= 24 {
            break;
        }
    }
    Ok(out)
}
