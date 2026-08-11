//! Pure diagnostic rule module.

use crate::diagnostics::rules::common::sanitize_id;
use crate::model::diagnostics::thresholds;
use crate::model::{
    Category, Confidence, DiagnosticSnapshot, DiagnosticTarget, Evidence, Finding, Screen, Severity,
};

pub fn rule_temperature(snap: &DiagnosticSnapshot) -> Vec<Finding> {
    snap.temperatures
        .iter()
        .filter(|t| t.celsius >= thresholds::TEMP_CELSIUS_HIGH)
        .map(|t| Finding {
            id: format!("temp.high.{}", sanitize_id(&t.label)),
            title: "High temperature observed".into(),
            summary: format!("{} at {:.0}°C", t.label, t.celsius),
            severity: Severity::Warning,
            confidence: Confidence::Medium,
            category: Category::Temperature,
            evidence: Evidence {
                summary: format!(
                    "{:.1}°C (threshold {}°C)",
                    t.celsius,
                    thresholds::TEMP_CELSIUS_HIGH
                ),
                details: vec!["Wording intentionally avoids claiming thermal throttling.".into()],
            },
            targets: vec![DiagnosticTarget {
                screen: Screen::Dashboard,
                search: None,
            }],
            suggested_check: None,
            degradable: true,
        })
        .collect()
}
