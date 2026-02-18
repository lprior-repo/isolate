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
    Action, ActionStatus, Assessment, Context, ErrorSeverity, Issue, IssueKind, IssueSeverity,
    OutputLine, OutputLineError, Plan, PlanStep, QueueEntry, QueueEntryStatus, QueueSummary,
    Recovery, RecoveryAction, ResultKind, ResultOutput, SessionOutput, Stack, StackEntry,
    StackEntryStatus, Summary, SummaryType, Train, TrainAction, TrainStatus, TrainStep,
    TrainStepStatus, Warning,
};
pub use writer::{emit, emit_all_stdout, emit_stdout, JsonlConfig, JsonlWriter};

#[cfg(test)]
mod tests;
