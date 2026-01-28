#!/usr/bin/env nu
# oc-agent.nu — OpenCode HTTP API Client
# Wraps opencode server REST API for session management and prompting

const DEFAULT_PORT = 4096
const DEFAULT_HOST = "http://localhost"

def base-url [--port: int = 4096]: nothing -> string {
  $"($DEFAULT_HOST):($port)"
}

# ── Server Lifecycle ─────────────────────────────────────────────────────────

# Start opencode server on given port (background process)
export def oc-serve [--port: int = 4096]: nothing -> record {
  let pid = (^opencode serve --port $port &)
  # Wait for server to be ready
  mut ready = false
  for _ in 0..30 {
    try {
      let health = (oc-health --port $port)
      if ($health.status? | default "" ) == "ok" {
        $ready = true
        break
      }
    } catch { }
    sleep 1sec
  }
  if not $ready {
    error make { msg: $"opencode server failed to start on port ($port)" }
  }
  { port: $port, status: "running" }
}

# Health check
export def oc-health [--port: int = 4096]: nothing -> record {
  let url = $"(base-url --port $port)/global/health"
  http get $url
}

# ── Session Management ───────────────────────────────────────────────────────

# Create a new opencode session
export def oc-session-create [title: string, --port: int = 4096]: nothing -> record {
  let url = $"(base-url --port $port)/session"
  let body = { title: $title }
  http post $url $body --content-type application/json
}

# Get session status
export def oc-session-status [session_id: string, --port: int = 4096]: nothing -> record {
  let url = $"(base-url --port $port)/session/($session_id)"
  http get $url
}

# ── Prompting ────────────────────────────────────────────────────────────────

# Send a prompt to a session (async)
export def oc-prompt [session_id: string, prompt: string, --port: int = 4096]: nothing -> record {
  let url = $"(base-url --port $port)/session/($session_id)/prompt_async"
  let body = {
    parts: [
      { type: "text", text: $prompt }
    ]
  }
  http post $url $body --content-type application/json
}

# Get messages from a session
export def oc-messages [session_id: string, --port: int = 4096]: nothing -> list {
  let url = $"(base-url --port $port)/session/($session_id)/message"
  http get $url
}

# Abort a running session
export def oc-abort [session_id: string, --port: int = 4096] {
  let url = $"(base-url --port $port)/session/($session_id)/abort"
  http post $url {} --content-type application/json
}

# ── Wait for Completion ──────────────────────────────────────────────────────

# Poll session until idle, with exponential backoff
export def oc-wait-idle [session_id: string, timeout_sec: int = 600, --port: int = 4096] {
  let start = (date now)
  mut delay = 2sec
  let max_delay = 30sec

  loop {
    let elapsed = ((date now) - $start | into int) / 1_000_000_000
    if $elapsed > $timeout_sec {
      error make { msg: $"Timeout waiting for session ($session_id) after ($timeout_sec)s" }
    }

    let status = (try {
      oc-session-status $session_id --port $port
    } catch {
      { status: "unknown" }
    })

    # Check if session is idle (no longer processing)
    let session_status = ($status.status? | default "unknown")
    if $session_status in ["idle", "completed", "done"] {
      # Get final messages
      let messages = (try { oc-messages $session_id --port $port } catch { [] })
      let last_assistant = ($messages | where role? == "assistant" | last)
      let content = if ($last_assistant | is-not-empty) {
        $last_assistant.content? | default ""
      } else {
        ""
      }
      return { status: $session_status, content: $content, messages: $messages }
    }

    if $session_status in ["failed", "error"] {
      error make { msg: $"Session ($session_id) failed: ($status.error? | default 'unknown')" }
    }

    sleep $delay
    $delay = [($delay * 2) $max_delay] | math min
  }
}
