# Configuration

ZJJ configuration file location: `.zjj/config.toml`

## Example Config

```toml
[core]
auto_sync = true
default_priority = 5

[zellij]
enabled = true
layout = "default"

[queue]
stale_timeout_seconds = 3600
max_retries = 3

[recovery]
policy = "warn"
log_recovered = true
```

## Core Settings

| Option | Default | Description |
|--------|---------|-------------|
| `auto_sync` | `true` | Auto-sync before landing |
| `default_priority` | `5` | Default queue priority |

## Zellij Settings

| Option | Default | Description |
|--------|---------|-------------|
| `enabled` | `true` | Enable Zellij integration |
| `layout` | `"default"` | Zellij layout name |

## Queue Settings

| Option | Default | Description |
|--------|---------|-------------|
| `stale_timeout_seconds` | `3600` | Mark claims stale after |
| `max_retries` | `3` | Max retry attempts |

## Recovery Settings

| Option | Default | Description |
|--------|---------|-------------|
| `policy` | `"warn"` | `silent`, `warn`, or `fail-fast` |
| `log_recovered` | `true` | Log recovery actions |

## Environment Variables

Override config with env vars:

| Variable | Description |
|----------|-------------|
| `ZJJ_AUTO_SYNC` | Override `core.auto_sync` |
| `ZJJ_ZELLIJ_ENABLED` | Override `zellij.enabled` |
| `ZJJ_QUEUE_TIMEOUT` | Override `queue.stale_timeout_seconds` |

Example:
```bash
export ZJJ_AUTO_SYNC=false
zjj done
```
