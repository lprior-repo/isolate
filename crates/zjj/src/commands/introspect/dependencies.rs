use im::HashMap;
use zjj_core::introspection::DependencyInfo;

use crate::cli::{is_command_available, run_command};

/// Get version of a command by running `command --version`
async fn get_command_version(command: &str) -> Option<String> {
    run_command(command, &["--version"])
        .await
        .ok()
        .and_then(|output| output.lines().next().map(|line| line.trim().to_string()))
}

/// Check dependencies and their status
pub(super) async fn check_dependencies() -> HashMap<String, DependencyInfo> {
    // JJ (required)
    let jj_installed = is_command_available("jj").await;
    let jj_info = DependencyInfo {
        required: true,
        installed: jj_installed,
        version: if jj_installed {
            get_command_version("jj").await
        } else {
            None
        },
        command: "jj".to_string(),
    };

    // Zellij (required)
    let zellij_installed = is_command_available("zellij").await;
    let zellij_info = DependencyInfo {
        required: true,
        installed: zellij_installed,
        version: if zellij_installed {
            get_command_version("zellij").await
        } else {
            None
        },
        command: "zellij".to_string(),
    };

    // Claude (optional)
    let claude_installed = is_command_available("claude").await;
    let claude_info = DependencyInfo {
        required: false,
        installed: claude_installed,
        version: if claude_installed {
            get_command_version("claude").await
        } else {
            None
        },
        command: "claude".to_string(),
    };

    // Beads (optional)
    let beads_installed = is_command_available("br").await;
    let beads_info = DependencyInfo {
        required: false,
        installed: beads_installed,
        version: if beads_installed {
            get_command_version("br").await
        } else {
            None
        },
        command: "br".to_string(),
    };

    HashMap::new()
        .update("jj".to_string(), jj_info)
        .update("zellij".to_string(), zellij_info)
        .update("claude".to_string(), claude_info)
        .update("beads".to_string(), beads_info)
}
