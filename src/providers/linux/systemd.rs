//! systemd integration via D-Bus (zbus).

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use async_trait::async_trait;
use zbus::proxy;
use zbus::zvariant::OwnedObjectPath;
use zbus::Connection;

use crate::error::AppError;
use crate::model::{ServiceActionKind, ServiceInfo, UnitFileState, UnitRegistry};
use crate::providers::ServiceProvider;
use crate::sanitize::{sanitize_cell, sanitize_path_display, sanitize_text};

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

    /// Bulk enablement states — avoids N× GetUnitFileState.
    #[zbus(name = "ListUnitFiles")]
    async fn list_unit_files(&self) -> zbus::Result<Vec<(String, String)>>;

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

    #[zbus(name = "GetUnit")]
    async fn get_unit(&self, name: &str) -> zbus::Result<OwnedObjectPath>;
}

#[proxy(
    default_service = "org.freedesktop.systemd1",
    interface = "org.freedesktop.systemd1.Unit"
)]
trait SystemdUnit {
    #[zbus(property)]
    fn fragment_path(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn description(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn load_state(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn active_state(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn sub_state(&self) -> zbus::Result<String>;
}

pub struct LinuxServiceProvider {
    registry: UnitRegistry,
    /// Optional fragment_path cache filled only by details(); TTL + list refresh invalidate.
    details_cache: Mutex<HashMap<String, (std::time::Instant, ServiceInfo)>>,
}

impl LinuxServiceProvider {
    pub fn new(registry: UnitRegistry) -> Self {
        Self {
            registry,
            details_cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn invalidate_details(&self, unit: Option<&str>) {
        if let Ok(mut cache) = self.details_cache.lock() {
            match unit {
                Some(u) => {
                    cache.remove(u);
                }
                None => cache.clear(),
            }
        }
    }

    pub fn registry(&self) -> &UnitRegistry {
        &self.registry
    }

    async fn connection() -> Result<Connection, AppError> {
        Connection::system()
            .await
            .map_err(|e| AppError::Systemd(format!("D-Bus system bus: {e}")))
    }

    pub async fn perform_action(
        unit: &str,
        action: ServiceActionKind,
        registry: Option<&UnitRegistry>,
    ) -> Result<(), AppError> {
        if !unit_looks_safe(unit) {
            return Err(AppError::Systemd("invalid unit name".into()));
        }
        if let Some(reg) = registry {
            // Empty registry means list has not completed yet — refuse rather than guess.
            if reg.is_empty() || !reg.contains(unit) {
                return Err(AppError::Systemd(format!(
                    "refusing action on unregistered unit {unit}"
                )));
            }
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
    // Prefer accurate systemd/D-Bus policy wording unless Polkit is explicitly indicated.
    let lower = msg.to_ascii_lowercase();
    let polkit_evidence = lower.contains("polkit")
        || lower.contains("org.freedesktop.policykit")
        || msg.contains("InteractiveAuthorizationRequired");
    if msg.contains("AccessDenied")
        || msg.contains("InteractiveAuthorizationRequired")
        || lower.contains("permission")
    {
        if polkit_evidence {
            AppError::Permission(format!(
                "No fue posible {} {}. Permiso denegado (systemd/D-Bus; evidencia Polkit en el error).",
                action.label(),
                unit
            ))
        } else {
            AppError::Permission(format!(
                "No fue posible {} {}. Permiso denegado por política systemd/D-Bus.",
                action.label(),
                unit
            ))
        }
    } else {
        AppError::Systemd(format!("No fue posible {} {unit}: {msg}", action.label()))
    }
}

fn unit_file_basename(path_or_name: &str) -> String {
    Path::new(path_or_name)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path_or_name.to_string())
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

        // One bulk call for enablement states.
        let mut file_states: HashMap<String, UnitFileState> = HashMap::new();
        if let Ok(files) = manager.list_unit_files().await {
            for (path, state) in files {
                let name = unit_file_basename(&path);
                if name.ends_with(".service") {
                    file_states.insert(name, UnitFileState::parse(&state));
                }
            }
        }

        let mut services = Vec::new();
        let mut registered = Vec::new();
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
            let unit_file_state = file_states
                .get(&name)
                .copied()
                .unwrap_or(UnitFileState::Unknown);

            let unit = sanitize_cell(&name);
            registered.push(unit.clone());
            services.push(ServiceInfo {
                unit,
                description: sanitize_text(&description),
                load_state: sanitize_cell(&load_state),
                active_state: sanitize_cell(&active_state),
                sub_state: sanitize_cell(&sub_state),
                unit_path: path.to_string(),
                unit_file_state,
                // Fragment path is fetched on demand via details().
                fragment_path: None,
            });
        }

        services.sort_by(|a, b| a.unit.cmp(&b.unit));
        self.registry.replace_all(registered);
        // List refresh is a natural invalidation point for detail snapshots.
        self.invalidate_details(None);

        Ok(services)
    }

    async fn details(&self, unit: &str) -> Result<ServiceInfo, AppError> {
        if !unit_looks_safe(unit) {
            return Err(AppError::Systemd("invalid unit name".into()));
        }
        const DETAILS_TTL: std::time::Duration = std::time::Duration::from_secs(3);
        if let Ok(cache) = self.details_cache.lock() {
            if let Some((at, cached)) = cache.get(unit) {
                if at.elapsed() < DETAILS_TTL {
                    return Ok(cached.clone());
                }
            }
        }

        let conn = Self::connection().await?;
        let manager = SystemdManagerProxy::new(&conn)
            .await
            .map_err(|e| AppError::Systemd(e.to_string()))?;

        let path = manager
            .get_unit(unit)
            .await
            .map_err(|e| AppError::Systemd(e.to_string()))?;

        let unit_proxy = SystemdUnitProxy::builder(&conn)
            .path(path.clone())
            .map_err(|e| AppError::Systemd(e.to_string()))?
            .build()
            .await
            .map_err(|e| AppError::Systemd(e.to_string()))?;

        let description = unit_proxy
            .description()
            .await
            .unwrap_or_else(|_| String::new());
        let load_state = unit_proxy
            .load_state()
            .await
            .unwrap_or_else(|_| "unknown".into());
        let active_state = unit_proxy
            .active_state()
            .await
            .unwrap_or_else(|_| "unknown".into());
        let sub_state = unit_proxy
            .sub_state()
            .await
            .unwrap_or_else(|_| "unknown".into());
        let fragment_path = unit_proxy
            .fragment_path()
            .await
            .ok()
            .map(|p| sanitize_path_display(Path::new(&p)));
        let unit_file_state = manager
            .get_unit_file_state(unit)
            .await
            .map(|s| UnitFileState::parse(&s))
            .unwrap_or(UnitFileState::Unknown);

        let info = ServiceInfo {
            unit: sanitize_cell(unit),
            description: sanitize_text(&description),
            load_state: sanitize_cell(&load_state),
            active_state: sanitize_cell(&active_state),
            sub_state: sanitize_cell(&sub_state),
            unit_path: path.to_string(),
            unit_file_state,
            fragment_path,
        };

        if let Ok(mut cache) = self.details_cache.lock() {
            cache.insert(unit.to_string(), (std::time::Instant::now(), info.clone()));
        }
        Ok(info)
    }

    async fn is_available(&self) -> bool {
        Self::connection().await.is_ok()
    }
}

/// Lexical unit-name safety check (not an allowlist by itself).
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basename_from_path() {
        assert_eq!(
            unit_file_basename("/usr/lib/systemd/system/ssh.service"),
            "ssh.service"
        );
        assert_eq!(unit_file_basename("nginx.service"), "nginx.service");
    }

    #[test]
    fn unit_safe() {
        assert!(unit_looks_safe("ssh.service"));
        assert!(!unit_looks_safe("../evil.service"));
        assert!(!unit_looks_safe("a b.service"));
    }
}
