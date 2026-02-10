#!/usr/bin/env bash
# QA Builder 6 - Full Moon Gatekeeper
# Continuous loop: find ready-qa-builder beads, run moon ci, route results

set -euo pipefail

ACTOR="qa-builder-6"
LOG_FILE="/home/lewis/src/zjj/qa-builder-6.log"
BEADS_LOG="/home/lewis/src/zjj/.crackpipe/BEADS.md"

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" | tee -a "$LOG_FILE"
}

append_beads_log() {
    echo "$1" >> "$BEADS_LOG"
}

check_beads() {
    local bead_id
    bead_id=$(br list --label "stage:ready-qa-builder" --status open --json 2>/dev/null | jq -r '.[0].id // empty')

    if [[ -z "$bead_id" ]]; then
        log "No beads ready for QA Builder. Sleeping 30s..."
        sleep 30
        return 1
    fi

    log "Found bead: $bead_id"
    echo "$bead_id"
}

claim_bead() {
    local bead_id="$1"
    log "Claiming $bead_id as $ACTOR..."
    br update "$bead_id" --status in_progress --add-label "stage:qa-building" --remove-label "stage:ready-qa-builder" --actor "$ACTOR"
}

switch_to_bead_tab() {
    local bead_id="$1"
    log "Switching to tab bead-$bead_id..."
    zellij action go-to-tab-name "bead-$bead_id" 2>/dev/null || true
}

run_moon_ci() {
    log "Running moon run :ci..."
    if moon run :ci 2>&1 | tee -a "$LOG_FILE"; then
        log "Moon CI: PASS"
        return 0
    else
        log "Moon CI: FAIL"
        return 1
    fi
}

handle_pass() {
    local bead_id="$1"
    log "CI passed for $bead_id - marking ready-gatekeeper"
    br update "$bead_id" --status open --add-label "stage:ready-gatekeeper" --remove-label "stage:qa-building"

    local timestamp
    timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    append_beads_log "[$timestamp] $bead_id ready-qa-builder → qa-building → ready-gatekeeper $ACTOR"
    log "Transition logged: $bead_id → ready-gatekeeper"
}

handle_fail() {
    local bead_id="$1"
    log "CI failed for $bead_id - marking needs-rework,needs-qa-fix"
    br update "$bead_id" --status open --add-label "stage:needs-rework" --add-label "needs-qa-fix" --remove-label "stage:qa-building"

    local timestamp
    timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    append_beads_log "[$timestamp] $bead_id ready-qa-builder → qa-building → needs-rework,needs-qa-fix $ACTOR"
    log "Transition logged: $bead_id → needs-rework"
}

main_loop() {
    log "QA Builder 6 starting..."
    log "Checking bazel-remote cache status..."
    if ! systemctl --user is-active bazel-remote >/dev/null; then
        log "WARNING: bazel-remote not active, starting..."
        systemctl --user start bazel-remote
        sleep 2
    fi

    log "Entering main loop..."
    while true; do
        if ! bead_id=$(check_beads); then
            continue
        fi

        if ! claim_bead "$bead_id"; then
            log "Failed to claim $bead_id, skipping..."
            sleep 10
            continue
        fi

        switch_to_bead_tab "$bead_id"

        if run_moon_ci; then
            handle_pass "$bead_id"
        else
            handle_fail "$bead_id"
        fi

        log "Iteration complete, sleeping 5s before next check..."
        sleep 5
    done
}

# Main execution
log "=== QA Builder 6 Started ==="
main_loop
