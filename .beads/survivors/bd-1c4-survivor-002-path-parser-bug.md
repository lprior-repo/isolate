# Survivor: Path Parser Bug with " -> " in Filename

**Campaign:** bd-1c4-redqueen
**Generation:** 1
**Severity:** MAJOR
**Status:** ALIVE

## Discovery

Red-queen static analysis discovered that file paths containing the string " -> " are incorrectly parsed as renames.

## Vulnerability

**Location:** `/home/lewis/src/zjj/crates/zjj/src/commands/done/conflict.rs:376-382`

```rust
if file_part.contains(" -> ") {
    // For renames, consider the destination file
    file_part.split(" -> ").last().map(std::string::ToString::to_string)
} else {
    Some(file_part.to_string())
}
```

**Issue:** If a file is actually named `"a -> b.txt"`, the code splits on " -> " and incorrectly treats "b.txt" as the filename, losing the actual filename.

## Impact

- **Likelihood:** LOW - unusual but legal filename
- **Severity:** MAJOR - data corruption, incorrect conflict reports
- **Scope:** Any repository with files containing " -> " in name

## Proof of Concept

```bash
# Create a file with " -> " in the name
touch "weird -> file.txt"

# JJ diff output: "M weird -> file.txt"
# Parser splits on " -> " and returns "file.txt"
# Correct answer should be: "weird -> file.txt"
```

## Root Cause

The parser assumes " -> " only appears as a rename marker in JJ output, but doesn't account for it appearing in the actual filename.

## Recommendations

1. **Option A:** Use more precise regex that only matches " -> " at specific positions
2. **Option B:** Parse the status character separately, then take the rest of the line
3. **Option C:** Use JJ's JSON output format if available (machine-parseable)

## Example Fix

```rust
// Better parsing: split only on first space to get status
let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
if parts.len() >= 2 {
    let status = parts[0];  // "M", "A", "D", "R"
    let path_part = parts[1];  // Everything after first space
    // Now check for R (rename) status specifically
    if status == "R" && path_part.contains(" -> ") {
        path_part.split(" -> ").last()
    } else {
        Some(path_part.to_string())
    }
}
```

## Files

- Implementation: `/home/lewis/src/zjj/crates/zjj/src/commands/done/conflict.rs:360-391`
- Test: `/home/lewis/src/zjj/crates/zjj/tests/conflict_adversarial_tests.rs:82-100`

## Fitness Impact

- Implementation correctness: -10%
- Edge case handling: -15%
- Overall fitness: -8%
