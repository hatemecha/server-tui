//! Pure diagnostic rule module.

use crate::model::{
    Category, Confidence, DiagnosticSnapshot, DiagnosticTarget, Evidence, Finding, Screen, Severity,
};

pub fn rule_etc_meta(snap: &DiagnosticSnapshot) -> Vec<Finding> {
    if snap.etc_mtime_notable.is_empty() {
        return Vec::new();
    }
    let details: Vec<String> = snap
        .etc_mtime_notable
        .iter()
        .take(12)
        .map(|e| format!("{} ({})", e.path, e.kind))
        .collect();
    vec![Finding {
        id: "config.etc.recent_meta".into(),
        title: "Notable /etc metadata changes".into(),
        summary: format!(
            "{} path(s) under /etc with recent metadata changes (info only)",
            snap.etc_mtime_notable.len()
        ),
        severity: Severity::Info,
        confidence: Confidence::Low,
        category: Category::Config,
        evidence: Evidence {
            summary: details.first().cloned().unwrap_or_default(),
            details,
        },
        targets: vec![DiagnosticTarget {
            screen: Screen::Diagnostics,
            search: Some("etc".into()),
        }],
        suggested_check: None,
        degradable: true,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EtcMetaChange;

    #[test]
    fn etc_meta_is_info_only() {
        let snap = DiagnosticSnapshot {
            etc_mtime_notable: vec![EtcMetaChange {
                path: "/etc/hosts".into(),
                kind: "mtime".into(),
            }],
            ..DiagnosticSnapshot::default()
        };
        let f = rule_etc_meta(&snap);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].id, "config.etc.recent_meta");
        assert_eq!(f[0].severity, Severity::Info);
        assert_eq!(f[0].confidence, Confidence::Low);
    }

    #[test]
    fn etc_meta_empty_is_silent() {
        assert!(rule_etc_meta(&DiagnosticSnapshot::default()).is_empty());
    }
}
