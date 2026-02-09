#!/bin/bash
# block-rust-unsafe.sh - Prevent unsafe Rust patterns in source code
# Enforces ZERO_UNWRAP_ZERO_EXPECT_ZERO_PANIC rule from CLAUDE.md

set -euo pipefail

# Read JSON input from stdin
INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

# Only check Rust source files (not tests)
if [[ ! "$FILE_PATH" =~ ^crates/.*/src/.*\.rs$ ]]; then
    exit 0
fi

# Check if file exists (for new files being created)
if [[ ! -f "$FILE_PATH" ]]; then
    # For Write operations on new files, check the content
    CONTENT=$(echo "$INPUT" | jq -r '.tool_input.content // empty')
    # Check for unsafe patterns using fixed string matching for reliability
    if echo "$CONTENT" | grep -qF 'unwrap()' || \
       echo "$CONTENT" | grep -qF 'expect(' || \
       echo "$CONTENT" | grep -qF 'panic!(' || \
       echo "$CONTENT" | grep -qF 'todo!(' || \
       echo "$CONTENT" | grep -qF 'unimplemented!('; then
        echo "❌ BLOCKED: Unsafe Rust pattern in new file $FILE_PATH" >&2
        echo "" >&2
        echo "The following patterns are PROHIBITED in src/ code:" >&2
        echo "  • unwrap() - Use proper error handling with Result<T, E>" >&2
        echo "  • expect() - Use Result<T, E> with ? operator" >&2
        echo "  • panic!() - Never panic in production code" >&2
        echo "  • todo!() - Implement the function or use a placeholder" >&2
        echo "  • unimplemented!() - Implement the function" >&2
        echo "" >&2
        echo "ALLOWED in test code: unwrap(), expect(), panic!() for test scenarios" >&2
        echo "See CLAUDE.md rule #5: ZERO_UNWRAP_ZERO_EXPECT_ZERO_PANIC" >&2
        exit 2
    fi
    exit 0
fi

# Check existing file for unsafe patterns
# Use grep to find violations
if grep -qF 'unwrap()' "$FILE_PATH" || \
   grep -qF 'expect(' "$FILE_PATH" || \
   grep -qF 'panic!(' "$FILE_PATH" || \
   grep -qF 'todo!(' "$FILE_PATH" || \
   grep -qF 'unimplemented!(' "$FILE_PATH"; then
    PATTERN=""
    if grep -qF 'unwrap()' "$FILE_PATH"; then PATTERN="unwrap()"; fi
    if grep -qF 'expect(' "$FILE_PATH"; then PATTERN="${PATTERN:-} expect("; fi
    if grep -qF 'panic!(' "$FILE_PATH"; then PATTERN="${PATTERN:-} panic!("; fi
    if grep -qF 'todo!(' "$FILE_PATH"; then PATTERN="${PATTERN:-} todo!("; fi
    if grep -qF 'unimplemented!(' "$FILE_PATH"; then PATTERN="${PATTERN:-} unimplemented!("; fi

    echo "❌ BLOCKED: Unsafe Rust pattern(s) found in $FILE_PATH" >&2
    echo "" >&2
    echo "Fix required: Replace with proper error handling" >&2
    echo "See CLAUDE.md rule #5: ZERO_UNWRAP_ZERO_EXPECT_ZERO_PANIC" >&2
    exit 2
fi

exit 0
