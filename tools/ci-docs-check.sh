#!/usr/bin/env bash
# CI Documentation Checker - zjj Command Validation
# Validates that documented zjj commands exist and have expected flags
# Returns: 0 if all documented commands exist, 1 otherwise

set -euo pipefail

cd /home/lewis/src/zjj

# Check if ZJJ_BIN is set, otherwise use default
if [[ -z "${ZJJ_BIN:-}" ]]; then
	ZJJ_BIN="/home/lewis/src/zjj/target/release/zjj"
fi

# Track failures
declare -a FAILED_COMMANDS=()
declare -a FAILED_FLAGS=()
TOTAL_DOCUMENTED=0
TOTAL_FOUND=0
TOTAL_ERRORS=0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_error() { echo -e "${RED}ERROR:${NC} $1" >&2; }
log_warn() { echo -e "${YELLOW}WARNING:${NC} $1"; }
log_success() { echo -e "${GREEN}SUCCESS:${NC} $1"; }

get_available_commands() {
	"$ZJJ_BIN" --help 2>&1 |
		grep -E '^\s+[a-z-]+' |
		awk '{print $1}' |
		sort -u || true
}

get_available_flags() {
	local command="$1"
	"$ZJJ_BIN" "$command" --help 2>&1 |
		grep -E '^\s+--?[a-z-]+' |
		sed -E 's/^\s+--?([a-z-]+).*/--\1/' |
		sort -u || true
}

check_json_flag() {
	local command="$1"
	local actual_flags
	actual_flags=$(get_available_flags "$command")
	echo "$actual_flags" | grep -qE '\-\-json|-j'
}

validate_command() {
	local command="$1"
	local expected_flags="$2"

	TOTAL_DOCUMENTED=$((TOTAL_DOCUMENTED + 1))

	local actual_commands
	actual_commands=$(get_available_commands)

	if echo "$actual_commands" | grep -q "^${command}$"; then
		TOTAL_FOUND=$((TOTAL_FOUND + 1))
		log_success "Command 'zjj $command' exists"
		if [[ "$expected_flags" == *"json"* ]]; then
			if check_json_flag "$command"; then
				log_success "  - Has --json flag"
			else
				log_error "  - Missing --json flag (expected)"
				FAILED_FLAGS+=("zjj $command --json")
				TOTAL_ERRORS=$((TOTAL_ERRORS + 1))
			fi
		fi
		return 0
	else
		log_error "Command 'zjj $command' not found in help"
		FAILED_COMMANDS+=("zjj $command")
		TOTAL_ERRORS=$((TOTAL_ERRORS + 1))
		return 1
	fi
}

get_documented_commands() {
	local temp_file="/tmp/zjj_cmds_$$"
	local actual_cmds
	actual_cmds=$(get_available_commands)

	grep -rhoE '\bzjj ([a-z][a-z-]*)' /home/lewis/src/zjj/docs 2>/dev/null |
		sed 's/zjj //' |
		sort -u >"$temp_file" || true

	if [[ -s "$temp_file" ]]; then
		grep -Fxf <(echo "$actual_cmds") "$temp_file" >"${temp_file}.filtered" || true
		mv "${temp_file}.filtered" "$temp_file"
	fi

	echo "Found $(wc -l <"$temp_file") documented commands" >&2
	cat "$temp_file"
}

validate_all_commands() {
	local documented
	documented=$(get_documented_commands)

	if [[ -z "$documented" ]]; then
		log_warn "No zjj commands found in documentation"
		return 0
	fi

	while IFS= read -r cmd; do
		if [[ -n "$cmd" ]]; then
			local expected=""
			case "$cmd" in
			context | status | list | done) expected="json" ;;
			*) expected="" ;;
			esac
			validate_command "$cmd" "$expected"
		fi
	done <<<"$documented"
}

generate_report() {
	local report_file="${1:-/tmp/ci-docs-check-report.json}"
	local missing_cmds=""
	local flag_issues=""

	if [[ ${#FAILED_COMMANDS[@]} -gt 0 ]]; then
		missing_cmds=$(printf '"%s",' "${FAILED_COMMANDS[@]}" | sed 's/,$//')
	fi
	if [[ ${#FAILED_FLAGS[@]} -gt 0 ]]; then
		flag_issues=$(printf '"%s",' "${FAILED_FLAGS[@]}" | sed 's/,$//')
	fi

	cat >"$report_file" <<EOF
{
  "timestamp": "$(date -Iseconds)",
  "binary": "$ZJJ_BIN",
  "summary": {
    "documented": $TOTAL_DOCUMENTED,
    "found": $TOTAL_FOUND,
    "missing": $((TOTAL_DOCUMENTED - TOTAL_FOUND)),
    "flag_issues": ${#FAILED_FLAGS[@]},
    "total_errors": $TOTAL_ERRORS
  },
  "missing_commands": [$missing_cmds],
  "flag_issues": [$flag_issues]
}
EOF
	echo "Report written to $report_file"
}

exit_code=0

main() {
	echo "=========================================="
	echo "  zjj Documentation Validation Tool"
	echo "=========================================="
	echo

	if [[ ! -x "$ZJJ_BIN" ]]; then
		log_error "zjj binary not found at $ZJJ_BIN"
		log_error "Please build with: moon run :build"
		exit 1
	fi
	log_success "zjj binary found at $ZJJ_BIN"

	validate_all_commands

	echo
	echo "=========================================="
	echo "  Validation Summary"
	echo "=========================================="
	echo "Documented commands: $TOTAL_DOCUMENTED"
	echo "Found commands:      $TOTAL_FOUND"
	echo "Missing commands:    $((TOTAL_DOCUMENTED - TOTAL_FOUND))"
	echo "Flag mismatches:     ${#FAILED_FLAGS[@]}"
	echo "Total errors:        $TOTAL_ERRORS"
	echo

	if [[ $TOTAL_ERRORS -gt 0 ]]; then
		log_error "Documentation validation FAILED"
		if [[ ${#FAILED_COMMANDS[@]} -gt 0 ]]; then
			echo "Missing commands:"
			for cmd in "${FAILED_COMMANDS[@]}"; do echo "  - $cmd"; done
		fi
		if [[ ${#FAILED_FLAGS[@]} -gt 0 ]]; then
			echo "Flag mismatches:"
			for flag in "${FAILED_FLAGS[@]}"; do echo "  - $flag"; done
		fi
		generate_report
		exit_code=1
	else
		log_success "All documented commands validated successfully"
		generate_report
		exit_code=0
	fi
}

main "$@"
exit $exit_code
