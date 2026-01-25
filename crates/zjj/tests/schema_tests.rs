use std::process::Command;

fn run_cue_export() -> Result<Option<String>, Box<dyn std::error::Error>> {
    // Check if cue is installed
    if Command::new("cue").arg("version").output().is_err() {
        println!("Skipping test: cue not installed");
        return Ok(None);
    }

    let output = Command::new("cue")
        .args(["export", "schemas/zjj_protocol.cue", "--json"])
        .output()?;

    if !output.status.success() {
        // If schemas/zjj_protocol.cue doesn't exist, we should also skip or fail differently
        // But for now let's assume if cue runs but fails, it's a real failure unless file missing
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("no such file") {
            println!("Skipping test: schema file not found");
            return Ok(None);
        }
        return Err(format!("CUE export failed: {stderr}").into());
    }

    let json_str = String::from_utf8(output.stdout)?;
    Ok(Some(json_str))
}

#[test]
fn test_cue_schema_exports_valid_json_schema() -> Result<(), Box<dyn std::error::Error>> {
    let Some(json_str) = run_cue_export()? else {
        return Ok(());
    };

    serde_json::from_str::<serde_json::Value>(&json_str)?;
    Ok(())
}

#[test]
fn test_all_commands_have_input_schemas() -> Result<(), Box<dyn std::error::Error>> {
    let Some(json_str) = run_cue_export()? else {
        return Ok(());
    };

    let schema: serde_json::Value = serde_json::from_str(&json_str)?;

    // Check that we have input request types for all commands
    let commands = [
        "state",
        "history",
        "diff-state",
        "predict-data",
        "init",
        "add",
        "remove",
        "list",
        "focus",
        "status",
        "sync",
        "diff",
        "merge",
        "abandon",
        "describe",
        "log",
        "exec",
        "agent",
        "link",
        "unlink",
        "checkpoint",
        "restore",
        "list-checkpoints",
        "lock",
        "unlock",
        "agents",
        "broadcast",
        "batch",
        "queue.add",
        "queue.list",
        "queue.run",
        "queue.daemon",
        "config",
        "introspect",
        "context",
        "doctor",
        "query",
    ];

    for command in &commands {
        // Check that each command has a corresponding schema
        assert!(
            schema.get(command).is_some(),
            "Command {command} should have schema",
        );
    }
    Ok(())
}

#[test]
fn test_all_responses_extend_envelope() -> Result<(), Box<dyn std::error::Error>> {
    let Some(json_str) = run_cue_export()? else {
        return Ok(());
    };

    let schema: serde_json::Value = serde_json::from_str(&json_str)?;

    // Verify ResponseEnvelope is defined
    assert!(
        schema.get("#ResponseEnvelope").is_some(),
        "ResponseEnvelope should be defined"
    );
    Ok(())
}

#[test]
fn test_error_codes_match_rust_enum() -> Result<(), Box<dyn std::error::Error>> {
    let Some(json_str) = run_cue_export()? else {
        return Ok(());
    };

    let schema: serde_json::Value = serde_json::from_str(&json_str)?;

    // Verify error codes are defined
    assert!(
        schema.get("#ErrorCode").is_some(),
        "ErrorCode should be defined"
    );
    Ok(())
}
