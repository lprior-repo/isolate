//! whoami command - Agent identity query
//!
//! Returns the current agent identity:
//! - `unregistered` - No agent registered
//! - `<agent-id>` - Agent ID if registered
//!
//! Also checks environment variables for agent context.

use anyhow::Result;
use serde::Serialize;
use zjj_core::{json::SchemaEnvelope, OutputFormat};

/// Output for whoami command
#[derive(Debug, Clone, Serialize)]
pub struct WhoAmIOutput {
    /// Whether an agent is registered
    pub registered: bool,
    /// Agent ID if registered
    pub agent_id: Option<String>,
    /// Current session being worked on
    pub current_session: Option<String>,
    /// Current bead being worked on (from env var)
    pub current_bead: Option<String>,
    /// Simple one-line representation
    pub simple: String,
}

/// Options for whoami command
pub struct WhoAmIOptions {
    pub format: OutputFormat,
}

/// Run the whoami command
///
/// # Errors
///
/// Returns an error if unable to determine identity
pub fn run(options: &WhoAmIOptions) -> Result<()> {
    // Check environment variables for agent context
    let env_agent_id = std::env::var("ZJJ_AGENT_ID").ok();
    let env_bead_id = std::env::var("ZJJ_BEAD_ID").ok();
    let env_workspace = std::env::var("ZJJ_WORKSPACE").ok();
    let env_session = std::env::var("ZJJ_SESSION").ok();

    // Determine current session from workspace path if not set
    let current_session = env_session.or_else(|| {
        env_workspace
            .as_ref()
            .and_then(|p| std::path::Path::new(p).file_name())
            .and_then(|n| n.to_str())
            .map(String::from)
    });

    let output = if let Some(agent_id) = env_agent_id {
        WhoAmIOutput {
            registered: true,
            agent_id: Some(agent_id.clone()),
            current_session,
            current_bead: env_bead_id,
            simple: agent_id,
        }
    } else {
        WhoAmIOutput {
            registered: false,
            agent_id: None,
            current_session,
            current_bead: env_bead_id,
            simple: "unregistered".to_string(),
        }
    };

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("whoami-response", "single", &output);
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope)
                .unwrap_or_else(|_| r#"{"error": "serialization failed"}"#.to_string())
        );
    } else {
        // Simple output - just the identity
        println!("{}", output.simple);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whoami_output_unregistered() {
        let output = WhoAmIOutput {
            registered: false,
            agent_id: None,
            current_session: None,
            current_bead: None,
            simple: "unregistered".to_string(),
        };

        assert!(!output.registered);
        assert_eq!(output.simple, "unregistered");
    }

    #[test]
    fn test_whoami_output_registered() {
        let output = WhoAmIOutput {
            registered: true,
            agent_id: Some("agent-12345".to_string()),
            current_session: Some("feature-auth".to_string()),
            current_bead: Some("zjj-abc12".to_string()),
            simple: "agent-12345".to_string(),
        };

        assert!(output.registered);
        assert_eq!(output.simple, "agent-12345");
        assert_eq!(output.agent_id, Some("agent-12345".to_string()));
    }

    #[test]
    fn test_whoami_output_serializes() {
        let output = WhoAmIOutput {
            registered: true,
            agent_id: Some("agent-12345".to_string()),
            current_session: None,
            current_bead: None,
            simple: "agent-12345".to_string(),
        };

        let json = serde_json::to_string(&output);
        assert!(json.is_ok());
        let json_str = json.unwrap_or_default();
        assert!(json_str.contains("\"registered\":true"));
        assert!(json_str.contains("\"agent_id\":\"agent-12345\""));
    }
}
