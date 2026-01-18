//! CLI utilities and helpers

pub mod args;
pub mod dispatch;
pub mod error;
pub mod help_json;
pub mod output;
pub mod setup;

#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::{io::IsTerminal, process::Command};

use anyhow::{Context, Result};
// Re-export error formatting functions for backward compatibility
pub use error::{classify_error_code, format_error, get_exit_code, output_json_error};
// Re-export output formatting functions
pub use output::output_help_json;

/// Default timeout for commands in seconds
const DEFAULT_COMMAND_TIMEOUT_SECS: u64 = 30;

/// Execute a shell command and return its output with timeout (zjj-8q9)
///
/// Uses a 30-second default timeout to prevent hung processes.
/// For operations that may take longer (git push, large rebases), use `run_command_with_timeout`.
pub fn run_command(program: &str, args: &[&str]) -> Result<String> {
    run_command_with_timeout(program, args, DEFAULT_COMMAND_TIMEOUT_SECS)
}

/// Execute a shell command with a custom timeout
///
/// # Arguments
/// * `program` - Program to execute
/// * `args` - Arguments to pass
/// * `timeout_secs` - Timeout in seconds (0 means no timeout)
pub fn run_command_with_timeout(program: &str, args: &[&str], timeout_secs: u64) -> Result<String> {
    use std::{process::Stdio, sync::mpsc::channel, thread, time::Duration};

    // If timeout is 0, use blocking behavior
    if timeout_secs == 0 {
        let output = Command::new(program)
            .args(args)
            .output()
            .with_context(|| format!("Failed to execute {program}"))?;

        return if output.status.success() {
            String::from_utf8(output.stdout).context("Invalid UTF-8 output from command")
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("{program} failed: {stderr}")
        };
    }

    // Spawn child process
    let child = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to spawn {program}"))?;

    // Create channel for timeout
    let (sender, receiver) = channel();

    // Spawn thread to wait for process
    let child_id = child.id();
    thread::spawn(move || {
        let result = child.wait_with_output();
        let _ = sender.send(result);
    });

    // Wait with timeout
    match receiver.recv_timeout(Duration::from_secs(timeout_secs)) {
        Ok(Ok(output)) => {
            if output.status.success() {
                String::from_utf8(output.stdout).context("Invalid UTF-8 output from command")
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("{program} failed: {stderr}")
            }
        }
        Ok(Err(e)) => Err(e).with_context(|| format!("Failed to execute {program}")),
        Err(_timeout) => {
            // Try to kill the process
            #[cfg(unix)]
            {
                use std::process::Command as StdCommand;
                let _ = StdCommand::new("kill")
                    .args(["-9", &child_id.to_string()])
                    .output();
            }

            anyhow::bail!(
                "Command timed out after {timeout_secs}s: {program} {}\n\
                 \n\
                 The command may be hung or waiting for external resources.\n\
                 \n\
                 Suggestions:\n\
                 • Check if {program} is responsive: {program} --version\n\
                 • Check for network issues if the command requires network access\n\
                 • Try running the command manually to diagnose: {program} {}",
                args.join(" "),
                args.join(" ")
            )
        }
    }
}

/// Check if we're inside a Zellij session
pub fn is_inside_zellij() -> bool {
    std::env::var("ZELLIJ").is_ok()
}

/// Check if stdout is a TTY (terminal)
/// Returns true if running in an interactive terminal, false otherwise
/// (e.g., in CI, piped output, SSH without TTY, background process)
pub fn is_tty() -> bool {
    std::io::stdout().is_terminal()
}

/// Check if stdin is a TTY (terminal) for reading user input
/// Returns true if stdin is connected to a terminal and can accept input
/// Use this before reading from stdin (e.g., confirmation prompts)
pub fn is_stdin_tty() -> bool {
    std::io::stdin().is_terminal()
}

/// Check if current directory is a JJ repository
pub fn is_jj_repo() -> Result<bool> {
    let result = Command::new("jj")
        .args(["root"])
        .output()
        .context("Failed to run jj")?;

    Ok(result.status.success())
}

/// Get JJ repository root
pub fn jj_root() -> Result<String> {
    run_command("jj", &["root"]).map(|s| s.trim().to_string())
}

/// Check if a command is available in PATH
pub fn is_command_available(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if JJ is installed
pub fn is_jj_installed() -> bool {
    is_command_available("jj")
}

/// Check if Zellij is installed
pub fn is_zellij_installed() -> bool {
    is_command_available("zellij")
}

/// Attach to or create a Zellij session, optionally with a layout
/// This function will exec into Zellij, replacing the current process
#[cfg(unix)]
pub fn attach_to_zellij_session(layout_content: Option<&str>) -> Result<()> {
    // Check if Zellij is installed
    if !is_zellij_installed() {
        anyhow::bail!("Zellij is not installed. Please install it first.");
    }

    // Get the session name from the JJ repo root or use default
    let session_name = jj_root()
        .ok()
        .and_then(|root| {
            std::path::Path::new(&root)
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| format!("zjj-{s}"))
        })
        .unwrap_or_else(|| "zjj".to_string());

    // Print a helpful message before attaching
    eprintln!("Attaching to Zellij session '{session_name}'...");

    // We'll attach to or create the Zellij session
    // Using exec to replace the current process
    let zellij_path = which::which("zellij").context("Failed to find zellij in PATH")?;

    let mut cmd = std::process::Command::new(zellij_path);

    // If layout content provided, write it to a temp file and use it
    if let Some(layout) = layout_content {
        let temp_dir = std::env::temp_dir();
        let layout_path = temp_dir.join(format!("zjj-{}.kdl", std::process::id()));
        std::fs::write(&layout_path, layout)?;

        cmd.args([
            "--layout",
            layout_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid layout path"))?,
            "attach",
            "-c",
            &session_name,
        ]);
    } else {
        cmd.args(["attach", "-c", &session_name]);
    }

    // Exec into Zellij
    let err = cmd.exec();

    // If we get here, exec failed
    Err(anyhow::anyhow!("Failed to exec into Zellij: {err}"))
}

/// Attach to or create a Zellij session, optionally with a layout
/// Windows version - not supported
#[cfg(not(unix))]
pub fn attach_to_zellij_session(_layout_content: Option<&str>) -> Result<()> {
    anyhow::bail!("Auto-spawning Zellij is only supported on Unix systems");
}

/// Check if current directory is a Git repository
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_command_success() {
        let result = run_command("echo", &["hello"]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap_or_default().trim(), "hello");
    }

    #[test]
    fn test_run_command_failure() {
        let result = run_command("false", &[]);
        assert!(result.is_err());
    }
}
