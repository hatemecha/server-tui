use server_tui::model::{
    filter_processes, filter_services, finding_matches, sort_processes, Category, Confidence,
    Evidence, Finding, ProcessInfo, ProcessSort, ServiceFilter, ServiceInfo, Severity,
};

#[test]
fn process_filter_and_sort() {
    let items = vec![
        ProcessInfo {
            pid: 10,
            user: "root".into(),
            name: "zzz".into(),
            cmd: "zzz".into(),
            cpu: 1.0,
            mem_pct: Some(9.0),
            mem_bytes: 1,
            state: "S".into(),
            run_time_secs: 1,
            start_time: 10,
        },
        ProcessInfo {
            pid: 20,
            user: "alex".into(),
            name: "nginx".into(),
            cmd: "nginx -g".into(),
            cpu: 30.0,
            mem_pct: Some(2.0),
            mem_bytes: 1,
            state: "R".into(),
            run_time_secs: 1,
            start_time: 20,
        },
    ];
    assert_eq!(filter_processes(&items, "nginx").len(), 1);
    assert_eq!(filter_processes(&items, "alex").len(), 1);
    assert_eq!(filter_processes(&items, "20").len(), 1);
    let mut refs: Vec<_> = items.iter().collect();
    sort_processes(&mut refs, ProcessSort::Cpu);
    assert_eq!(refs[0].pid, 20);
    sort_processes(&mut refs, ProcessSort::Name);
    assert_eq!(refs[0].name, "nginx");
}

#[test]
fn service_failed_filter() {
    let items = vec![ServiceInfo {
        unit: "a.service".into(),
        description: "Web frontend".into(),
        load_state: "loaded".into(),
        active_state: "failed".into(),
        sub_state: "failed".into(),
        unit_path: "/".into(),
        unit_file_state: server_tui::model::UnitFileState::Disabled,
        fragment_path: Some("/etc/systemd/system/a.service".into()),
    }];
    assert_eq!(filter_services(&items, "", ServiceFilter::Failed).len(), 1);
    assert_eq!(
        filter_services(&items, "a.service", ServiceFilter::All).len(),
        1
    );
    assert_eq!(
        filter_services(&items, "frontend", ServiceFilter::All).len(),
        1
    );
    assert_eq!(
        filter_services(&items, "disabled", ServiceFilter::All).len(),
        1
    );
    assert_eq!(
        filter_services(&items, "/etc/systemd", ServiceFilter::All).len(),
        1
    );
    assert_eq!(
        filter_services(&items, "missing", ServiceFilter::All).len(),
        0
    );
}

#[test]
fn finding_matches_multi_field() {
    let f = Finding {
        id: "mem.oom".into(),
        title: "OOM killer active".into(),
        summary: "process slain".into(),
        severity: Severity::Critical,
        confidence: Confidence::High,
        category: Category::Memory,
        evidence: Evidence {
            summary: "oom-kill shown in journal".into(),
            details: vec!["pid 42 nginx".into()],
        },
        targets: vec![],
        suggested_check: Some(server_tui::model::SuggestedCheck {
            description: "check dmesg".into(),
            command_hint: Some("journalctl -k".into()),
        }),
        degradable: true,
    };
    assert!(finding_matches(&f, "nginx"));
    assert!(finding_matches(&f, "critical"));
    assert!(finding_matches(&f, "journalctl"));
    assert!(finding_matches(&f, "memory"));
    assert!(!finding_matches(&f, "xyzzy"));
}

#[test]
fn glossary_filter_is_product_oriented() {
    use server_tui::app::action::Screen;
    use server_tui::glossary::{filtered_terms, terms};
    assert!(terms().iter().any(|t| t.term.contains("search")));
    assert!(terms().iter().any(|t| t.term.contains("--demo")));
    let hits = filtered_terms(Screen::Diagnostics, "confidence");
    assert!(hits.iter().any(|t| t.term.contains("confidence")));
    let alias = filtered_terms(Screen::Diagnostics, "ack");
    assert!(alias.iter().any(|t| t.term.contains("acknowledge")));
}
