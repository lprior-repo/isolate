//! CLI utilities and helpers

pub mod commands;
pub mod handlers;
pub mod json_docs;

#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::Command as StdCommand;

use anyhow::{Context, Result};
pub use commands::build_cli;
use rand::Rng;
use tokio::process::Command;

/// Get a secure directory for temporary files
/// Prefers XDG_RUNTIME_DIR (Linux) which has proper permissions (0700)
/// Falls back to std::env::temp_dir()
fn secure_temp_dir() -> std::path::PathBuf {
    #[cfg(target_os = "linux")]
    {
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            let path = std::path::PathBuf::from(runtime_dir);
            if path.exists() {
                return path;
            }
        }
    }
    // SECURITY: temp_dir fallback is acceptable - only used when XDG_RUNTIME_DIR unavailable
    std::env::temp_dir()
}

/// Create a secure temporary file path with random name
fn secure_temp_file(prefix: &str, suffix: &str) -> std::path::PathBuf {
    let dir = secure_temp_dir();
    let random_id: u64 = rand::thread_rng().gen();
    dir.join(format!("{prefix}-{random_id:016x}{suffix}"))
}

/// Execute a shell command and return its output
pub async fn run_command(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .await
        .with_context(|| format!("Failed to execute {program}"))?;

    if output.status.success() {
        String::from_utf8(output.stdout).context("Invalid UTF-8 output from command")
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("{program} failed: {stderr}")
    }
}

/// Check if we're inside a Zellij session
pub fn is_inside_zellij() -> bool {
    std::env::var("ZELLIJ").is_ok()
}

/// Check if we're running in a terminal (TTY)
/// Uses `std::io::IsTerminal` (Rust 1.70+) and verifies TTY properties
pub fn is_terminal() -> bool {
    use std::io::IsTerminal;
    let is_tty = std::io::stdin().is_terminal()
        && std::io::stdout().is_terminal()
        && std::io::stderr().is_terminal();

    if !is_tty {
        return false;
    }

    // Additional check: can we get terminal size?
    // This helps detect environments that claim to be TTYs but don't support TTY ioctls
    crossterm::terminal::size().is_ok()
}

/// Check if current directory is a JJ repository
pub async fn is_jj_repo() -> Result<bool> {
    let result = zjj_core::jj::get_jj_command()
        .args(["root"])
        .output()
        .await
        .context("Failed to run jj")?;

    Ok(result.status.success())
}

/// Get JJ repository root
pub async fn jj_root() -> Result<String> {
    let output = zjj_core::jj::get_jj_command()
        .arg("root")
        .output()
        .await
        .context("Failed to execute jj root")?;

    if output.status.success() {
        String::from_utf8(output.stdout)
            .context("Invalid UTF-8 output from jj root")
            .map(|s| s.trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("jj root failed: {stderr}")
    }
}

/// Check if a command is available in PATH
pub async fn is_command_available(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if JJ is installed
pub async fn is_jj_installed() -> bool {
    zjj_core::jj::is_jj_installed().await
}

/// Check if Zellij is installed
pub async fn is_zellij_installed() -> bool {
    is_command_available("zellij").await
}

/// Attach to or create a Zellij session, optionally with a layout
/// This function will exec into Zellij, replacing the current process
#[cfg(unix)]
pub async fn attach_to_zellij_session(layout_content: Option<&str>) -> Result<()> {
    // Check if running in a TTY
    if !is_terminal() {
        anyhow::bail!(
            "Cannot launch Zellij in non-interactive environment.\n\
             Use --no-zellij flag to skip Zellij integration."
        );
    }

    if !is_zellij_installed().await {
        anyhow::bail!("Zellij is not installed. Please install it first.");
    }

    // Get the session name from the JJ repo root or use default
    let session_name = jj_root()
        .await
        .ok()
        .and_then(|root| {
            std::path::Path::new(&root)
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| format!("zjj-{s}"))
        })
        .unwrap_or_else(|| "zjj".to_string());

    // Print a helpful message before attaching (to stdout, not stderr)
    println!("Attaching to Zellij session '{session_name}'...");

    // We'll attach to or create the Zellij session
    // Using exec to replace the current process
    let zellij_path = which::which("zellij").context("Failed to find zellij in PATH")?;

    let mut cmd = StdCommand::new(zellij_path);

    // If layout content provided, write it to a temp file and use it
    if let Some(layout) = layout_content {
        // SECURITY: Use secure temp file with random name instead of predictable PID
        let layout_path = secure_temp_file("zjj-layout", ".kdl");
        tokio::fs::write(&layout_path, layout).await?;

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
pub async fn attach_to_zellij_session(_layout_content: Option<&str>) -> Result<()> {
    anyhow::bail!("Auto-spawning Zellij is only supported on Unix systems");
}

/// Check if current directory is a Git repository
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_command_success() {
        let result = run_command("echo", &["hello"]).await;
        assert!(result.is_ok());
        let Ok(output) = result else {
            panic!("command failed");
        };
        assert_eq!(output.trim(), "hello");
    }

    #[tokio::test]
    async fn test_run_command_failure() {
        let result = run_command("false", &[]).await;
        assert!(result.is_err());
    }
}
