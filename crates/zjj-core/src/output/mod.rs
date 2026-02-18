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

mod types;
mod writer;

pub use types::{
    Action, ActionStatus, Assessment, ConflictAnalysis, ConflictDetail, ConflictType, Context,
    Error, ErrorSeverity, Issue, IssueKind, OutputLine, Plan, PlanStep, Recovery, RecoveryAction,
    ResolutionOption, ResolutionRisk, ResolutionStrategy, Result, ResultKind, Session,
    SessionState, Summary, Warning,
};
pub use writer::{emit, emit_value, JsonlWriter};

#[cfg(test)]
mod tests;
