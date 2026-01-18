#!/usr/bin/env bash
set -euo pipefail

# =============================================================================
# ZJJ Parallel Workspace Setup
# =============================================================================
# Sets up independent workspaces for parallel task implementation
# Each workspace is a full clone with isolated work on a specific bead
# =============================================================================

MAIN_REPO="/home/lewis/src/zjj"
WORKSPACE_ROOT="/tmp/zjj-parallel-workspaces"
TRACKING_FILE="${WORKSPACE_ROOT}/.workspace-manifest.json"
LOG_DIR="${WORKSPACE_ROOT}/.logs"

# Create directory structure
mkdir -p "$WORKSPACE_ROOT" "$LOG_DIR"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1" | tee -a "${LOG_DIR}/setup.log"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1" | tee -a "${LOG_DIR}/setup.log"
}

log_error() {
    echo -e "${RED}[✗]${NC} $1" | tee -a "${LOG_DIR}/setup.log"
}

# =============================================================================
# Define all tasks to parallelize (Tier 1 + Tier 2)
# =============================================================================

declare -A TASKS=(
    # Error Handling Foundation
    ["task-01-legq"]="zjj-legq:Standardize error field to ErrorDetail:60"
    ["task-02-t283"]="zjj-t283:Implement error code semantic mapping:90"
    ["task-03-renm"]="zjj-renm:Standardize JSON error field types:75"

    # CLI Quick Wins
    ["task-04-a50v"]="zjj-a50v:Standardize filter flag naming:30"
    ["task-05-aa6o"]="zjj-aa6o:Fix JSON session_name inconsistency:30"
    ["task-06-acyn"]="zjj-acyn:Fix config command argument clarity:45"

    # JSON Output Pipeline
    ["task-07-wx57"]="zjj-wx57:CLI Standardization 26/26 integration tests:120"
    ["task-08-tcnc"]="zjj-tcnc:Init command wire JSON output support:60"
    ["task-09-4pws"]="zjj-4pws:Status command JSON output:45"
    ["task-10-3ais"]="zjj-3ais:Init command JSON output:60"

    # Features
    ["task-11-viue"]="zjj-viue:Config command subcommands:90"
    ["task-12-o14q"]="zjj-o14q:Add --silent flag to output commands:120"
    ["task-13-95gd"]="zjj-95gd:Comprehensive help text:90"

    # Infrastructure
    ["task-14-gm9a"]="zjj-gm9a:Create CUE schema for JSON outputs:90"
    ["task-15-acyn"]="zjj-acyn:Fix config command clarity:45"
)

# Dependencies: which tasks must complete before others
declare -A TASK_DEPS=(
    ["task-02-t283"]="task-01-legq"
    ["task-07-wx57"]="task-01-legq"
    ["task-08-tcnc"]="task-07-wx57"
    ["task-09-4pws"]="task-07-wx57"
    ["task-10-3ais"]="task-08-tcnc"
)

# =============================================================================
# Create individual workspace for each task
# =============================================================================

create_workspace() {
    local task_id="$1"
    local bead_info="$2"

    IFS=':' read -r bead_name description minutes <<< "$bead_info"

    local ws_dir="${WORKSPACE_ROOT}/${task_id}"

    log_info "Creating workspace: $task_id ($bead_name)"

    # Create workspace directory
    mkdir -p "$ws_dir"

    # Clone repo into workspace (using jj clone for better performance)
    cd "$ws_dir" || exit 1

    if command -v jj &> /dev/null; then
        log_info "  Using jj clone..."
        jj clone "$MAIN_REPO" . --shallow > "${LOG_DIR}/${task_id}.clone.log" 2>&1 || {
            log_error "jj clone failed, trying git clone"
            git clone --depth 1 "$MAIN_REPO" . >> "${LOG_DIR}/${task_id}.clone.log" 2>&1
        }
    else
        log_info "  Using git clone..."
        git clone --depth 1 "$MAIN_REPO" . > "${LOG_DIR}/${task_id}.clone.log" 2>&1
    fi

    # Create task branch
    cd "$ws_dir" || exit 1

    if command -v jj &> /dev/null; then
        jj branch create "task/${task_id}-${bead_name}"
    else
        git checkout -b "task/${task_id}-${bead_name}"
    fi

    # Create task manifest
    cat > "${ws_dir}/.task-manifest.json" << EOF
{
  "task_id": "${task_id}",
  "bead": "${bead_name}",
  "description": "${description}",
  "estimated_minutes": ${minutes},
  "status": "ready",
  "created_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "dependencies": [$(echo "${TASK_DEPS[$task_id]:-}" | sed 's/^/"/;s/$/"/;s/ /", "/g')],
  "commits": [],
  "completed_at": null
}
EOF

    # Create README for the task
    cat > "${ws_dir}/TASK_README.md" << EOF
# Task: $task_id

## Bead
- **ID**: $bead_name
- **Description**: $description
- **Estimated Time**: ${minutes} minutes

## Status
- **Directory**: $ws_dir
- **Branch**: task/${task_id}-${bead_name}
- **Created**: $(date)

## Next Steps
1. Review the architecture blueprint from \`.beads/\` or agent outputs
2. Make changes to implement the bead
3. Run: \`moon run :quick\` to verify formatting
4. Run: \`moon run :test\` to verify tests pass
5. Commit changes with: \`jj commit -m "..."\`
6. When complete, run: \`$WORKSPACE_ROOT/scripts/report-completion.sh $task_id\`

## Dependencies
$([ -z "${TASK_DEPS[$task_id]:-}" ] && echo "None" || echo "${TASK_DEPS[$task_id]}")

## How to Pull Latest from Main
\`\`\`bash
cd $ws_dir
jj pull origin main
\`\`\`

## How to Merge Changes to Main
\`\`\`bash
cd $ws_dir
jj branch main
jj merge task/${task_id}-${bead_name}
jj git push --allow-new-branches
\`\`\`
EOF

    log_success "Workspace created: $ws_dir"
}

# =============================================================================
# Create task coordination scripts
# =============================================================================

create_coordination_scripts() {
    # Script to report task completion
    cat > "${WORKSPACE_ROOT}/scripts/report-completion.sh" << 'REPORT_SCRIPT'
#!/usr/bin/env bash
set -euo pipefail

TASK_ID="$1"
WS_DIR="/tmp/zjj-parallel-workspaces/${TASK_ID}"
MANIFEST="${WS_DIR}/.task-manifest.json"

if [ ! -f "$MANIFEST" ]; then
    echo "Error: Task $TASK_ID not found"
    exit 1
fi

# Update manifest
jq --arg status "completed" \
   --arg timestamp "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
   '.status = $status | .completed_at = $timestamp' \
   "$MANIFEST" > "${MANIFEST}.tmp" && mv "${MANIFEST}.tmp" "$MANIFEST"

echo "✓ Task $TASK_ID marked as completed"
echo "✓ Manifest updated: $MANIFEST"

# Show completion status
jq '.' "$MANIFEST"

# Reminder to push
echo ""
echo "Next: Push changes to remote"
echo "  cd $WS_DIR && jj git push --allow-new-branches"
REPORT_SCRIPT

    chmod +x "${WORKSPACE_ROOT}/scripts/report-completion.sh"

    # Script to check status of all tasks
    cat > "${WORKSPACE_ROOT}/scripts/status-all.sh" << 'STATUS_SCRIPT'
#!/usr/bin/env bash

ROOT="/tmp/zjj-parallel-workspaces"
echo "=== ZJJ Parallel Workspace Status ==="
echo ""

total=0
completed=0
ready=0
in_progress=0

for ws_dir in "${ROOT}"/task-*; do
    if [ -d "$ws_dir" ]; then
        task_id=$(basename "$ws_dir")
        manifest="${ws_dir}/.task-manifest.json"

        if [ -f "$manifest" ]; then
            status=$(jq -r '.status' "$manifest")
            bead=$(jq -r '.bead' "$manifest")

            total=$((total + 1))
            case "$status" in
                completed)
                    echo "✓ $task_id ($bead): COMPLETED"
                    completed=$((completed + 1))
                    ;;
                in_progress)
                    echo "⏳ $task_id ($bead): IN PROGRESS"
                    in_progress=$((in_progress + 1))
                    ;;
                ready)
                    echo "○ $task_id ($bead): READY"
                    ready=$((ready + 1))
                    ;;
                *)
                    echo "? $task_id ($bead): $status"
                    ;;
            esac
        fi
    fi
done

echo ""
echo "Summary:"
echo "  Total: $total"
echo "  Completed: $completed"
echo "  In Progress: $in_progress"
echo "  Ready: $ready"
STATUS_SCRIPT

    chmod +x "${WORKSPACE_ROOT}/scripts/status-all.sh"

    # Script to merge all to main
    cat > "${WORKSPACE_ROOT}/scripts/merge-all-to-main.sh" << 'MERGE_SCRIPT'
#!/usr/bin/env bash
set -euo pipefail

MAIN_REPO="/home/lewis/src/zjj"
ROOT="/tmp/zjj-parallel-workspaces"

echo "=== Merging All Workspaces to Main ==="
echo ""

# First, pull latest main in all workspaces
echo "Step 1: Updating all workspaces with latest main..."
for ws_dir in "${ROOT}"/task-*; do
    if [ -d "$ws_dir" ]; then
        task_id=$(basename "$ws_dir")
        echo "  Updating $task_id..."
        cd "$ws_dir"
        jj pull origin main > /dev/null 2>&1 || echo "    (no changes on main)"
    fi
done

# Merge each task to main
echo ""
echo "Step 2: Merging tasks to main..."
for ws_dir in "${ROOT}"/task-*; do
    if [ -d "$ws_dir" ]; then
        task_id=$(basename "$ws_dir")
        manifest="${ws_dir}/.task-manifest.json"

        if [ -f "$manifest" ]; then
            status=$(jq -r '.status' "$manifest")
            bead=$(jq -r '.bead' "$manifest")

            if [ "$status" = "completed" ]; then
                echo "  Merging $task_id ($bead) to main..."
                cd "$ws_dir"

                # Switch to main, merge task branch
                jj branch main
                task_branch=$(jj branch list | grep "task/${task_id}" | head -1)
                jj merge "$task_branch" 2>&1 | head -3

                # Push
                jj git push --allow-new-branches > /dev/null 2>&1
                echo "    ✓ Merged and pushed"
            fi
        fi
    fi
done

echo ""
echo "Step 3: Final status check..."
cd "$MAIN_REPO"
echo "Main branch commits:"
jj log --oneline --limit 10 || git log --oneline --max-count 10

echo ""
echo "✓ Merge complete!"
MERGE_SCRIPT

    chmod +x "${WORKSPACE_ROOT}/scripts/merge-all-to-main.sh"

    # Script to work on a task
    cat > "${WORKSPACE_ROOT}/scripts/work-on-task.sh" << 'WORK_SCRIPT'
#!/usr/bin/env bash
set -euo pipefail

TASK_ID="$1"
WS_DIR="/tmp/zjj-parallel-workspaces/${TASK_ID}"

if [ ! -d "$WS_DIR" ]; then
    echo "Error: Task workspace $TASK_ID not found"
    exit 1
fi

echo "=== Starting Work on Task $TASK_ID ==="
echo ""

# Update manifest status
manifest="${WS_DIR}/.task-manifest.json"
jq '.status = "in_progress"' "$manifest" > "${manifest}.tmp" && mv "${manifest}.tmp" "$manifest"

# Show task info
echo "Task Info:"
jq '.' "$manifest" | head -20
echo ""

echo "Workspace: $WS_DIR"
echo ""
echo "Next steps:"
echo "1. cd $WS_DIR"
echo "2. Review TASK_README.md"
echo "3. Make changes to implement the bead"
echo "4. Run tests: moon run :test"
echo "5. Commit: jj commit -m \"...\""
echo "6. Report done: $WORKSPACE_ROOT/scripts/report-completion.sh $TASK_ID"
echo ""

# Jump to workspace
cd "$WS_DIR"
bash -i
WORK_SCRIPT

    chmod +x "${WORKSPACE_ROOT}/scripts/work-on-task.sh"
}

# =============================================================================
# Create master manifest
# =============================================================================

create_manifest() {
    log_info "Creating master manifest..."

    # Collect all task info
    local tasks_json="["
    local first=true

    for task_id in "${!TASKS[@]}"; do
        if [ "$first" = true ]; then
            first=false
        else
            tasks_json+=","
        fi

        bead_info="${TASKS[$task_id]}"
        IFS=':' read -r bead_name description minutes <<< "$bead_info"

        dep="${TASK_DEPS[$task_id]:-}"

        tasks_json+="{\"task_id\":\"$task_id\",\"bead\":\"$bead_name\",\"description\":\"$description\",\"minutes\":$minutes,\"dependency\":\"$dep\"}"
    done

    tasks_json+="]"

    # Create manifest
    cat > "$TRACKING_FILE" << EOF
{
  "created_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "main_repo": "$MAIN_REPO",
  "workspace_root": "$WORKSPACE_ROOT",
  "total_tasks": ${#TASKS[@]},
  "estimated_total_minutes": $(( $(echo "${TASKS[@]}" | grep -o ':' | wc -l) / 2 * 45 )),
  "tasks": $tasks_json
}
EOF

    log_success "Master manifest created: $TRACKING_FILE"
}

# =============================================================================
# Main execution
# =============================================================================

main() {
    log_info "Starting ZJJ Parallel Workspace Setup"
    log_info "Creating ${#TASKS[@]} workspaces in $WORKSPACE_ROOT"
    echo ""

    # Create scripts directory
    mkdir -p "${WORKSPACE_ROOT}/scripts"

    # Create coordination scripts
    log_info "Creating coordination scripts..."
    create_coordination_scripts
    log_success "Coordination scripts created"
    echo ""

    # Create master manifest first
    create_manifest
    echo ""

    # Create workspaces
    for task_id in "${!TASKS[@]}"; do
        create_workspace "$task_id" "${TASKS[$task_id]}"
    done

    echo ""
    log_success "All workspaces created!"
    echo ""

    # Print summary
    cat << SUMMARY
${GREEN}=== Setup Complete ===${NC}

Workspace Location: $WORKSPACE_ROOT

Quick Commands:
  Check status:     ${WORKSPACE_ROOT}/scripts/status-all.sh
  Work on task:     ${WORKSPACE_ROOT}/scripts/work-on-task.sh <task-id>
  Report complete:  ${WORKSPACE_ROOT}/scripts/report-completion.sh <task-id>
  Merge all to main: ${WORKSPACE_ROOT}/scripts/merge-all-to-main.sh

Example workflow:
  1. Check status:  ${WORKSPACE_ROOT}/scripts/status-all.sh
  2. Work on task:  ${WORKSPACE_ROOT}/scripts/work-on-task.sh task-04-a50v
  3. Make changes & commit
  4. Report done:   ${WORKSPACE_ROOT}/scripts/report-completion.sh task-04-a50v
  5. Move to next task
  6. When all done:  ${WORKSPACE_ROOT}/scripts/merge-all-to-main.sh

Total Workspaces: ${#TASKS[@]}
Workspace Root: $WORKSPACE_ROOT
Tracking File: $TRACKING_FILE
Logs: $LOG_DIR

Ready for parallel implementation! 🚀
SUMMARY
}

# Run main
main "$@"
