//! journalctl-backed log provider. Child process only; no shell.

use std::process::Stdio;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

use crate::actions::trusted::{run_trusted, run_trusted_optional, ExecutionPolicy, TrustedCommand};
use crate::error::AppError;
use crate::model::{LogEntry, LogPriority};
use crate::providers::LogProvider;
use crate::sanitize::sanitize_text;

pub struct LinuxLogProvider;

#[derive(Debug, Deserialize)]
struct JournalJson {
    #[serde(rename = "__REALTIME_TIMESTAMP")]
    timestamp: Option<String>,
    #[serde(rename = "_SYSTEMD_UNIT")]
    unit: Option<String>,
    #[serde(rename = "SYSLOG_IDENTIFIER")]
    syslog_id: Option<String>,
    #[serde(rename = "_PID")]
    pid: Option<String>,
    #[serde(rename = "PRIORITY")]
    priority: Option<String>,
    #[serde(rename = "MESSAGE")]
    message: Option<serde_json::Value>,
}

#[async_trait]
impl LogProvider for LinuxLogProvider {
    async fn recent(&self, unit: Option<&str>, lines: usize) -> Result<Vec<LogEntry>, AppError> {
        self.recent_preset(unit, lines, crate::model::LogPreset::All)
            .await
    }

    async fn recent_preset(
        &self,
        unit: Option<&str>,
        lines: usize,
        preset: crate::model::LogPreset,
    ) -> Result<Vec<LogEntry>, AppError> {
        if let Some(u) = unit {
            if !crate::providers::linux::systemd::unit_looks_safe(u) {
                return Err(AppError::Journal("invalid unit for journal query".into()));
            }
        }

        let lines_arg = format!("--lines={lines}");
        let mut args: Vec<&str> = vec!["--no-pager", "--output=json", lines_arg.as_str()];

        let unit_arg;
        match preset {
            crate::model::LogPreset::Important => {
                args.push("-p");
                args.push("0..4");
                args.push("--boot=0");
            }
            crate::model::LogPreset::CurrentBoot => {
                args.push("--boot=0");
            }
            crate::model::LogPreset::LastHour => {
                args.push("--since=-1h");
            }
            crate::model::LogPreset::Kernel => {
                args.push("--dmesg");
            }
            crate::model::LogPreset::SelectedService | crate::model::LogPreset::All => {}
        }

        if let Some(u) = unit {
            unit_arg = format!("--unit={u}");
            args.push(unit_arg.as_str());
        }

        let output = run_trusted(
            TrustedCommand::Journalctl,
            &args,
            ExecutionPolicy::journalctl(),
        )
        .await
        .map_err(|e| match e {
            AppError::Unsupported(m) => AppError::Journal(m),
            other => other,
        })?;

        if !output.success {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::Journal(sanitize_text(&err)));
        }

        Ok(parse_journal_stdout(&output.stdout))
    }

    async fn is_available(&self) -> bool {
        matches!(
            run_trusted_optional(
                TrustedCommand::Journalctl,
                &["--version"],
                ExecutionPolicy::probe()
            )
            .await,
            Ok(Some(o)) if o.success
        )
    }
}

pub fn parse_journal_stdout(bytes: &[u8]) -> Vec<LogEntry> {
    let text = String::from_utf8_lossy(bytes);
    let mut out = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<JournalJson>(line) {
            Ok(j) => out.push(journal_to_entry(j)),
            Err(_) => {
                // Tolerant: skip corrupt lines.
                continue;
            }
        }
    }
    out
}

fn journal_to_entry(j: JournalJson) -> LogEntry {
    let message = match j.message {
        Some(serde_json::Value::String(s)) => sanitize_text(&s),
        Some(serde_json::Value::Array(arr)) => {
            // journald can encode MESSAGE as byte array
            let bytes: Vec<u8> = arr
                .iter()
                .filter_map(|v| v.as_u64().map(|n| n as u8))
                .collect();
            sanitize_text(&String::from_utf8_lossy(&bytes))
        }
        Some(other) => sanitize_text(&other.to_string()),
        None => String::new(),
    };
    let unit = j.unit.or(j.syslog_id).unwrap_or_else(|| "-".into());
    LogEntry {
        timestamp: sanitize_text(&j.timestamp.unwrap_or_else(|| "-".into())),
        unit: sanitize_text(&unit),
        pid: j.pid.and_then(|p| p.parse().ok()),
        priority: LogPriority::from_journal(j.priority.as_deref()),
        message,
    }
}

/// Follow journalctl with cancellation. Caller must cancel and await join.
/// No oneshot timeout — long-running stream with kill_on_drop + cancel.
pub async fn follow_journal(
    unit: Option<String>,
    cancel: CancellationToken,
    tx: tokio::sync::mpsc::Sender<Vec<LogEntry>>,
) -> Result<(), AppError> {
    if let Some(ref u) = unit {
        if !crate::providers::linux::systemd::unit_looks_safe(u) {
            return Err(AppError::Journal("invalid unit for follow".into()));
        }
    }

    let journalctl = TrustedCommand::Journalctl
        .resolve()
        .map_err(|e| AppError::Journal(e.to_string()))?;

    let mut cmd = Command::new(&journalctl);
    cmd.arg("--no-pager")
        .arg("--output=json")
        .arg("--follow")
        .arg("--lines=0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    if let Some(u) = &unit {
        cmd.arg(format!("--unit={u}"));
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| AppError::Journal(format!("follow spawn failed: {e}")))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::Journal("missing stdout".into()))?;
    let mut reader = BufReader::new(stdout).lines();
    let mut batch = Vec::new();

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                return Ok(());
            }
            line = reader.next_line() => {
                match line {
                    Ok(Some(l)) => {
                        if let Ok(j) = serde_json::from_str::<JournalJson>(&l) {
                            batch.push(journal_to_entry(j));
                            if batch.len() >= 16 {
                                // Never block cancel on a full channel.
                                match tx.try_send(std::mem::take(&mut batch)) {
                                    Ok(()) => {}
                                    Err(tokio::sync::mpsc::error::TrySendError::Full(b)) => {
                                        batch = b;
                                    }
                                    Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        tracing::warn!("journal follow read error: {e}");
                        break;
                    }
                }
            }
        }
    }
    if !batch.is_empty() {
        let _ = tx.try_send(batch);
    }
    let _ = child.kill().await;
    let _ = child.wait().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_lines() {
        let raw = br#"{"MESSAGE":"hello","PRIORITY":"3","_SYSTEMD_UNIT":"nginx.service","_PID":"42","__REALTIME_TIMESTAMP":"1"}
{"MESSAGE":[119,111,114,108,100],"PRIORITY":"6","SYSLOG_IDENTIFIER":"kernel"}
not-json
"#;
        let entries = parse_journal_stdout(raw);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].message, "hello");
        assert_eq!(entries[0].priority, LogPriority::Err);
        assert_eq!(entries[1].message, "world");
    }

    #[test]
    fn malformed_around_valid_still_parses() {
        let raw = br#"nope
{"MESSAGE":"ok","PRIORITY":"6"}
{"MESSAGE":
{"MESSAGE":"also","PRIORITY":"4","_SYSTEMD_UNIT":"x.service"}
"#;
        let entries = parse_journal_stdout(raw);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].message, "ok");
        assert_eq!(entries[1].message, "also");
    }

    #[tokio::test]
    async fn follow_cancel_does_not_require_journald() {
        // Invalid unit fails before spawn — exercises cancel/safe argv gate without journald.
        let cancel = CancellationToken::new();
        let (tx, _rx) = tokio::sync::mpsc::channel(4);
        let err = follow_journal(Some("bad unit".into()), cancel, tx)
            .await
            .expect_err("unsafe unit");
        assert!(err.to_string().contains("invalid unit"));
    }
}
