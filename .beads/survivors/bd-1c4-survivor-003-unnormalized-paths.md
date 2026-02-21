# Survivor: Unnormalized File Paths

**Campaign:** bd-1c4-redqueen
**Generation:** 1
**Severity:** MAJOR
**Status:** ALIVE

## Discovery

Red-queen contract analysis discovered that the implementation does not normalize file paths as required by POST-DET-005.

## Vulnerability

**Location:** `/home/lewis/src/zjj/crates/zjj/src/commands/done/conflict.rs:360-391`

The `parse_diff_summary` function returns file paths directly from JJ output without any normalization.

**Contract Requirement:** POST-DET-005 states "All file paths are normalized (no `./` or `../`)"

**Issue:** JJ may return paths like:
- `./src/lib.rs`
- `../include/header.h`
- `src/../../lib.rs`

These would be passed through as-is.

## Impact

- **Likelihood:** MEDIUM - depends on JJ behavior
- **Severity:** MAJOR - contract violation, potential downstream bugs
- **Scope:** All file paths in conflict reports

## Analysis

Looking at JJ's source code and documentation, JJ typically normalizes paths internally. However, the contract explicitly requires normalization, and the implementation doesn't verify or enforce this.

## Recommendations

1. **Verify JJ behavior:** Test if JJ ever returns unnormalized paths
2. **Add defensive normalization:**
```rust
use std::path::Path;

fn normalize_path(path: &str) -> String {
    Path::new(path)
        .components()
        .filter(|c| !matches!(c, std::path::Component::CurDir | std::path::Component::ParentDir))
        .collect::<std::path::PathBuf>()
        .to_string_lossy()
        .to_string()
}
```

3. **Add contract verification test:**
```rust
#[test]
fn verify_path_normalization() {
    let paths = vec!["./file.rs", "../lib.rs", "src/./main.rs"];
    for path in paths {
        let normalized = normalize_path(path);
        assert!(!normalized.contains("./"));
        assert!(!normalized.contains("../"));
    }
}
```

## Files

- Implementation: `/home/lewis/src/zjj/crates/zjj/src/commands/done/conflict.rs:360-391`
- Contract: `/home/lewis/src/zjj/contracts/bd-1c4-contract-spec.md:404`

## Fitness Impact

- Contract compliance: -10%
- Robustness: -5%
- Overall fitness: -6%
