//! Event callbacks and processing
//!
//! Handles processing of file system events from the watcher,
//! extracting workspace paths and normalizing event data.

use std::path::PathBuf;

use notify_debouncer_mini::DebouncedEvent;

// ═══════════════════════════════════════════════════════════════════════════
// EVENT HANDLERS
// ═══════════════════════════════════════════════════════════════════════════

/// Extract workspace path from a debounced event
///
/// Takes a file system event and extracts the workspace root directory
/// by walking up from `.beads/beads.db` to the workspace parent.
///
/// # Example
///
/// For an event with path `/workspace/.beads/beads.db`, returns `Some(/workspace)`
pub fn extract_workspace_path(event: &DebouncedEvent) -> Option<PathBuf> {
    event
        .path
        .parent() // .beads
        .and_then(|p| p.parent()) // workspace root
        .map(std::path::Path::to_path_buf)
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(test)]
    use notify_debouncer_mini::DebouncedEventKind;

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Test 1: Extract workspace path from event
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_extract_workspace_path() {
        let event = DebouncedEvent {
            path: PathBuf::from("/workspace/.beads/beads.db"),
            kind: DebouncedEventKind::Any,
        };

        let result = extract_workspace_path(&event);
        assert!(result.is_some());
        if let Some(path) = result {
            assert_eq!(path, PathBuf::from("/workspace"));
        }
    }
}
