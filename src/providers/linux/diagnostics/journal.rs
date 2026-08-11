//! Journalctl probes (typed args, no shell).

use std::collections::HashMap;
use std::ffi::OsString;

use crate::actions::trusted::{run_trusted_optional, ExecutionPolicy, TrustedCommand};
use crate::error::AppError;
use crate::model::{JournalCriticalGroup, OomEvent};
use crate::sanitize::{sanitize_cell, sanitize_text};

pub struct JournalQuery {
    pub args: Vec<OsString>,
}

impl JournalQuery {
    pub fn critical_recent(lines: usize) -> Self {
        Self {
            args: vec![
                OsString::from("--no-pager"),
                OsString::from("--output=json"),
                OsString::from("-p"),
                OsString::from("0..2"),
                OsString::from("-n"),
                OsString::from(lines.to_string()),
                OsString::from("--boot=0"),
            ],
        }
    }

    pub fn kernel_oom(lines: usize) -> Self {
        Self {
            args: vec![
                OsString::from("--no-pager"),
                OsString::from("--output=cat"),
                OsString::from("-k"),
                OsString::from("-g"),
                OsString::from("oom|Out of memory|Killed process"),
                OsString::from("-n"),
                OsString::from(lines.to_string()),
                OsString::from("--boot=0"),
            ],
        }
    }

    pub fn previous_boot_tail(lines: usize) -> Self {
        Self {
            args: vec![
                OsString::from("--no-pager"),
                OsString::from("--output=short-iso"),
                OsString::from("-b"),
                OsString::from("-1"),
                OsString::from("-n"),
                OsString::from(lines.to_string()),
            ],
        }
    }
}

pub(crate) async fn run_journalctl(query: &JournalQuery) -> Result<String, AppError> {
    let args: Vec<&str> = query.args.iter().filter_map(|a| a.to_str()).collect();
    if args.len() != query.args.len() {
        return Err(AppError::Journal("non-utf8 journalctl args".into()));
    }
    let Some(output) = run_trusted_optional(
        TrustedCommand::Journalctl,
        &args,
        ExecutionPolicy::journalctl(),
    )
    .await?
    else {
        return Err(AppError::Journal("journalctl unavailable".into()));
    };
    if !output.success {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Journal(format!(
            "journalctl exited {:?}: {}",
            output.status_code, err
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub(crate) async fn probe_journal(
) -> Result<(Vec<JournalCriticalGroup>, Vec<OomEvent>, bool), AppError> {
    let raw = match run_journalctl(&JournalQuery::critical_recent(200)).await {
        Ok(s) => s,
        Err(_) => return Ok((Vec::new(), Vec::new(), false)),
    };

    let mut groups: HashMap<String, (usize, String)> = HashMap::new();
    for line in raw.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let unit = v
            .get("_SYSTEMD_UNIT")
            .or_else(|| v.get("SYSLOG_IDENTIFIER"))
            .and_then(|x| x.as_str())
            .unwrap_or("unknown");
        let msg = v
            .get("MESSAGE")
            .map(|m| match m {
                serde_json::Value::String(s) => sanitize_text(s),
                serde_json::Value::Array(arr) => {
                    let bytes: Vec<u8> = arr
                        .iter()
                        .filter_map(|b| b.as_u64().map(|n| n as u8))
                        .collect();
                    sanitize_text(&String::from_utf8_lossy(&bytes))
                }
                _ => String::new(),
            })
            .unwrap_or_default();
        let entry = groups
            .entry(sanitize_cell(unit))
            .or_insert((0, msg.clone()));
        entry.0 += 1;
        if entry.1.is_empty() {
            entry.1 = msg;
        }
    }

    let critical: Vec<JournalCriticalGroup> = groups
        .into_iter()
        .map(|(unit, (count, sample_message))| JournalCriticalGroup {
            unit,
            count,
            sample_message,
        })
        .collect();

    let mut ooms = Vec::new();
    if let Ok(oom_raw) = run_journalctl(&JournalQuery::kernel_oom(40)).await {
        for line in oom_raw.lines().take(20) {
            let msg = sanitize_text(line);
            if msg.is_empty() {
                continue;
            }
            let process = msg
                .split_whitespace()
                .find(|w| w.starts_with("process") || w.contains('('))
                .unwrap_or("unknown")
                .to_string();
            ooms.push(OomEvent {
                process: sanitize_cell(&process),
                message: msg,
            });
        }
    }

    Ok((critical, ooms, true))
}
