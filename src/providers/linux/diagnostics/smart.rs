//! Read-only SMART via smartctl (-H -A -l error). Never runs self-tests.

use std::fs;
use std::path::Path;
use std::process::Stdio;

use tokio::process::Command;

use crate::error::AppError;
use crate::model::SmartDiskSnapshot;
use crate::sanitize::{sanitize_path_display, sanitize_text};

pub(crate) async fn probe_smart_readonly() -> Result<Vec<SmartDiskSnapshot>, AppError> {
    let smartctl = match crate::actions::trusted::resolve_trusted("smartctl") {
        Ok(p) => p,
        Err(_) => return Ok(Vec::new()), // soft: smartctl optional
    };
    let mut disks = Vec::new();
    // Conservative device enumeration: only common block names under /dev.
    let candidates = list_smart_device_candidates();
    for dev in candidates.into_iter().take(8) {
        let output = Command::new(&smartctl)
            .arg("-H")
            .arg("-A")
            .arg("-l")
            .arg("error")
            .arg(&dev)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;
        let Ok(out) = output else {
            continue;
        };
        let stdout = String::from_utf8_lossy(&out.stdout);
        let passed = if stdout.to_ascii_lowercase().contains("result: passed") {
            Some(true)
        } else if stdout.to_ascii_lowercase().contains("result: failed") {
            Some(false)
        } else {
            None
        };
        let crc = parse_uda_crc(&stdout);
        let device_key = sanitize_path_display(Path::new(&dev));
        // Providers never write state.toml — runtime merges CRC observations.
        disks.push(SmartDiskSnapshot {
            device: device_key.clone(),
            available: true,
            passed,
            uda_crc_error_count: crc,
            prev_uda_crc_error_count: None,
            summary: if passed == Some(false) {
                "SMART FAILED".into()
            } else {
                "SMART probed (read-only)".into()
            },
            details: stdout.lines().take(20).map(sanitize_text).collect(),
        });
    }
    Ok(disks)
}

fn list_smart_device_candidates() -> Vec<String> {
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir("/dev") else {
        return out;
    };
    for ent in rd.flatten() {
        let name = ent.file_name().to_string_lossy().into_owned();
        // sdX / nvmeXn1 only — never partitions like sda1.
        let ok = (name.starts_with("sd") && name.len() == 3)
            || (name.starts_with("nvme") && name.ends_with("n1") && !name.contains('p'));
        if ok {
            out.push(format!("/dev/{name}"));
        }
    }
    out.sort();
    out
}

fn parse_uda_crc(attrs: &str) -> Option<u64> {
    for line in attrs.lines() {
        if line.contains("UDMA_CRC_Error_Count") || line.contains("CRC_Error_Count") {
            let parts: Vec<_> = line.split_whitespace().collect();
            if let Some(last) = parts.last() {
                if let Ok(v) = last.parse::<u64>() {
                    return Some(v);
                }
            }
        }
    }
    None
}
