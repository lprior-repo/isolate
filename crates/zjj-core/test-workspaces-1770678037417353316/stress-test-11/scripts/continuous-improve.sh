#!/usr/bin/env bash
set -euo pipefail

# Continuous quality loop for zjj.
# Defaults to 4 hours, configurable with DURATION_SECONDS.

DURATION_SECONDS="${DURATION_SECONDS:-14400}"
SLEEP_SECONDS="${SLEEP_SECONDS:-120}"
CI_EVERY_N_CYCLES="${CI_EVERY_N_CYCLES:-10}"
LOG_DIR="${LOG_DIR:-./tmp.continuous-improve}"
TIMESTAMP="$(date +%Y%m%d-%H%M%S)"
RUN_LOG="${LOG_DIR}/run-${TIMESTAMP}.log"

mkdir -p "${LOG_DIR}"

start_epoch="$(date +%s)"
cycle=0

log() {
	printf '[%s] %s\n' "$(date -Iseconds)" "$*" | tee -a "${RUN_LOG}"
}

run_step() {
	local name="$1"
	shift
	log "START ${name}: $*"
	if "$@" 2>&1 | tee -a "${RUN_LOG}"; then
		log "PASS ${name}"
		return 0
	fi
	log "FAIL ${name}"
	return 1
}

log "Continuous improvement loop started"
log "duration=${DURATION_SECONDS}s sleep=${SLEEP_SECONDS}s ci_every=${CI_EVERY_N_CYCLES}"

while true; do
	now_epoch="$(date +%s)"
	elapsed="$((now_epoch - start_epoch))"
	if [ "${elapsed}" -ge "${DURATION_SECONDS}" ]; then
		break
	fi

	cycle="$((cycle + 1))"
	log "---- cycle=${cycle} elapsed=${elapsed}s remaining=$((DURATION_SECONDS - elapsed))s ----"

	run_step "triage" bv --robot-triage || true
	run_step "ready" br ready || true
	run_step "quick" moon run :quick || true

	if [ "$((cycle % CI_EVERY_N_CYCLES))" -eq 0 ]; then
		run_step "ci" moon run :ci || true
	fi

	run_step "git-status" git status --short || true

	if [ -n "${AUTOFIX_CMD:-}" ]; then
		log "START autofix: ${AUTOFIX_CMD}"
		if bash -lc "${AUTOFIX_CMD}" 2>&1 | tee -a "${RUN_LOG}"; then
			log "PASS autofix"
		else
			log "FAIL autofix"
		fi
	fi

	sleep "${SLEEP_SECONDS}"
done

log "Continuous improvement loop finished after ${cycle} cycles"
log "Run log: ${RUN_LOG}"
