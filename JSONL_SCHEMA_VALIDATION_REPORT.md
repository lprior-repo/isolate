# JSONL Schema Validation Report

## Executive Summary

After executing comprehensive integration tests comparing Rust JSONL output types against CUE schema specifications, **2 critical discrepancies** were identified.

## Test Execution

```bash
Command: cargo test --package zjj-core --test jsonl_schema_validation_test
Exit Code: 101 (tests failed)
Passed: 16/18
Failed: 2/18
```

## Findings

### 1. CRITICAL: OutputLine enum serialization structure mismatch

**Severity:** CRITICAL
**File:** `/home/lewis/src/zjj/crates/zjj-core/src/output/types.rs:43-59`
**Line:** 43-79

**Expected (CUE schema zjj-20260217-001):**
```rust
#[serde(tag = "type")]
pub enum OutputLine {
    Summary { ... },
    Session { ... },
    // ...
}
```

Expected JSON output:
```json
{
  "type": "summary",
  "message": "Test",
  "timestamp": 1771735016195
}
```

**Actual (Rust code):**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputLine {
    Summary(Summary),
    Session(SessionOutput),
    // ...
}
```

Actual JSON output:
```json
{
  "summary": {
    "type": "info",
    "message": "Test",
    "timestamp": 1771735016195
  }
}
```

**Impact:**
- **Breaking change** for AI agents parsing JSONL output
- The type discriminator is nested inside the variant, not at the top level
- AI agents must know the variant name in advance to parse correctly
- Violates the design principle of "self-describing JSON objects"

**Evidence:**
```
thread 'test_output_line_enum_discriminator' (2765712) panicked at crates/zjj-core/tests/jsonl_schema_validation_test.rs:290:9:
OutputLine variant missing type discriminator: {"summary":{"message":"Test","timestamp":1771735016195,"type":"info"}}
```

**Fix Required:**
Change OutputLine enum from newtype variants to struct variants with `#[serde(tag = "type")]`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum OutputLine {
    #[serde(rename = "summary")]
    Summary {
        #[serde(flatten)]
        inner: Summary
    },
    // ... or restructure to:
    Summary {
        type_field: SummaryType,
        message: String,
        // ... move all Summary fields here
    },
}
```

---

### 2. CRITICAL: ConflictAnalysis type field hardcoded incorrectly

**Severity:** CRITICAL
**File:** `/home/lewis/src/zjj/crates/zjj-core/src/output/types.rs:1113`
**Line:** 1113

**Expected:**
The `type` field should be "conflict_analysis" to match the OutputLine variant name.

**Actual:**
```rust
pub fn conflict_analysis(
    session: &str,
    merge_safe: bool,
    conflicts: Vec<ConflictDetail>,
) -> Self {
    Self::ConflictAnalysis(ConflictAnalysis {
        type_field: "conflictdetail".to_string(),  // ← WRONG! Should be "conflict_analysis"
        // ...
    })
}
```

**Evidence:**
```
thread 'test_conflict_analysis_serialization' (2765712) panicked at crates/zjj-core/tests/jsonl_schema_validation_test.rs:13:9:
Missing required field 'type' in JSON: {... "type":"conflictdetail" ...}
```

**Impact:**
- Type field doesn't match the enum variant name
- Inconsistent with other OutputLine variants
- May confuse AI agents expecting "conflict_analysis"

**Fix Required:**
```rust
type_field: "conflict_analysis".to_string(),  // Fix typo: conflictdetail → conflict_analysis
```

---

## Discrepancy Details

### Finding 1: OutputLine uses newtype variants instead of struct variants with tag

**CUE Schema (zjj-20260217-001-jsonl-core-types.cue):**
```cue
types: {
    OutputLine: #"""
        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(tag = "type")]
        pub enum OutputLine {
            Summary {
                total: usize,
                active: usize,
                // ...
            },
            Session {
                name: String,
                state: String,
                // ...
            },
            // ...
        }
        """#
}
```

**Rust Implementation:**
```rust
// Line 43-59 of types.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]  // ← Should be #[serde(tag = "type")]
pub enum OutputLine {
    Summary(Summary),  // ← Newtype variant, not struct variant
    Session(SessionOutput),
    // ...
}
```

**Root Cause:**
The implementation uses newtype variants (wrapping existing structs) instead of inline struct variants with a tag discriminator. This is a common pattern in Rust for keeping code DRY, but it violates the CUE schema specification.

**JSON Output Comparison:**

Expected (tagged enum):
```json
{"type":"summary","message":"Test","timestamp":1771735016195}
```

Actual (nested newtype):
```json
{"summary":{"type":"info","message":"Test","timestamp":1771735016195}}
```

---

## All Other Tests Passed

✅ Summary serialization
✅ SessionOutput serialization
✅ Issue serialization
✅ Plan serialization
✅ Stack serialization
✅ QueueSummary serialization
✅ QueueEntry serialization
✅ Train serialization
✅ ConflictDetail serialization
✅ Enum value serialization (lowercase snake_case)
✅ ActionStatus serialization
✅ TrainAction serialization
✅ ResolutionStrategy serialization
✅ Timestamp format (milliseconds)
✅ Optional fields handling
✅ Recovery type serialization

---

## Recommendations

### Priority 1: Fix OutputLine enum structure
- Change from newtype variants to tagged enum
- This is a **breaking change** for any consumers
- Requires updating all parsers that expect the current nested structure
- Update CUE schema to match actual implementation if tagged enum is not feasible

### Priority 2: Fix ConflictAnalysis type field
- Change "conflictdetail" to "conflict_analysis"
- This is a simple typo fix

### Priority 3: Update documentation
- Document the actual JSON structure if the current implementation is intentional
- Update CUE schemas to match implementation
- Ensure AI contracts in `json_docs.rs` match the actual output

---

## Test Evidence

All test files:
- `/home/lewis/src/zjj/crates/zjj-core/tests/jsonl_schema_validation_test.rs`
- CUE schemas: `/home/lewis/src/zjj/.beads/beads/zjj-20260217-*.cue`

Command to reproduce:
```bash
cargo test --package zjj-core --test jsonl_schema_validation_test
```

---

## Severity Assessment

| Finding | Severity | Impact | Fix Complexity |
|---------|----------|--------|----------------|
| OutputLine structure | CRITICAL | Breaking change for AI agents | High (requires redesign) |
| ConflictAnalysis type field | CRITICAL | Inconsistent type naming | Low (typo fix) |

## Conclusion

The JSONL output types are **mostly consistent** with CUE schemas (16/18 tests passing), but there are **2 critical issues** that violate the design principles:

1. The OutputLine enum uses newtype variants instead of tagged enums, breaking the "self-describing" principle
2. The ConflictAnalysis type field has a typo ("conflictdetail" instead of "conflict_analysis")

These issues should be fixed before the JSONL output system is used by production AI agents.
