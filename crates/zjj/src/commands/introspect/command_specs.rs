//! Command introspection specifications
//!
//! This module provides detailed introspection data for all jjz commands,
//! including arguments, flags, examples, prerequisites, and error conditions.
//!
//! The implementation is split into the `specs` submodule to maintain clarity:
//! - Command builders are in `specs::builders`
//! - Command routing is in `specs::commands`
//!
//! For internal use, this module re-exports the public API.

use crate::commands::introspect::specs;

/// Get introspection data for a command by name
///
/// # Errors
/// Returns error if command name is not recognized
pub fn get_command_spec(
    command: &str,
) -> Result<zjj_core::introspection::CommandIntrospection, String> {
    specs::get_command_spec(command)
}

/// Get all command names that have introspection specs
#[allow(dead_code)]
pub fn all_command_names() -> Vec<&'static str> {
    specs::all_command_names()
}
