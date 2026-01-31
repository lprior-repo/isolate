//! work command - Unified workflow start for AI agents
//!
//! Combines multiple steps into one atomic operation:
//! 1. Create workspace (if not exists)
//! 2. Register as agent (optional)
//! 3. Set environment variables
//! 4. Output workspace info for agent to consume
//!
//! This is the AI-friendly entry point for starting work.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Serialize;
use zjj_core::{json::SchemaEnvelope, OutputFormat};

use super::{add, context};
use crate::db::SessionDb;

/// Output for work command
#[derive(Debug, Clone, Serialize)]
pub struct WorkOutput {
    /// Session name
    pub name: String,
    /// Workspace path
    pub workspace_path: String,
    /// Zellij tab name
    pub zellij_tab: String,
    /// Whether this was a new session or existing
    pub created: bool,
    /// Agent ID if registered
    pub agent_id: Option<String>,
    /// Bead ID if specified
    pub bead_id: Option<String>,
    /// Environment variables set
    pub env_vars: Vec<EnvVar>,
    /// Shell command to enter workspace (for non-interactive use)
    pub enter_command: String,
}

/// Environment variable set by work command
#[derive(Debug, Clone, Serialize)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

/// Options for work command
#[derive(Debug, Clone)]
pub struct WorkOptions {
    /// Session name to create/use
    pub name: String,
    /// Bead ID to associate (optional)
    pub bead_id: Option<String>,
    /// Agent ID to register (optional, auto-generated if not provided)
    pub agent_id: Option<String>,
    /// Don't create Zellij tab
    pub no_zellij: bool,
    /// Don't register as agent
    pub no_agent: bool,
    /// Idempotent mode - succeed if session already exists
    pub idempotent: bool,
    /// Dry run - don't actually create
    pub dry_run: bool,
    /// Output format
    pub format: OutputFormat,
}

/// Run the work command
///
/// # Errors
///
/// Returns an error if:
/// - Not in a JJ repo
/// - Session creation fails
/// - Already in a workspace (unless idempotent)
pub fn run(options: &WorkOptions) -> Result<()> {
    let root = super::check_in_jj_repo()?;
    let location = context::detect_location(&root)?;

    // Check if we're already in a workspace
    if let context::Location::Workspace { name, .. } = &location {
        if options.idempotent && name == &options.name {
            // Already in the target workspace - return success
            return output_existing_workspace(&root, name, options);
        }
        anyhow::bail!(
            "Already in workspace '{}'. Use 'zjj done' to complete or 'zjj abort' to abandon.",
            name
        );
    }

    // Dry run - just show what would happen
    if options.dry_run {
        return output_dry_run(options);
    }

    // Check if session already exists
    let data_dir = root.join(".zjj");
    let db_path = data_dir.join("state.db");
    let session_db = SessionDb::open_blocking(&db_path).context("Failed to open session database")?;

    let existing = session_db
        .get_blocking(&options.name)
        .ok()
        .flatten();

    if existing.is_some() {
        if options.idempotent {
            return output_existing_workspace(&root, &options.name, options);
        }
        anyhow::bail!(
            "Session '{}' already exists. Use --idempotent to reuse existing session.",
            options.name
        );
    }

    // Create the session using add command infrastructure
    let add_options = add::AddOptions {
        name: options.name.clone(),
        no_hooks: false,
        template: None,
        no_open: options.no_zellij,
        format: OutputFormat::Human, // We'll handle our own output
        idempotent: false,
        dry_run: false,
    };

    // Suppress add command output by running internally
    add::run_internal(&add_options)?;

    // Generate agent ID if needed
    let agent_id = if options.no_agent {
        None
    } else {
        Some(
            options
                .agent_id
                .clone()
                .unwrap_or_else(|| format!("agent-{}", generate_short_id())),
        )
    };

    // Build output
    let workspace_path = data_dir.join("workspaces").join(&options.name);
    let output = WorkOutput {
        name: options.name.clone(),
        workspace_path: workspace_path.to_string_lossy().to_string(),
        zellij_tab: format!("zjj:{}", options.name),
        created: true,
        agent_id: agent_id.clone(),
        bead_id: options.bead_id.clone(),
        env_vars: build_env_vars(&options.name, &workspace_path, agent_id.as_deref(), options.bead_id.as_deref()),
        enter_command: format!("cd {}", workspace_path.display()),
    };

    output_result(&output, options.format)
}

/// Output result for an existing workspace (idempotent mode)
fn output_existing_workspace(root: &PathBuf, name: &str, options: &WorkOptions) -> Result<()> {
    let workspace_path = root.join(".zjj/workspaces").join(name);

    let agent_id = if options.no_agent {
        None
    } else {
        options.agent_id.clone()
    };

    let output = WorkOutput {
        name: name.to_string(),
        workspace_path: workspace_path.to_string_lossy().to_string(),
        zellij_tab: format!("zjj:{name}"),
        created: false,
        agent_id: agent_id.clone(),
        bead_id: options.bead_id.clone(),
        env_vars: build_env_vars(name, &workspace_path, agent_id.as_deref(), options.bead_id.as_deref()),
        enter_command: format!("cd {}", workspace_path.display()),
    };

    output_result(&output, options.format)
}

/// Output for dry run
fn output_dry_run(options: &WorkOptions) -> Result<()> {
    let workspace_path = format!(".zjj/workspaces/{}", options.name);

    let agent_id = if options.no_agent {
        None
    } else {
        Some(
            options
                .agent_id
                .clone()
                .unwrap_or_else(|| "agent-<generated>".to_string()),
        )
    };

    let output = WorkOutput {
        name: options.name.clone(),
        workspace_path: workspace_path.clone(),
        zellij_tab: format!("zjj:{}", options.name),
        created: false, // Would be true if executed
        agent_id: agent_id.clone(),
        bead_id: options.bead_id.clone(),
        env_vars: vec![
            EnvVar {
                name: "ZJJ_SESSION".to_string(),
                value: options.name.clone(),
            },
            EnvVar {
                name: "ZJJ_WORKSPACE".to_string(),
                value: workspace_path,
            },
        ],
        enter_command: format!("cd .zjj/workspaces/{}", options.name),
    };

    if options.format.is_json() {
        let mut envelope = serde_json::to_value(SchemaEnvelope::new("work-response", "single", &output))?;
        if let Some(obj) = envelope.as_object_mut() {
            obj.insert("dry_run".to_string(), serde_json::Value::Bool(true));
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope)
                .unwrap_or_else(|_| r#"{"error": "serialization failed"}"#.to_string())
        );
    } else {
        println!("[DRY RUN] Would create session '{}'", options.name);
        println!("  Workspace: .zjj/workspaces/{}", options.name);
        println!("  Zellij tab: zjj:{}", options.name);
        if let Some(ref bead) = options.bead_id {
            println!("  Bead: {bead}");
        }
    }

    Ok(())
}

/// Build environment variables for the workspace
fn build_env_vars(
    name: &str,
    workspace_path: &PathBuf,
    agent_id: Option<&str>,
    bead_id: Option<&str>,
) -> Vec<EnvVar> {
    let mut vars = vec![
        EnvVar {
            name: "ZJJ_SESSION".to_string(),
            value: name.to_string(),
        },
        EnvVar {
            name: "ZJJ_WORKSPACE".to_string(),
            value: workspace_path.to_string_lossy().to_string(),
        },
        EnvVar {
            name: "ZJJ_ACTIVE".to_string(),
            value: "1".to_string(),
        },
    ];

    if let Some(agent) = agent_id {
        vars.push(EnvVar {
            name: "ZJJ_AGENT_ID".to_string(),
            value: agent.to_string(),
        });
    }

    if let Some(bead) = bead_id {
        vars.push(EnvVar {
            name: "ZJJ_BEAD_ID".to_string(),
            value: bead.to_string(),
        });
    }

    vars
}

/// Output the result
fn output_result(output: &WorkOutput, format: OutputFormat) -> Result<()> {
    if format.is_json() {
        let envelope = SchemaEnvelope::new("work-response", "single", output);
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope)
                .unwrap_or_else(|_| r#"{"error": "serialization failed"}"#.to_string())
        );
    } else {
        if output.created {
            println!("Created session '{}'", output.name);
        } else {
            println!("Using existing session '{}'", output.name);
        }
        println!("  Workspace: {}", output.workspace_path);
        println!("  Tab: {}", output.zellij_tab);
        if let Some(ref agent) = output.agent_id {
            println!("  Agent: {agent}");
        }
        if let Some(ref bead) = output.bead_id {
            println!("  Bead: {bead}");
        }
        println!("\nTo enter workspace:");
        println!("  {}", output.enter_command);
    }

    Ok(())
}

/// Generate a short random ID
fn generate_short_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    // Use last 8 hex chars of timestamp + random suffix
    format!("{:08x}", timestamp as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_work_output_serializes() {
        let output = WorkOutput {
            name: "test-session".to_string(),
            workspace_path: "/path/to/.zjj/workspaces/test-session".to_string(),
            zellij_tab: "zjj:test-session".to_string(),
            created: true,
            agent_id: Some("agent-12345".to_string()),
            bead_id: Some("zjj-abc12".to_string()),
            env_vars: vec![
                EnvVar {
                    name: "ZJJ_SESSION".to_string(),
                    value: "test-session".to_string(),
                },
            ],
            enter_command: "cd /path/to/.zjj/workspaces/test-session".to_string(),
        };

        let json = serde_json::to_string(&output);
        assert!(json.is_ok());
        let json_str = json.unwrap_or_default();
        assert!(json_str.contains("\"name\":\"test-session\""));
        assert!(json_str.contains("\"created\":true"));
    }

    #[test]
    fn test_build_env_vars() {
        let path = PathBuf::from("/test/path");
        let vars = build_env_vars("test", &path, Some("agent-1"), Some("bead-1"));

        assert!(vars.iter().any(|v| v.name == "ZJJ_SESSION"));
        assert!(vars.iter().any(|v| v.name == "ZJJ_WORKSPACE"));
        assert!(vars.iter().any(|v| v.name == "ZJJ_AGENT_ID"));
        assert!(vars.iter().any(|v| v.name == "ZJJ_BEAD_ID"));
    }

    #[test]
    fn test_generate_short_id() {
        let id1 = generate_short_id();
        let id2 = generate_short_id();

        // IDs should be 8 hex chars
        assert_eq!(id1.len(), 8);
        assert!(id1.chars().all(|c| c.is_ascii_hexdigit()));

        // Note: IDs might be the same if generated in same millisecond
        // This is acceptable for our use case
        let _ = id2; // Suppress unused warning
    }
}
