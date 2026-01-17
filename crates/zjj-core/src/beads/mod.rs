#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(clippy::arithmetic_side_effects)]

//! Beads issue tracking integration
//!
//! This module provides comprehensive issue tracking functionality organized into
//! specialized submodules:
//! - `analysis`: Dependency and critical path analysis
//! - `similarity`: Duplicate detection using word intersection algorithm
//! - `trending`: Temporal analysis for staleness detection
//! - `categorization`: Issue extraction and classification utilities
//! - `filter`: Filtering, sorting, and pagination
//! - `summary`: Aggregation and grouping operations
//! - `types`: Domain types and error types
//! - `query`: Query execution for beads database

mod analysis;
mod categorization;
mod filter;
mod query;
mod similarity;
mod summary;
mod trending;
mod types;

pub use analysis::{
    calculate_critical_path, find_blocked, find_blockers, find_ready, get_dependency_graph,
};
pub use categorization::{extract_labels, get_issue, get_issues_by_id, to_ids, to_titles};
pub use filter::{
    all_match, any_match, apply_query, filter_issues, paginate, sort_issues, BeadFilter, BeadQuery,
    BeadSort, SortDirection,
};
pub use query::query_beads;
pub use similarity::find_potential_duplicates;
pub use summary::{count_by_status, group_by_status, group_by_type, summarize, BeadsSummary};
pub use trending::find_stale;
pub use types::*;
