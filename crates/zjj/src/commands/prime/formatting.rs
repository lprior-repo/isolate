//! Output formatting for prime command
//!
//! This module handles formatting of prime context output to markdown
//! format for human consumption.

use super::output_types::PrimeOutput;

/// Print markdown-formatted context
///
/// Formats and displays the prime output as human-readable markdown.
pub fn print_markdown_context(output: &PrimeOutput) {
    println!("# ZJJ Workflow Context");
    println!();
    println!("> **Context Recovery**: Run `zjj prime` after compaction or new session");
    println!("> AI agents: This provides essential workflow context for ZJJ");
    println!();

    // JJ Repository Status
    println!("## JJ Repository Status");
    println!();
    if output.jj_status.in_repo {
        if let Some(ref root) = output.jj_status.repo_root {
            println!("- **Root**: `{}`", root);
        }
        if let Some(ref bookmark) = output.jj_status.current_bookmark {
            println!("- **Current Bookmark**: `{}`", bookmark);
        } else {
            println!("- **Current Bookmark**: (none)");
        }
        if output.jj_status.has_changes {
            println!("- **Changes**: Yes");
            if let Some(ref summary) = output.jj_status.change_summary {
                println!("```");
                println!("{}", summary);
                println!("```");
            }
        } else {
            println!("- **Changes**: No changes");
        }
    } else {
        println!("⚠️  Not in a JJ repository");
        println!();
        println!("Run `jj init` or `zjj init` to initialize.");
    }
    println!();

    // ZJJ Status
    println!("## ZJJ Status");
    println!();
    if output.zjj_status.initialized {
        if let Some(ref dir) = output.zjj_status.data_dir {
            println!("- **Data Dir**: `{}`", dir);
        }
        println!("- **Total Sessions**: {}", output.zjj_status.total_sessions);
        println!(
            "- **Active Sessions**: {}",
            output.zjj_status.active_sessions
        );
    } else {
        println!("⚠️  ZJJ not initialized");
        println!();
        println!("Run `zjj init` to initialize ZJJ in this repository.");
    }
    println!();

    // Active Sessions
    if !output.sessions.is_empty() {
        println!("## Active Sessions");
        println!();
        for session in &output.sessions {
            println!("### {}", session.name);
            println!("- **Status**: {}", session.status);
            println!("- **Workspace**: `{}`", session.workspace_path);
            println!("- **Zellij Tab**: `{}`", session.zellij_tab);
            println!();
        }
    }

    // Essential Commands
    println!("## Essential Commands");
    println!();

    println!("### Session Lifecycle");
    for cmd in &output.commands.session_lifecycle {
        println!("- `{}` - {}", cmd.name, cmd.description);
    }
    println!();

    println!("### Workspace Sync");
    for cmd in &output.commands.workspace_sync {
        println!("- `{}` - {}", cmd.name, cmd.description);
    }
    println!();

    println!("### System");
    for cmd in &output.commands.system {
        println!("- `{}` - {}", cmd.name, cmd.description);
    }
    println!();

    println!("### Introspection (For AI Agents)");
    for cmd in &output.commands.introspection {
        println!("- `{}` - {}", cmd.name, cmd.description);
    }
    println!();

    // Common Workflows
    println!("## Common Workflows");
    println!();

    for workflow in &output.workflows {
        println!("### {}", workflow.title);
        println!("```bash");
        for step in &workflow.steps {
            println!("{}", step);
        }
        println!("```");
        println!();
    }

    // Beads Integration
    if output.beads_status.available {
        println!("## Beads Integration");
        println!();
        println!("✓ Beads is available for issue tracking");
        if let Some(ref dir) = output.beads_status.beads_dir {
            println!("- **Beads Dir**: `{}`", dir);
        }
        println!();
        println!("**Essential Beads Commands**:");
        println!("- `bd ready` - Find available work");
        println!("- `bd show <id>` - View issue details");
        println!("- `bd update <id> --status in_progress` - Claim work");
        println!("- `bd close <id>` - Complete work");
        println!("- `bd sync` - Sync with git");
        println!();
    }

    // Core Rules
    println!("## Core Rules");
    println!();
    println!("1. **Zero Panics**: No `.unwrap()`, `.expect()`, or `panic!()`");
    println!("2. **Functional Patterns**: Use `?`, `map`, `and_then` for error handling");
    println!("3. **Moon Build System**: Always use `moon run :test`, never raw `cargo`");
    println!("4. **JSON Support**: All commands support `--json` flag");
    println!(
        "5. **Exit Codes**: 0=success, 1=user error, 2=system error, 3=not found, 4=invalid state"
    );
    println!();

    // AI Agent Notes
    println!("## AI Agent Notes");
    println!();
    println!("- Use `zjj context --json` for complete environment snapshot");
    println!("- Use `zjj introspect --json` for CLI metadata");
    println!("- All operations support `--dry-run` for preview");
    println!("- Check CLAUDE.md and docs/12_AI_GUIDE.md for detailed patterns");
    println!();
}
