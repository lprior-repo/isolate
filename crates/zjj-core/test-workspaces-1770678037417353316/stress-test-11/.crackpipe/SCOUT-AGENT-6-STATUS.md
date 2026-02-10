# QA Builder 6 Status

**Started**: 2026-02-08 07:21:17
**Agent**: qa-builder-6
**Process ID**: Running (bash script in background)

## Workflow
Continuous infinite loop:
1. Poll for beads labeled `stage:ready-qa-builder`
2. If found: claim bead, switch to Zellij tab, run `moon run :ci`
3. On PASS: label `stage:ready-gatekeeper`, log transition
4. On FAIL: label `stage:needs-rework,needs-qa-fix`, log transition
5. Sleep 30s between polls

## Current State
- **Status**: Active monitoring
- **Waiting for**: Beads marked `stage:ready-qa-builder` by builders
- **Last poll**: 2026-02-08 07:21:47 - No ready beads found

## Quality Gates
Full Moon CI pipeline (all must pass):
- Format checks (`moon run :fmt-fix`)
- Lint checks (clippy, zero unwrap/expect/panic)
- Test suite (parallel nextest)
- Release build

## Script Location
`/home/lewis/src/zjj/.crackpipe/qa-builder-6.sh`

## Log Files
- **Main log**: `/home/lewis/src/zjj/qa-builder-6.log`
- **Transitions**: `/home/lewis/src/zjj/.crackpipe/BEADS.md`

## Dependencies
- bazel-remote cache (systemd user service)
- moon build system
- br (beads CLI)
- zellij (for tab switching)

## Integration Points
- **Input**: Builder agents mark beads `stage:ready-qa-builder`
- **Output**: Either `stage:ready-gatekeeper` (pass) or `stage:needs-rework` (fail)
- **Next stage**: Gatekeeper agent reviews `stage:ready-gatekeeper` beads

## Monitoring Commands
```bash
# Check if running
ps aux | grep '[q]a-builder-6.sh'

# View live logs
tail -f /home/lewis/src/zjj/qa-builder-6.log

# View transition history
tail -20 /home/lewis/src/zjj/.crackpipe/BEADS.md

# Find ready beads
br list --label stage:ready-qa-builder --status open
```

## Notes
- Script uses `--add-label` and `--remove-label` flags for br command
- Zellij tab switching is best-effort (fails silently if tab doesn't exist)
- All transitions are timestamped and logged to BEADS.md
- Process runs with nohup in background, survives shell disconnect
