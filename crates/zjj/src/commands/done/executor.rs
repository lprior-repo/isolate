//! JJ command executor trait for dependency injection
//!
//! This module provides a trait for executing JJ commands, allowing
//! for easy testing via MockJjExecutor.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::{
    collections::HashMap,
    process::Command,
    sync::{Arc, Mutex},
};

use thiserror::Error;

use super::newtypes::JjOutput;

/// Errors from JJ command execution
#[derive(Debug, Error, PartialEq, Eq)]
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

/// Trait for executing JJ commands
pub trait JjExecutor {
    /// Run a JJ command with arguments
    fn run(&self, args: &[&str]) -> Result<JjOutput, ExecutorError>;

    /// Run a JJ command with environment variables
    fn run_with_env(&self, args: &[&str], env: &[(&str, &str)]) -> Result<JjOutput, ExecutorError>;
}

/// Real JJ executor that runs actual commands
#[derive(Debug, Default)]
pub struct RealJjExecutor {
    working_dir: Option<String>,
}

impl RealJjExecutor {
    /// Create a new RealJjExecutor
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with a specific working directory
    pub fn with_working_dir(working_dir: String) -> Self {
        Self {
            working_dir: Some(working_dir),
        }
    }
}

impl JjExecutor for RealJjExecutor {
    fn run(&self, args: &[&str]) -> Result<JjOutput, ExecutorError> {
        self.run_with_env(args, &[])
    }

    fn run_with_env(&self, args: &[&str], env: &[(&str, &str)]) -> Result<JjOutput, ExecutorError> {
        let mut cmd = Command::new("jj");

        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        cmd.args(args);

        for (key, value) in env {
            cmd.env(key, value);
        }

        let output = cmd.output().map_err(|e| {
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
    }
}

/// Mock JJ executor for testing
#[derive(Debug, Clone)]
pub struct MockJjExecutor {
    responses: Arc<Mutex<HashMap<String, Result<String, ExecutorError>>>>,
    calls: Arc<Mutex<Vec<Vec<String>>>>,
}

impl MockJjExecutor {
    /// Create a new MockJjExecutor
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(HashMap::new())),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Configure expected output for specific args
    pub fn expect(&mut self, args: &[&str], output: Result<String, ExecutorError>) {
        let key = args.join(" ");
        if let Ok(mut responses) = self.responses.lock() {
            responses.insert(key, output);
        }
    }

    /// Get recorded calls
    pub fn calls(&self) -> Vec<Vec<String>> {
        self.calls.lock().map(|c| c.clone()).unwrap_or_default()
    }

    /// Simulate a command failure
    pub fn fail_next(&mut self, args: &[&str], code: i32, stderr: String) {
        let key = args.join(" ");
        if let Ok(mut responses) = self.responses.lock() {
            responses.insert(key, Err(ExecutorError::CommandFailed { code, stderr }));
        }
    }
}

impl Default for MockJjExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl JjExecutor for MockJjExecutor {
    fn run(&self, args: &[&str]) -> Result<JjOutput, ExecutorError> {
        self.run_with_env(args, &[])
    }

    fn run_with_env(
        &self,
        args: &[&str],
        _env: &[(&str, &str)],
    ) -> Result<JjOutput, ExecutorError> {
        // Record the call
        if let Ok(mut calls) = self.calls.lock() {
            calls.push(args.iter().map(|s| s.to_string()).collect());
        }

        // Look up response
        let key = args.join(" ");
        let response = self
            .responses
            .lock()
            .ok()
            .and_then(|responses| responses.get(&key).cloned());

        match response {
            Some(Ok(output)) => {
                JjOutput::new(output).map_err(|e| ExecutorError::InvalidUtf8(e.to_string()))
            }
            Some(Err(e)) => Err(e),
            None => Ok(JjOutput::new(String::new())
                .map_err(|e| ExecutorError::InvalidUtf8(e.to_string()))?),
        }
    }
}
