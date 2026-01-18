#!/usr/bin/env bash
set -euo pipefail

# =============================================================================
# ZJJ Parallel Workflow Management
# =============================================================================
# Manages the parallel implementation workflow:
# - Start workspaces
# - Monitor progress
# - Coordinate merges to main
# - Handle dependencies
# =============================================================================

WORKSPACE_ROOT="/tmp/zjj-parallel-workspaces"
MAIN_REPO="/home/lewis/src/zjj"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# =============================================================================
# Functions
# =============================================================================

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[✓]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[!]${NC} $1"; }
log_error() { echo -e "${RED}[✗]${NC} $1"; }

print_dashboard() {
    clear
    cat << 'DASHBOARD'
╔══════════════════════════════════════════════════════════════════════════════╗
║                  ZJJ PARALLEL IMPLEMENTATION DASHBOARD                       ║
╚══════════════════════════════════════════════════════════════════════════════╝
DASHBOARD

    echo ""
    echo "Task Status:"
    echo "─────────────────────────────────────────────────────────────────────────"

    local total=0
    local completed=0
    local in_progress=0
    local ready=0
    local failed=0

    for ws_dir in "${WORKSPACE_ROOT}"/task-*; do
        if [ -d "$ws_dir" ]; then
            task_id=$(basename "$ws_dir")
            manifest="${ws_dir}/.task-manifest.json"

            if [ -f "$manifest" ]; then
                status=$(jq -r '.status' "$manifest" 2>/dev/null || echo "unknown")
                bead=$(jq -r '.bead' "$manifest" 2>/dev/null || echo "?")
                minutes=$(jq -r '.estimated_minutes' "$manifest" 2>/dev/null || echo "?")

                total=$((total + 1))

                case "$status" in
                    completed)
                        echo "  ✓ $task_id ($bead) - COMPLETED [$minutes min]"
                        completed=$((completed + 1))
                        ;;
                    in_progress)
                        echo "  ⏳ $task_id ($bead) - IN PROGRESS [$minutes min]"
                        in_progress=$((in_progress + 1))
                        ;;
                    ready)
                        echo "  ○ $task_id ($bead) - READY [$minutes min]"
                        ready=$((ready + 1))
                        ;;
                    failed)
                        echo "  ✗ $task_id ($bead) - FAILED [$minutes min]"
                        failed=$((failed + 1))
                        ;;
                    *)
                        echo "  ? $task_id ($bead) - $status [$minutes min]"
                        ;;
                esac
            fi
        fi
    done

    echo ""
    echo "─────────────────────────────────────────────────────────────────────────"
    echo "Summary:"
    echo "  Total Tasks:    $total"
    echo "  Completed:      $completed ($((completed * 100 / total))%)"
    echo "  In Progress:    $in_progress"
    echo "  Ready:          $ready"
    echo "  Failed:         $failed"
    echo ""

    # Estimated time
    local total_minutes=0
    for ws_dir in "${WORKSPACE_ROOT}"/task-*; do
        manifest="${ws_dir}/.task-manifest.json"
        if [ -f "$manifest" ]; then
            minutes=$(jq -r '.estimated_minutes' "$manifest" 2>/dev/null || echo "0")
            total_minutes=$((total_minutes + minutes))
        fi
    done

    echo "Estimated Total Time: $total_minutes minutes (~$((total_minutes / 60)) hours)"
    echo ""
}

start_task() {
    local task_id="$1"
    local ws_dir="${WORKSPACE_ROOT}/${task_id}"
    local manifest="${ws_dir}/.task-manifest.json"

    if [ ! -f "$manifest" ]; then
        log_error "Task $task_id not found"
        return 1
    fi

    log_info "Starting work on $task_id..."

    # Update status
    jq '.status = "in_progress"' "$manifest" > "${manifest}.tmp"
    mv "${manifest}.tmp" "$manifest"

    # Show task info
    echo ""
    jq '.' "$manifest"
    echo ""

    log_success "Task started: $task_id"
    log_info "Workspace: $ws_dir"
    log_info "To continue: cd $ws_dir && bash"
}

complete_task() {
    local task_id="$1"
    local ws_dir="${WORKSPACE_ROOT}/${task_id}"
    local manifest="${ws_dir}/.task-manifest.json"

    if [ ! -f "$manifest" ]; then
        log_error "Task $task_id not found"
        return 1
    fi

    log_info "Marking $task_id as completed..."

    # Update status
    jq --arg ts "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
       '.status = "completed" | .completed_at = $ts' \
       "$manifest" > "${manifest}.tmp"
    mv "${manifest}.tmp" "$manifest"

    log_success "Task completed: $task_id"
    echo ""
    jq '.' "$manifest"
}

check_dependencies() {
    local task_id="$1"
    local manifest="${WORKSPACE_ROOT}/${task_id}/.task-manifest.json"

    if [ ! -f "$manifest" ]; then
        return 0
    fi

    local dep=$(jq -r '.dependencies[0]' "$manifest" 2>/dev/null || echo "")

    if [ -z "$dep" ] || [ "$dep" = "null" ]; then
        return 0
    fi

    # Extract task id from dependency
    local dep_task="${dep%%-*}"
    local dep_manifest="${WORKSPACE_ROOT}/${dep}/.task-manifest.json"

    if [ ! -f "$dep_manifest" ]; then
        log_warn "Dependency not found: $dep"
        return 1
    fi

    local dep_status=$(jq -r '.status' "$dep_manifest")

    if [ "$dep_status" != "completed" ]; then
        log_warn "Dependency not completed: $dep (status: $dep_status)"
        return 1
    fi

    return 0
}

merge_task_to_main() {
    local task_id="$1"
    local ws_dir="${WORKSPACE_ROOT}/${task_id}"

    if [ ! -d "$ws_dir" ]; then
        log_error "Workspace not found: $ws_dir"
        return 1
    fi

    log_info "Merging $task_id to main..."
    cd "$ws_dir"

    # Get the task branch
    local task_branch=$(jj branch list | grep "task/${task_id}" | head -1 || echo "")

    if [ -z "$task_branch" ]; then
        log_error "Task branch not found for $task_id"
        return 1
    fi

    # Switch to main
    jj branch main > /dev/null 2>&1 || {
        log_error "Cannot switch to main branch"
        return 1
    }

    # Merge task branch
    log_info "Merging branch: $task_branch"
    jj merge "$task_branch" > /dev/null 2>&1 || {
        log_error "Merge failed - conflicts may exist"
        log_info "Workspace: $ws_dir"
        log_info "Please resolve conflicts manually"
        return 1
    }

    # Push
    jj git push --allow-new-branches > /dev/null 2>&1 || {
        log_error "Push failed"
        return 1
    }

    log_success "Merged to main: $task_id"
    return 0
}

merge_all_to_main() {
    log_info "Merging all completed tasks to main..."
    echo ""

    local merged=0
    local failed=0

    for ws_dir in "${WORKSPACE_ROOT}"/task-*; do
        if [ -d "$ws_dir" ]; then
            task_id=$(basename "$ws_dir")
            manifest="${ws_dir}/.task-manifest.json"

            if [ -f "$manifest" ]; then
                status=$(jq -r '.status' "$manifest" 2>/dev/null || echo "unknown")

                if [ "$status" = "completed" ]; then
                    if check_dependencies "$task_id"; then
                        if merge_task_to_main "$task_id"; then
                            merged=$((merged + 1))
                        else
                            failed=$((failed + 1))
                        fi
                    else
                        log_warn "Skipping $task_id - dependencies not met"
                    fi
                fi
            fi
        fi
    done

    echo ""
    echo "─────────────────────────────────────────────────────────────────────────"
    log_success "Merge complete: $merged successful, $failed failed"

    if [ $failed -gt 0 ]; then
        log_warn "Please resolve $failed merge conflicts and retry"
        return 1
    fi

    return 0
}

update_all_from_main() {
    log_info "Updating all workspaces with latest main..."
    echo ""

    for ws_dir in "${WORKSPACE_ROOT}"/task-*; do
        if [ -d "$ws_dir" ]; then
            task_id=$(basename "$ws_dir")
            cd "$ws_dir"

            log_info "Pulling in $task_id..."
            jj pull origin main > /dev/null 2>&1 || log_warn "Pull failed in $task_id"
        fi
    done

    log_success "All workspaces updated"
}

generate_report() {
    local report_file="${WORKSPACE_ROOT}/.implementation-report.txt"

    cat > "$report_file" << 'REPORT'
ZJJ PARALLEL IMPLEMENTATION REPORT
Generated: $(date)

TASKS COMPLETED
===============
REPORT

    for ws_dir in "${WORKSPACE_ROOT}"/task-*; do
        if [ -d "$ws_dir" ]; then
            task_id=$(basename "$ws_dir")
            manifest="${ws_dir}/.task-manifest.json"

            if [ -f "$manifest" ]; then
                status=$(jq -r '.status' "$manifest" 2>/dev/null || echo "unknown")

                if [ "$status" = "completed" ]; then
                    bead=$(jq -r '.bead' "$manifest")
                    echo "✓ $task_id: $bead" >> "$report_file"
                fi
            fi
        fi
    done

    echo "" >> "$report_file"
    echo "Report saved: $report_file"
    cat "$report_file"
}

# =============================================================================
# CLI
# =============================================================================

show_usage() {
    cat << 'USAGE'
Usage: parallel-workflow.sh <command> [args]

Commands:
  dashboard              Show live dashboard of task status
  start <task-id>        Start work on a specific task
  complete <task-id>     Mark a task as completed
  merge-all              Merge all completed tasks to main
  update-all             Pull latest main to all workspaces
  report                 Generate implementation report
  list                   List all tasks

Examples:
  ./parallel-workflow.sh dashboard
  ./parallel-workflow.sh start task-04-a50v
  ./parallel-workflow.sh complete task-04-a50v
  ./parallel-workflow.sh merge-all
  ./parallel-workflow.sh report
USAGE
}

case "${1:-}" in
    dashboard)
        print_dashboard
        ;;
    start)
        start_task "${2:-}" || exit 1
        ;;
    complete)
        complete_task "${2:-}" || exit 1
        ;;
    merge-all)
        merge_all_to_main || exit 1
        ;;
    update-all)
        update_all_from_main || exit 1
        ;;
    report)
        generate_report
        ;;
    list)
        print_dashboard
        ;;
    *)
        show_usage
        exit 1
        ;;
esac
