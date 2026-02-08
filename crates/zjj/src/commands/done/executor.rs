//! JJ command executor trait for dependency injection
//!
//! This module provides a trait for executing JJ commands.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]

use std::{future::Future, pin::Pin};

use thiserror::Error;
use tokio::process::Command;

use super::newtypes::JjOutput;

/// Errors from JJ command execution
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ExecutorError {
    #[error("JJ command not found: {0}")]
    CommandNotFound(String),

    #[error("JJ command failed with exit code {code}: {stderr}")]
    CommandFailed { code: i32, stderr: String },

    #[error("Invalid UTF-8 in command output: {0}")]
    InvalidUtf8(String),

    #[error("IO error: {0}")]
    IoError(String),
}

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Trait for executing JJ commands
pub trait JjExecutor: Send + Sync {
    /// Run a JJ command with arguments
    fn run<'a>(&'a self, args: &'a [&'a str]) -> BoxFuture<'a, Result<JjOutput, ExecutorError>>;

    /// Run a JJ command with environment variables
    fn run_with_env<'a>(
        &'a self,
        args: &'a [&'a str],
        env: &'a [(&'a str, &'a str)],
    ) -> BoxFuture<'a, Result<JjOutput, ExecutorError>>;
}

/// Real JJ executor that runs actual commands
#[derive(Debug, Default)]
pub struct RealJjExecutor {
    working_dir: Option<String>,
}

impl RealJjExecutor {
    /// Create a new `RealJjExecutor`
    pub fn new() -> Self {
        Self::default()
    }
}

impl JjExecutor for RealJjExecutor {
    fn run<'a>(&'a self, args: &'a [&'a str]) -> BoxFuture<'a, Result<JjOutput, ExecutorError>> {
        Box::pin(async move { self.run_with_env(args, &[]).await })
    }

    fn run_with_env<'a>(
        &'a self,
        args: &'a [&'a str],
        env: &'a [(&'a str, &'a str)],
    ) -> BoxFuture<'a, Result<JjOutput, ExecutorError>> {
        Box::pin(async move {
            let mut cmd = Command::new("jj");

            if let Some(ref dir) = self.working_dir {
                cmd.current_dir(dir);
            }

            cmd.args(args);

            for (key, value) in env {
                cmd.env(key, value);
            }

            let output = cmd.output().await.map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    ExecutorError::CommandNotFound("jj command not found in PATH".to_string())
                } else {
                    ExecutorError::IoError(e.to_string())
                }
            })?;

            if !output.status.success() {
                let code = output.status.code().unwrap_or(-1);
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                return Err(ExecutorError::CommandFailed { code, stderr });
            }

            let stdout = String::from_utf8(output.stdout)
                .map_err(|e| ExecutorError::InvalidUtf8(e.to_string()))?;

            JjOutput::new(stdout).map_err(|e| ExecutorError::InvalidUtf8(e.to_string()))
        })
    }
}
