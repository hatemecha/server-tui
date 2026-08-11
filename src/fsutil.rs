//! Secure filesystem helpers with explicit directory ownership semantics.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::AppError;

static TEMP_NONCE: AtomicU64 = AtomicU64::new(0);
const TEMP_CREATE_ATTEMPTS: u64 = 64;

/// Create or harden a directory that is owned exclusively by server-tui.
///
/// Callers must not use this for a user-selected directory or an arbitrary
/// parent directory.
pub fn ensure_private_app_dir(path: &Path) -> Result<(), AppError> {
    fs::create_dir_all(path).map_err(internal)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(internal)?;
    }
    Ok(())
}

/// Open an append-only private file without changing its parent permissions.
pub fn open_private_append(path: &Path) -> Result<File, AppError> {
    ensure_parent_exists(path)?;
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(nix::libc::O_NOFOLLOW);
    }
    let file = options.open(path).map_err(internal)?;
    set_private_file_permissions(&file)?;
    Ok(file)
}

/// Atomically replace `path` with a private file, preserving parent permissions.
///
/// Linux durability sequence: unique `O_EXCL`-style sibling, file `sync_all`,
/// rename, then parent-directory `sync_all`. Filesystem guarantees may vary.
pub fn write_private_atomic(path: &Path, data: &[u8]) -> Result<(), AppError> {
    ensure_parent_exists(path)?;
    let start = TEMP_NONCE.fetch_add(TEMP_CREATE_ATTEMPTS, Ordering::Relaxed);
    write_private_atomic_from(path, data, start)
}

fn write_private_atomic_from(path: &Path, data: &[u8], start: u64) -> Result<(), AppError> {
    let (mut file, temp_path) = create_unique_temp(path, start)?;
    let mut cleanup = TempCleanup::new(temp_path.clone());

    file.write_all(data).map_err(internal)?;
    file.sync_all().map_err(internal)?;
    drop(file);

    fs::rename(&temp_path, path).map_err(internal)?;
    cleanup.disarm();
    sync_parent(path)?;
    Ok(())
}

fn ensure_parent_exists(path: &Path) -> Result<(), AppError> {
    let parent = parent_dir(path);
    if !parent.exists() {
        fs::create_dir_all(parent).map_err(internal)?;
    }
    Ok(())
}

fn parent_dir(path: &Path) -> &Path {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn create_unique_temp(path: &Path, start: u64) -> Result<(File, PathBuf), AppError> {
    for offset in 0..TEMP_CREATE_ATTEMPTS {
        let candidate = temp_candidate(path, start.saturating_add(offset));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&candidate) {
            Ok(file) => return Ok((file, candidate)),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(internal(err)),
        }
    }
    Err(AppError::Internal(format!(
        "could not create a unique temporary sibling for {}",
        crate::sanitize::sanitize_path_display(path)
    )))
}

fn temp_candidate(path: &Path, nonce: u64) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("server-tui-file");
    parent_dir(path).join(format!(
        ".{name}.server-tui-{}-{nonce}.tmp",
        std::process::id()
    ))
}

fn set_private_file_permissions(file: &File) -> Result<(), AppError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(internal)?;
    }
    Ok(())
}

fn sync_parent(path: &Path) -> Result<(), AppError> {
    #[cfg(unix)]
    {
        File::open(parent_dir(path))
            .and_then(|dir| dir.sync_all())
            .map_err(internal)?;
    }
    Ok(())
}

fn internal(error: impl std::fmt::Display) -> AppError {
    AppError::Internal(error.to_string())
}

struct TempCleanup {
    path: Option<PathBuf>,
}

impl TempCleanup {
    fn new(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    fn disarm(&mut self) {
        self.path = None;
    }
}

impl Drop for TempCleanup {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            let _ = fs::remove_file(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    use std::os::unix::fs::{symlink, PermissionsExt};

    #[test]
    fn explicit_write_preserves_existing_parent_mode() {
        let dir = tempfile::tempdir().expect("tempdir");
        #[cfg(unix)]
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).expect("parent mode");

        write_private_atomic(&dir.path().join("report.md"), b"shareable").expect("write");

        #[cfg(unix)]
        assert_eq!(mode(dir.path()), 0o755);
    }

    #[test]
    fn atomic_write_has_private_mode_and_correct_contents() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("state.toml");
        write_private_atomic(&path, b"hello").expect("write");
        assert_eq!(fs::read(&path).expect("read"), b"hello");
        #[cfg(unix)]
        assert_eq!(mode(&path), 0o600);
    }

    #[test]
    fn atomic_write_replaces_complete_contents() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        write_private_atomic(&path, b"old-long-value").expect("old write");
        write_private_atomic(&path, b"new").expect("new write");
        assert_eq!(fs::read(&path).expect("read"), b"new");
    }

    #[test]
    fn atomic_write_creates_missing_user_managed_parent_without_hardening_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let parent = dir.path().join("custom");
        write_private_atomic(&parent.join("config.toml"), b"value").expect("write");
        assert!(parent.is_dir());
    }

    #[test]
    fn app_owned_directory_is_private_only_when_explicitly_requested() {
        let dir = tempfile::tempdir().expect("tempdir");
        let app_dir = dir.path().join("server-tui");
        ensure_private_app_dir(&app_dir).expect("private app dir");
        #[cfg(unix)]
        assert_eq!(mode(&app_dir), 0o700);
    }

    #[test]
    fn temp_collision_does_not_overwrite_foreign_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("report.md");
        let foreign = temp_candidate(&path, 10);
        fs::write(&foreign, b"foreign").expect("foreign temp");

        write_private_atomic_from(&path, b"report", 10).expect("collision retry");

        assert_eq!(fs::read(&foreign).expect("foreign read"), b"foreign");
    }

    #[cfg(unix)]
    #[test]
    fn preexisting_temp_symlink_is_not_followed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        let victim = dir.path().join("victim");
        fs::write(&victim, b"do not touch").expect("victim");
        symlink(&victim, temp_candidate(&path, 20)).expect("temp symlink");

        write_private_atomic_from(&path, b"config", 20).expect("symlink collision retry");

        assert_eq!(fs::read(&victim).expect("victim read"), b"do not touch");
    }

    #[test]
    fn successful_write_leaves_no_owned_temp_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("report.txt");
        write_private_atomic_from(&path, b"body", 30).expect("write");
        let names: Vec<_> = fs::read_dir(dir.path())
            .expect("read dir")
            .map(|entry| entry.expect("entry").file_name())
            .collect();
        assert_eq!(names, vec![std::ffi::OsString::from("report.txt")]);
    }

    #[test]
    fn failed_rename_cleans_up_owned_temp_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let destination = dir.path().join("destination");
        fs::create_dir(&destination).expect("destination directory");
        assert!(write_private_atomic_from(&destination, b"body", 40).is_err());
        let names: Vec<_> = fs::read_dir(dir.path())
            .expect("read dir")
            .map(|entry| entry.expect("entry").file_name())
            .collect();
        assert_eq!(names, vec![std::ffi::OsString::from("destination")]);
    }

    #[cfg(unix)]
    #[test]
    fn private_append_is_0600_and_refuses_symlinks() {
        let dir = tempfile::tempdir().expect("tempdir");
        let log = dir.path().join("debug.log");
        drop(open_private_append(&log).expect("open log"));
        assert_eq!(mode(&log), 0o600);

        let victim = dir.path().join("victim");
        fs::write(&victim, b"safe").expect("victim");
        let link = dir.path().join("link.log");
        symlink(&victim, &link).expect("symlink");
        assert!(open_private_append(&link).is_err());
        assert_eq!(fs::read(&victim).expect("read victim"), b"safe");
    }

    #[cfg(unix)]
    fn mode(path: &Path) -> u32 {
        fs::metadata(path).expect("metadata").permissions().mode() & 0o777
    }
}
