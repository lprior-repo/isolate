// Characterization tests for hooks.rs refactoring
//
// These tests capture current behavior of hooks.rs to ensure
// refactoring doesn't break functionality.

use zjj_core::hooks::HookType;

#[cfg(test)]
mod hook_type_event_names {
    use super::*;

    #[test]
    fn test_post_create_event_name() {
        assert_eq!(HookType::PostCreate.event_name(), "post_create");
    }

    #[test]
    fn test_pre_remove_event_name() {
        assert_eq!(HookType::PreRemove.event_name(), "pre_remove");
    }

    #[test]
    fn test_post_merge_event_name() {
        assert_eq!(HookType::PostMerge.event_name(), "post_merge");
    }
}

// This demonstrates RED phase:
// We're writing characterization tests that capture existing behavior
// before refactoring hooks.rs into modular structure
