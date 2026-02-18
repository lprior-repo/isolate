//! JSONL output module for AI-first CLI design
//!
//! This module provides streaming JSONL output types for the AI-first control plane.
//! Each line of output is a valid JSON object that can be parsed independently.
//!
//! # Design Principles
//!
//! - Every output line is a complete, parseable JSON object
//! - Types are self-describing with a "type" field
//! - No human-readable formatting - AI consumers only
//! - Streaming-friendly: emit one line at a time

#![allow(dead_code)]
#![allow(clippy::missing_const_for_fn)]

mod types;
mod writer;

pub use types::{
    Action, ActionStatus, Assessment, Context, ErrorSeverity, Issue, IssueKind, OutputLine,
    OutputLineError, Plan, PlanStep, Recovery, RecoveryAction, ResultKind, ResultOutput,
    SessionOutput, Summary, Warning,
};
pub use writer::{emit, JsonlWriter};

#[cfg(test)]
mod tests;
