//! Security validation for workspace creation

use std::{fs, path::PathBuf};

use anyhow::{bail, Result};

#[allow(dead_code)]
pub struct WorkspaceLockGuard {
    _lock_file: std::fs::File,
    lock_path: PathBuf,
}

impl Drop for WorkspaceLockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.lock_path);
    }
}

#[allow(dead_code)]
pub fn acquire_workspace_lock(workspace_path: &str) -> Result<WorkspaceLockGuard> {
    use std::{fs::File, time::Duration};

    use fs2::FileExt;

    let path = PathBuf::from(workspace_path);
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Workspace path has no parent directory"))?;

    if !parent.exists() {
        fs::create_dir_all(parent)?;
    }

    let lock_path = parent.join(".jjz.workspace.lock");
    let lock_file = File::create(&lock_path)?;

    let lock_result = lock_file.try_lock_exclusive().or_else(|_| {
        std::thread::sleep(Duration::from_secs(5));
        lock_file.try_lock_exclusive()
    });

    if let Err(e) = lock_result {
        let _ = fs::remove_file(&lock_path);
        return Err(e.into());
    }

    Ok(WorkspaceLockGuard {
        _lock_file: lock_file,
        lock_path,
    })
}

#[allow(dead_code)]
pub fn validate_workspace_path(
    _workspace_path: &str,
    _repo_root: &std::path::Path,
    config_workspace_dir: &str,
) -> Result<()> {
    use std::path::{Component, Path};

    let has_root = Path::new(config_workspace_dir)
        .components()
        .any(|c| matches!(c, Component::RootDir | Component::Prefix(_)));

    if has_root {
        bail!("Security: workspace_dir must be a relative path");
    }

    let parent_dir_count = Path::new(config_workspace_dir)
        .components()
        .filter(|c| matches!(c, Component::ParentDir))
        .count();

    if parent_dir_count > 1 {
        bail!(
            "Security: workspace_dir uses excessive parent directory references (..) for directory traversal (DEBT-04)\n\
             \nSuggestions:\n\
             - Review your workspace_dir configuration in .jjz/config.toml\n\
             - Use at most one level of parent directory reference (../)\n\
             - Consider using an absolute path or a relative path within the repository"
        );
    }

    Ok(())
}

#[allow(dead_code)]
#[allow(clippy::too_many_lines)]
pub fn validate_no_symlinks(path: &str, repo_root: &std::path::Path) -> Result<()> {
    let path_buf = PathBuf::from(path);

    if path_buf.exists() {
        let symlink_metadata = fs::symlink_metadata(&path_buf)?;
        if symlink_metadata.is_symlink() {
            bail!("Workspace path is a symlink");
        }
    }

    if let Some(parent) = path_buf.parent() {
        if parent.exists() {
            let mut current = parent.to_path_buf();
            loop {
                if current.exists() {
                    let meta = fs::symlink_metadata(&current)?;
                    if meta.is_symlink() {
                        bail!("Workspace path contains symlinks in parent chain");
                    }
                }

                match current.parent() {
                    Some(p) if !p.as_os_str().is_empty() => current = p.to_path_buf(),
                    _ => break,
                }
            }
        }
    }

    let jjz_dir = repo_root.join(".jjz");
    if jjz_dir.exists() {
        let canonical_jjz = jjz_dir.canonicalize()?;
        let canonical_repo = repo_root.canonicalize()?;

        if !canonical_jjz.starts_with(&canonical_repo) {
            bail!("Security: .jjz directory escapes repository bounds");
        }
    }

    Ok(())
}

#[allow(dead_code)]
pub fn validate_workspace_dir(path: &str) -> Result<()> {
    let path_buf = PathBuf::from(path);

    if path_buf.exists() {
        let metadata = fs::metadata(&path_buf)?;
        if metadata.is_file() {
            bail!("Workspace directory path is a file, not a directory");
        }
    }

    if let Some(parent) = path_buf.parent() {
        if parent.exists() {
            let parent_metadata = fs::metadata(parent)?;
            if parent_metadata.is_file() {
                bail!("Workspace parent path is a file, not a directory");
            }
        }
    }

    Ok(())
}

#[allow(dead_code)]
#[cfg(unix)]
pub fn check_workspace_writable(workspace_path: &str) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let path_buf = PathBuf::from(workspace_path);
    let parent = path_buf
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Workspace path has no parent directory"))?;

    if !parent.exists() {
        return Ok(());
    }

    let metadata = fs::metadata(parent)?;
    let permissions = metadata.permissions();
    let mode = permissions.mode();

    if mode & 0o200 == 0 {
        bail!("Workspace directory is not writable");
    }

    let test_file = parent.join(format!(".jjz_write_test_{}", std::process::id()));
    match fs::write(&test_file, b"test") {
        Ok(()) => {
            fs::remove_file(&test_file).ok();
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            bail!("Workspace directory is not writable: Permission denied");
        }
        Err(e) => {
            eprintln!("Warning: Write test failed: {e}");
            Ok(())
        }
    }
}

#[cfg(not(unix))]
pub fn check_workspace_writable(_workspace_path: &str) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn test_validate_no_symlinks_accepts_nonexistent_path() {
        use tempfile::TempDir;

        let temp = TempDir::new().ok();
        if let Some(temp) = temp {
            let nonexistent = temp.path().join("does").join("not").join("exist");
            let result = validate_no_symlinks(nonexistent.to_str().unwrap_or(""), temp.path());
            assert!(result.is_ok());
        }
    }
}
