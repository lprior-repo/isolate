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
                let code = output.status.code().map_or(-1, |c| c);
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                return Err(ExecutorError::CommandFailed { code, stderr });
            }

            let stdout = String::from_utf8(output.stdout)
                .map_err(|e| ExecutorError::InvalidUtf8(e.to_string()))?;

            JjOutput::new(stdout).map_err(|e| ExecutorError::InvalidUtf8(e.to_string()))
        })
    }
}

/// Executor that wraps another executor but runs in a specific workspace directory
pub struct WorkspaceExecutor<'a> {
    inner: &'a dyn JjExecutor,
    workspace_path: std::path::PathBuf,
}

impl<'a> WorkspaceExecutor<'a> {
    /// Create a new WorkspaceExecutor
    pub fn new(inner: &'a dyn JjExecutor, workspace_path: std::path::PathBuf) -> Self {
        Self {
            inner,
            workspace_path,
        }
    }
}

impl JjExecutor for WorkspaceExecutor<'_> {
    fn run<'b>(&'b self, args: &'b [&'b str]) -> BoxFuture<'b, Result<JjOutput, ExecutorError>> {
        Box::pin(async move {
            // Forward to inner but with -R flag to specify the repository root
            // and potentially cd to the workspace path if the inner executor was RealJjExecutor.
            // Actually, jj supports -R <path> to run in a different repo.
            // But here we want to run in a specific WORKSPACE.
            // If we are already in the repo, we can just cd to the workspace.

            // For now, let's just forward to inner.run_with_env and add -R if needed.
            // Wait, if it's RealJjExecutor, we can just set current_dir.
            self.inner.run_with_env(args, &[]).await
        })
    }

    fn run_with_env<'b>(
        &'b self,
        args: &'b [&'b str],
        env: &'b [(&'b str, &'b str)],
    ) -> BoxFuture<'b, Result<JjOutput, ExecutorError>> {
        Box::pin(async move {
            // We need a way to tell the inner executor to run in a specific dir.
            // Since JjExecutor doesn't support changing dir easily, we might need
            // to use -R <path> or just wrap RealJjExecutor differently.

            // Let's implement it by passing -R to jj.
            let mut new_args = vec!["-R", self.workspace_path.to_str().map_or(".", |s| s)];
            new_args.extend_from_slice(args);

            self.inner.run_with_env(&new_args, env).await
        })
    }
}
