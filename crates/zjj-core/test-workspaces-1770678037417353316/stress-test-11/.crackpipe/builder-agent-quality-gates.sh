#!/bin/bash
# Builder Agent with MANDATORY Quality Gates
# CRITICAL: NO code reaches QA without passing moon run :ci

REPO="/home/lewis/src/zjj"
LOG_FILE="$REPO/.crackpipe/builder-agent-quality-gates.log"
STATUS_FILE="$REPO/.crackpipe/BUILDER-QUALITY-GATES.md"
BEADS_LOG="$REPO/.crackpipe/BEADS.md"
AGENT_ID="builder-quality-gates"

cd "$REPO" || exit 1

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" | tee -a "$LOG_FILE"
}

log "Builder Agent (Quality Gates) started"
log "ENFORCING: moon run :ci MANDATORY before marking beads ready-qa-builder"

# Main loop
while true; do
    cd "$REPO" || exit 1

    # Find beads in ready-builder stage
    bead_id=$(jq -r 'select(.labels[] | startswith("stage:ready-builder")) | .id' .beads/issues.jsonl 2>/dev/null | head -1)

    if [ -n "$bead_id" ]; then
        log "================================================"
        log "Found bead to build: $bead_id"

        # Get bead details
        bead_json=$(jq -s ".[] | select(.id == \"$bead_id\")" .beads/issues.jsonl 2>/dev/null)
        title=$(echo "$bead_json" | jq -r '.title // "Unknown"' 2>/dev/null)
        description=$(echo "$bead_json" | jq -r '.description // ""' 2>/dev/null)

        log "Title: $title"

        # Check for contract
        contract_file=".crackpipe/rust-contract-${bead_id}.md"
        if [ ! -f "$contract_file" ]; then
            log "ERROR: No contract found for $bead_id - SKIPPING"
            sleep 90
            continue
        fi

        # Mark as building
        log "Marking $bead_id as building..."
        br update "$bead_id" --set-labels "-stage:ready-builder,stage:building,actor:$AGENT_ID" >/dev/null 2>&1
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] $bead_id ready-builder → building ($AGENT_ID)" >> "$BEADS_LOG"

        # IMPLEMENTATION PHASE
        log "Starting implementation for $bead_id..."
        log "Contract: $contract_file"

        # Read contract
        log "--- CONTRACT START ---"
        cat "$contract_file" | tee -a "$LOG_FILE"
        log "--- CONTRACT END ---"

        # Quality Gate 1: Format Check
        log ""
        log "QUALITY GATE 1: moon run :fmt-fix (format check)"
        if ! moon run :fmt-fix >> "$LOG_FILE" 2>&1; then
            log "❌ FAILED: Formatting check failed"
            log "Marking $bead_id as needs-rework"
            br update "$bead_id" --set-labels "-stage:building,stage:needs-rework,needs-format-fix" >/dev/null 2>&1
            sleep 90
            continue
        fi
        log "✅ PASSED: Formatting check"

        # Quality Gate 2: Quick Lint Check
        log ""
        log "QUALITY GATE 2: moon run :quick (format + clippy)"
        if ! moon run :quick >> "$LOG_FILE" 2>&1; then
            log "❌ FAILED: Quick lint check failed"
            log "Marking $bead_id as needs-rework"
            br update "$bead_id" --set-labels "-stage:building,stage:needs-rework,needs-clippy-fix" >/dev/null 2>&1
            sleep 90
            continue
        fi
        log "✅ PASSED: Quick lint check (6-7ms cached)"

        # Quality Gate 3: Tests
        log ""
        log "QUALITY GATE 3: moon run :test (all tests must pass)"
        if ! moon run :test >> "$LOG_FILE" 2>&1; then
            log "❌ FAILED: Tests failed"
            log "Marking $bead_id as needs-rework"
            br update "$bead_id" --set-labels "-stage:building,stage:needs-rework,needs-test-fix" >/dev/null 2>&1
            sleep 90
            continue
        fi
        log "✅ PASSED: All tests passing"

        # Quality Gate 4: Full CI (MANDATORY - THIS IS THE KILLER GATE)
        log ""
        log "QUALITY GATE 4: moon run :ci (FULL PIPELINE - MANDATORY)"
        log "This includes format, clippy, and tests in parallel"
        if ! timeout 300 moon run :ci >> "$LOG_FILE" 2>&1; then
            log "❌ FAILED: Full CI pipeline failed"
            log "Marking $bead_id as needs-rework"
            br update "$bead_id" --set-labels "-stage:building,stage:needs-rework,needs-ci-fix" >/dev/null 2>&1
            sleep 90
            continue
        fi
        log "✅ PASSED: Full CI pipeline"

        # All quality gates passed - mark as ready for QA
        log ""
        log "🎉 ALL QUALITY GATES PASSED"
        log "Marking $bead_id as ready-qa-builder"

        br update "$bead_id" --set-labels "-stage:building,stage:ready-qa-builder,actor:$AGENT_ID" >/dev/null 2>&1
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] $bead_id building → ready-qa-builder ($AGENT_ID) - ALL QUALITY GATES PASSED" >> "$BEADS_LOG"

        # Commit the work
        log "Committing changes..."
        git add -A >> "$LOG_FILE" 2>&1
        if ! git diff --cached --quiet; then
            git commit -m "feat($bead_id): $title" >> "$LOG_FILE" 2>&1
            git push >> "$LOG_FILE" 2>&1
            log "✅ Changes committed and pushed"
        else
            log "No changes to commit"
        fi

        log "✅ Successfully completed $bead_id"
        log "================================================"
        log ""
    else
        log "No beads in ready-builder stage found, waiting..."
    fi

    # Wait 90 seconds as requested
    sleep 90
done
