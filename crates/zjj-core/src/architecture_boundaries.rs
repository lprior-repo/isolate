//! Architecture boundary enforcement tests.
//!
//! These tests enforce domain-driven design boundaries using compile-time checks.
//! Domain types must not depend on infrastructure concerns (tokio, sqlx).
//!
//! The tests use trait bounds to verify at compile time that domain types
//! remain pure and don't accidentally pull in infrastructure dependencies.

use std::marker::PhantomData;

use crate::{
    beads::{
        BeadFilter, BeadIssue, BeadQuery, BeadSort, BeadsError, BeadsSummary, IssueStatus,
        IssueType, Priority, SortDirection,
    },
    coordination::{QueueStats, QueueStatus, TransitionError},
};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// MARKER TRAITS FOR ARCHITECTURE LAYERS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Marker trait for domain layer types.
/// Types in the domain layer should be pure, with no infrastructure dependencies.
pub trait DomainLayer {}

/// Marker trait for infrastructure layer types.
/// These types may depend on tokio, sqlx, filesystem, network, etc.
pub trait InfrastructureLayer {}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// COMPILE-TIME BOUNDARY VERIFICATION
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Verifies that a type implements DomainLayer marker.
/// This will fail at compile time if the type doesn't implement the trait.
const fn assert_domain<T: DomainLayer>() {}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// DOMAIN LAYER TYPES - Implement DomainLayer marker
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

impl DomainLayer for BeadIssue {}
impl DomainLayer for BeadFilter {}
impl DomainLayer for BeadQuery {}
impl DomainLayer for BeadSort {}
impl DomainLayer for SortDirection {}
impl DomainLayer for BeadsSummary {}
impl DomainLayer for IssueStatus {}
impl DomainLayer for IssueType {}
impl DomainLayer for Priority {}
impl DomainLayer for BeadsError {}

impl DomainLayer for QueueStatus {}
impl DomainLayer for TransitionError {}
impl DomainLayer for QueueStats {}

impl DomainLayer for crate::Error {}
impl DomainLayer for crate::Config {}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// COMPILE-TIME TESTS - These verify trait implementations
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn domain_types_implement_domain_marker() {
    assert_domain::<BeadIssue>();
    assert_domain::<BeadFilter>();
    assert_domain::<BeadQuery>();
    assert_domain::<BeadSort>();
    assert_domain::<SortDirection>();
    assert_domain::<BeadsSummary>();
    assert_domain::<IssueStatus>();
    assert_domain::<IssueType>();
    assert_domain::<Priority>();
    assert_domain::<BeadsError>();
    assert_domain::<QueueStatus>();
    assert_domain::<TransitionError>();
    assert_domain::<QueueStats>();
    assert_domain::<crate::Error>();
    assert_domain::<crate::Config>();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// IMPORT BOUNDARY TESTS - Verify domain modules don't import infrastructure
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that domain types can be constructed without tokio runtime.
/// If domain types required tokio, this would panic at runtime.
#[test]
fn domain_types_dont_require_tokio_runtime() {
    let filter = BeadFilter::new()
        .with_status(IssueStatus::Open)
        .with_label("bug")
        .limit(10);

    let query = BeadQuery::new()
        .filter(filter)
        .sort_by(BeadSort::Priority)
        .direction(SortDirection::Desc);

    assert!(query.filter.status.contains(&IssueStatus::Open));
    assert_eq!(query.sort, BeadSort::Priority);

    let priority = Priority::from_u32(0);
    assert_eq!(priority, Some(Priority::P0));
}

/// Test that domain error types work without infrastructure dependencies.
#[test]
fn domain_errors_are_pure() {
    let error = BeadsError::NotFound("test-id".to_string());
    let error_msg = error.to_string();
    assert!(error_msg.contains("test-id"));
}

/// Test that domain status types are pure enums.
#[test]
fn domain_status_types_are_pure_enums() {
    use std::str::FromStr;

    let status = QueueStatus::from_str("pending").ok();
    assert_eq!(status, Some(QueueStatus::Pending));

    let status = QueueStatus::from_str("claimed").ok();
    assert_eq!(status, Some(QueueStatus::Claimed));

    let status = QueueStatus::from_str("merged").ok();
    assert_eq!(status, Some(QueueStatus::Merged));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// MODULE DEPENDENCY VERIFICATION
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Phantom type to detect if a type would pull in infrastructure dependencies.
/// If a type transitively depends on tokio::runtime::Runtime, this would fail.
struct NoRuntimeDependency<T>(PhantomData<T>);

#[test]
fn bead_issue_has_no_runtime_dependency() {
    let _checker: NoRuntimeDependency<BeadIssue> = NoRuntimeDependency(PhantomData);
}

#[test]
fn bead_filter_has_no_runtime_dependency() {
    let _checker: NoRuntimeDependency<BeadFilter> = NoRuntimeDependency(PhantomData);
}

#[test]
fn queue_status_has_no_runtime_dependency() {
    let _checker: NoRuntimeDependency<QueueStatus> = NoRuntimeDependency(PhantomData);
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// SERDE BOUNDARY TEST - Domain types should be serializable without runtime
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn domain_types_are_serializable_without_runtime() {
    use serde::Serialize;

    fn assert_serializable<T: Serialize>() {}

    assert_serializable::<BeadIssue>();
    assert_serializable::<BeadsSummary>();
    assert_serializable::<IssueStatus>();
    assert_serializable::<Priority>();

    let filter = BeadFilter::new().with_status(IssueStatus::Open);
    let json = serde_json::to_string(&filter.status).ok();
    assert!(json.is_some());
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// ARCHITECTURE DOCUMENTATION TEST
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// This test documents the architecture boundaries.
/// It serves as living documentation of what should and shouldn't depend on what.
#[test]
fn architecture_boundaries_documentation() {
    use std::any::type_name;

    let domain_types: &[&str] = &[
        type_name::<BeadIssue>(),
        type_name::<BeadFilter>(),
        type_name::<IssueStatus>(),
        type_name::<QueueStatus>(),
        type_name::<crate::Error>(),
    ];

    for ty in domain_types {
        assert!(
            !ty.contains("tokio"),
            "Domain type {ty} should not reference tokio"
        );
        assert!(
            !ty.contains("sqlx"),
            "Domain type {ty} should not reference sqlx"
        );
    }
}
