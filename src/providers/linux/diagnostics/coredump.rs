//! Coredumpctl listing (read-only).

use std::path::Path;

use crate::actions::trusted::{run_trusted_optional, ExecutionPolicy, TrustedCommand};
use crate::error::AppError;
use crate::model::CoredumpEntry;
use crate::sanitize::{sanitize_cell, sanitize_path_display};

pub(crate) async fn probe_coredumps() -> Result<Vec<CoredumpEntry>, AppError> {
    let Some(output) = run_trusted_optional(
        TrustedCommand::Coredumpctl,
        &["--no-pager", "--json=short", "-n", "10"],
        ExecutionPolicy::coredumpctl(),
    )
    .await?
    else {
        return Ok(Vec::new());
    };
    if !output.success {
        return Ok(Vec::new());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut out = Vec::new();
    // coredumpctl --json=short may emit a JSON array or line-delimited objects.
    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&text) {
        for v in arr.into_iter().take(10) {
            out.push(parse_coredump_json(&v));
        }
    } else {
        for line in text.lines().take(10) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                out.push(parse_coredump_json(&v));
            }
        }
    }
    Ok(out)
}

fn parse_coredump_json(v: &serde_json::Value) -> CoredumpEntry {
    let exe = v
        .get("exe")
        .or_else(|| v.get("COREDUMP_EXE"))
        .and_then(|x| x.as_str())
        .unwrap_or("unknown");
    let signal = v
        .get("signal")
        .or_else(|| v.get("COREDUMP_SIGNAL"))
        .map(|x| match x {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => "?".into(),
        });
    let timestamp = v
        .get("time")
        .or_else(|| v.get("COREDUMP_TIMESTAMP"))
        .map(|x| x.to_string())
        .unwrap_or_else(|| "?".into());
    CoredumpEntry {
        exe: sanitize_path_display(Path::new(exe)),
        signal: signal.map(|s| sanitize_cell(&s)),
        timestamp: sanitize_cell(&timestamp),
    }
}
