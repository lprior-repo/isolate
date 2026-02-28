//! Can-I command - Check if an action is permitted
//!
//! Allows AI agents to check preconditions before attempting operations.

use anyhow::Result;
use isolate_core::{OutputFormat, SchemaEnvelope};
use serde::{Deserialize, Serialize};

use crate::commands::get_session_db;

/// Options for the can-i command
#[derive(Debug, Clone)]
pub struct CanIOptions {
    /// Action to check
    pub action: String,
    /// Resource to check (optional, for resource-specific checks)
    pub resource: Option<String>,
    /// Output format
    pub format: OutputFormat,
}

/// Result of can-i check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanIResult {
    /// Whether the action is allowed
    pub allowed: bool,
    /// The action that was checked
    pub action: String,
    /// The resource if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
    /// Reason for the result
    pub reason: String,
    /// Prerequisites that must be met
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub prerequisites: Vec<Prerequisite>,
    /// Commands to run to make it possible
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub fix_commands: Vec<String>,
}

/// A prerequisite check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prerequisite {
    /// What needs to be checked
    pub check: String,
    /// Whether it passes
    pub passed: bool,
    /// Description
    pub description: String,
}

/// Run the can-i command
pub async fn run(options: &CanIOptions) -> Result<()> {
    let result = check_permission(&options.action, options.resource.as_deref()).await?;

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("can-i-response", "single", &result);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else if result.allowed {
        println!("✓ Yes, you can: {}", options.action);
        if let Some(resource) = &result.resource {
            println!("  Resource: {resource}");
        }
        println!("  Reason: {}", result.reason);
    } else {
        println!("✗ No, you cannot: {}", options.action);
        if let Some(resource) = &result.resource {
            println!("  Resource: {resource}");
        }
        println!("  Reason: {}", result.reason);

        if !result.prerequisites.is_empty() {
            println!();
            println!("Prerequisites:");
            result.prerequisites.iter().for_each(|prereq| {
                let icon = if prereq.passed { "✓" } else { "✗" };
                println!("  {icon} {}: {}", prereq.check, prereq.description);
            });
        }

        if !result.fix_commands.is_empty() {
            println!();
            println!("To fix, run:");
            result.fix_commands.iter().for_each(|cmd| {
                println!("  {cmd}");
            });
        }
    }

    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
async fn check_permission(action: &str, resource: Option<&str>) -> Result<CanIResult> {
    match action {
        "add" | "work" => Ok(check_can_add(resource).await),
        "remove" => Ok(check_can_remove(resource).await),
        "done" => Ok(check_can_done(resource).await),
        "undo" => Ok(check_can_undo().await),
        "sync" => Ok(check_can_sync(resource).await),
        "spawn" => Ok(check_can_spawn(resource).await),
        "merge" => Ok(check_can_merge(resource).await),
        _ => Ok(CanIResult {
            allowed: true,
            action: action.to_string(),
            resource: resource.map(String::from),
            reason: "Action is generally allowed".to_string(),
            prerequisites: vec![],
            fix_commands: vec![],
        }),
    }
}

async fn check_can_add(resource: Option<&str>) -> CanIResult {
    let mut prerequisites = Vec::new();

    // Check if isolate is initialized
    let db_result = get_session_db().await;
    let isolate_initialized = db_result.is_ok();
    prerequisites.push(Prerequisite {
        check: "isolate_initialized".to_string(),
        passed: isolate_initialized,
        description: if isolate_initialized {
            "Isolate is initialized".to_string()
        } else {
            "Isolate not initialized".to_string()
        },
    });

    // Check if session name already exists
    let name_available = match (resource, &db_result) {
        (Some(name), Ok(db)) => db.get(name).await.is_ok_and(|session| session.is_none()),
        _ => true,
    };
    if resource.is_some() {
        prerequisites.push(Prerequisite {
            check: "name_available".to_string(),
            passed: name_available,
            description: if name_available {
                "Session name is available".to_string()
            } else {
                "Session name already exists".to_string()
            },
        });
    }

    let allowed = isolate_initialized && name_available;
    let reason = if allowed {
        "Can create session".to_string()
    } else if !isolate_initialized {
        "Isolate not initialized".to_string()
    } else {
        "Session name already exists".to_string()
    };

    let fix_commands = if !isolate_initialized {
        vec!["isolate init".to_string()]
    } else if !name_available {
        resource.map_or_else(Vec::new, |name| vec![format!("isolate remove {name}")])
    } else {
        vec![]
    };

    CanIResult {
        allowed,
        action: "add".to_string(),
        resource: resource.map(String::from),
        reason,
        prerequisites,
        fix_commands,
    }
}

async fn check_can_remove(resource: Option<&str>) -> CanIResult {
    let mut prerequisites = Vec::new();

    let db_result = get_session_db().await;
    let isolate_initialized = db_result.is_ok();

    prerequisites.push(Prerequisite {
        check: "isolate_initialized".to_string(),
        passed: isolate_initialized,
        description: if isolate_initialized {
            "Isolate is initialized".to_string()
        } else {
            "Isolate not initialized".to_string()
        },
    });

    // Check if session exists
    let session_exists = match (resource, &db_result) {
        (Some(name), Ok(db)) => db.get(name).await.is_ok_and(|session| session.is_some()),
        _ => false,
    };
    if resource.is_some() {
        prerequisites.push(Prerequisite {
            check: "session_exists".to_string(),
            passed: session_exists,
            description: if session_exists {
                "Session exists".to_string()
            } else {
                "Session does not exist".to_string()
            },
        });
    }

    let allowed = isolate_initialized && (resource.is_none() || session_exists);
    let reason = if allowed {
        "Can remove session".to_string()
    } else if !isolate_initialized {
        "Isolate not initialized".to_string()
    } else {
        "Session does not exist".to_string()
    };

    CanIResult {
        allowed,
        action: "remove".to_string(),
        resource: resource.map(String::from),
        reason,
        prerequisites,
        fix_commands: vec![],
    }
}

async fn check_can_done(resource: Option<&str>) -> CanIResult {
    let mut prerequisites = Vec::new();

    let db_result = get_session_db().await;
    let isolate_initialized = db_result.is_ok();

    prerequisites.push(Prerequisite {
        check: "isolate_initialized".to_string(),
        passed: isolate_initialized,
        description: if isolate_initialized {
            "Isolate is initialized".to_string()
        } else {
            "Isolate not initialized".to_string()
        },
    });

    // Check if we're in a workspace or session is specified
    let in_workspace = std::env::current_dir().is_ok_and(|p| p.join(".jj").exists());
    prerequisites.push(Prerequisite {
        check: "in_workspace".to_string(),
        passed: in_workspace || resource.is_some(),
        description: if in_workspace {
            "In a JJ workspace".to_string()
        } else if resource.is_some() {
            "Session specified".to_string()
        } else {
            "Not in a workspace and no session specified".to_string()
        },
    });

    let allowed = isolate_initialized && (in_workspace || resource.is_some());
    let reason = if allowed {
        "Can complete and merge session".to_string()
    } else if !isolate_initialized {
        "Isolate not initialized".to_string()
    } else {
        "Not in a workspace - specify session or cd to workspace".to_string()
    };

    CanIResult {
        allowed,
        action: "done".to_string(),
        resource: resource.map(String::from),
        reason,
        prerequisites,
        fix_commands: vec![],
    }
}

async fn check_can_undo() -> CanIResult {
    let mut prerequisites = Vec::new();

    // Check if undo history exists
    let data_dir = super::isolate_data_dir().await;
    let undo_file_exists = data_dir.is_ok_and(|d| d.join("undo-history.jsonl").exists());

    prerequisites.push(Prerequisite {
        check: "undo_history_exists".to_string(),
        passed: undo_file_exists,
        description: if undo_file_exists {
            "Undo history exists".to_string()
        } else {
            "No undo history available".to_string()
        },
    });

    let allowed = undo_file_exists;
    let reason = if allowed {
        "Can undo last operation".to_string()
    } else {
        "No undo history - nothing to undo".to_string()
    };

    CanIResult {
        allowed,
        action: "undo".to_string(),
        resource: None,
        reason,
        prerequisites,
        fix_commands: vec![],
    }
}

async fn check_can_sync(resource: Option<&str>) -> CanIResult {
    let mut prerequisites = Vec::new();

    let db_result = get_session_db().await;
    let isolate_initialized = db_result.is_ok();

    prerequisites.push(Prerequisite {
        check: "isolate_initialized".to_string(),
        passed: isolate_initialized,
        description: if isolate_initialized {
            "Isolate is initialized".to_string()
        } else {
            "Isolate not initialized".to_string()
        },
    });

    // Check if there are sessions to sync
    let has_sessions = match &db_result {
        Ok(db) => db.list(None).await.is_ok_and(|list| !list.is_empty()),
        Err(_) => false,
    };

    prerequisites.push(Prerequisite {
        check: "has_sessions".to_string(),
        passed: has_sessions || resource.is_some(),
        description: if has_sessions {
            "Sessions available to sync".to_string()
        } else if resource.is_some() {
            "Session specified".to_string()
        } else {
            "No sessions to sync".to_string()
        },
    });

    let allowed = isolate_initialized && (has_sessions || resource.is_some());
    let reason = if allowed {
        "Can sync sessions".to_string()
    } else if !isolate_initialized {
        "Isolate not initialized".to_string()
    } else {
        "No sessions to sync".to_string()
    };

    CanIResult {
        allowed,
        action: "sync".to_string(),
        resource: resource.map(String::from),
        reason,
        prerequisites,
        fix_commands: vec![],
    }
}

async fn check_can_spawn(resource: Option<&str>) -> CanIResult {
    let mut prerequisites = Vec::new();

    let db_result = get_session_db().await;
    let isolate_initialized = db_result.is_ok();

    prerequisites.push(Prerequisite {
        check: "isolate_initialized".to_string(),
        passed: isolate_initialized,
        description: if isolate_initialized {
            "Isolate is initialized".to_string()
        } else {
            "Isolate not initialized".to_string()
        },
    });

    // Check if bead ID is provided
    let bead_provided = resource.is_some();
    prerequisites.push(Prerequisite {
        check: "bead_provided".to_string(),
        passed: bead_provided,
        description: if bead_provided {
            "Bead ID provided".to_string()
        } else {
            "No bead ID specified".to_string()
        },
    });

    let allowed = isolate_initialized && bead_provided;
    let reason = if allowed {
        "Can spawn agent session".to_string()
    } else if !isolate_initialized {
        "Isolate not initialized".to_string()
    } else {
        "Bead ID required".to_string()
    };

    CanIResult {
        allowed,
        action: "spawn".to_string(),
        resource: resource.map(String::from),
        reason,
        prerequisites,
        fix_commands: vec![],
    }
}

async fn check_can_merge(resource: Option<&str>) -> CanIResult {
    // Same as done for now
    let mut r = check_can_done(resource).await;
    r.action = "merge".to_string();
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_i_result_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let result = CanIResult {
            allowed: true,
            action: "add".to_string(),
            resource: Some("test-session".to_string()),
            reason: "Can create session".to_string(),
            prerequisites: vec![],
            fix_commands: vec![],
        };

        let json = serde_json::to_string(&result)?;
        assert!(json.contains("\"allowed\":true"));
        assert!(json.contains("\"action\":\"add\""));
        Ok(())
    }

    #[test]
    fn test_can_i_result_with_prerequisites() -> Result<(), Box<dyn std::error::Error>> {
        let result = CanIResult {
            allowed: false,
            action: "spawn".to_string(),
            resource: Some("isolate-abc12".to_string()),
            reason: "Bead ID required".to_string(),
            prerequisites: vec![
                Prerequisite {
                    check: "isolate_initialized".to_string(),
                    passed: true,
                    description: "Isolate is initialized".to_string(),
                },
                Prerequisite {
                    check: "bead_provided".to_string(),
                    passed: false,
                    description: "No bead ID specified".to_string(),
                },
            ],
            fix_commands: vec![],
        };

        let json = serde_json::to_string(&result)?;
        assert!(json.contains("\"allowed\":false"));
        assert!(json.contains("\"prerequisites\""));
        Ok(())
    }

    #[test]
    fn test_prerequisite_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let prereq = Prerequisite {
            check: "test_check".to_string(),
            passed: true,
            description: "Test passed".to_string(),
        };

        let json = serde_json::to_string(&prereq)?;
        assert!(json.contains("\"passed\":true"));
        Ok(())
    }
}
