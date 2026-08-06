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
    pub fn user_message(&self) -> String {
        match self {
            Self::Terminal(m) => format!("Problema con la terminal: {m}"),
            Self::Configuration { path, message } => {
                format!("Configuración inválida ({}): {message}", path.display())
            }
            Self::Metrics(m) => format!("No se pudieron obtener métricas: {m}"),
            Self::Process(m) => format!("Operación sobre proceso: {m}"),
            Self::Permission(m) => {
                format!("Permiso denegado: {m}. La aplicación continúa en modo de observación.")
            }
            Self::Systemd(m) => format!("systemd: {m}"),
            Self::Journal(m) => format!("journald/journalctl: {m}"),
            Self::Storage(m) => format!("Almacenamiento: {m}"),
            Self::Unsupported(m) => format!("No disponible: {m}"),
            Self::ExternalCommand(m) => format!("Comando externo: {m}"),
            Self::Internal(m) => format!("Error interno: {m}"),
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
