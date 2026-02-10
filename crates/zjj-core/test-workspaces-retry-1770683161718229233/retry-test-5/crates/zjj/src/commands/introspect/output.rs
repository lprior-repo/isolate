use zjj_core::introspection::IntrospectOutput;

/// Print introspection output in human-readable format
pub(super) fn print_human_readable(output: &IntrospectOutput) {
    println!("ZJJ Version: {}", output.zjj_version);
    println!();

    println!("Capabilities:");
    println!("  Session Management:");
    for cmd in &output.capabilities.session_management.commands {
        println!("    - {cmd}");
    }
    println!("  Version Control:");
    for cmd in &output.capabilities.version_control.commands {
        println!("    - {cmd}");
    }
    println!("  Introspection:");
    for cmd in &output.capabilities.introspection.commands {
        println!("    - {cmd}");
    }
    println!();

    println!("Dependencies:");
    for (name, info) in &output.dependencies {
        let status = if info.installed { "✓" } else { "✗" };
        let required = if info.required {
            " (required)"
        } else {
            " (optional)"
        };
        let version = info
            .version
            .as_ref()
            .map(|v| format!(" - {v}"))
            .map_or(String::new(), |value| value);
        println!("  {status} {name}{required}{version}");
    }
    println!();

    println!("System State:");
    println!(
        "  Initialized: {}",
        if output.system_state.initialized {
            "yes"
        } else {
            "no"
        }
    );
    println!(
        "  JJ Repo: {}",
        if output.system_state.jj_repo {
            "yes"
        } else {
            "no"
        }
    );

    if let Some(ref path) = output.system_state.config_path {
        println!("  Config: {path}");
    }
    if let Some(ref db) = output.system_state.state_db {
        println!("  Database: {db}");
    }
    println!("  Sessions: {}", output.system_state.sessions_count);
    println!("  Active Sessions: {}", output.system_state.active_sessions);

    println!();

    println!("Prerequisites:");
    println!("  (prerequisites not available in this version)");
    println!();

    println!("Command Help:");
    println!("  (detailed command help not available in this version)");
}
