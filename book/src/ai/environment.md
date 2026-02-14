```

---

## Agent Lifecycle

```bash
# Register (optional but recommended)
zjj agent register

# Send heartbeat while working
zjj agent heartbeat --command "implementing"

# Check your status
zjj agent status

# Unregister when done
zjj agent unregister
```

---

## Common Patterns

### Start Fresh
```bash
zjj whereami                        # Should return "main"
zjj work feature-auth --idempotent
```
