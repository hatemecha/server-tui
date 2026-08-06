use std::path::PathBuf;

use server_tui::model::{
    aggregate_sizes, default_exclusions, format_size, should_exclude, StorageNode,
};

#[test]
fn storage_aggregation_and_format() {
    let mut root = StorageNode {
        name: "r".into(),
        path: PathBuf::from("/tmp/r"),
        is_dir: true,
        size: 0,
        apparent_size: 0,
        children: vec![StorageNode {
            name: "f".into(),
            path: PathBuf::from("/tmp/r/f"),
            is_dir: false,
            size: 2048,
            apparent_size: 2048,
            children: vec![],
            inaccessible: false,
            error: None,
        }],
        inaccessible: false,
        error: None,
    };
    aggregate_sizes(&mut root);
    assert_eq!(root.size, 2048);
    assert!(format_size(2048).contains("KiB"));
}

#[test]
fn excluded_paths() {
    assert!(should_exclude(
        std::path::Path::new("/sys/fs"),
        &default_exclusions()
    ));
}
