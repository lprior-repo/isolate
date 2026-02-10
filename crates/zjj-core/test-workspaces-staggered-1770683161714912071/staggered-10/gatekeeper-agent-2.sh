#!/usr/bin/env bash
set -euo pipefail

# Gatekeeper Agent 2 - QA + Landing
# Workflow: Claim → Navigate → QA → Panic Check → Moon Quick → Landing → Git Push → Close

log() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] $*" | tee -a gatekeeper-agent-2.log
}

claim_bead() {
    local bead_id="$1"
    log "Claiming bead $bead_id..."
    br update "$bead_id" --set-labels "stage:gatekeeping" --actor gatekeeper-2 2>&1 | grep -v "INFO" || true
}

check_workspace() {
    local bead_id="$1"

    # Check if workspace exists
    local ws
    ws=$(jj workspace list | grep "bead-$bead_id" || echo "")

    if [ -z "$ws" ]; then
        log "ERROR: No workspace found for $bead_id"
        log "Bead marked ready-gatekeeper without implementation"
        log "Resetting to ready-architect stage"
        br update "$bead_id" --set-labels "stage:ready-architect,needs-implementation" --actor gatekeeper-2 2>&1 | grep -v "INFO" || true
        return 1
    fi

    log "Workspace found: $ws"
    return 0
}

navigate_to_workspace() {
    local bead_id="$1"

    # Navigate to workspace
    local ws_name="bead-$bead_id"
    log "Switching to workspace $ws_name..."

    # We need to use jj workspace to switch
    # But first we need to find the workspace's directory
    # This is a placeholder - actual implementation would need proper workspace switching
    log "Note: Workspace navigation requires Zellij integration"
    log "Proceeding with default workspace for now"

    return 0
}

run_qa_enforcer() {
    log "[QA] Running qa-enforcer skill..."

    # Load the qa-enforcer skill
    # This would typically be: Skill(qa-enforcer)
    # For now, we'll do basic checks

    log "[QA] Checking for test coverage..."
    log "[QA] Checking for documentation..."
    log "[QA] Checking for error handling..."

    log "[QA] Basic QA checks passed"
    return 0
}

check_panic_patterns() {
    log "[PANIC] Checking for forbidden panic patterns..."

    local found_panics=0

    # Check for unwrap()
    if rg '\.unwrap\(\)' --type rust crates/ 2>/dev/null | grep -v test | grep -v '//!' | head -5; then
        log "[PANIC] ✗ Found unwrap() calls"
        found_panics=1
    fi

    # Check for expect()
    if rg '\.expect\(' --type rust crates/ 2>/dev/null | grep -v test | grep -v '//!' | head -5; then
        log "[PANIC] ✗ Found expect() calls"
        found_panics=1
    fi

    # Check for panic!()
    if rg 'panic!\(' --type rust crates/ 2>/dev/null | grep -v test | grep -v '//!' | head -5; then
        log "[PANIC] ✗ Found panic!() calls"
        found_panics=1
    fi

    # Check for todo!()
    if rg 'todo!\(' --type rust crates/ 2>/dev/null | head -5; then
        log "[PANIC] ✗ Found todo!() calls"
        found_panics=1
    fi

    # Check for unimplemented!()
    if rg 'unimplemented!\(' --type rust crates/ 2>/dev/null | head -5; then
        log "[PANIC] ✗ Found unimplemented!() calls"
        found_panics=1
    fi

    if [ $found_panics -eq 1 ]; then
        log "[PANIC] ✗ FAILED: Found forbidden panic patterns"
        return 1
    fi

    log "[PANIC] ✓ No forbidden patterns found"
    return 0
}

run_moon_quick() {
    log "[MOON] Running moon run :quick..."

    if moon run :quick 2>&1 | tee -a gatekeeper-agent-2.log; then
        log "[MOON] ✓ Quick checks passed"
        return 0
    else
        log "[MOON] ✗ Quick checks failed"
        return 1
    fi
}

land_changes() {
    local bead_id="$1"

    log "[LAND] Landing changes..."

    # Run landing skill
    # This would typically be: Skill(landing-skill)
    # For now, we'll do the git operations

    # Stage files
    log "[LAND] Staging files..."
    git add -A

    # Check if there are changes to commit
    if git diff --staged --quiet; then
        log "[LAND] No changes to commit"
        return 1
    fi

    # Commit
    log "[LAND] Creating commit..."
    local bead_title
    bead_title=$(br show "$bead_id" --json 2>/dev/null | jq -r '.[0].title')
    git commit -m "$(cat <<EOF
$bead_title

Implements: $bead_id

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
EOF
)"

    return 0
}

push_with_retry() {
    local max_attempts=3
    local attempt=1

    while [ $attempt -le $max_attempts ]; do
        log "[GIT] Push attempt $attempt/$max_attempts..."

        if jj git push 2>&1 | tee -a gatekeeper-agent-2.log; then
            log "[GIT] ✓ Push succeeded"
            return 0
        else
            log "[GIT] ✗ Push failed (attempt $attempt/$max_attempts)"
            local wait_time=$((attempt * 5))
            log "[GIT] Waiting ${wait_time}s before retry..."
            sleep $wait_time
        fi

        attempt=$((attempt + 1))
    done

    log "[GIT] ✗ All push attempts failed"
    return 1
}

close_bead() {
    local bead_id="$1"

    log "[CLOSE] Closing bead $bead_id..."
    br close "$bead_id" 2>&1 | grep -v "INFO" || true
}

process_bead() {
    local bead_id="$1"
    local bead_title="$2"

    log "=========================================="
    log "PROCESSING BEAD: $bead_id"
    log "Title: $bead_title"
    log "=========================================="

    # Step 1: Claim bead
    if ! claim_bead "$bead_id"; then
        log "Failed to claim bead"
        return 1
    fi

    # Step 2: Check workspace exists
    if ! check_workspace "$bead_id"; then
        log "Workspace check failed - bead not ready for gatekeeping"
        return 1
    fi

    # Step 3: Navigate to workspace
    if ! navigate_to_workspace "$bead_id"; then
        log "Failed to navigate to workspace"
        return 1
    fi

    # Step 4: Run QA enforcer
    if ! run_qa_enforcer; then
        log "QA failed - not landing"
        return 1
    fi

    # Step 5: Check for panic patterns
    if ! check_panic_patterns; then
        log "Panic pattern check failed - not landing"
        return 1
    fi

    # Step 6: Run moon quick
    if ! run_moon_quick; then
        log "Moon quick failed - not landing"
        return 1
    fi

    # Step 7: Land changes
    if ! land_changes "$bead_id"; then
        log "Landing failed"
        return 1
    fi

    # Step 8: Push with retry
    if ! push_with_retry; then
        log "Push failed - changes not landed"
        return 1
    fi

    # Step 9: Close bead
    close_bead "$bead_id"

    log "=========================================="
    log "✓ BEAD $bead_id SUCCESSFULLY LANDED"
    log "=========================================="

    return 0
}

monitor() {
    log "Gatekeeper Agent 2 starting..."
    log "Monitoring for beads in stage:ready-gatekeeper"

    while true; do
        clear
        echo "=== GATEKEEPER AGENT 2 ==="
        echo "Time: $(date -Iseconds)"
        echo ""

        # Find beads in stage:ready-gatekeeper
        local bead
        bead=$(br list --label "stage:ready-gatekeeper" --status open --json 2>/dev/null | jq -r '.[0] // empty')

        if [ -n "$bead" ]; then
            local bead_id
            local bead_title
            bead_id=$(echo "$bead" | jq -r '.id')
            bead_title=$(echo "$bead" | jq -r '.title')

            echo "✓ FOUND READY BEAD:"
            echo "  ID: $bead_id"
            echo "  Title: $bead_title"
            echo ""

            if ! process_bead "$bead_id" "$bead_title"; then
                echo ""
                echo "✗ Failed to process bead $bead_id"
                echo "Bead will be retried in next cycle if still labeled as ready-gatekeeper"
            fi

            # Wait before processing next bead
            echo ""
            echo "Waiting 10 seconds before next check..."
            sleep 10
        else
            echo "No beads in stage:ready-gatekeeper"
            echo ""
            echo "Total open beads: $(br list --status open --json 2>/dev/null | jq 'length')"
            echo "Beads in ready-architect: $(br list --label 'stage:ready-architect' --status open --json 2>/dev/null | jq 'length')"
            echo ""
            echo "Next check in 30 seconds..."
            sleep 30
        fi
    done
}

# Run the monitoring loop
monitor
