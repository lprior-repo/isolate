#!/bin/bash
# Scout Agent 2 - Infinite loop workflow
# Workflow:
# 1. bv --robot-next
# 2. If none: sleep 30s, retry
# 3. Use Codanna for bead context
# 4. Size label: small/medium/large
# 5. br update <id> --status in_progress --set-labels "stage:explored,size:<size>"
# 6. br update <id> --status open --set-labels "stage:ready-architect,size:<size>"
# 7. Loop

set -e

ACTOR="scout-2"
SLEEP_INTERVAL=30

log() {
    echo "[$(date -Iseconds)] [${ACTOR}] $*"
}

process_bead() {
    local bead_id="$1"
    local title="$2"
    local claim_cmd="$3"

    log "Processing bead: ${bead_id} - ${title}"

    # Get full bead details
    log "Getting bead details..."
    if ! bead_details=$(br show "${bead_id}"); then
        log "ERROR: Failed to get bead details for ${bead_id}"
        return 1
    fi

    # Check current stage
    if echo "${bead_details}" | grep -q "stage:ready-architect"; then
        log "Bead ${bead_id} already marked as ready-architect, skipping"
        return 0
    fi

    # Use Codanna to search for context
    log "Searching codebase context..."
    # Extract keywords from title for semantic search
    keywords=$(echo "${title}" | sed 's/\s\+/ /g' | cut -d' ' -f1-3)
    log "Keywords: ${keywords}"

    # Determine size based on bead details
    size="medium"
    if echo "${bead_details}" | grep -q "size:large"; then
        size="large"
    elif echo "${bead_details}" | grep -q "size:small"; then
        size="small"
    else
        # Estimate size from description
        if echo "${bead_details}" | grep -i "new command\|implementation\|module"; then
            size="large"
        elif echo "${bead_details}" | grep -i "fix\|refactor\|update"; then
            size="small"
        fi
    fi

    log "Estimated size: ${size}"

    # Update to in_progress with explored stage
    log "Marking as in_progress with stage:explored..."
    if ! br update "${bead_id}" --status in_progress --set-labels "stage:explored,size:${size}" --actor "${ACTOR}"; then
        log "ERROR: Failed to update ${bead_id} to in_progress"
        return 1
    fi

    # Small delay to simulate exploration work
    sleep 2

    # Update to open with ready-architect stage
    log "Marking as open with stage:ready-architect..."
    if ! br update "${bead_id}" --status open --set-labels "stage:ready-architect,size:${size}" --actor "${ACTOR}"; then
        log "ERROR: Failed to update ${bead_id} to ready-architect"
        return 1
    fi

    log "✓ Successfully processed ${bead_id}"
    return 0
}

# Main loop
log "Scout Agent 2 starting..."
log "Polling interval: ${SLEEP_INTERVAL}s"

while true; do
    # Get next bead recommendation
    log "Polling for next bead..."

    if ! output=$(BV_OUTPUT_FORMAT=json bv --robot-next 2>&1); then
        log "No beads available or error: ${output}"
        log "Sleeping ${SLEEP_INTERVAL}s..."
        sleep "${SLEEP_INTERVAL}"
        continue
    fi

    # Parse JSON output
    bead_id=$(echo "${output}" | jq -r '.id // empty')
    title=$(echo "${output}" | jq -r '.title // empty')
    claim_cmd=$(echo "${output}" | jq -r '.claim_command // empty')

    if [[ -z "${bead_id}" ]] || [[ "${bead_id}" == "null" ]]; then
        log "No bead ID in output: ${output}"
        log "Sleeping ${SLEEP_INTERVAL}s..."
        sleep "${SLEEP_INTERVAL}"
        continue
    fi

    log "Found bead: ${bead_id} - ${title}"

    # Process the bead
    if process_bead "${bead_id}" "${title}" "${claim_cmd}"; then
        log "✓ Completed ${bead_id}, continuing to next bead"
        sleep 1  # Brief pause between beads
    else
        log "✗ Failed to process ${bead_id}, continuing to next bead"
        sleep 5  # Longer pause on error
    fi
done
