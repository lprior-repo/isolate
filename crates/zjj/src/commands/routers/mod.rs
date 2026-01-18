//! Command routers
//!
//! This module contains routers that organize and dispatch commands into logical groups:
//! - Session commands: Manage the lifecycle of ZJJ sessions
//! - Utility commands: Provide supporting functionality like backups and completions
//! - Introspection commands: Provide metadata and diagnostics

pub mod introspection;
pub mod session;
pub mod utility;
