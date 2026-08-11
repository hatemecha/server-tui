//! Secure filesystem helpers: explicit modes (not just umask).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::error::AppError;

/// Create a directory and all parents with mode `0o700`.
pub fn ensure_private_dir(path: &Path) -> Result<(), AppError> {
    fs::create_dir_all(path).map_err(|e| AppError::Internal(e.to_string()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o700);
        fs::set_permissions(path, perms).map_err(|e| AppError::Internal(e.to_string()))?;
    }
    Ok(())
}

/// Create/truncate a file with mode `0o600` and write `data`, then sync.
pub fn write_private_file(path: &Path, data: &[u8]) -> Result<(), AppError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        f.write_all(data)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        f.sync_all()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        // Belts and braces if umask interfered with create mode.
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(path, perms).map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let mut f = File::create(path).map_err(|e| AppError::Internal(e.to_string()))?;
        f.write_all(data)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        f.sync_all()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    }
}

/// Atomic write: private temp sibling → fsync → rename.
pub fn write_private_atomic(path: &Path, data: &[u8]) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        ensure_private_dir(parent)?;
    }
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|e| e.to_str()).unwrap_or("dat")
    ));
    write_private_file(&tmp, data)?;
    fs::rename(&tmp, path).map_err(|e| AppError::Internal(e.to_string()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(path, perms).map_err(|e| AppError::Internal(e.to_string()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_private_atomic_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secret.txt");
        write_private_atomic(&path, b"hello").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"hello");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
            let dmode = fs::metadata(dir.path()).unwrap().permissions().mode() & 0o777;
            // tempfile may not be 0700; our nested private dir would be.
            let nested = dir.path().join("nested");
            ensure_private_dir(&nested).unwrap();
            let nmode = fs::metadata(&nested).unwrap().permissions().mode() & 0o777;
            assert_eq!(nmode, 0o700);
            let _ = dmode;
        }
    }
}
