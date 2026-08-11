//! Persistent store listing (/sys/fs/pstore).

use std::fs;
use std::path::Path;

use crate::error::AppError;
use crate::model::PstoreEntry;
use crate::sanitize::sanitize_cell;

pub(crate) fn probe_pstore() -> Result<Vec<PstoreEntry>, AppError> {
    let dir = Path::new("/sys/fs/pstore");
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let entries = fs::read_dir(dir).map_err(|e| AppError::Internal(e.to_string()))?;
    for entry in entries.flatten() {
        let meta = entry.metadata().ok();
        let bytes = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        out.push(PstoreEntry {
            name: sanitize_cell(&entry.file_name().to_string_lossy()),
            bytes,
        });
        if out.len() >= 32 {
            break;
        }
    }
    Ok(out)
}
