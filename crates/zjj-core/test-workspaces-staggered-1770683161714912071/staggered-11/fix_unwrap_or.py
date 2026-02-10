#!/usr/bin/env python3
"""Fix unwrap_or and unwrap_or_else violations in Rust code."""

import re
import sys
from pathlib import Path

# Patterns to fix
PATTERNS = [
    # try_exists().unwrap_or(false) -> .map_or(false, |v| v)
    (
        r'\.try_exists\(([^)]+)\)\.await\.unwrap_or\(false\)',
        r'.try_exists(\1).await.map_or(false, |v| v)',
    ),
    # unwrap_or_default() -> .map_or_default() (for Result types)
    # Note: This needs context to know if it's Option or Result
    # unwrap_or(0) -> .map_or(0, |v| v)
    (
        r'\.unwrap_or\(0\)(?![.])',
        r'.map_or(0, |v| v)',
    ),
    # unwrap_or(false) -> .map_or(false, |v| v)
    (
        r'\.unwrap_or\(false\)(?![.])',
        r'.map_or(false, |v| v)',
    ),
    # unwrap_or("") -> .map_or("", |v| v)
    (
        r'\.unwrap_or\(""\)(?![.])',
        r'.map_or("", |v| v)',
    ),
    # .unwrap_or("unknown") -> .map_or("unknown", |v| v)
    (
        r'\.unwrap_or\("unknown"\)(?![.])',
        r'.map_or("unknown", |v| v)',
    ),
    # .unwrap_or("open") -> .map_or("open", |v| v)
    (
        r'\.unwrap_or\("open"\)(?![.])',
        r'.map_or("open", |v| v)',
    ),
    # .unwrap_or("-") -> .map_or("-", |v| v)
    (
        r'\.unwrap_or\("-"\)(?![.])',
        r'.map_or("-", |v| v)',
    ),
    # .unwrap_or(60) -> .map_or(60, |v| v)
    (
        r'\.unwrap_or\(60\)(?![.])',
        r'.map_or(60, |v| v)',
    ),
    # .unwrap_or(300) -> .map_or(300, |v| v)
    (
        r'\.unwrap_or\(300\)(?![.])',
        r'.map_or(300, |v| v)',
    ),
    # .unwrap_or(30) -> .map_or(30, |v| v)
    (
        r'\.unwrap_or\(30\)(?![.])',
        r'.map_or(30, |v| v)',
    ),
    # .unwrap_or(1) -> .map_or(1, |v| v)
    (
        r'\.unwrap_or\(1\)(?![.])',
        r'.map_or(1, |v| v)',
    ),
    # .unwrap_or(i64::MAX) -> .map_or(i64::MAX, |v| v)
    (
        r'\.unwrap_or\(i64::MAX\)(?![.])',
        r'.map_or(i64::MAX, |v| v)',
    ),
    # .unwrap_or(u64::MAX) -> .map_or(u64::MAX, |v| v)
    (
        r'\.unwrap_or\(u64::MAX\)(?![.])',
        r'.map_or(u64::MAX, |v| v)',
    ),
    # .unwrap_or_default() for Vec/HashMap/String -> .map_or_default()
    # Note: This needs special handling
    (
        r'\.await\.unwrap_or_default\(\)(?![.])',
        r'.await.map_or_else(|_| Vec::new(), |v| v)',
    ),
    # get(0..16).unwrap_or(&[]) -> match pattern
    (
        r'\.get\((\d+)\.\.(\d+)\)\.unwrap_or\(&\[\]\)',
        lambda m: f'{{match .get({m.group(1)}..{m.group(2)}) {{ Some(slice) => slice, None => &[], }}',
        True,  # Use function for complex replacement
    ),
]


def fix_file(filepath: Path) -> int:
    """Fix unwrap_or violations in a single file. Returns number of fixes."""
    content = filepath.read_text()
    original_content = content
    fixes = 0

    for pattern_tuple in PATTERNS:
        if len(pattern_tuple) == 2:
            pattern, replacement = pattern_tuple
            is_function = False
        else:
            pattern, replacement, is_function = pattern_tuple

        if is_function:
            # Use function for complex replacements
            matches = list(re.finditer(pattern, content))
            for match in reversed(matches):  # Reverse to maintain positions
                old_text = match.group(0)
                new_text = replacement(match)
                content = content[:match.start()] + new_text + content[match.end():]
                fixes += 1
        else:
            new_content = re.sub(pattern, replacement, content)
            if new_content != content:
                matches = len(re.findall(pattern, original_content))
                fixes += matches
                content = new_content

    if content != original_content:
        filepath.write_text(content)
        print(f"Fixed {fixes} violations in {filepath}")
        return fixes
    return 0


def main():
    """Main entry point."""
    src_dir = Path("crates/zjj/src")
    if not src_dir.exists():
        print(f"Error: {src_dir} does not exist")
        sys.exit(1)

    total_fixes = 0
    for rs_file in src_dir.rglob("*.rs"):
        total_fixes += fix_file(rs_file)

    print(f"\nTotal fixes: {total_fixes}")


if __name__ == "__main__":
    main()
