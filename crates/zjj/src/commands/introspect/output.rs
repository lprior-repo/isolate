//! Output formatting for introspection data
//!
//! This module provides human-readable and JSON formatting for
//! introspection output, including full system capabilities and
//! individual command specifications.

use zjj_core::introspection::{CommandIntrospection, IntrospectOutput};

/// Print introspection output in human-readable format
///
/// Displays:
/// - ZJJ version
/// - Capabilities organized by category
/// - Dependencies with installation status
/// - System state and session statistics
pub fn print_full_output(output: &IntrospectOutput) {
    println!("ZJJ Version: {}", output.zjj_version);
    println!();

    print_capabilities(output);
    print_dependencies(output);
    print_system_state(output);
}

/// Print capabilities section
fn print_capabilities(output: &IntrospectOutput) {
    println!("Capabilities:");
    println!("  Session Management:");
    output
        .capabilities
        .session_management
        .commands
        .iter()
        .for_each(|cmd| {
            println!("    - {cmd}");
        });
    println!("  Version Control:");
    output
        .capabilities
        .version_control
        .commands
        .iter()
        .for_each(|cmd| {
            println!("    - {cmd}");
        });
    println!("  Introspection:");
    output
        .capabilities
        .introspection
        .commands
        .iter()
        .for_each(|cmd| {
            println!("    - {cmd}");
        });
    println!();
}

/// Print dependencies section
fn print_dependencies(output: &IntrospectOutput) {
    println!("Dependencies:");
    output.dependencies.iter().for_each(|(name, info)| {
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
            .unwrap_or_default();
        println!("  {status} {name}{required}{version}");
    });
    println!();
}

/// Print system state section
fn print_system_state(output: &IntrospectOutput) {
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
        "  JJ Repository: {}",
        if output.system_state.jj_repo {
            "yes"
        } else {
            "no"
        }
    );
    if let Some(ref path) = output.system_state.config_path {
        println!("  Config: {path}");
    }
    if let Some(ref path) = output.system_state.state_db {
        println!("  Database: {path}");
    }
    println!(
        "  Sessions: {} total, {} active",
        output.system_state.sessions_count, output.system_state.active_sessions
    );
}

/// Print command introspection in human-readable format
///
/// Displays detailed information about a specific command:
/// - Description and aliases
/// - Arguments with types and validation
/// - Flags with defaults and possible values
/// - Usage examples
/// - Prerequisites and side effects
pub fn print_command_spec(cmd: &CommandIntrospection) {
    println!("Command: {}", cmd.command);
    println!("Description: {}", cmd.description);
    println!();

    print_arguments(cmd);
    print_flags(cmd);
    print_examples(cmd);
    print_prerequisites(cmd);
}

/// Print arguments section
fn print_arguments(cmd: &CommandIntrospection) {
    if cmd.arguments.is_empty() {
        return;
    }

    println!("Arguments:");
    cmd.arguments.iter().for_each(|arg| {
        let required = if arg.required {
            " (required)"
        } else {
            " (optional)"
        };
        println!("  {}{required}", arg.name);
        println!("    Type: {}", arg.arg_type);
        println!("    Description: {}", arg.description);
        if !arg.examples.is_empty() {
            println!("    Examples: {}", arg.examples.join(", "));
        }
    });
    println!();
}

/// Print flags section
fn print_flags(cmd: &CommandIntrospection) {
    if cmd.flags.is_empty() {
        return;
    }

    println!("Flags:");
    cmd.flags.iter().for_each(|flag| {
        let short = flag
            .short
            .as_ref()
            .map(|s| format!("-{s}, "))
            .unwrap_or_default();
        println!("  {short}--{}", flag.long);
        println!("    Type: {}", flag.flag_type);
        println!("    Description: {}", flag.description);
        if let Some(ref default) = flag.default {
            println!("    Default: {default}");
        }
        if !flag.possible_values.is_empty() {
            println!("    Values: {}", flag.possible_values.join(", "));
        }
    });
    println!();
}

/// Print examples section
fn print_examples(cmd: &CommandIntrospection) {
    if cmd.examples.is_empty() {
        return;
    }

    println!("Examples:");
    cmd.examples.iter().for_each(|example| {
        println!("  {}", example.command);
        println!("    {}", example.description);
    });
    println!();
}

/// Print prerequisites section
fn print_prerequisites(cmd: &CommandIntrospection) {
    println!("Prerequisites:");
    println!("  Initialized: {}", cmd.prerequisites.initialized);
    println!("  JJ Installed: {}", cmd.prerequisites.jj_installed);
    println!("  Zellij Running: {}", cmd.prerequisites.zellij_running);
}
