//! Service action helpers.

use crate::error::AppError;
use crate::model::ServiceActionKind;
use crate::providers::linux::systemd::{unit_looks_safe, LinuxServiceProvider};

pub async fn execute_service_action(unit: &str, action: ServiceActionKind) -> Result<(), AppError> {
    if !unit_looks_safe(unit) {
        return Err(AppError::Systemd("refusing unsafe unit name".into()));
    }
    LinuxServiceProvider::perform_action(unit, action).await
}
