# Queue Operations Runbook

This runbook is for operators managing the `zjj queue` command in live or local automation workflows.

## Scope

- Add workspaces to the merge queue.
- Inspect queue state and ordering.
- Diagnose common queue issues.
- Recover safely from operator mistakes.

## Command Model (Important)

`zjj queue` uses flags, not subcommands.

Use this pattern:

```bash
zjj queue --list
zjj queue --add <workspace> --bead <id> --priority <n>
zjj queue --next
zjj queue --status <workspace>
zjj queue --remove <workspace>
zjj queue --stats
```

## Preflight Checks

Run these before intervention:

```bash
zjj queue --stats
zjj queue --list
```

If you are automating, prefer JSON:

```bash
zjj queue --stats --json
zjj queue --list --json
```

## Standard Operating Procedures

### 1) Add a workspace to queue

```bash
zjj queue --add <workspace> --bead <bead-id> --priority 5
```

Expected output:

- `Added workspace '<workspace>' to queue at position X/Y`

Verify:

```bash
zjj queue --status <workspace>
zjj queue --list
```

### 2) Inspect next work item

```bash
zjj queue --next
```

Expected output:

- Queue non-empty: shows workspace, id, status, priority, bead id.
- Queue empty: `Queue is empty - no pending entries`.

### 3) Remove an entry

```bash
zjj queue --remove <workspace>
```

Expected output:

- Success: `Removed workspace '<workspace>' from queue`
- Missing: `Workspace '<workspace>' not found in queue`

### 4) Monitor queue health

```bash
zjj queue --stats
```

Watch:

- `processing` that does not drop over time.
- `failed` increasing during agent activity.
- `pending` growth without corresponding completions.

## Troubleshooting Matrix

| Symptom | Likely cause | Confirm with | Remediation |
|---|---|---|---|
| `Queue is empty` | No items enqueued | `zjj queue --list` | Add workspace: `zjj queue --add <workspace> --bead <id>` |
| `Workspace '<name>' is not in the queue` | Typo or already removed | `zjj queue --list` | Re-add with exact workspace name |
| `Workspace '<name>' not found in queue` on remove | Entry already drained or never queued | `zjj queue --status <name>` | No-op, continue |
| `Workspace '<name>' is already in the queue` | Duplicate enqueue attempt | `zjj queue --status <name>` | Avoid duplicate `--add`; keep existing entry |
| Queue order looks wrong | Priority overrides FIFO | `zjj queue --list` | Use consistent priority policy and re-add entries if needed |
| Automation parser failed | Text output consumed instead of JSON | `zjj queue --list --json` | Switch automation to `--json` mode |

## Known Behavior Notes

- Help text documents `--priority` as `1-10`, but current runtime accepted `99` during manual validation.
- Treat priority as implementation-defined until strict validation is enforced.

## Manual Validation Evidence

Validated in this repository session:

- `zjj queue --list` reports empty queue and summary stats.
- `zjj queue --stats` reports total/pending/processing/completed/failed counts.
- `zjj queue --next` returns pending entry when present and explicit empty message when not.
- `zjj queue --add ...` inserts an entry and exposes queue position.
- Duplicate `--add` returns `Invalid configuration` with already-in-queue detail.
- `zjj queue --status <workspace>` returns detailed entry metadata when present.
- `zjj queue --remove <workspace>` removes entry cleanly.

## Operational Guardrails

- Prefer read-first operations (`--stats`, `--list`, `--status`) before mutation.
- For scripts and agents, always use `--json` and assert `success: true`.
- After any cleanup/remediation, verify with:

```bash
zjj queue --stats
zjj queue --list --json
```
