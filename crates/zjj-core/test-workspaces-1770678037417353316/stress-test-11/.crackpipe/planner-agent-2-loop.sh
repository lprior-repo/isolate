#!/bin/bash
# Planner Agent 2: Create contracts for beads without contracts

REPO="/home/lewis/src/zjj"
LOG_FILE="$REPO/.crackpipe/planner-agent-2.log"
STATUS_FILE="$REPO/.crackpipe/PLANNER-AGENT-2-STATUS.md"
BEADS_LOG="$REPO/.crackpipe/BEADS.md"

cd "$REPO" || exit 1

echo "[$(date '+%Y-%m-%d %H:%M:%S')] Planner Agent 2 started" >> "$LOG_FILE"
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Monitoring for beads needing contracts..." >> "$LOG_FILE"

# Main loop
while true; do
    cd "$REPO" || exit 1
    
    # Get existing contract IDs
    existing_contracts=$(ls .crackpipe/rust-contract-*.md 2>/dev/null | xargs -I{} basename {} .md | sed 's/rust-contract-//' | sort)
    
    # Find beads that need contracts (in_progress or open status without contracts)
    bead_id=$(jq -s '.[] | select(.status == "in_progress" or .status == "open") | .id' .beads/issues.jsonl 2>/dev/null | head -1 | tr -d '"')
    
    if [ -n "$bead_id" ]; then
        # Check if contract already exists
        if ! echo "$existing_contracts" | grep -q "^${bead_id}$"; then
            echo "[$(date '+%Y-%m-%d %H:%M:%S')] Found bead needing contract: $bead_id" >> "$LOG_FILE"
            
            # Get bead details
            bead_json=$(jq -s ".[] | select(.id == \"$bead_id\")" .beads/issues.jsonl 2>/dev/null)
            title=$(echo "$bead_json" | jq -r '.title // "Unknown"' 2>/dev/null)
            description=$(echo "$bead_json" | jq -r '.description // ""' 2>/dev/null)
            bead_type=$(echo "$bead_json" | jq -r '.issue_type // "task"' 2>/dev/null)
            
            echo "[$(date '+%Y-%m-%d %H:%M:%S')] Processing: $title" >> "$LOG_FILE"
            
            # Create rust contract
            cat > ".crackpipe/rust-contract-${bead_id}.md" << CONTRACT_EOF
# Rust Contract: ${bead_id}

## Title
${title}

## Type
${bead_type}

## Description
${description}

## Preconditions
- TBD (will be filled by AI analysis)

## Postconditions
- TBD (will be filled by AI analysis)

## Invariants
- TBD (will be filled by AI analysis)
CONTRACT_EOF
            
            # Create martin fowler test plan
            cat > ".crackpipe/martin-fowler-tests-${bead_id}.md" << TEST_EOF
# Martin Fowler Test Plan: ${bead_id}

## Title
${title}

## Test Cases
- TBD (will be filled by AI analysis)
TEST_EOF
            
            echo "[$(date '+%Y-%m-%d %H:%M:%S')] Created contract and test plan for $bead_id" >> "$LOG_FILE"
            echo "[$(date '+%Y-%m-%d %H:%M:%S')] $bead_id: Created contract + test plan (planner-2)" >> "$BEADS_LOG"
            
            # Update bead stage to ready-builder
            br update "$bead_id" --set-labels "stage:ready-builder,actor:planner-2" >/dev/null 2>&1
            
            echo "[$(date '+%Y-%m-%d %H:%M:%S')] Updated $bead_id to stage:ready-builder" >> "$LOG_FILE"
        fi
    else
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] No beads needing contracts found" >> "$LOG_FILE"
    fi
    
    # Wait 90 seconds as requested
    sleep 90
done
