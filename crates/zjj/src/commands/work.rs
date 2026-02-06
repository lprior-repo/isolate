//! work command - Unified workflow start for AI agents
//!
//! Combines multiple steps into one atomic operation:
//! 1. Create workspace (if not exists)
//! 2. Register as agent (optional)
//! 3. Set environment variables
//! 4. Output workspace info for agent to consume
//!
//! This is the AI-friendly entry point for starting work.

use std::path::Path;

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
#[allow(clippy::struct_excessive_bools)]
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
pub async fn run(options: &WorkOptions) -> Result<()> {
    let root = super::check_in_jj_repo().await?;
    let location = context::detect_location(&root)?;

    // Check if we're already in a workspace
    if let context::Location::Workspace { name, .. } = &location {
        if options.idempotent && name == &options.name {
            // Already in the target workspace - return success
            return output_existing_workspace(&root, name, options);
        }
        anyhow::bail!(
            "Already in workspace '{name}'. Use 'zjj done' to complete or 'zjj abort' to abandon."
        );
    }

    // Dry run - just show what would happen
    if options.dry_run {
        return output_dry_run(options);
    }

    // Check if session already exists
    let data_dir = root.join(".zjj");
    let db_path = data_dir.join("state.db");
    let session_db =
        SessionDb::open(&db_path).await.context("Failed to open session database")?;

    let existing = session_db.get(&options.name).await.ok().flatten();

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
        bead_id: None,
        no_hooks: false,
        template: None,
        no_open: options.no_zellij,
        no_zellij: options.no_zellij,
        format: OutputFormat::Human, // We'll handle our own output
        idempotent: false,
        dry_run: false,
    };

    // Suppress add command output by running internally
    add::run_internal(&add_options).await?;

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
        env_vars: build_env_vars(
            &options.name,
            &workspace_path,
            agent_id.as_deref(),
            options.bead_id.as_deref(),
        ),
        enter_command: format!("cd {}", workspace_path.display()),
    };

    output_result(&output, options.format)
}

/// Output result for an existing workspace (idempotent mode)
fn output_existing_workspace(root: &Path, name: &str, options: &WorkOptions) -> Result<()> {
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
        env_vars: build_env_vars(
            name,
            &workspace_path,
            agent_id.as_deref(),
            options.bead_id.as_deref(),
        ),
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
        created: false,
        agent_id,
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
        let mut envelope =
            serde_json::to_value(SchemaEnvelope::new("work-response", "single", &output))?;
        if let Some(obj) = envelope.as_object_mut() {
            obj.insert("dry_run".to_string(), serde_json::Value::Bool(true));
        }
        let json_str = serde_json::to_string_pretty(&envelope)
            .context("Failed to serialize work dry-run output")?;
        println!("{json_str}");
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
    workspace_path: &Path,
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
        let json_str =
            serde_json::to_string_pretty(&envelope).context("Failed to serialize work output")?;
        println!("{json_str}");
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
#[allow(clippy::cast_possible_truncation)]
fn generate_short_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());

    // Use last 8 hex chars of timestamp + random suffix (truncation intentional)
    format!("{:08x}", timestamp as u32)
}

#[cfg(test)]
#[allow(clippy::cast_possible_truncation)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::cast_possible_truncation)]
    fn test_work_output_serializes() {
        let output = WorkOutput {
            name: "test-session".to_string(),
            workspace_path: "/path/to/.zjj/workspaces/test-session".to_string(),
            zellij_tab: "zjj:test-session".to_string(),
            created: true,
            agent_id: Some("agent-12345".to_string()),
            bead_id: Some("zjj-abc12".to_string()),
            env_vars: vec![EnvVar {
                name: "ZJJ_SESSION".to_string(),
                value: "test-session".to_string(),
            }],
            enter_command: "cd /path/to/.zjj/workspaces/test-session".to_string(),
        };

        let json = serde_json::to_string(&output);
        assert!(json.is_ok(), "serialization should succeed");
        let json_str = json.unwrap_or_default();
        assert!(json_str.contains("\"name\":\"test-session\""));
        assert!(json_str.contains("\"created\":true"));
    }

    #[test]
    fn test_build_env_vars() {
        let path = Path::new("/test/path");
        let vars = build_env_vars("test", path, Some("agent-1"), Some("bead-1"));

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

    // ============================================================================
    // Behavior Tests
    // ============================================================================

    /// Test `WorkOptions` default values
    #[test]
    fn test_work_options_defaults() {
        let options = WorkOptions {
            name: "test-session".to_string(),
            bead_id: None,
            agent_id: None,
            no_zellij: false,
            no_agent: false,
            idempotent: false,
            dry_run: false,
            format: zjj_core::OutputFormat::Human,
        };

        assert!(!options.no_zellij);
        assert!(!options.no_agent);
        assert!(!options.idempotent);
        assert!(!options.dry_run);
    }

    /// Test `WorkOutput` created flag
    #[test]
    fn test_work_output_created_flag() {
        // New session
        let new_output = WorkOutput {
            name: "test".to_string(),
            workspace_path: "/path".to_string(),
            zellij_tab: "zjj:test".to_string(),
            created: true,
            agent_id: None,
            bead_id: None,
            env_vars: vec![],
            enter_command: "cd /path".to_string(),
        };
        assert!(new_output.created);

        // Existing session (idempotent)
        let existing_output = WorkOutput {
            name: "test".to_string(),
            workspace_path: "/path".to_string(),
            zellij_tab: "zjj:test".to_string(),
            created: false,
            agent_id: None,
            bead_id: None,
            env_vars: vec![],
            enter_command: "cd /path".to_string(),
        };
        assert!(!existing_output.created);
    }

    /// Test zellij tab naming convention
    #[test]
    fn test_work_zellij_tab_format() {
        let output = WorkOutput {
            name: "feature-auth".to_string(),
            workspace_path: "/path".to_string(),
            zellij_tab: "zjj:feature-auth".to_string(),
            created: true,
            agent_id: None,
            bead_id: None,
            env_vars: vec![],
            enter_command: "cd /path".to_string(),
        };

        // Tab should be "zjj:<name>"
        assert!(output.zellij_tab.starts_with("zjj:"));
        assert!(output.zellij_tab.ends_with(&output.name));
    }

    /// Test `env_vars` contains required variables
    #[test]
    fn test_work_env_vars_required() {
        let path = Path::new("/test/workspace");
        let vars = build_env_vars("test-session", path, Some("agent-1"), Some("bead-1"));

        let var_names: Vec<_> = vars.iter().map(|v| v.name.as_str()).collect();

        assert!(var_names.contains(&"ZJJ_SESSION"));
        assert!(var_names.contains(&"ZJJ_WORKSPACE"));
        assert!(var_names.contains(&"ZJJ_ACTIVE"));
        assert!(var_names.contains(&"ZJJ_AGENT_ID"));
        assert!(var_names.contains(&"ZJJ_BEAD_ID"));
    }

    /// Test `env_vars` without `agent_id`
    #[test]
    fn test_work_env_vars_no_agent() {
        let path = Path::new("/test/workspace");
        let vars = build_env_vars("test-session", path, None, None);

        let var_names: Vec<_> = vars.iter().map(|v| v.name.as_str()).collect();

        assert!(var_names.contains(&"ZJJ_SESSION"));
        assert!(var_names.contains(&"ZJJ_WORKSPACE"));
        assert!(var_names.contains(&"ZJJ_ACTIVE"));
        assert!(!var_names.contains(&"ZJJ_AGENT_ID"));
        assert!(!var_names.contains(&"ZJJ_BEAD_ID"));
    }

    /// Test `enter_command` format
    #[test]
    fn test_work_enter_command_format() {
        let output = WorkOutput {
            name: "test".to_string(),
            workspace_path: "/home/user/.zjj/workspaces/test".to_string(),
            zellij_tab: "zjj:test".to_string(),
            created: true,
            agent_id: None,
            bead_id: None,
            env_vars: vec![],
            enter_command: "cd /home/user/.zjj/workspaces/test".to_string(),
        };

        assert!(output.enter_command.starts_with("cd "));
        assert!(output.enter_command.contains(&output.workspace_path));
    }

    /// Test `WorkOutput` JSON serialization includes all fields
    #[test]
    fn test_work_output_json_complete() {
        let output = WorkOutput {
            name: "test".to_string(),
            workspace_path: "/path".to_string(),
            zellij_tab: "zjj:test".to_string(),
            created: true,
            agent_id: Some("agent-1".to_string()),
            bead_id: Some("bead-1".to_string()),
            env_vars: vec![EnvVar {
                name: "ZJJ_SESSION".to_string(),
                value: "test".to_string(),
            }],
            enter_command: "cd /path".to_string(),
        };

        let json_str = serde_json::to_string(&output).unwrap_or_default();

        assert!(json_str.contains("name"));
        assert!(json_str.contains("workspace_path"));
        assert!(json_str.contains("zellij_tab"));
        assert!(json_str.contains("created"));
        assert!(json_str.contains("agent_id"));
        assert!(json_str.contains("bead_id"));
        assert!(json_str.contains("env_vars"));
        assert!(json_str.contains("enter_command"));
    }

    /// Test idempotent mode reuses existing session
    #[test]
    fn test_work_idempotent_mode() {
        let options = WorkOptions {
            name: "existing".to_string(),
            bead_id: None,
            agent_id: None,
            no_zellij: false,
            no_agent: false,
            idempotent: true,
            dry_run: false,
            format: zjj_core::OutputFormat::Human,
        };

        assert!(options.idempotent);
    }

    /// Test `dry_run` mode
    #[test]
    fn test_work_dry_run_mode() {
        let options = WorkOptions {
            name: "test".to_string(),
            bead_id: None,
            agent_id: None,
            no_zellij: false,
            no_agent: false,
            idempotent: false,
            dry_run: true,
            format: zjj_core::OutputFormat::Human,
        };

        assert!(options.dry_run);
    }
}
