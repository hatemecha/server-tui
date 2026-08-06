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

#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub unit: String,
    pub description: String,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub unit_path: String,
    pub enabled: Option<bool>,
    pub fragment_path: Option<String>,
}

impl ServiceInfo {
    pub fn is_failed(&self) -> bool {
        self.active_state.eq_ignore_ascii_case("failed")
    }

    pub fn is_active(&self) -> bool {
        self.active_state.eq_ignore_ascii_case("active")
    }

    pub fn is_inactive(&self) -> bool {
        self.active_state.eq_ignore_ascii_case("inactive")
            || self.active_state.eq_ignore_ascii_case("dead")
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
                s.unit.to_lowercase().contains(&q) || s.description.to_lowercase().contains(&q)
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
                enabled: Some(true),
                fragment_path: None,
            },
            ServiceInfo {
                unit: "broken.service".into(),
                description: "Broken thing".into(),
                load_state: "loaded".into(),
                active_state: "failed".into(),
                sub_state: "failed".into(),
                unit_path: "/org/freedesktop".into(),
                enabled: Some(false),
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
}
