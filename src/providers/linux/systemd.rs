//! systemd integration via D-Bus (zbus).

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use zbus::proxy;
use zbus::zvariant::OwnedObjectPath;
use zbus::Connection;

use crate::error::AppError;
use crate::model::{ServiceActionKind, ServiceInfo};
use crate::providers::ServiceProvider;
use crate::sanitize::{sanitize_cell, sanitize_text};

#[proxy(
    default_service = "org.freedesktop.systemd1",
    interface = "org.freedesktop.systemd1.Manager",
    default_path = "/org/freedesktop/systemd1"
)]
trait SystemdManager {
    #[zbus(name = "ListUnits")]
    async fn list_units(
        &self,
    ) -> zbus::Result<
        Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            OwnedObjectPath,
            u32,
            String,
            OwnedObjectPath,
        )>,
    >;

    async fn start_unit(&self, name: &str, mode: &str) -> zbus::Result<OwnedObjectPath>;
    async fn stop_unit(&self, name: &str, mode: &str) -> zbus::Result<OwnedObjectPath>;
    async fn restart_unit(&self, name: &str, mode: &str) -> zbus::Result<OwnedObjectPath>;
    async fn reload_unit(&self, name: &str, mode: &str) -> zbus::Result<OwnedObjectPath>;

    async fn enable_unit_files(
        &self,
        files: &[&str],
        runtime: bool,
        force: bool,
    ) -> zbus::Result<(bool, Vec<(String, String, String)>)>;

    async fn disable_unit_files(
        &self,
        files: &[&str],
        runtime: bool,
    ) -> zbus::Result<Vec<(String, String, String)>>;

    async fn reload(&self) -> zbus::Result<()>;

    #[zbus(name = "GetUnitFileState")]
    async fn get_unit_file_state(&self, file: &str) -> zbus::Result<String>;
}

#[proxy(
    default_service = "org.freedesktop.systemd1",
    interface = "org.freedesktop.systemd1.Unit"
)]
trait SystemdUnit {
    #[zbus(property)]
    fn fragment_path(&self) -> zbus::Result<String>;
}

pub struct LinuxServiceProvider {
    known_units: Mutex<HashMap<String, ServiceInfo>>,
}

impl Default for LinuxServiceProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxServiceProvider {
    pub fn new() -> Self {
        Self {
            known_units: Mutex::new(HashMap::new()),
        }
    }

    async fn connection() -> Result<Connection, AppError> {
        Connection::system()
            .await
            .map_err(|e| AppError::Systemd(format!("D-Bus system bus: {e}")))
    }

    pub async fn perform_action(unit: &str, action: ServiceActionKind) -> Result<(), AppError> {
        if !unit_looks_safe(unit) {
            return Err(AppError::Systemd("invalid unit name".into()));
        }
        let conn = Self::connection().await?;
        let manager = SystemdManagerProxy::new(&conn)
            .await
            .map_err(|e| AppError::Systemd(e.to_string()))?;

        match action {
            ServiceActionKind::Start => {
                manager
                    .start_unit(unit, "replace")
                    .await
                    .map_err(|e| map_systemd_error(action, unit, e))?;
            }
            ServiceActionKind::Stop => {
                manager
                    .stop_unit(unit, "replace")
                    .await
                    .map_err(|e| map_systemd_error(action, unit, e))?;
            }
            ServiceActionKind::Restart => {
                manager
                    .restart_unit(unit, "replace")
                    .await
                    .map_err(|e| map_systemd_error(action, unit, e))?;
            }
            ServiceActionKind::Reload => {
                manager
                    .reload_unit(unit, "replace")
                    .await
                    .map_err(|e| map_systemd_error(action, unit, e))?;
            }
            ServiceActionKind::Enable => {
                manager
                    .enable_unit_files(&[unit], false, false)
                    .await
                    .map_err(|e| map_systemd_error(action, unit, e))?;
                manager
                    .reload()
                    .await
                    .map_err(|e| map_systemd_error(action, unit, e))?;
            }
            ServiceActionKind::Disable => {
                manager
                    .disable_unit_files(&[unit], false)
                    .await
                    .map_err(|e| map_systemd_error(action, unit, e))?;
                manager
                    .reload()
                    .await
                    .map_err(|e| map_systemd_error(action, unit, e))?;
            }
        }

        Ok(())
    }
}

fn map_systemd_error(action: ServiceActionKind, unit: &str, err: zbus::Error) -> AppError {
    let msg = err.to_string();
    if msg.contains("AccessDenied")
        || msg.contains("InteractiveAuthorizationRequired")
        || msg.contains("permission")
        || msg.contains("Permission")
    {
        AppError::Permission(format!(
            "No fue posible {} {}. Permiso denegado por systemd/Polkit.",
            action.label(),
            unit
        ))
    } else {
        AppError::Systemd(format!("No fue posible {} {unit}: {msg}", action.label()))
    }
}

#[async_trait]
impl ServiceProvider for LinuxServiceProvider {
    async fn list_services(&self) -> Result<Vec<ServiceInfo>, AppError> {
        let conn = Self::connection().await?;
        let manager = SystemdManagerProxy::new(&conn)
            .await
            .map_err(|e| AppError::Systemd(e.to_string()))?;

        let units = manager
            .list_units()
            .await
            .map_err(|e| AppError::Systemd(e.to_string()))?;

        let mut services = Vec::new();
        for (
            name,
            description,
            load_state,
            active_state,
            sub_state,
            _followed,
            path,
            _job_id,
            _job_type,
            _job_path,
        ) in units
        {
            if !name.ends_with(".service") {
                continue;
            }
            let enabled = manager.get_unit_file_state(&name).await.ok().map(|s| {
                matches!(
                    s.as_str(),
                    "enabled" | "enabled-runtime" | "static" | "linked" | "linked-runtime"
                )
            });

            let fragment_path = {
                let unit_proxy = SystemdUnitProxy::builder(&conn)
                    .path(path.clone())
                    .map_err(|e| AppError::Systemd(e.to_string()))?
                    .build()
                    .await
                    .ok();
                match unit_proxy {
                    Some(p) => p.fragment_path().await.ok(),
                    None => None,
                }
            };

            services.push(ServiceInfo {
                unit: sanitize_cell(&name),
                description: sanitize_text(&description),
                load_state: sanitize_cell(&load_state),
                active_state: sanitize_cell(&active_state),
                sub_state: sanitize_cell(&sub_state),
                unit_path: path.to_string(),
                enabled,
                fragment_path: fragment_path.map(|p| sanitize_text(&p)),
            });
        }

        services.sort_by(|a, b| a.unit.cmp(&b.unit));

        if let Ok(mut map) = self.known_units.lock() {
            map.clear();
            for s in &services {
                map.insert(s.unit.clone(), s.clone());
            }
        }

        Ok(services)
    }

    async fn is_available(&self) -> bool {
        Self::connection().await.is_ok()
    }
}

/// Validate that a unit was previously listed (admin path uses this).
pub fn unit_looks_safe(unit: &str) -> bool {
    !unit.is_empty()
        && unit.ends_with(".service")
        && !unit.contains('/')
        && !unit.contains('\0')
        && !unit.contains(' ')
        && unit
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '@' | ':'))
}
