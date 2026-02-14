- Potential merge conflicts accumulate

**Instead:** Push yourself. Verify `git status` shows "up to date". Only then is work complete.

### Failure Recovery

If `git push` fails:
1. Check network: `ping github.com`
2. Check auth: `git remote -v` && `ssh -T git@github.com`
3. Pull rebase: `git pull --rebase`
4. Resolve conflicts if any
5. Push again: `git push`
6. Repeat until success
7. Only then report completion

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `ZJJ_AGENT_ID` | Your agent ID (set by register) |
| `ZJJ_SESSION` | Current session name |
| `ZJJ_WORKSPACE` | Current workspace path |
| `ZJJ_BEAD_ID` | Associated bead ID |
| `ZJJ_ACTIVE` | "1" when in workspace |

---

## JSON Output Pattern

All commands support `--json` and return:
```json
{
  "$schema": "zjj://<command>-response/v1",
  "_schema_version": "1.0",
  "schema_type": "single",
  "success": true,
  ...
}
```

---

## Error Handling

Exit codes:
- 0: Success
- 1: Validation error (user input)
- 2: Not found
- 3: System error
- 4: External command error
- 5: Lock contention

Errors include suggestions:
```json
{
  "success": false,
  "error": {
    "code": "SESSION_NOT_FOUND",
    "message": "...",
    "suggestion": "Use 'zjj list' to see available sessions"
  }
}
```

---

## Introspection

```bash
zjj introspect              # All capabilities
zjj introspect <cmd>        # Command details
zjj introspect --env-vars   # Environment variables
