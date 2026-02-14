# First-Run Test Debt Audit

Scope: current workspace edits plus first-run artifacts.

Source artifacts:
- `/home/lewis/.local/share/opencode/tool-output/tool_c421715700019t6glC8bqDuBy3` (initial failing run)
- `/home/lewis/.local/share/opencode/tool-output/tool_c421caf97001KQRqWzLcYURpTg` (first passing run with skips)

Validation status:
- Skip inventory count is internally consistent (`11 + 17 + 6 = 34`).
- All skip names in `first-run-skips.json` map to existing tests under `crates/zjj/tests/*.rs`.
- Full-suite rerun intentionally not performed in this audit.

Lane command policy used by this audit:
- Required lane (default): `moon run :test`
- Research lane (explicit opt-in): `moon run :test-research`
- Required lane remains the source of truth for merge/CI readiness.

## First-Run Trigger and Blast Radius

- Primary trigger in initial run: `zjj-core::test_lock_concurrency_stress::test_lock_contention_metrics`.
- Observed side effect: subsequent `zjj:test` workers received `SIGTERM` and aborted.
- Interpretation: these SIGTERM cases are interruption noise, not standalone product failures.

Interrupted tests captured from that run:
- `zjj::test_export_flag_removal common::tests::test_workspace_path_from_env_var_is_resolved_correctly`
- `zjj::test_error_scenarios test_status_nonexistent_session`
- `zjj::test_error_scenarios test_session_name_exactly_64_chars`
- `zjj::test_concurrent_workflows test_100_concurrent_session_creation`
- `zjj::test_concurrent_workflows test_multi_agent_workflow_integration`
- `zjj::test_concurrent_workflows test_high_volume_session_management`
- `zjj::test_concurrent_workflows test_database_connection_pool_stress`
- `zjj::test_concurrent_workflows test_rapid_operations_stability`
- `zjj::test_error_scenarios test_sync_nonexistent_session`
- `zjj::test_concurrent_workflows test_concurrent_status_checks`
- `zjj::test_export_flag_removal common::tests::test_workspace_dir_is_configurable_via_env`
- `zjj::test_error_scenarios test_session_name_with_numbers_only_rejected`
- `zjj::test_error_scenarios test_session_name_zero_width_chars`
- `zjj::test_concurrent_workflows test_parallel_agents_overlapping_namespaces`
- `zjj::test_error_scenarios test_status_with_manually_deleted_workspace`
- `zjj::test_error_display test_database_error_display`
- `zjj::test_concurrent_workflows test_concurrent_create_delete`
- `zjj::test_database_concurrency_race_conditions test_connection_pool_under_pressure`
- `zjj::test_database_concurrency_race_conditions test_high_frequency_update_storm`
- `zjj::test_error_scenarios test_list_with_no_sessions_after_remove_all`
- `zjj::test_error_scenarios test_remove_already_removed_session`
- `zjj::test_error_scenarios test_rapid_add_remove_cycles`
- `zjj::test_concurrent_workflows test_parallel_read_operations`
- `zjj::test_error_scenarios test_session_name_path_traversal_parent_ref`
- `zjj::test_error_scenarios test_workspace_directory_creation_failure`
- `zjj::test_export_flag_removal cli_rejects_include_files_flag`
- `zjj::test_concurrent_workflows test_rapid_create_remove_cycles`
- `zjj::test_error_scenarios test_rapid_sequential_add_remove`
- `zjj::test_error_scenarios test_session_name_path_traversal_double_dot`
- `zjj::test_export_flag_removal common::tests::test_harness_has_jj_repo`
- `zjj::test_error_scenarios test_remove_workspace_symlink_cleanup`

## Skip Inventory (First Passing Run)

Total skipped: 34

- DRQ adversarial bank: 11
- DRQ agent arena bank: 17
- remove idempotent feature tests: 6

See `docs/testing/first-run-skips.json` for exact test-level inventory.

## Workspace Delta Since Artifacts

Current workspace diffs under `crates/zjj/tests` show a broad shift from quarantine-style tests to active contract tests:

- `drq_adversarial.rs`: `#[ignore]` markers removed from the listed DRQ adversarial tests.
- `drq_agent_arena.rs`: `#[ignore]` markers removed from the listed DRQ agent arena tests.
- `test_remove_idempotent.rs`: `#[ignore]` markers removed and assertions aligned to current JSON envelope format.
- `test_work_idempotent.rs`: cwd-dependent setup adjusted; schema assertions moved toward envelope/payload expectations.
- `common/mod.rs`: process-level env mutation removed from harness constructor (lower cross-test coupling).

This makes the original root-cause labels stale in one important way: DRQ and remove-idempotent are no longer merely "intentionally ignored" debt in the workspace; they are being promoted into active coverage and now represent stabilization debt.

## Updated Debt Classification

- Concurrency gate debt: lock-contention stress can cascade into broad SIGTERM noise and hide true failures.
- Contract migration debt: tests are being migrated from legacy JSON assumptions to envelope/payload structure.
- Harness isolation debt: historical global env/cwd coupling made tests order-sensitive and brittle.
- Promotion debt: formerly ignored DRQ/remove-idempotent banks now need deterministic runtime behavior and ownership.

## Commands To Use (No Full Suite)

For this first-run debt workflow, use lane-targeted commands only:

- Required lane check: `moon run :test`
- Research lane check: `moon run :test-research`

If required lane fails because of a known stress trigger, fix or isolate that trigger first, then re-run `moon run :test` before using any research-lane signal.
