//! Query operations for build lock system.
//!
//! Pure functions for checking process liveness, parsing PIDs, and validating configuration.

use std::path::PathBuf;
use std::time::Duration;

use super::types::{BuildLockError, IoErrorKind};

/// Validate timeout is non-zero.
pub(super) fn validate_timeout(timeout: Duration) -> Result<(), BuildLockError> {
    if timeout.is_zero() {
        Err(BuildLockError::InvalidConfiguration {
            reason: "timeout must be > 0".to_string(),
        })
    } else {
        Ok(())
    }
}

/// Validate poll interval is less than timeout.
pub(super) fn validate_poll_interval(
    poll_interval: Duration,
    timeout: Duration,
) -> Result<(), BuildLockError> {
    if poll_interval >= timeout {
        Err(BuildLockError::InvalidConfiguration {
            reason: format!("poll_interval ({poll_interval:?}) must be < timeout ({timeout:?})"),
        })
    } else {
        Ok(())
    }
}

/// Parse PID from lock file content.
pub(super) fn parse_pid(content: &str) -> Result<u32, BuildLockError> {
    content
        .trim()
        .parse::<u32>()
        .map_err(|_| BuildLockError::InvalidPid {
            raw: content.to_string(),
        })
}

/// Check if a process is still alive (cross-platform).
#[cfg(unix)]
pub(super) fn is_process_alive(pid: u32) -> bool {
    // On Unix, check /proc/<pid>
    let proc_path = PathBuf::from(format!("/proc/{pid}"));
    proc_path.exists()
}

#[cfg(not(unix))]
pub(super) fn is_process_alive(pid: u32) -> bool {
    // PID 0 is never a valid process ID
    if pid == 0 {
        return false;
    }
    // On non-Unix systems, conservatively assume process is alive
    // This means we rely purely on timeout mechanism
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_timeout() {
        assert!(validate_timeout(Duration::from_secs(0)).is_err());
        assert!(validate_timeout(Duration::from_millis(1)).is_ok());
        assert!(validate_timeout(Duration::from_secs(300)).is_ok());
    }

    #[test]
    fn test_validate_poll_interval() {
        let timeout = Duration::from_secs(10);

        assert!(validate_poll_interval(Duration::from_secs(11), timeout).is_err());
        assert!(validate_poll_interval(Duration::from_secs(10), timeout).is_err());
        assert!(validate_poll_interval(Duration::from_secs(9), timeout).is_ok());
        assert!(validate_poll_interval(Duration::from_millis(100), timeout).is_ok());
    }

    #[test]
    fn test_parse_pid_valid() -> Result<(), BuildLockError> {
        assert_eq!(parse_pid("12345")?, 12345);
        assert_eq!(parse_pid("  98765  \n")?, 98765);
        Ok(())
    }

    #[test]
    fn test_parse_pid_invalid() {
        assert!(matches!(
            parse_pid("not-a-pid"),
            Err(BuildLockError::InvalidPid { .. })
        ));
        assert!(matches!(
            parse_pid(""),
            Err(BuildLockError::InvalidPid { .. })
        ));
        assert!(matches!(
            parse_pid("-123"),
            Err(BuildLockError::InvalidPid { .. })
        ));
    }

    #[test]
    fn test_is_process_alive_current_process() {
        let current_pid = std::process::id();
        assert!(is_process_alive(current_pid));
    }

    #[test]
    fn test_is_process_alive_nonexistent() {
        // PID 1 is usually init/systemd on Unix, unlikely to be a random user process
        // Use a very high PID that's unlikely to exist
        let fake_pid = u32::MAX - 1;
        let _result = is_process_alive(fake_pid);

        // Just check it doesn't panic
        // On Unix, should return false (no /proc/<pid>)
        // On non-Unix, conservatively returns true
    }
}
