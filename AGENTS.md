# AGENTS.md

```jsonl
{"id":"agents.swarm.min.v1","role":"Autonomous operator using CLI control planes only"}
{"truth":["swarm=orchestration_state","bv=bead_triage","br=bead_lifecycle"]}
{"hard_rule":"Never invent state. Every decision references latest CLI JSON"}
{"hard_rule":"Never leak secrets: mask DATABASE_URL/tokens/.env as ********"}
{"hard_rule":"Never claim success without post-action swarm status verification"}
{"hard_rule":"Use moon for Rust checks/tests/builds; never run raw cargo commands"}
{"hard_rule":"Do not modify lint/clippy config files or lint policy"}
{"hard_rule":"Manual testing mandatory: we manually test all of our shit"}
```

## Mandatory Rules

```jsonl
{"rule":"NO_CLIPPY_EDITS","action":"Fix code, not lint config"}
{"rule":"MOON_ONLY","cmds":["moon run :quick","moon run :test","moon run :build","moon run :ci","moon run :fmt-fix"],"never":["cargo fmt","cargo test","cargo clippy","cargo build"]}
{"rule":"CODANNA_MANDATORY","cmds":["semantic_search_with_context","analyze_impact","find_symbol","get_calls","find_callers","search_symbols","semantic_search_docs","get_index_info"],"prefer":["Codanna MCP tools for ALL exploration/search/retrieval"],"fallback":["NONE - Codanna is required"]}
{"rule":"ZERO_UNWRAP_PANIC","required":["Result<T,E>","?","map","and_then"],"banned":["unwrap()","unwrap_or()","unwrap_or_else()","unwrap_or_default()","expect()","panic!()","todo!()","unimplemented!()"]}
{"rule":"GIT_PUSH_MANDATORY","action":"Not done until git push succeeds"}
{"rule":"BR_SYNC","action":"After br sync --flush-only: git add .beads/ && git commit -m 'sync beads'"}
{"rule":"FUNCTIONAL_RUST_SKILL","action":"Load functional-rust-generator skill for ALL Rust implementation"}
{"rule":"DOMAIN_DRIVEN_DESIGN","patterns":["Bounded contexts","Aggregates","Value objects","Domain events","Repository pattern","Factory pattern"],"action":"Model domain logic explicitly; separate domain from infrastructure"}
{"rule":"MANUAL_TESTING","action":"After implementation: manually test via CLI; verify actual behavior; no mocking reality"}
```

## Commands

```jsonl
{"run.first":["swarm --help","swarm doctor","swarm state","swarm status","bv --robot-next","br --help"]}
{"run.if_uninitialized":["swarm init-db --seed-agents 4","swarm register --count 4","swarm status"]}
{"assert.after_init":["status.total==4","status.idle==4"]}
{"run.select_work":["bv --robot-triage || bv --robot-next","br update <bead-id> --status in_progress","br show <bead-id>"]}
{"assert.claim":["bead.status==in_progress"]}
{"run.smoke":["swarm agent --id 1 --dry","swarm agent --id 1","swarm monitor --view failures","swarm status"]}
{"assert.smoke":["no_runtime_crash","structured_response_present","state_transitions_visible"]}
{"run.fanout_if_smoke_ok":["swarm agent --id 1","swarm agent --id 2","swarm agent --id 3","swarm agent --id 4"]}
{"run.monitor_loop":["swarm monitor --view active","swarm monitor --view progress","swarm monitor --view failures","swarm status"]}
{"loop":"Observe->Decide->Act->Verify until terminal_condition"}
{"pipeline.order":["rust-contract","implement","qa-enforcer","red-queen"]}
{"retry.policy":"qa-enforcer/red-queen failure => feedback->implement retry; max_implement_attempts=3; then blocked(reason)"}
{"run.on_db_error":["swarm state","swarm status","swarm init-db","retry_failed_command_once"]}
{"run.on_stuck_agent":["swarm monitor --view failures","swarm release --agent_id <id>","swarm agent --id <id>"]}
{"run.on_no_work":["report no_work (not failure)"]}
{"run.on_completion":["br update <bead-id> --status done || accepted_per_repo","swarm status","swarm state"]}
{"run.on_terminal_failure":["br update <bead-id> --status blocked --reason '<explicit-reason>'"]}
{"run.manual_test":["Execute actual CLI command","Observe real output","Verify behavior matches contract","Report findings"]}
```

## Workflow

```jsonl
{"step":"TRIAGE","cmds":["bv --robot-triage","bv --robot-next"],"output":"Select highest priority bead"}
{"step":"CLAIM","cmds":["br update <bead-id> --status in_progress","br show <bead-id>"],"output":"Bead marked in_progress"}
{"step":"ISOLATE","cmds":["zjj add <workspace-name>","zjj focus <workspace-name>"],"output":"Isolated workspace active"}
{"step":"IMPLEMENT","cmds":["Load functional-rust-generator skill","Implement with Result<T,E> + DDD patterns","moon run :quick","moon run :test"],"output":"Code passes all checks"}
{"step":"MANUAL_TEST","cmds":["Run actual CLI commands","Verify real behavior","Test edge cases","Document findings"],"output":"Manual verification complete"}
{"step":"REVIEW","cmds":["moon run :ci","swarm monitor --view failures"],"output":"All quality gates pass"}
{"step":"LAND","cmds":["git add .","git commit -m '<msg>'","git push"],"output":"Changes pushed to remote"}
{"step":"MERGE","cmds":["br update <bead-id> --status done","jj rebase -d main"],"output":"Work merged and complete"}
```

## Functional Rust

```jsonl
{"pattern":"Railway-Oriented Programming","use":["Result<T,E> everywhere","? operator for propagation","map/and_then for transformation","Early returns with Err"],"avoid":["unwrap variants","panic variants","null/option where Result fits"]}
{"pattern":"Pure Functions","use":["Input -> Output","No side effects in domain logic","Deterministic behavior"],"avoid":["Global state","Hidden mutations","Unpredictable behavior"]}
{"pattern":"Type Safety","use":["Newtype pattern","Phantom types","Type-driven design","Compile-time guarantees"],"avoid":["Stringly-typed APIs","Primitive obsession","Runtime validation only"]}
{"pattern":"Immutability","use":["Immutable by default","let instead of let mut","Clone when needed"],"avoid":["Unnecessary mut","Shared mutable state"]}
```

## Domain-Driven Design

```jsonl
{"pattern":"Bounded Context","action":"Each module is a clear boundary; explicit interfaces between contexts"}
{"pattern":"Aggregates","action":"Cluster entities and value objects; enforce invariants at aggregate root"}
{"pattern":"Value Objects","action":"Immutable types for domain concepts; equality by value not identity"}
{"pattern":"Domain Events","action":"Model state changes as events; enable event sourcing patterns"}
{"pattern":"Repository Pattern","action":"Abstract persistence; domain doesn't know about storage details"}
{"pattern":"Factory Pattern","action":"Complex object creation logic; validate invariants at construction"}
{"pattern":"Ubiquitous Language","action":"Code uses exact domain terminology; types mirror domain concepts"}
```

## Output Parsing

```jsonl
{"parse.output":["use fields: ok,err,d,next,state","prefer parsed counters: done,working,waiting,error,idle,total"]}
{"report.format":["Bead Selected","Commands Run","State Observed","Action Taken","Result","Next Command"]}
{"report.style":"Concise, factual, parsed JSON facts only; no long raw log dumps unless asked"}
```

## Terminal Conditions

```jsonl
{"terminal.conditions":["done","no_work","blocked_with_reason","hard_error_requires_human"]}
{"stop.rule":"Stop only when terminal condition is reached and reported"}
```

## Banned Commands

```jsonl
{"banned":["cat .env","printenv | grep -i token","echo $DATABASE_URL","cargo fmt","cargo test","cargo clippy","cargo build","git reset --hard","git checkout -- ."]}
{"allowed":["swarm ...","bv --robot-*","br ...","moon run :check|:test|:build|:quick|:ci|:fmt-fix"]}
```
