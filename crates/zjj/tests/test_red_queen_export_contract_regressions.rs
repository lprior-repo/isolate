// Martin Fowler-style adversarial regressions for export parser/contract behavior.

mod common;

use common::{parse_json_output, payload, TestHarness};

#[test]
fn bdd_export_help_json_returns_single_envelope() {
    // Given a user requesting help in JSON mode
    // When I run export with --help --json
    // Then output is exactly one JSON envelope and no mixed plain-text stream
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.zjj(&["export", "--help", "--json"]);
    assert!(
        result.success,
        "Expected help command to succeed\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    assert_eq!(
        result.exit_code,
        Some(0),
        "Help should exit 0\nstdout: {}\nstderr: {}",
        result.stdout,
        result.stderr
    );
    assert!(
        result.stderr.trim().is_empty(),
        "JSON help should not leak human output to stderr\nstdout: {}\nstderr: {}",
        result.stdout,
        result.stderr
    );

    let parsed = parse_json_output(result.stdout.trim())
        .expect("export --help --json should be parseable JSON envelope");
    assert_eq!(
        parsed.get("$schema").and_then(serde_json::Value::as_str),
        Some("zjj://cli-display-response/v1")
    );
    assert_eq!(
        parsed.get("success").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        parsed
            .get("display_type")
            .and_then(serde_json::Value::as_str),
        Some("help")
    );
}

#[test]
fn bdd_export_parser_rejects_duplicate_json_flag_with_single_json_error() {
    // Given an initialized repository
    // When parser receives duplicated --json flag
    // Then parsing fails with non-zero and a single JSON error envelope
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["export", "--json", "--json"]);
    assert!(
        !result.success,
        "Expected duplicate flag parse failure\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    assert!(
        result.exit_code.unwrap_or_default() > 0,
        "Expected non-zero exit for parse error\nstdout: {}\nstderr: {}",
        result.stdout,
        result.stderr
    );
    assert!(
        result.stderr.trim().is_empty(),
        "Parse failure in --json mode should not emit extra plain stderr\nstdout: {}\nstderr: {}",
        result.stdout,
        result.stderr
    );

    let parsed = parse_json_output(result.stdout.trim())
        .expect("duplicate --json parse failure should return parseable JSON envelope");
    assert_eq!(
        parsed.get("$schema").and_then(serde_json::Value::as_str),
        Some("zjj://error-response/v1")
    );
    assert_eq!(
        parsed.get("success").and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert_eq!(
        payload(&parsed)
            .get("error")
            .and_then(|err| err.get("code"))
            .and_then(serde_json::Value::as_str),
        Some("INVALID_ARGUMENT")
    );
}

#[test]
fn bdd_export_missing_session_fails_non_zero_in_human_and_json_modes() {
    // Given an initialized repository with no matching session
    // When I export a missing session in human and JSON modes
    // Then both modes fail non-zero and JSON output keeps one success field
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let human = harness.zjj(&["export", "missing-session"]);
    assert!(
        !human.success,
        "Expected missing-session export to fail in human mode\nstdout: {}\nstderr: {}",
        human.stdout, human.stderr
    );
    assert!(
        human.exit_code.unwrap_or_default() > 0,
        "Expected non-zero human exit code\nstdout: {}\nstderr: {}",
        human.stdout,
        human.stderr
    );

    let json_result = harness.zjj(&["export", "missing-session", "--json"]);
    assert!(
        !json_result.success,
        "Expected missing-session export to fail in JSON mode\nstdout: {}\nstderr: {}",
        json_result.stdout, json_result.stderr
    );
    assert!(
        json_result.exit_code.unwrap_or_default() > 0,
        "Expected non-zero JSON exit code\nstdout: {}\nstderr: {}",
        json_result.stdout,
        json_result.stderr
    );

    let parsed = parse_json_output(json_result.stdout.trim())
        .expect("missing-session --json should return parseable JSON envelope");
    assert_eq!(
        parsed.get("$schema").and_then(serde_json::Value::as_str),
        Some("zjj://error-response/v1")
    );
    assert_eq!(
        json_result.stdout.matches("\"success\"").count(),
        1,
        "JSON envelope should expose one success field\nstdout: {}",
        json_result.stdout
    );
}

#[test]
fn bdd_contract_export_and_import_exist_with_runtime_schemas() {
    // Given the contract command
    // When I request export/import contracts in JSON mode
    // Then both contracts are discoverable with schemas matching runtime output
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let export_contract = harness.zjj(&["contract", "export", "--json"]);
    assert!(
        export_contract.success,
        "Expected export contract to exist\nstdout: {}\nstderr: {}",
        export_contract.stdout, export_contract.stderr
    );
    let export_json = parse_json_output(export_contract.stdout.trim())
        .expect("contract export --json should be parseable");
    let export_payload = payload(&export_json);
    assert_eq!(
        export_payload
            .get("name")
            .and_then(serde_json::Value::as_str),
        Some("export")
    );
    assert_eq!(
        export_payload
            .get("output_schema")
            .and_then(serde_json::Value::as_str),
        Some("zjj://export-response/v1")
    );

    let import_contract = harness.zjj(&["contract", "import", "--json"]);
    assert!(
        import_contract.success,
        "Expected import contract to exist\nstdout: {}\nstderr: {}",
        import_contract.stdout, import_contract.stderr
    );
    let import_json = parse_json_output(import_contract.stdout.trim())
        .expect("contract import --json should be parseable");
    let import_payload = payload(&import_json);
    assert_eq!(
        import_payload
            .get("name")
            .and_then(serde_json::Value::as_str),
        Some("import")
    );
    assert_eq!(
        import_payload
            .get("output_schema")
            .and_then(serde_json::Value::as_str),
        Some("zjj://import-response/v1")
    );
}
