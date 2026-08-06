use server_tui::model::{
    filter_processes, filter_services, sort_processes, ProcessInfo, ProcessSort, ServiceFilter,
    ServiceInfo,
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
            mem_pct: 9.0,
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
            mem_pct: 2.0,
            mem_bytes: 1,
            state: "R".into(),
            run_time_secs: 1,
            start_time: 20,
        },
    ];
    assert_eq!(filter_processes(&items, "nginx").len(), 1);
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
        description: "A".into(),
        load_state: "loaded".into(),
        active_state: "failed".into(),
        sub_state: "failed".into(),
        unit_path: "/".into(),
        enabled: Some(false),
        fragment_path: None,
    }];
    assert_eq!(filter_services(&items, "", ServiceFilter::Failed).len(), 1);
}
