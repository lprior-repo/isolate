//! Query command - state queries for AI agents
//!
//! This command provides programmatic access to system state
//! for AI agents to make informed decisions.
//!
//! The module is organized into functional concerns:
//! - `types.rs`: Query type definitions and metadata
//! - `filtering.rs`: Filter parsing and error categorization
//! - `formatting.rs`: Result formatting and serialization
//! - `handlers.rs`: Query execution and result construction

mod filtering;
mod formatting;
mod handlers;
mod types;

pub use handlers::run;

use anyhow::Result;



/// Run a query
///
/// Entry point for query command execution. Routes the query to the appropriate handler.
pub async fn handle_query(query_type: &str, args: Option<&str>) -> Result<()> {
    handlers::run(query_type, args).await
}
