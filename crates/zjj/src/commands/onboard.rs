//! Onboard command - outputs AGENTS.md template snippet for AI agents
//!
//! This command provides a quick integration snippet that AI agents can paste
//! into their project's AGENTS.md file to document ZJJ usage patterns.

#![allow(dead_code)]

use anyhow::Result;
use serde::Serialize;

/// Output structure for onboard command
#[derive(Debug, Serialize)]
pub struct OnboardOutput {
    /// Success status
    pub success: bool,
    /// Markdown snippet for AGENTS.md
    pub snippet: String,
    /// Path to full AI guide
    pub docs_path: String,
    /// Essential commands list
    pub essential_commands: Vec<CommandInfo>,
}

/// Single command information
#[derive(Debug, Serialize)]
pub struct CommandInfo {
    /// Command name
    pub command: String,
    /// Brief description
    pub description: String,
}

/// Run the onboard command
///
/// Outputs a template snippet that AI agents can use to document ZJJ
/// in their project's AGENTS.md file.
///
/// # Arguments
/// * `json` - Whether to output in JSON format
///
/// # Returns
/// Returns `Ok(())` on success with exit code 0
pub fn run(json: bool) -> Result<()> {
    let snippet = generate_snippet();
    let essential_commands = get_essential_commands();

    if json {
        let output = OnboardOutput {
            success: true,
            snippet,
            docs_path: "docs/12_AI_GUIDE.md".to_string(),
            essential_commands,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        let snippet = generate_snippet();
        println!("{snippet}");
    }

    Ok(())
}

/// Generate the markdown snippet for AGENTS.md
fn generate_snippet() -> String {
    r"## ZJJ Session Management

This project uses ZJJ for JJ workspace + Zellij session management.

### Quick Start

Initialize once per repository:
```bash
jjz init
```

### Essential Commands

- `jjz context --json` - Get full environment state (AI agents: run this first)
- `jjz add <name>` - Create new session (workspace + Zellij tab)
- `jjz list --json` - Show all sessions
- `jjz status [name]` - Check session details
- `jjz sync [name]` - Rebase current session on main
- `jjz remove <name>` - Cleanup session when done
- `jjz doctor` - Check system health
- `jjz introspect --json` - Discover all commands (machine-readable)

### Workflow Pattern

```bash
jjz add feature-auth      # Create isolated session
# [work in session]
jjz sync                  # Rebase on main
jjz remove feature-auth   # Cleanup when done
```

### AI Agent Integration

All commands support `--json` for structured output with semantic exit codes:
- 0 = success
- 1 = user error
- 2 = system error
- 3 = not found
- 4 = invalid state

**Context Discovery**: Run `jjz context --json` to get complete environment state.

**Full Documentation**: See `docs/12_AI_GUIDE.md`

### Key Concepts

- **Session**: Named development task with JJ workspace + Zellij tab + DB record
- **Workspace**: Isolated JJ working directory (like git worktree)
- **Sync Strategy**: Rebase (`jj rebase -d main`)
- **Tab Naming**: `jjz:<session-name>`
"
    .to_string()
}

/// Get list of essential commands with descriptions
fn get_essential_commands() -> Vec<CommandInfo> {
    vec![
        CommandInfo {
            command: "jjz context --json".to_string(),
            description: "Get full environment state (AI agents: run this first)".to_string(),
        },
        CommandInfo {
            command: "jjz add <name>".to_string(),
            description: "Create new session with workspace and Zellij tab".to_string(),
        },
        CommandInfo {
            command: "jjz list --json".to_string(),
            description: "Show all sessions with status".to_string(),
        },
        CommandInfo {
            command: "jjz status [name]".to_string(),
            description: "Check detailed session information".to_string(),
        },
        CommandInfo {
            command: "jjz sync [name]".to_string(),
            description: "Rebase session on main branch".to_string(),
        },
        CommandInfo {
            command: "jjz remove <name>".to_string(),
            description: "Cleanup session and workspace".to_string(),
        },
        CommandInfo {
            command: "jjz doctor".to_string(),
            description: "Check system health and dependencies".to_string(),
        },
        CommandInfo {
            command: "jjz introspect --json".to_string(),
            description: "Discover all commands (machine-readable)".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_onboard_text_output() {
        let result = run(false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_onboard_json_output() {
        let result = run(true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_snippet_contains_essential_info() {
        let snippet = generate_snippet();

        // Should mention key concepts
        assert!(snippet.contains("ZJJ Session Management"));
        assert!(snippet.contains("context --json"));
        assert!(snippet.contains("add <name>"));
        assert!(snippet.contains("list --json"));
        assert!(snippet.contains("sync"));
        assert!(snippet.contains("remove"));
        assert!(snippet.contains("doctor"));
        assert!(snippet.contains("introspect --json"));

        // Should document exit codes
        assert!(snippet.contains("0 = success"));
        assert!(snippet.contains("semantic exit codes"));

        // Should reference full docs
        assert!(snippet.contains("docs/12_AI_GUIDE.md"));

        // Should document workflow pattern
        assert!(snippet.contains("Workflow Pattern"));
    }

    #[test]
    fn test_essential_commands_complete() {
        let commands = get_essential_commands();

        // Should have 8 essential commands
        assert_eq!(commands.len(), 8);

        // Verify key commands are present
        let command_names: Vec<_> = commands.iter().map(|c| c.command.as_str()).collect();
        assert!(command_names.contains(&"jjz context --json"));
        assert!(command_names.contains(&"jjz add <name>"));
        assert!(command_names.contains(&"jjz list --json"));
        assert!(command_names.contains(&"jjz introspect --json"));

        // All commands should have descriptions
        for cmd in commands {
            assert!(!cmd.command.is_empty());
            assert!(!cmd.description.is_empty());
        }
    }

    #[test]
    fn test_snippet_is_valid_markdown() {
        let snippet = generate_snippet();

        // Should have markdown headers
        assert!(snippet.contains("## ZJJ Session Management"));
        assert!(snippet.contains("### Quick Start"));
        assert!(snippet.contains("### Essential Commands"));
        assert!(snippet.contains("### Workflow Pattern"));
        assert!(snippet.contains("### AI Agent Integration"));
        assert!(snippet.contains("### Key Concepts"));

        // Should have code blocks
        assert!(snippet.contains("```bash"));

        // Should have bullet lists
        assert!(snippet.contains("- `jjz"));
    }
}
