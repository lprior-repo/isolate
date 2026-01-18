//! Shell completion generation command
//!
//! Generates shell completion scripts for bash, zsh, and fish shells.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::io;

use anyhow::{Context, Result};
use clap_complete::{generate, Shell};

/// Supported shell types for completion generation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionShell {
    /// Bash shell
    Bash,
    /// Zsh shell
    Zsh,
    /// Fish shell
    Fish,
}

impl CompletionShell {
    /// Parse shell name from string
    ///
    /// # Errors
    ///
    /// Returns an error if the shell name is not recognized
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "bash" => Ok(Self::Bash),
            "zsh" => Ok(Self::Zsh),
            "fish" => Ok(Self::Fish),
            _ => anyhow::bail!("Unsupported shell: {s}\nSupported shells: bash, zsh, fish"),
        }
    }

    /// Convert to clap Shell type
    const fn to_clap_shell(self) -> Shell {
        match self {
            Self::Bash => Shell::Bash,
            Self::Zsh => Shell::Zsh,
            Self::Fish => Shell::Fish,
        }
    }

    /// Get shell name as string
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Fish => "fish",
        }
    }

    /// Get installation instructions for this shell
    pub const fn installation_instructions(self) -> &'static str {
        match self {
            Self::Bash => concat!(
                "# Bash completion installation:\n",
                "# Linux:\n",
                "jjz completions bash > ~/.local/share/bash-completion/completions/jjz\n",
                "\n",
                "# macOS (with Homebrew):\n",
                "jjz completions bash > $(brew --prefix)/etc/bash_completion.d/jjz\n",
                "\n",
                "# Or add to ~/.bashrc:\n",
                "source <(jjz completions bash)"
            ),
            Self::Zsh => concat!(
                "# Zsh completion installation:\n",
                "# Create completions directory if needed:\n",
                "mkdir -p ~/.zsh/completions\n",
                "\n",
                "# Generate completion file:\n",
                "jjz completions zsh > ~/.zsh/completions/_jjz\n",
                "\n",
                "# Add to ~/.zshrc (if not already present):\n",
                "fpath=(~/.zsh/completions $fpath)\n",
                "autoload -Uz compinit && compinit"
            ),
            Self::Fish => concat!(
                "# Fish completion installation:\n",
                "# Generate completion file:\n",
                "jjz completions fish > ~/.config/fish/completions/jjz.fish\n",
                "\n",
                "# Completions are automatically loaded by fish"
            ),
        }
    }

    /// List all supported shells
    #[allow(dead_code)]
    pub const fn all() -> &'static [Self] {
        &[Self::Bash, Self::Zsh, Self::Fish]
    }
}

/// Generate shell completions to stdout
///
/// # Errors
///
/// Returns an error if:
/// - Shell type is not recognized
/// - Writing to stdout fails
pub fn generate_completions(shell: CompletionShell) {
    // We need a mutable Command for clap_complete::generate
    // This is one of the few places where mut is necessary for external API compatibility
    let mut cmd = crate::cli::args::build_cli();
    let shell_type = shell.to_clap_shell();

    generate(shell_type, &mut cmd, "zjj", &mut io::stdout());
}

/// Run completions command
///
/// # Errors
///
/// Returns an error if shell parsing or generation fails
pub async fn run(shell_name: &str, print_instructions: bool) -> Result<()> {
    // Yield to make function legitimately async
    tokio::task::yield_now().await;

    let shell = CompletionShell::from_str(shell_name).context("Failed to parse shell type")?;

    if print_instructions {
        // Print installation instructions to stderr (so they don't interfere with completion
        // output)
        eprintln!("{}", shell.installation_instructions());
        eprintln!();
        eprintln!("Generating {} completions...\n", shell.as_str());
    }

    generate_completions(shell);
    Ok(())
}

/// List all supported shells with installation instructions
///
/// # Errors
///
/// This function does not return errors in normal operation
#[allow(dead_code)]
pub fn list_shells() -> Result<()> {
    println!("Supported shells:\n");

    CompletionShell::all().iter().try_for_each(|shell| {
        println!("{}:", shell.as_str());
        println!("{}\n", shell.installation_instructions());
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_from_str_valid() {
        assert_eq!(
            CompletionShell::from_str("bash").ok(),
            Some(CompletionShell::Bash)
        );
        assert_eq!(
            CompletionShell::from_str("zsh").ok(),
            Some(CompletionShell::Zsh)
        );
        assert_eq!(
            CompletionShell::from_str("fish").ok(),
            Some(CompletionShell::Fish)
        );
    }

    #[test]
    fn test_shell_from_str_case_insensitive() {
        assert_eq!(
            CompletionShell::from_str("BASH").ok(),
            Some(CompletionShell::Bash)
        );
        assert_eq!(
            CompletionShell::from_str("Zsh").ok(),
            Some(CompletionShell::Zsh)
        );
        assert_eq!(
            CompletionShell::from_str("FiSh").ok(),
            Some(CompletionShell::Fish)
        );
    }

    #[test]
    fn test_shell_from_str_invalid() {
        assert!(CompletionShell::from_str("invalid").is_err());
        assert!(CompletionShell::from_str("powershell").is_err());
        assert!(CompletionShell::from_str("").is_err());
    }

    #[test]
    fn test_shell_as_str() {
        assert_eq!(CompletionShell::Bash.as_str(), "bash");
        assert_eq!(CompletionShell::Zsh.as_str(), "zsh");
        assert_eq!(CompletionShell::Fish.as_str(), "fish");
    }

    #[test]
    fn test_all_shells() {
        let shells = CompletionShell::all();
        assert_eq!(shells.len(), 3);
        assert!(shells.contains(&CompletionShell::Bash));
        assert!(shells.contains(&CompletionShell::Zsh));
        assert!(shells.contains(&CompletionShell::Fish));
    }

    #[test]
    fn test_installation_instructions_not_empty() {
        for shell in CompletionShell::all() {
            let instructions = shell.installation_instructions();
            assert!(
                !instructions.is_empty(),
                "{} instructions should not be empty",
                shell.as_str()
            );
            assert!(
                instructions.contains("jjz completions"),
                "{} instructions should mention 'jjz completions'",
                shell.as_str()
            );
        }
    }

    #[test]
    fn test_to_clap_shell() {
        assert_eq!(CompletionShell::Bash.to_clap_shell(), Shell::Bash);
        assert_eq!(CompletionShell::Zsh.to_clap_shell(), Shell::Zsh);
        assert_eq!(CompletionShell::Fish.to_clap_shell(), Shell::Fish);
    }
}
