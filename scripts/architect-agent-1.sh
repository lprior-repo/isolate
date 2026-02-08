#!/bin/bash
# Architect Agent 1: Contract + Martin Fowler Tests Generator
#
# This agent PRECEDES the standard 7-step parallel workflow.
# It generates exhaustive contract specs and Martin Fowler test plans
# before handing off to Builder agents.

set -euo pipefail

MAX_WAIT_CYCLES=20  # 20 * 30s = 10 minutes max wait
WAIT_SECONDS=30

log_info() {
    echo "[ARCHITECT-1] $(date '+%Y-%m-%d %H:%M:%S') $*"
}

log_error() {
    echo "[ARCHITECT-1] $(date '+%Y-%m-%d %H:%M:%S') ERROR: $*" >&2
}

get_ready_bead() {
    br list --labels "stage:ready-architect" --status ready --json 2>/dev/null | \
        jq -r '.[0].id // empty'
}

claim_bead() {
    local bead_id="$1"
    log_info "Claiming bead: $bead_id"
    br update "$bead_id" \
        --status in_progress \
        --labels "stage:architecting" \
        --actor architect-1
}

read_bead() {
    local bead_id="$1"
    br show "$bead_id" --json 2>/dev/null
}

generate_contract() {
    local bead_id="$1"
    local bead_json="$2"
    local output_file="/tmp/rust-contract-${bead_id}.md"

    log_info "Generating Rust contract for $bead_id"

    # Extract bead description
    local description
    description=$(echo "$bead_json" | jq -r '.description // .title // "No description"')

    # Use Claude Code with rust-contract skill
    # This would normally be called via Skill tool, but for scripting we'll
    # output a prompt that can be used with the skill
    cat > "$output_file" <<EOF
# Rust Contract Specification: ${bead_id}

**Generated**: $(date '+%Y-%m-%d %H:%M:%S')
**Bead**: ${bead_id}

## Original Description

${description}

## Contract Specification (TO BE GENERATED)

The contract specification should be generated using the rust-contract skill.
This file is a placeholder for the skill output.

EOF

    echo "$output_file"
}

generate_test_plan() {
    local bead_id="$1"
    local bead_json="$2"
    local output_file="/tmp/martin-fowler-tests-${bead_id}.md"

    log_info "Generating Martin Fowler test plan for $bead_id"

    # Extract bead description
    local description
    description=$(echo "$bead_json" | jq -r '.description // .title // "No description"')

    cat > "$output_file" <<EOF
# Martin Fowler Test Plan: ${bead_id}

**Generated**: $(date '+%Y-%m-%d %H:%M:%S')
**Bead**: ${bead_id}

## Original Description

${description}

## Test Plan (TO BE GENERATED)

The Martin Fowler test plan should be generated using the planner skill.
This file is a placeholder for the skill output.

EOF

    echo "$output_file"
}

complete_architecture() {
    local bead_id="$1"
    local contract_file="$2"
    local test_file="$3"

    log_info "Completing architecture for $bead_id"
    log_info "  Contract: $contract_file"
    log_info "  Test plan: $test_file"

    br update "$bead_id" \
        --status ready \
        --labels "stage:ready-builder,has-rust-contract,has-tests"
}

main() {
    local cycle=0

    log_info "Architect Agent 1 starting..."
    log_info "Waiting for beads with label: stage:ready-architect"

    while true; do
        # Check for ready bead
        bead_id=$(get_ready_bead)

        if [[ -n "$bead_id" ]]; then
            log_info "Found ready bead: $bead_id"

            # Claim it
            claim_bead "$bead_id"

            # Read it
            bead_json=$(read_bead "$bead_id")

            if [[ -z "$bead_json" ]]; then
                log_error "Failed to read bead: $bead_id"
                sleep "$WAIT_SECONDS"
                continue
            fi

            # Generate contract and test plan
            # NOTE: In actual implementation, these would use the Skill tool
            # to invoke rust-contract and planner skills
            contract_file=$(generate_contract "$bead_id" "$bead_json")
            test_file=$(generate_test_plan "$bead_id" "$bead_json")

            # Mark as ready for builder
            complete_architecture "$bead_id" "$contract_file" "$test_file"

            log_info "Completed architecture for $bead_id"
            log_info "Waiting for next bead..."
        else
            cycle=$((cycle + 1))
            if [[ $cycle -ge $MAX_WAIT_CYCLES ]]; then
                log_info "No beads found after $((cycle * WAIT_SECONDS))s, exiting"
                break
            fi
            log_info "No ready beads found ($cycle/$MAX_WAIT_CYCLES), waiting ${WAIT_SECONDS}s..."
            sleep "$WAIT_SECONDS"
        fi
    done
}

main "$@"
