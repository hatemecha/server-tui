//! Systemd service models.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ServiceFilter {
    #[default]
    All,
    Active,
    Inactive,
    Failed,
}

impl ServiceFilter {
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Active,
            Self::Active => Self::Inactive,
            Self::Inactive => Self::Failed,
            Self::Failed => Self::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceActionKind {
    Start,
    Stop,
    Restart,
    Reload,
    Enable,
    Disable,
}

impl ServiceActionKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Restart => "restart",
            Self::Reload => "reload",
            Self::Enable => "enable",
            Self::Disable => "disable",
        }
    }
}

/// Explicit unit-file enablement state from systemd (`ListUnitFiles` / GetUnitFileState).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnitFileState {
    Enabled,
    EnabledRuntime,
    Linked,
    LinkedRuntime,
    Masked,
    MaskedRuntime,
    Static,
    Disabled,
    Invalid,
    #[default]
    Unknown,
}

impl UnitFileState {
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "enabled" => Self::Enabled,
            "enabled-runtime" => Self::EnabledRuntime,
            "linked" => Self::Linked,
            "linked-runtime" => Self::LinkedRuntime,
            "masked" => Self::Masked,
            "masked-runtime" => Self::MaskedRuntime,
            "static" => Self::Static,
            "disabled" => Self::Disabled,
            "invalid" => Self::Invalid,
            "" => Self::Unknown,
            _ => Self::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::EnabledRuntime => "enabled-runtime",
            Self::Linked => "linked",
            Self::LinkedRuntime => "linked-runtime",
            Self::Masked => "masked",
            Self::MaskedRuntime => "masked-runtime",
            Self::Static => "static",
            Self::Disabled => "disabled",
            Self::Invalid => "invalid",
            Self::Unknown => "unknown",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::Enabled | Self::EnabledRuntime => "enabled",
            Self::Linked | Self::LinkedRuntime => "linked",
            Self::Masked | Self::MaskedRuntime => "masked",
            Self::Static => "static",
            Self::Disabled => "disabled",
            Self::Invalid => "invalid",
            Self::Unknown => "?",
        }
    }
}

/// Exact systemd `ActiveState` values (never substring-match "active" ⊂ "inactive").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActiveState {
    Active,
    Reloading,
    Inactive,
    Failed,
    Activating,
    Deactivating,
    Maintenance,
    #[default]
    Unknown,
}

impl ActiveState {
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "active" => Self::Active,
            "reloading" => Self::Reloading,
            "inactive" => Self::Inactive,
            "failed" => Self::Failed,
            "activating" => Self::Activating,
            "deactivating" => Self::Deactivating,
            "maintenance" => Self::Maintenance,
            // Legacy / uncommon: treat "dead" like inactive for filters/paint.
            "dead" => Self::Inactive,
            _ => Self::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Reloading => "reloading",
            Self::Inactive => "inactive",
            Self::Failed => "failed",
            Self::Activating => "activating",
            Self::Deactivating => "deactivating",
            Self::Maintenance => "maintenance",
            Self::Unknown => "unknown",
        }
    }

    pub fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }

    pub fn is_inactive(self) -> bool {
        matches!(self, Self::Inactive)
    }

    pub fn is_failed(self) -> bool {
        matches!(self, Self::Failed)
    }
}

#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub unit: String,
    pub description: String,
    pub load_state: String,
    /// Raw ActiveState string from systemd (kept for search/display).
    pub active_state: String,
    pub sub_state: String,
    pub unit_path: String,
    pub unit_file_state: UnitFileState,
    pub fragment_path: Option<String>,
}

impl ServiceInfo {
    pub fn active(&self) -> ActiveState {
        ActiveState::parse(&self.active_state)
    }

    pub fn is_failed(&self) -> bool {
        self.active().is_failed()
    }

    pub fn is_active(&self) -> bool {
        self.active().is_active()
    }

    pub fn is_inactive(&self) -> bool {
        self.active().is_inactive() || self.active_state.eq_ignore_ascii_case("dead")
    }
}

pub fn filter_services<'a>(
    items: &'a [ServiceInfo],
    query: &str,
    filter: ServiceFilter,
) -> Vec<&'a ServiceInfo> {
    let q = query.to_lowercase();
    items
        .iter()
        .filter(|s| match filter {
            ServiceFilter::All => true,
            ServiceFilter::Active => s.is_active(),
            ServiceFilter::Inactive => s.is_inactive(),
            ServiceFilter::Failed => s.is_failed(),
        })
        .filter(|s| {
            if q.is_empty() {
                true
            } else {
                s.unit.to_lowercase().contains(&q)
                    || s.description.to_lowercase().contains(&q)
                    || s.active_state.to_lowercase().contains(&q)
                    || s.sub_state.to_lowercase().contains(&q)
                    || s.load_state.to_lowercase().contains(&q)
                    || s.unit_file_state.label().to_lowercase().contains(&q)
                    || s.fragment_path
                        .as_ref()
                        .is_some_and(|p| p.to_lowercase().contains(&q))
            }
        })
        .collect()
}

pub fn preserve_service_selection(filtered: &[&ServiceInfo], selected: Option<&str>) -> usize {
    if filtered.is_empty() {
        return 0;
    }
    if let Some(unit) = selected {
        if let Some(idx) = filtered.iter().position(|s| s.unit == unit) {
            return idx;
        }
    }
    0
}

pub fn count_failed(items: &[ServiceInfo]) -> usize {
    items.iter().filter(|s| s.is_failed()).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<ServiceInfo> {
        vec![
            ServiceInfo {
                unit: "nginx.service".into(),
                description: "Web server".into(),
                load_state: "loaded".into(),
                active_state: "active".into(),
                sub_state: "running".into(),
                unit_path: "/org/freedesktop".into(),
                unit_file_state: UnitFileState::Enabled,
                fragment_path: None,
            },
            ServiceInfo {
                unit: "broken.service".into(),
                description: "Broken thing".into(),
                load_state: "loaded".into(),
                active_state: "failed".into(),
                sub_state: "failed".into(),
                unit_path: "/org/freedesktop".into(),
                unit_file_state: UnitFileState::Disabled,
                fragment_path: None,
            },
        ]
    }

    #[test]
    fn filters_failed() {
        let items = sample();
        assert_eq!(filter_services(&items, "", ServiceFilter::Failed).len(), 1);
    }

    #[test]
    fn preserves_unit() {
        let items = sample();
        let refs: Vec<_> = items.iter().collect();
        assert_eq!(preserve_service_selection(&refs, Some("broken.service")), 1);
    }

    #[test]
    fn parses_unit_file_state() {
        assert_eq!(UnitFileState::parse("enabled"), UnitFileState::Enabled);
        assert_eq!(
            UnitFileState::parse("masked-runtime"),
            UnitFileState::MaskedRuntime
        );
        assert_eq!(UnitFileState::parse("weird"), UnitFileState::Unknown);
    }

    #[test]
    fn active_state_exact_not_substring() {
        assert_eq!(ActiveState::parse("active"), ActiveState::Active);
        assert_eq!(ActiveState::parse("inactive"), ActiveState::Inactive);
        assert_ne!(ActiveState::parse("inactive"), ActiveState::Active);
        assert!(ActiveState::parse("inactive").is_inactive());
        assert!(!ActiveState::parse("inactive").is_active());
        assert!(ActiveState::parse("failed").is_failed());
        assert!(ActiveState::parse("dead").is_inactive());
    }
}
