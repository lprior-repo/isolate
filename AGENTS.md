# AGENTS.md

## Mandatory Rules

```jsonl
{"rule":"NO_CLIPPY_EDITS","action":"Fix code, not lint config"}
{"rule":"MOON_ONLY","cmds":["moon run :quick","moon run :test","moon run :build","moon run :ci","moon run :fmt-fix"],"never":["cargo fmt","cargo test","cargo clippy","cargo build"]}
{"rule":"CODANNA_MANDATORY","cmds":["semantic_search_with_context","analyze_impact","find_symbol","get_calls","find_callers","search_symbols","semantic_search_docs","get_index_info"],"prefer":["Codanna MCP tools for ALL exploration/search/retrieval"],"fallback":["NONE - Codanna is required"]}
{"rule":"ZERO_UNWRAP_PANIC","required":["Result<T,E>","?","map","and_then"],"banned":["unwrap()","unwrap_or()","unwrap_or_else()","unwrap_or_default()","expect()","panic!()","todo!()","unimplemented!()"]}
{"rule":"GIT_PUSH_MANDATORY","action":"Not done until git push succeeds"}
{"rule":"FUNCTIONAL_RUST_SKILL","action":"Load functional-rust-generator skill for ALL Rust implementation"}
{"rule":"DOMAIN_DRIVEN_DESIGN","patterns":["Bounded contexts","Aggregates","Value objects","Domain events","Repository pattern","Factory pattern"],"action":"Model domain logic explicitly; separate domain from infrastructure"}
{"rule":"MANUAL_TESTING","action":"After implementation: manually test via CLI; verify actual behavior; no mocking reality"}
```

## Workflow

```jsonl
{"step":"IMPLEMENT","cmds":["Load functional-rust-generator skill","Implement with Result<T,E> + DDD patterns","moon run :quick","moon run :test"],"output":"Code passes all checks"}
{"step":"MANUAL_TEST","cmds":["Run actual CLI commands","Verify real behavior","Test edge cases","Document findings"],"output":"Manual verification complete"}
{"step":"REVIEW","cmds":["moon run :ci"],"output":"All quality gates pass"}
{"step":"LAND","cmds":["git add .","git commit -m '<msg>'","git push"],"output":"Changes pushed to remote"}
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

## Banned Commands

```jsonl
{"banned":["cat .env","printenv | grep -i token","echo $DATABASE_URL","cargo fmt","cargo test","cargo clippy","cargo build","git reset --hard","git checkout -- ."]}
{"allowed":["moon run :check|:test|:build|:quick|:ci|:fmt-fix"]}
```
