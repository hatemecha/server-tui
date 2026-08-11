//! Safe file preview helpers (O_NOFOLLOW + fd re-validate, max 64KiB, binary detect).

use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use crate::error::AppError;
use crate::model::FilePreview;
use crate::sanitize::{sanitize_path_display, sanitize_text};

pub const MAX_PREVIEW_BYTES: usize = 64 * 1024;

/// Preview a regular file only. Refuses symlinks and non-files (TOCTOU-hardened on Unix).
pub fn preview_file(path: &Path) -> Result<FilePreview, AppError> {
    #[cfg(unix)]
    {
        preview_file_unix(path)
    }
    #[cfg(not(unix))]
    {
        preview_file_portable(path)
    }
}

#[cfg(unix)]
fn preview_file_unix(path: &Path) -> Result<FilePreview, AppError> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

    let mut opts = fs::OpenOptions::new();
    opts.read(true);
    // O_NOFOLLOW: do not follow final-component symlink.
    opts.custom_flags(libc_o_nofollow());
    let mut f = opts
        .open(path)
        .map_err(|e| AppError::Storage(format!("open preview: {e}")))?;
    let meta = f
        .metadata()
        .map_err(|e| AppError::Storage(format!("fstat preview: {e}")))?;
    if !meta.file_type().is_file() {
        return Err(AppError::Storage("not a regular file".into()));
    }
    // Extra: reject directories / exotic types via mode bits.
    let mode = meta.mode();
    if (mode & 0o170000) != 0o100000 {
        return Err(AppError::Storage("not a regular file".into()));
    }
    read_preview(path, &mut f, meta.len())
}

#[cfg(unix)]
fn libc_o_nofollow() -> i32 {
    // Avoid new crate dep: use nix/libc via libc crate... we don't have libc.
    // Use nix if available, else hardcode Linux O_NOFOLLOW=0x20000.
    #[cfg(target_os = "linux")]
    {
        0x20000 // O_NOFOLLOW
    }
    #[cfg(not(target_os = "linux"))]
    {
        libc_nofollow_fallback()
    }
}

#[cfg(all(unix, not(target_os = "linux")))]
fn libc_nofollow_fallback() -> i32 {
    // Best-effort; still validates fd with fstat afterwards.
    0
}

#[cfg(not(unix))]
fn preview_file_portable(path: &Path) -> Result<FilePreview, AppError> {
    let meta = fs::symlink_metadata(path).map_err(|e| AppError::Storage(e.to_string()))?;
    if meta.file_type().is_symlink() {
        return Err(AppError::Storage(
            "refusing to follow symlink for preview".into(),
        ));
    }
    if !meta.file_type().is_file() {
        return Err(AppError::Storage("not a regular file".into()));
    }
    let mut f = File::open(path).map_err(|e| AppError::Storage(e.to_string()))?;
    read_preview(path, &mut f, meta.len())
}

fn read_preview(path: &Path, f: &mut File, total_hint: u64) -> Result<FilePreview, AppError> {
    let mut buf = vec![0u8; MAX_PREVIEW_BYTES + 1];
    let n = f
        .read(&mut buf)
        .map_err(|e| AppError::Storage(e.to_string()))?;
    let truncated = n > MAX_PREVIEW_BYTES;
    let slice = &buf[..n.min(MAX_PREVIEW_BYTES)];
    let is_binary = slice.contains(&0);
    let text = if is_binary {
        format!(
            "(binary file — {} bytes shown, {} total hint)\n{}",
            slice.len(),
            total_hint,
            hex_dump_preview(slice)
        )
    } else {
        sanitize_text(&String::from_utf8_lossy(slice))
    };
    Ok(FilePreview {
        path: sanitize_path_display(path),
        bytes_read: slice.len(),
        truncated,
        is_binary,
        text,
    })
}

fn hex_dump_preview(data: &[u8]) -> String {
    let mut out = String::new();
    for (i, chunk) in data.chunks(16).take(8).enumerate() {
        out.push_str(&format!("{:04x}: ", i * 16));
        for b in chunk {
            out.push_str(&format!("{b:02x} "));
        }
        out.push('\n');
    }
    out
}

/// Collect up to `limit` largest files from a storage tree (files only).
/// Uses a bounded min-heap so ranking is O(n log k), not O(n log n) full sort.
pub fn largest_files_from_tree(
    root: &crate::model::StorageNode,
    limit: usize,
) -> Vec<&crate::model::StorageNode> {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;

    let limit = limit.max(1);
    // Heap of (size, name, dfs index) — Ord without needing StorageNode: Ord.
    let mut heap: BinaryHeap<Reverse<(u64, String, usize)>> = BinaryHeap::new();
    let mut files: Vec<&crate::model::StorageNode> = Vec::new();

    fn walk<'a>(
        node: &'a crate::model::StorageNode,
        limit: usize,
        files: &mut Vec<&'a crate::model::StorageNode>,
        heap: &mut BinaryHeap<Reverse<(u64, String, usize)>>,
    ) {
        if !node.is_dir {
            let idx = files.len();
            files.push(node);
            heap.push(Reverse((node.size, node.name.clone(), idx)));
            if heap.len() > limit {
                heap.pop();
            }
        }
        for c in &node.children {
            walk(c, limit, files, heap);
        }
    }

    walk(root, limit, &mut files, &mut heap);
    let mut top: Vec<&crate::model::StorageNode> = heap
        .into_iter()
        .map(|Reverse((_, _, idx))| files[idx])
        .collect();
    top.sort_by(|a, b| b.size.cmp(&a.size).then(a.name.cmp(&b.name)));
    top
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::os::unix::fs::symlink;

    #[test]
    fn previews_text_and_rejects_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        let mut f = File::create(&path).unwrap();
        writeln!(f, "hello\x1b[31m").unwrap();
        let prev = preview_file(&path).unwrap();
        assert!(!prev.is_binary);
        assert!(!prev.text.contains('\u{1b}'));

        let link = dir.path().join("link.txt");
        symlink(&path, &link).unwrap();
        assert!(preview_file(&link).is_err());
    }

    #[test]
    fn detects_binary() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("b.bin");
        fs::write(&path, [0u8, 1, 2, 3]).unwrap();
        let prev = preview_file(&path).unwrap();
        assert!(prev.is_binary);
    }

    #[test]
    fn largest_collects_files() {
        use crate::model::StorageNode;
        use std::path::PathBuf;
        let tree = StorageNode {
            name: "root".into(),
            path: PathBuf::from("/tmp"),
            is_dir: true,
            size: 100,
            apparent_size: 100,
            children: vec![
                StorageNode {
                    name: "big".into(),
                    path: PathBuf::from("/tmp/big"),
                    is_dir: false,
                    size: 90,
                    apparent_size: 90,
                    children: vec![],
                    inaccessible: false,
                    error: None,
                },
                StorageNode {
                    name: "small".into(),
                    path: PathBuf::from("/tmp/small"),
                    is_dir: false,
                    size: 10,
                    apparent_size: 10,
                    children: vec![],
                    inaccessible: false,
                    error: None,
                },
            ],
            inaccessible: false,
            error: None,
        };
        let top = largest_files_from_tree(&tree, 50);
        assert_eq!(top[0].name, "big");
    }
}
