//! Typed application errors. UI messages stay human-readable; details go to debug log.

use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum AppError {
    #[error("terminal error: {0}")]
    Terminal(String),

    #[error("configuration error at {path}: {message}")]
    Configuration { path: PathBuf, message: String },

    #[error("metrics unavailable: {0}")]
    Metrics(String),

    #[error("process error: {0}")]
    Process(String),

    #[error("permission denied: {0}")]
    Permission(String),

    #[error("systemd error: {0}")]
    Systemd(String),

    #[error("journal error: {0}")]
    Journal(String),

    #[error("storage error: {0}")]
    Storage(String),

    #[error("not supported: {0}")]
    Unsupported(String),

    #[error("external command failed: {0}")]
    ExternalCommand(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl AppError {
    /// Human-readable UI/CLI text. English is the product language (AGENTS.md).
    pub fn user_message(&self) -> String {
        match self {
            Self::Terminal(m) => format!("Terminal problem: {m}"),
            Self::Configuration { path, message } => {
                format!("Invalid configuration ({}): {message}", path.display())
            }
            Self::Metrics(m) => format!("Could not collect metrics: {m}"),
            Self::Process(m) => format!("Process operation: {m}"),
            Self::Permission(m) => {
                format!("Permission denied: {m}. Continuing in observation mode.")
            }
            Self::Systemd(m) => format!("systemd: {m}"),
            Self::Journal(m) => format!("journald/journalctl: {m}"),
            Self::Storage(m) => format!("Storage: {m}"),
            Self::Unsupported(m) => format!("Unavailable: {m}"),
            Self::ExternalCommand(m) => format!("External command: {m}"),
            Self::Internal(m) => format!("Internal error: {m}"),
        }
    }

    pub fn is_recoverable(&self) -> bool {
        !matches!(self, Self::Terminal(_))
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::Internal(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_messages_are_english() {
        let samples = [
            AppError::Terminal("x".into()).user_message(),
            AppError::Configuration {
                path: PathBuf::from("/tmp/c"),
                message: "bad".into(),
            }
            .user_message(),
            AppError::Metrics("x".into()).user_message(),
            AppError::Process("x".into()).user_message(),
            AppError::Permission("x".into()).user_message(),
            AppError::Storage("x".into()).user_message(),
            AppError::Unsupported("x".into()).user_message(),
            AppError::ExternalCommand("x".into()).user_message(),
            AppError::Internal("x".into()).user_message(),
        ];
        for msg in samples {
            assert!(
                !msg.contains("Problema")
                    && !msg.contains("Permiso")
                    && !msg.contains("Configuración")
                    && !msg.contains("Almacenamiento")
                    && !msg.contains("denegado"),
                "non-English UI message: {msg}"
            );
        }
        assert!(AppError::Permission("x".into())
            .user_message()
            .contains("Permission denied"));
    }
}
