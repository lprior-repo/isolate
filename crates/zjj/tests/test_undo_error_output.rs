mod common;

use common::TestHarness;

#[test]
fn undo_plain_text_error_is_emitted_once() {
    // Given an initialized repository without undo history
    // When undo executes in human mode
    // Then the error message appears once (no duplicated error blocks)
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["undo"]);
    assert!(!result.success, "undo should fail when history is missing");

    let combined = format!("{}\n{}", result.stdout, result.stderr);
    let needle = "No undo history found. Cannot undo.";
    let occurrences = combined.matches(needle).count();
    assert_eq!(
        occurrences, 1,
        "expected exactly one undo error message, got {occurrences}:\n{combined}"
    );
}
