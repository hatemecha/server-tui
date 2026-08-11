//! Log entry models and ring buffer helpers.

use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogPreset {
    #[default]
    Important,
    CurrentBoot,
    LastHour,
    Kernel,
    SelectedService,
    All,
}

impl LogPreset {
    pub fn next(self) -> Self {
        match self {
            Self::Important => Self::CurrentBoot,
            Self::CurrentBoot => Self::LastHour,
            Self::LastHour => Self::Kernel,
            Self::Kernel => Self::SelectedService,
            Self::SelectedService => Self::All,
            Self::All => Self::Important,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Important => Self::All,
            Self::CurrentBoot => Self::Important,
            Self::LastHour => Self::CurrentBoot,
            Self::Kernel => Self::LastHour,
            Self::SelectedService => Self::Kernel,
            Self::All => Self::SelectedService,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Important => "important",
            Self::CurrentBoot => "boot",
            Self::LastHour => "1h",
            Self::Kernel => "kernel",
            Self::SelectedService => "service",
            Self::All => "all",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogPriority {
    Emerg = 0,
    Alert = 1,
    Crit = 2,
    Err = 3,
    Warning = 4,
    Notice = 5,
    Info = 6,
    Debug = 7,
}

impl LogPriority {
    pub fn from_journal(value: Option<&str>) -> Self {
        match value.and_then(|v| v.parse::<u8>().ok()) {
            Some(0) => Self::Emerg,
            Some(1) => Self::Alert,
            Some(2) => Self::Crit,
            Some(3) => Self::Err,
            Some(4) => Self::Warning,
            Some(5) => Self::Notice,
            Some(6) => Self::Info,
            Some(7) => Self::Debug,
            _ => Self::Info,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Emerg => "emerg",
            Self::Alert => "alert",
            Self::Crit => "crit",
            Self::Err => "err",
            Self::Warning => "warn",
            Self::Notice => "notice",
            Self::Info => "info",
            Self::Debug => "debug",
        }
    }

    pub fn next_minimum(self) -> Self {
        match self {
            Self::Emerg => Self::Alert,
            Self::Alert => Self::Crit,
            Self::Crit => Self::Err,
            Self::Err => Self::Warning,
            Self::Warning => Self::Notice,
            Self::Notice => Self::Info,
            Self::Info => Self::Debug,
            Self::Debug => Self::Emerg,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub unit: String,
    pub pid: Option<u32>,
    pub priority: LogPriority,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct LogBuffer {
    capacity: usize,
    entries: VecDeque<LogEntry>,
}

impl LogBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            entries: VecDeque::new(),
        }
    }

    pub fn push(&mut self, entry: LogEntry) {
        if self.entries.len() >= self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    pub fn extend(&mut self, entries: impl IntoIterator<Item = LogEntry>) {
        for e in entries {
            self.push(e);
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &LogEntry> {
        self.entries.iter()
    }

    pub fn filtered<'a>(&'a self, query: &str, min_priority: LogPriority) -> Vec<&'a LogEntry> {
        self.filtered_preset(query, min_priority, LogPreset::All, None)
    }

    pub fn filtered_preset<'a>(
        &'a self,
        query: &str,
        min_priority: LogPriority,
        preset: LogPreset,
        selected_unit: Option<&str>,
    ) -> Vec<&'a LogEntry> {
        let q = query.to_lowercase();
        let min = match preset {
            LogPreset::Important => min_priority.min(LogPriority::Warning),
            LogPreset::All | LogPreset::CurrentBoot | LogPreset::LastHour => min_priority,
            LogPreset::Kernel => min_priority,
            LogPreset::SelectedService => min_priority,
        };
        let now_us = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros() as u64)
            .unwrap_or(0);
        let hour_ago = now_us.saturating_sub(3_600_000_000);
        self.entries
            .iter()
            .filter(|e| e.priority <= min)
            .filter(|e| match preset {
                LogPreset::Important => e.priority <= LogPriority::Warning,
                LogPreset::Kernel => {
                    e.unit.to_lowercase().contains("kernel")
                        || e.message.to_lowercase().contains("kernel")
                }
                LogPreset::SelectedService => selected_unit
                    .map(|u| e.unit.eq_ignore_ascii_case(u))
                    .unwrap_or(true),
                LogPreset::LastHour => match e.timestamp.parse::<u64>() {
                    Ok(ts) => {
                        // journal µs, or seconds → µs when clearly epoch-seconds.
                        let us = if ts > 1_000_000_000_000 {
                            ts
                        } else {
                            ts.saturating_mul(1_000_000)
                        };
                        us >= hour_ago
                    }
                    Err(_) => true,
                },
                LogPreset::CurrentBoot | LogPreset::All => true,
            })
            .filter(|e| {
                if q.is_empty() {
                    true
                } else {
                    e.message.to_lowercase().contains(&q)
                        || e.unit.to_lowercase().contains(&q)
                        || e.priority.label().contains(q.as_str())
                        || e.timestamp.to_lowercase().contains(&q)
                        || e.pid.map(|p| p.to_string().contains(&q)).unwrap_or(false)
                }
            })
            .collect()
    }

    pub fn context_around(
        &self,
        idx: usize,
        before: usize,
        after: usize,
    ) -> Option<(Vec<LogEntry>, LogEntry, Vec<LogEntry>)> {
        let entries: Vec<_> = self.entries.iter().cloned().collect();
        let focus = entries.get(idx)?.clone();
        let start = idx.saturating_sub(before);
        let before_v = entries[start..idx].to_vec();
        let end = (idx + 1 + after).min(entries.len());
        let after_v = entries[idx + 1..end].to_vec();
        Some((before_v, focus, after_v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respects_capacity() {
        let mut buf = LogBuffer::new(2);
        for i in 0..5 {
            buf.push(LogEntry {
                timestamp: format!("{i}"),
                unit: "u".into(),
                pid: None,
                priority: LogPriority::Info,
                message: format!("m{i}"),
            });
        }
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn filters_by_priority_and_query() {
        let mut buf = LogBuffer::new(10);
        buf.push(LogEntry {
            timestamp: "t".into(),
            unit: "nginx.service".into(),
            pid: Some(1),
            priority: LogPriority::Err,
            message: "failure".into(),
        });
        buf.push(LogEntry {
            timestamp: "t".into(),
            unit: "ssh.service".into(),
            pid: None,
            priority: LogPriority::Debug,
            message: "ok".into(),
        });
        assert_eq!(buf.filtered("", LogPriority::Err).len(), 1);
        assert_eq!(buf.filtered("nginx", LogPriority::Debug).len(), 1);
    }
}
