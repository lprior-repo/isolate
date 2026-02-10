#!/bin/bash
# Planner Agent 1 - Infinite Loop Workflow
# Processes beads from stage:ready-planner → stage:planning → stage:ready-architect

set -e

BEADS_FILE="/home/lewis/src/zjj/.crackpipe/BEADS.md"
AGENT="planner-1"

log_transition() {
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    local bead_id="$1"
    local transition="$2"
    echo "[${timestamp}] ${bead_id} ${transition} ${AGENT}" >> "$BEADS_FILE"
}

create_bead_entry() {
    local bead_id="$1"
    local bead_info="$2"

    # Extract key information from bead_info (handle array response)
    local title=$(echo "$bead_info" | jq -r '.[0].title // "Unknown"')
    local description=$(echo "$bead_info" | jq -r '.[0].description // "No description"')
    local labels=$(echo "$bead_info" | jq -r '.[0].labels // []' | jq -r 'join(", ")')

    # Create markdown entry in BEADS.md
    {
        echo ""
        echo "## ${bead_id} - ${title}"
        echo ""
        echo "**Status**: planning"
        echo "**Labels**: ${labels}"
        echo "**Created**: $(date '+%Y-%m-%d %H:%M:%S')"
        echo ""
        echo "### Description"
        echo "${description}"
        echo ""
        echo "### Requirements"
        echo "- [ ] Architect: Create contract and test plan"
        echo "- [ ] Builder: Implement feature"
        echo "- [ ] QA: Verify implementation"
        echo ""
        echo "### Notes"
        echo "Planned by ${AGENT} on $(date '+%Y-%m-%d %H:%M:%S')"
        echo ""
    } >> "$BEADS_FILE"
}

echo "[$(date '+%Y-%m-%d %H:%M:%S')] PLANNER-1 STARTED"
echo "[$(date '+%Y-%m-%d %H:%M:%S')] PLANNER-1: Monitoring for beads with label: stage:ready-planner"
echo "[$(date '+%Y-%m-%d %H:%M:%S')] PLANNER-1: Workflow: ready-planner → planning → ready-architect"

while true; do
    # Step 1: Get the first bead with stage:ready-planner
    BEAD_ID=$(br list --label "stage:ready-planner" --status open --json | jq -r '.[0].id // empty')

    if [ -z "$BEAD_ID" ]; then
        # No beads found, sleep and continue
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] PLANNER-1: No beads ready for planning, sleeping 30s..."
        sleep 30
        continue
    fi

    echo "[$(date '+%Y-%m-%d %H:%M:%S')] PLANNER-1: Found bead ${BEAD_ID}, starting processing..."

    # Step 3: Claim the bead
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] PLANNER-1: Claiming ${BEAD_ID}..."
    br update "$BEAD_ID" --status in_progress --set-labels "stage:planning,actor:${AGENT}"

    # Step 4: Read bead details
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] PLANNER-1: Reading bead details..."
    BEAD_INFO=$(br show "$BEAD_ID" --json)

    # Step 5: Create/append bead entry in BEADS.md
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] PLANNER-1: Creating markdown entry..."
    create_bead_entry "$BEAD_ID" "$BEAD_INFO"

    # Step 6: Update to ready-architect
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] PLANNER-1: Marking ${BEAD_ID} ready for architect..."
    br update "$BEAD_ID" --status open --set-labels "stage:ready-architect"

    # Step 7: Log transition
    log_transition "$BEAD_ID" "ready-planner → planning → ready-architect"

    echo "[$(date '+%Y-%m-%d %H:%M:%S')] PLANNER-1: Completed processing ${BEAD_ID}"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] PLANNER-1: Transition: ready-planner → planning → ready-architect"
    echo ""
done
