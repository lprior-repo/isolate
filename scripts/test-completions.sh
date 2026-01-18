#!/usr/bin/env bash
# Test shell completions generation
#
# This script tests that completions can be generated for all supported shells
# Run from repository root: ./scripts/test-completions.sh

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "Testing zjj shell completions..."
echo

# Check if zjj is built
if ! command -v zjj &> /dev/null; then
    echo -e "${YELLOW}zjj not found in PATH. Building...${NC}"
    moon run :build

    # Try to find the built binary
    if [ -f "target/release/zjj" ]; then
        JJZ_CMD="./target/release/zjj"
    elif [ -f "target/debug/zjj" ]; then
        JJZ_CMD="./target/debug/zjj"
    else
        echo -e "${RED}Failed to build zjj${NC}"
        exit 1
    fi
else
    JJZ_CMD="zjj"
fi

echo "Using: $JJZ_CMD"
echo

# Test each shell
SHELLS=("bash" "zsh" "fish")
FAILED=0

for shell in "${SHELLS[@]}"; do
    echo -n "Testing $shell completions... "

    if $JJZ_CMD completions "$shell" > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗${NC}"
        FAILED=$((FAILED + 1))
    fi
done

echo

# Test with instructions flag
echo -n "Testing --instructions flag... "
if $JJZ_CMD completions bash --instructions > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗${NC}"
    FAILED=$((FAILED + 1))
fi

echo

# Test invalid shell
echo -n "Testing invalid shell (should fail)... "
if ! $JJZ_CMD completions invalid > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗ (should have failed)${NC}"
    FAILED=$((FAILED + 1))
fi

echo

# Summary
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}$FAILED test(s) failed${NC}"
    exit 1
fi
