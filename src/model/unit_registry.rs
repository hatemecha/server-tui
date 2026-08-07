//! Shared known-unit allowlist between service listing and admin actions.

use std::collections::HashSet;
use std::sync::{Arc, RwLock};

/// Thread-safe registry of unit names last observed via ListUnits.
#[derive(Debug, Clone, Default)]
pub struct UnitRegistry {
    inner: Arc<RwLock<HashSet<String>>>,
}

impl UnitRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    pub fn replace_all(&self, units: impl IntoIterator<Item = String>) {
        if let Ok(mut guard) = self.inner.write() {
            guard.clear();
            guard.extend(units);
        }
    }

    pub fn contains(&self, unit: &str) -> bool {
        self.inner.read().map(|g| g.contains(unit)).unwrap_or(false)
    }

    pub fn len(&self) -> usize {
        self.inner.read().map(|g| g.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_and_lookup() {
        let reg = UnitRegistry::new();
        reg.replace_all(["a.service".into(), "b.service".into()]);
        assert!(reg.contains("a.service"));
        assert!(!reg.contains("c.service"));
        assert_eq!(reg.len(), 2);
    }
}
