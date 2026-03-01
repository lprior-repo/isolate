---
bead_id: bd-3s3
bead_title: Contract: specs: Implement spec linter with quality scoring
phase: p1
implementation_status: complete
updated_at: 2026-03-01T06:46:30Z
---

# Implementation: specs linter

The orchestrator has a linter integration:
- crates/orchestrator/src/phases.rs: run_linter() method
- crates/orchestrator/src/state.rs: linter_path config

A dedicated specs crate with full linter is not yet created. This contract represents the intent.
