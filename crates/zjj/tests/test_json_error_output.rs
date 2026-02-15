<<<<<<< Conflict 1 of 1
+++++++ Contents of side #1

%%%%%%% Changes from base to side #2
-// Integration tests have relaxed clippy settings for brutal test scenarios.
-// Production code (src/) must use strict zero-unwrap/panic patterns.
-#![allow(clippy::expect_used)]
-
 use assert_cmd::Command;
 use serde_json::Value;
 
 fn assert_json_only_parse_error(args: &[&str]) {
     let output = Command::new(env!("CARGO_BIN_EXE_zjj"))
         .args(args)
         .output()
         .expect("command should run");
 
     assert_eq!(output.status.code(), Some(2), "invalid args should exit 2");
 
     let stdout: Value = serde_json::from_slice(&output.stdout)
         .expect("stdout should contain only a JSON error document");
     assert_eq!(
         stdout
             .get("error")
             .and_then(|error| error.get("code"))
             .and_then(Value::as_str),
         Some("INVALID_ARGUMENT")
     );
 
     let stderr = String::from_utf8_lossy(&output.stderr);
     assert!(
         !stderr.contains("Usage:") && !stderr.contains("error:"),
         "stderr should not contain clap output in JSON mode: {stderr}"
     );
 }
 
 #[test]
 fn json_mode_parse_error_submit_is_json_only() {
     // Given submit in JSON mode
     // When argument parsing fails due to unknown flag
     // Then output is machine-readable JSON only
     assert_json_only_parse_error(&["submit", "--json", "--wat"]);
 }
 
 #[test]
 fn json_mode_parse_error_status_is_json_only() {
     // Given status in JSON mode
     // When argument parsing fails due to invalid boolean syntax
     // Then output is machine-readable JSON only
     assert_json_only_parse_error(&["status", "--json", "--watch=1"]);
 }
 
 #[test]
 fn json_mode_parse_error_undo_is_json_only() {
     // Given undo in JSON mode
     // When argument parsing fails due to unknown flag
     // Then output is machine-readable JSON only
     assert_json_only_parse_error(&["undo", "--json", "--bogus"]);
 }
>>>>>>> Conflict 1 of 1 ends
