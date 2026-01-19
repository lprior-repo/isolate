//! Effects-as-Data pattern for pure functional side-effect management
//!
//! This module enables Functional Core, Imperative Shell architecture.
//! Pure functions return `Vector<Effect>` describing what should happen,
//! and the shell executes them.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use im::Vector;
use std::path::PathBuf;
use strum::EnumDiscriminants;

use crate::Result;

/// Output stream target
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    Stdout,
    Stderr,
}

/// All possible side effects in the system
#[derive(Debug, Clone, EnumDiscriminants)]
#[strum_discriminants(name(EffectType))]
pub enum Effect {
    /// Print a message to stdout or stderr
    Print {
        message: String,
        stream: Stream,
    },

    /// Write content to a file
    WriteFile {
        path: PathBuf,
        content: String,
    },

    /// Delete a file
    DeleteFile {
        path: PathBuf,
    },

    /// Create a directory (recursive)
    CreateDir {
        path: PathBuf,
    },

    /// Set file permissions
    SetPermissions {
        path: PathBuf,
        mode: u32,
    },

    /// Run an external command
    RunCommand {
        program: String,
        args: Vector<String>,
        working_dir: Option<PathBuf>,
    },

    /// Zellij tab operations
    ZellijCreateTab {
        name: String,
    },
    ZellijCloseTab {
        name: String,
    },
    ZellijFocusTab {
        name: String,
    },
}

impl Effect {
    /// Create a stdout print effect
    #[must_use]
    pub fn println(message: impl Into<String>) -> Self {
        Self::Print {
            message: message.into(),
            stream: Stream::Stdout,
        }
    }

    /// Create a stderr print effect
    #[must_use]
    pub fn eprintln(message: impl Into<String>) -> Self {
        Self::Print {
            message: message.into(),
            stream: Stream::Stderr,
        }
    }

    /// Create a write file effect
    #[must_use]
    pub fn write_file(path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        Self::WriteFile {
            path: path.into(),
            content: content.into(),
        }
    }

    /// Create a create directory effect
    #[must_use]
    pub fn create_dir(path: impl Into<PathBuf>) -> Self {
        Self::CreateDir { path: path.into() }
    }
}

/// Builder for collecting effects
#[derive(Debug, Clone, Default)]
pub struct EffectBuilder {
    effects: Vector<Effect>,
}

impl EffectBuilder {
    /// Create a new effect builder
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an effect
    #[must_use]
    pub fn with(self, effect: Effect) -> Self {
        Self {
            effects: self.effects.push_back(effect),
        }
    }

    /// Add multiple effects
    #[must_use]
    pub fn with_all(self, effects: Vector<Effect>) -> Self {
        Self {
            effects: effects
                .iter()
                .fold(self.effects, |acc, e| acc.push_back(e.clone())),
        }
    }

    /// Build the final effect vector
    #[must_use]
    pub fn build(self) -> Vector<Effect> {
        self.effects
    }
}

/// Execute a vector of effects (Imperative Shell)
///
/// This is the ONLY place where side effects actually happen.
/// All other code should be pure and return `Vector<Effect>`.
pub fn execute_effects(effects: &Vector<Effect>) -> Result<()> {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    for effect in effects {
        match effect {
            Effect::Print { message, stream } => match stream {
                Stream::Stdout => println!("{message}"),
                Stream::Stderr => eprintln!("{message}"),
            },
            Effect::WriteFile { path, content } => {
                fs::write(path, content).map_err(|e| {
                    crate::Error::io_error(format!("Failed to write {}: {e}", path.display()))
                })?;
            }
            Effect::DeleteFile { path } => {
                if path.exists() {
                    fs::remove_file(path).map_err(|e| {
                        crate::Error::io_error(format!("Failed to delete {}: {e}", path.display()))
                    })?;
                }
            }
            Effect::CreateDir { path } => {
                fs::create_dir_all(path).map_err(|e| {
                    crate::Error::io_error(format!("Failed to create dir {}: {e}", path.display()))
                })?;
            }
            Effect::SetPermissions { path, mode } => {
                let perms = fs::Permissions::from_mode(*mode);
                fs::set_permissions(path, perms).map_err(|e| {
                    crate::Error::io_error(format!(
                        "Failed to set permissions on {}: {e}",
                        path.display()
                    ))
                })?;
            }
            Effect::RunCommand {
                program,
                args,
                working_dir,
            } => {
                let mut cmd = std::process::Command::new(program);
                cmd.args(args.iter().map(String::as_str));
                if let Some(dir) = working_dir {
                    cmd.current_dir(dir);
                }
                cmd.status().map_err(|e| {
                    crate::Error::command_error(format!("Failed to run {program}: {e}"))
                })?;
            }
            Effect::ZellijCreateTab { name }
            | Effect::ZellijCloseTab { name }
            | Effect::ZellijFocusTab { name } => {
                // Zellij effects handled by zellij module
                let _ = name; // Placeholder - integrate with zellij module
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effect_builder() {
        let effects = EffectBuilder::new()
            .with(Effect::println("Hello"))
            .with(Effect::eprintln("Warning"))
            .build();

        assert_eq!(effects.len(), 2);
    }

    #[test]
    fn test_effect_constructors() {
        let print = Effect::println("test");
        assert!(matches!(
            print,
            Effect::Print {
                stream: Stream::Stdout,
                ..
            }
        ));

        let eprint = Effect::eprintln("error");
        assert!(matches!(
            eprint,
            Effect::Print {
                stream: Stream::Stderr,
                ..
            }
        ));
    }
}
