//! Storage tree models and aggregation.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StorageTab {
    #[default]
    Mounts,
    DirectoryUsage,
    LargestFiles,
}

impl StorageTab {
    pub fn next(self) -> Self {
        match self {
            Self::Mounts => Self::DirectoryUsage,
            Self::DirectoryUsage => Self::LargestFiles,
            Self::LargestFiles => Self::Mounts,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Mounts => "mounts",
            Self::DirectoryUsage => "dirs",
            Self::LargestFiles => "largest",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StorageSort {
    #[default]
    Size,
    Name,
}

impl StorageSort {
    pub fn next(self) -> Self {
        match self {
            Self::Size => Self::Name,
            Self::Name => Self::Size,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Size => "size",
            Self::Name => "name",
        }
    }
}

#[derive(Debug, Clone)]
pub struct StorageNode {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub apparent_size: u64,
    pub children: Vec<StorageNode>,
    pub inaccessible: bool,
    pub error: Option<String>,
}

impl StorageNode {
    pub fn find_child(&self, name: &str) -> Option<&StorageNode> {
        self.children.iter().find(|c| c.name == name)
    }

    pub fn sort_children(&mut self, sort: StorageSort) {
        match sort {
            StorageSort::Size => {
                self.children
                    .sort_by(|a, b| b.size.cmp(&a.size).then(a.name.cmp(&b.name)));
            }
            StorageSort::Name => {
                self.children.sort_by(|a, b| a.name.cmp(&b.name));
            }
        }
        for child in &mut self.children {
            child.sort_children(sort);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct StorageProgress {
    pub root: PathBuf,
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    pub current: PathBuf,
    pub errors: u64,
}

#[derive(Debug, Clone)]
pub struct StorageTree {
    pub root: StorageNode,
    pub files: u64,
    pub dirs: u64,
    pub errors: u64,
    pub excluded: Vec<PathBuf>,
    /// True when scan stopped early due to max_scan_entries budget.
    pub truncated: bool,
}

pub fn default_exclusions() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/proc"),
        PathBuf::from("/sys"),
        PathBuf::from("/dev"),
        PathBuf::from("/run"),
    ]
}

pub fn should_exclude(path: &Path, exclusions: &[PathBuf]) -> bool {
    exclusions
        .iter()
        .any(|ex| path == ex || path.starts_with(ex))
}

/// Aggregate child sizes into parent (bottom-up).
pub fn aggregate_sizes(node: &mut StorageNode) {
    if !node.is_dir {
        return;
    }
    let mut size = 0u64;
    let mut apparent = 0u64;
    for child in &mut node.children {
        aggregate_sizes(child);
        size = size.saturating_add(child.size);
        apparent = apparent.saturating_add(child.apparent_size);
    }
    // Directory itself has no disk size beyond children in this MVP model.
    node.size = size;
    node.apparent_size = apparent;
}

pub fn format_size(bytes: u64) -> String {
    crate::model::metrics::format_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregates_children() {
        let mut root = StorageNode {
            name: "root".into(),
            path: PathBuf::from("/tmp/root"),
            is_dir: true,
            size: 0,
            apparent_size: 0,
            children: vec![
                StorageNode {
                    name: "a".into(),
                    path: PathBuf::from("/tmp/root/a"),
                    is_dir: false,
                    size: 100,
                    apparent_size: 100,
                    children: vec![],
                    inaccessible: false,
                    error: None,
                },
                StorageNode {
                    name: "b".into(),
                    path: PathBuf::from("/tmp/root/b"),
                    is_dir: true,
                    size: 0,
                    apparent_size: 0,
                    children: vec![StorageNode {
                        name: "c".into(),
                        path: PathBuf::from("/tmp/root/b/c"),
                        is_dir: false,
                        size: 50,
                        apparent_size: 50,
                        children: vec![],
                        inaccessible: false,
                        error: None,
                    }],
                    inaccessible: false,
                    error: None,
                },
            ],
            inaccessible: false,
            error: None,
        };
        aggregate_sizes(&mut root);
        assert_eq!(root.size, 150);
        assert_eq!(root.children[1].size, 50);
    }

    #[test]
    fn excludes_virtual_fs() {
        assert!(should_exclude(Path::new("/proc/1"), &default_exclusions()));
        assert!(!should_exclude(Path::new("/home"), &default_exclusions()));
    }
}
