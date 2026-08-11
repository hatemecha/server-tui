//! Background disk usage scan (ncdu-inspired, no deletion).

use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use jwalk::WalkDir;
use tokio_util::sync::CancellationToken;

use crate::error::AppError;
use crate::model::{
    aggregate_sizes, default_exclusions, should_exclude, StorageNode, StorageProgress, StorageTree,
};
use crate::providers::StorageProvider;
use crate::sanitize::sanitize_cell;

pub struct LinuxStorageProvider;

#[async_trait]
impl StorageProvider for LinuxStorageProvider {
    async fn scan(
        &self,
        root: PathBuf,
        stay_on_fs: bool,
        follow_symlinks: bool,
        cancel: CancellationToken,
        progress: tokio::sync::mpsc::Sender<StorageProgress>,
    ) -> Result<StorageTree, AppError> {
        tokio::task::spawn_blocking(move || {
            scan_blocking(root, stay_on_fs, follow_symlinks, cancel, progress)
        })
        .await
        .map_err(|e| AppError::Storage(format!("scan join error: {e}")))?
    }
}

fn scan_blocking(
    root: PathBuf,
    stay_on_fs: bool,
    follow_symlinks: bool,
    cancel: CancellationToken,
    progress: tokio::sync::mpsc::Sender<StorageProgress>,
) -> Result<StorageTree, AppError> {
    if !root.exists() {
        return Err(AppError::Storage(format!(
            "path does not exist: {}",
            crate::sanitize::sanitize_path_display(&root)
        )));
    }

    let exclusions = if root == Path::new("/") {
        default_exclusions()
    } else {
        // Still skip virtual fs if somehow under them.
        default_exclusions()
    };

    let root_dev = fs::metadata(&root).ok().map(|m| m.dev());
    let files = AtomicU64::new(0);
    let dirs = AtomicU64::new(0);
    let bytes = AtomicU64::new(0);
    let errors = AtomicU64::new(0);
    let last_progress = AtomicU64::new(0);

    const MAX_SCAN_ENTRIES: usize = 250_000;
    let mut truncated = false;
    // Collect flat entries then build tree.
    let mut entries: Vec<(PathBuf, bool, u64, u64, Option<String>)> = Vec::new();
    // (path, is_dir, size, apparent, error)

    let cancel_walk = cancel.clone();
    let root_for_walk = root.clone();
    let exclusions_for_walk = exclusions.clone();
    let walker = WalkDir::new(&root)
        .follow_links(follow_symlinks)
        .skip_hidden(false)
        .process_read_dir(move |_depth, path, _state, children| {
            if cancel_walk.is_cancelled() {
                children.clear();
                return;
            }
            if should_exclude(path, &exclusions_for_walk) && path != root_for_walk.as_path() {
                children.clear();
                return;
            }
            if stay_on_fs {
                if let (Some(rd), Ok(meta)) = (root_dev, fs::metadata(path)) {
                    if meta.dev() != rd {
                        children.clear();
                    }
                }
            }
        });

    for entry in walker {
        if cancel.is_cancelled() {
            return Err(AppError::Storage("scan cancelled".into()));
        }
        match entry {
            Ok(e) => {
                let path = e.path();
                if should_exclude(&path, &exclusions) && path != root {
                    continue;
                }
                let meta = match e.metadata() {
                    Ok(m) => m,
                    Err(err) => {
                        errors.fetch_add(1, Ordering::Relaxed);
                        entries.push((path, e.file_type().is_dir(), 0, 0, Some(err.to_string())));
                        continue;
                    }
                };
                if meta.file_type().is_symlink() && !follow_symlinks {
                    // Count symlink as zero-size leaf.
                    files.fetch_add(1, Ordering::Relaxed);
                    entries.push((path, false, 0, 0, None));
                    continue;
                }
                let is_dir = meta.is_dir();
                let apparent = meta.len();
                // Prefer allocated blocks when available (512-byte blocks).
                let size = {
                    let blocks = meta.blocks();
                    if blocks > 0 {
                        blocks.saturating_mul(512)
                    } else {
                        apparent
                    }
                };
                if is_dir {
                    dirs.fetch_add(1, Ordering::Relaxed);
                } else {
                    files.fetch_add(1, Ordering::Relaxed);
                    bytes.fetch_add(size, Ordering::Relaxed);
                }
                entries.push((path.clone(), is_dir, size, apparent, None));
                if entries.len() >= MAX_SCAN_ENTRIES {
                    truncated = true;
                    break;
                }

                let n = files.load(Ordering::Relaxed) + dirs.load(Ordering::Relaxed);
                let last = last_progress.load(Ordering::Relaxed);
                if n.saturating_sub(last) >= 200 {
                    last_progress.store(n, Ordering::Relaxed);
                    let _ = progress.blocking_send(StorageProgress {
                        root: root.clone(),
                        files: files.load(Ordering::Relaxed),
                        dirs: dirs.load(Ordering::Relaxed),
                        bytes: bytes.load(Ordering::Relaxed),
                        current: path,
                        errors: errors.load(Ordering::Relaxed),
                    });
                }
            }
            Err(err) => {
                errors.fetch_add(1, Ordering::Relaxed);
                tracing::debug!("storage walk error: {err}");
            }
        }
    }

    if cancel.is_cancelled() {
        return Err(AppError::Storage("scan cancelled".into()));
    }

    let tree = build_tree(&root, &entries)?;
    let _ = progress.blocking_send(StorageProgress {
        root: root.clone(),
        files: files.load(Ordering::Relaxed),
        dirs: dirs.load(Ordering::Relaxed),
        bytes: bytes.load(Ordering::Relaxed),
        current: root.clone(),
        errors: errors.load(Ordering::Relaxed),
    });

    // Small yield for UI breathing room isn't needed in blocking; return.
    let _ = Duration::from_millis(0);

    let mut error_count = errors.load(Ordering::Relaxed);
    if truncated {
        error_count = error_count.saturating_add(1);
        tracing::warn!("storage scan truncated at {MAX_SCAN_ENTRIES} entries (partial results)");
    }

    Ok(StorageTree {
        root: tree,
        files: files.load(Ordering::Relaxed),
        dirs: dirs.load(Ordering::Relaxed),
        errors: error_count,
        excluded: exclusions,
        truncated,
    })
}

fn build_tree(
    root: &Path,
    entries: &[(PathBuf, bool, u64, u64, Option<String>)],
) -> Result<StorageNode, AppError> {
    let mut nodes: HashMap<PathBuf, StorageNode> = HashMap::new();

    for (path, is_dir, size, apparent, error) in entries {
        let name = path
            .file_name()
            .map(|s| sanitize_cell(&s.to_string_lossy()))
            .unwrap_or_else(|| sanitize_cell(&path.to_string_lossy()));
        nodes.insert(
            path.clone(),
            StorageNode {
                name,
                path: path.clone(),
                is_dir: *is_dir,
                size: if *is_dir { 0 } else { *size },
                apparent_size: if *is_dir { 0 } else { *apparent },
                children: vec![],
                inaccessible: error.is_some(),
                error: error.clone(),
            },
        );
    }

    // Ensure root exists.
    if !nodes.contains_key(root) {
        nodes.insert(
            root.to_path_buf(),
            StorageNode {
                name: root
                    .file_name()
                    .map(|s| sanitize_cell(&s.to_string_lossy()))
                    .unwrap_or_else(|| sanitize_cell(&root.to_string_lossy())),
                path: root.to_path_buf(),
                is_dir: true,
                size: 0,
                apparent_size: 0,
                children: vec![],
                inaccessible: false,
                error: None,
            },
        );
    }

    let mut paths: Vec<PathBuf> = nodes.keys().cloned().collect();
    paths.sort_by_key(|p| std::cmp::Reverse(p.components().count()));

    for path in paths {
        if path == root {
            continue;
        }
        let Some(parent) = path.parent().map(Path::to_path_buf) else {
            continue;
        };
        let Some(node) = nodes.remove(&path) else {
            continue;
        };
        if let Some(parent_node) = nodes.get_mut(&parent) {
            parent_node.children.push(node);
        } else if let Some(root_node) = nodes.get_mut(root) {
            // Orphan under unexpected layout — attach to root.
            root_node.children.push(node);
        }
    }

    let mut root_node = nodes
        .remove(root)
        .ok_or_else(|| AppError::Storage("failed to build storage tree".into()))?;
    aggregate_sizes(&mut root_node);
    Ok(root_node)
}
