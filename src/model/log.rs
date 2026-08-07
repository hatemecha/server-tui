//! Log entry models and ring buffer helpers.

use std::collections::VecDeque;

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
        let q = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| e.priority <= min_priority)
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
