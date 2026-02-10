| area | symptom | root cause | risk | fix strategy | owner file(s) |
|---|---|---|---|---|---|
| Concurrency stress gate | A single lock-contention failure fans out into many SIGTERM-aborted tests | Test execution does not isolate downstream workers after critical core failure | High: masks real regressions and increases triage noise | Add fail-fast boundary for lock stress stage; rerun aborted bins separately to distinguish noise vs defect | `crates/zjj-core/tests/test_lock_concurrency_stress.rs`, `docs/testing/first-run-audit.md` |
| DRQ adversarial promotion | Previously skipped DRQ tests are now unignored and likely unstable under real runtime behavior | DRQ bank started as aspirational/adversarial contract coverage, not hardened regression suite | High: frequent red builds if promoted without deterministic semantics | Split DRQ into `required` vs `research` lanes; keep strict assertions only where command contracts are finalized | `crates/zjj/tests/drq_adversarial.rs` |
| DRQ agent arena schema drift | Assertions had to be adapted to envelope/payload shapes (`data` fallback) | JSON contract migrated while tests still encoded legacy field access | Medium-High: false failures or false confidence on schema compliance | Add shared JSON accessor helper and schema-level contract checks for all query/list commands | `crates/zjj/tests/drq_agent_arena.rs`, `crates/zjj/tests/test_json_standardization_comprehensive.rs` |
| Remove idempotent contract debt | `--idempotent` tests moved from ignored to active with revised schema assertions | Runtime behavior and test contract were out of sync during feature rollout | Medium-High: command semantics may regress across exit code/JSON cases | Lock contract in one canonical fixture matrix (exists/nonexistent/uninitialized) and enforce in focused test file | `crates/zjj/tests/test_remove_idempotent.rs`, `crates/zjj/src/commands/remove.rs`, `crates/zjj/src/commands/remove/atomic.rs` |
| Work idempotent context coupling | Tests now mutate `current_dir` directly to simulate in-workspace behavior | Prior approach relied on command side effects and implicit environment | Medium: order-dependent failures and hidden setup assumptions | Centralize cwd/workspace state transitions in harness helpers with explicit preconditions | `crates/zjj/tests/test_work_idempotent.rs`, `crates/zjj/tests/common/mod.rs` |
| Signal handling assertion weakness | Orphan checks shifted from exact count to subset containment | Count-based assertions were brittle, but replacement can miss extra leaked dirs | Medium: orphan regressions may pass silently | Keep robust name-based assertions plus explicit "no unexpected workspace" check for test namespace | `crates/zjj/tests/test_signal_handling.rs` |

## Lane Commands (Required vs Research)

Use these commands consistently:

- Required lane (default, CI-safe): `moon run :test`
- Research lane (DRQ/adversarial, explicit opt-in): `moon run :test-research`

Command intent:

- `moon run :test` runs required coverage and excludes `package(zjj) & binary(drq_adversarial)`.
- `moon run :test-research` runs only `package(zjj) & binary(drq_adversarial)`.

Do not replace required-lane validation with research-lane results.
