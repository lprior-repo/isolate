#!/bin/bash
# Script to replace .expect() calls with match statements in test files

# Function to fix expect calls in a file
fix_file() {
    local file="$1"
    echo "Fixing $file..."

    # Backup the file
    cp "$file" "${file}.bak"

    # Use sed to replace patterns
    # Pattern 1: serde_json::from_str(...).expect("msg")
    # Pattern 2: .as_array().expect("msg")
    # Pattern 3: .as_str().expect("msg")
    # Pattern 4: tempfile::tempdir().expect("msg")

    # This is a complex replacement, so we'll do it with Perl for better regex support
    perl -i -pe '
        # Fix serde_json::from_str().expect()
        s/serde_json::from_str\(([^)]+)\)\.expect\("([^"]+)"\)/match serde_json::from_str($1) { Ok(v) => v, Err(e) => panic!("$2: {e}") }/g;

        # Fix .as_array().expect()
        s/\.as_array\(\)\.expect\("([^"]+)"\)/match .as_array() { Some(a) => a, None => panic!("$1") }/g;

        # Fix .as_str().expect()
        s/\.as_str\(\)\.expect\("([^"]+)"\)/match .as_str() { Some(s) => s, None => panic!("$1") }/g;

        # Fix tempfile::tempdir().expect()
        s/tempfile::tempdir\(\)\.expect\("([^"]+)"\)/match tempfile::tempdir() { Ok(d) => d, Err(e) => panic!("$1: {e}") }/g;

        # Fix TempDir::new().expect()
        s/TempDir::new\(\)\.expect\("([^"]+)"\)/match TempDir::new() { Ok(d) => d, Err(e) => panic!("$1: {e}") }/g;
    ' "$file"

    echo "Fixed $file"
}

# Fix all test files
for file in crates/zjj/tests/*.rs; do
    if [ -f "$file" ]; then
        fix_file "$file"
    fi
done
