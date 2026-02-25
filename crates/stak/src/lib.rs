//! Stak - Merge queue and PR coordination
//!
//! This crate handles:
//! - Merge queue management
//! - Agent coordination
//! - Resource locking
//! - Event broadcasting

pub mod cli;
pub mod commands;
pub mod coordination;
pub mod db;
pub mod error;

pub use error::{Error, Result};
