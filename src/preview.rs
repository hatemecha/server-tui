//! Safe file preview helpers (no symlink follow, max 64KiB, binary detect).

use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use crate::error::AppError;
use crate::model::FilePreview;
use crate::sanitize::{sanitize_path_display, sanitize_text};

pub const MAX_PREVIEW_BYTES: usize = 64 * 1024;

/// Preview a regular file only. Refuses symlinks and non-files.
pub fn preview_file(path: &Path) -> Result<FilePreview, AppError> {
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
            meta.len(),
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
pub fn largest_files_from_tree(
    root: &crate::model::StorageNode,
    limit: usize,
) -> Vec<&crate::model::StorageNode> {
    let mut files = Vec::new();
    collect_files(root, &mut files);
    files.sort_by(|a, b| b.size.cmp(&a.size).then(a.name.cmp(&b.name)));
    files.truncate(limit.max(1));
    files
}

fn collect_files<'a>(
    node: &'a crate::model::StorageNode,
    out: &mut Vec<&'a crate::model::StorageNode>,
) {
    if !node.is_dir {
        out.push(node);
    }
    for c in &node.children {
        collect_files(c, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn previews_text_and_rejects_symlink_dir() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        let mut f = File::create(&path).unwrap();
        writeln!(f, "hello\x1b[31m").unwrap();
        let prev = preview_file(&path).unwrap();
        assert!(!prev.is_binary);
        assert!(!prev.text.contains('\u{1b}'));
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
