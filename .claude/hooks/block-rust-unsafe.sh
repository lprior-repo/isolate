#!/bin/bash
# block-rust-unsafe.sh - Prevent unsafe Rust patterns in source code
# Enforces ZERO_UNWRAP_ZERO_EXPECT_ZERO_PANIC rule from CLAUDE.md

set -euo pipefail

# Read JSON input from stdin
INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')
OPERATION=$(echo "$INPUT" | jq -r '.tool // empty' | head -c 4)  # "Edit" or "Writ"

# Only check Rust source files (not tests directories)
if [[ ! "$FILE_PATH" =~ ^crates/.*/src/.*\.rs$ ]]; then
    exit 0
fi

# Skip files that contain test code (allow unwrap/expect in test modules)
if [[ -f "$FILE_PATH" ]]; then
    # Check if file contains test markers
    if grep -qF '#[cfg(test)]' "$FILE_PATH" || \
       grep -qF '#[test]' "$FILE_PATH" || \
       grep -qF '#[tokio::test]' "$FILE_PATH"; then
        exit 0
    fi
fi

# For Write operations on new files, check the content for test markers first
if [[ "$OPERATION" == "Writ" ]]; then
    CONTENT=$(echo "$INPUT" | jq -r '.tool_input.content // empty')
    # Allow if content contains test markers
    if echo "$CONTENT" | grep -qF '#[cfg(test)]' || \
       echo "$CONTENT" | grep -qF '#[test]' || \
       echo "$CONTENT" | grep -qF '#[tokio::test]'; then
        exit 0
    fi
    # Check for unsafe patterns using fixed string matching for reliability
    if echo "$CONTENT" | grep -qF 'unwrap()' || \
       echo "$CONTENT" | grep -qF 'expect(' || \
       echo "$CONTENT" | grep -qF 'panic!(' || \
       echo "$CONTENT" | grep -qF 'todo!(' || \
       echo "$CONTENT" | grep -qF 'unimplemented!('; then
        echo "❌ BLOCKED: Unsafe Rust pattern in new file $FILE_PATH" >&2
        echo "" >&2
        echo "The following patterns are PROHIBITED in production src/ code:" >&2
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

# For Edit operations, check the new_string being introduced
NEW_STRING=$(echo "$INPUT" | jq -r '.tool_input.new_string // empty')
if [[ -n "$NEW_STRING" ]]; then
    # Check for unsafe patterns in the new string being added
    if echo "$NEW_STRING" | grep -qF 'unwrap()' || \
       echo "$NEW_STRING" | grep -qF 'expect(' || \
       echo "$NEW_STRING" | grep -qF 'panic!(' || \
       echo "$NEW_STRING" | grep -qF 'todo!(' || \
       echo "$NEW_STRING" | grep -qF 'unimplemented!('; then
        echo "❌ BLOCKED: Unsafe Rust pattern in edit to $FILE_PATH" >&2
        echo "" >&2
        echo "The following patterns are PROHIBITED in production src/ code:" >&2
        echo "  • unwrap() - Use proper error handling with Result<T, E>" >&2
        echo "  • expect() - Use Result<T, E> with ? operator" >&2
        echo "  • panic!() - Never panic in production code" >&2
        echo "  • todo!() - Implement the function or use a placeholder" >&2
        echo "  • unimplemented!() - Implement the function" >&2
        echo "" >&2
        echo "ALLOWED in test code: unwrap(), expect(), panic!() for test scenarios" >&2
        echo "See CLAUDE.md rule #5: ZERO_UNWRAP_ZERO_EXPECT_ZERO_PANIC" >&2
        echo "" >&2
        echo "Pattern found in:" >&2
        echo "$NEW_STRING" | head -c 200 >&2
        exit 2
    fi
fi

exit 0
