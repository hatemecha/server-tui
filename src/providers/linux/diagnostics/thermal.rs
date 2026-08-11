//! Thermal zone temperature probes.

use std::fs;

use crate::error::AppError;
use crate::model::TempSnapshot;
use crate::sanitize::sanitize_cell;

pub(crate) fn probe_temperatures() -> Result<Vec<TempSnapshot>, AppError> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir("/sys/class/thermal") else {
        return Ok(out);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path
            .file_name()
            .map(|n| n.to_string_lossy().starts_with("thermal_zone"))
            .unwrap_or(false)
        {
            continue;
        }
        let Ok(raw) = fs::read_to_string(path.join("temp")) else {
            continue;
        };
        let Ok(milli) = raw.trim().parse::<f32>() else {
            continue;
        };
        let label = fs::read_to_string(path.join("type"))
            .map(|s| sanitize_cell(s.trim()))
            .unwrap_or_else(|_| "thermal".into());
        out.push(TempSnapshot {
            label,
            celsius: milli / 1000.0,
        });
        if out.len() >= 8 {
            break;
        }
    }
    Ok(out)
}
