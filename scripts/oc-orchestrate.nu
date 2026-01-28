#!/usr/bin/env nu
# oc-orchestrate.nu — Top-Level CLI Orchestrator
# Tork's "coordinator": outer bead DAG + inner TDD15 phase DAGs

use oc-engine.nu *
use oc-tdd15.nu *
use oc-agent.nu *

const OC_PORT = 4096

# ── Main Entry Point ─────────────────────────────────────────────────────────

# Run TDD15 on Red Queen beads
export def "main run" [
  --beads: list<string> = []  # Specific beads to run (empty = all ready)
  --port: int = 4096          # OpenCode server port
  --dry-run                   # Show what would run without executing
] {
  print "== OC Workflow Engine =="
  print ""

  # 1. Initialize journal DB
  db-init
  print "[ok] Journal database initialized"

  # 2. Start opencode server
  if not $dry_run {
    try {
      let health = (oc-health --port $port)
      print $"[ok] OpenCode server already running on port ($port)"
    } catch {
      print $"[..] Starting opencode server on port ($port)..."
      oc-serve --port $port
      print "[ok] OpenCode server started"
    }
  }

  # 3. Load bead dependency graph
  let bead_list = if ($beads | is-not-empty) {
    $beads | each {|b| { id: $b, title: $b, deps: [] } }
  } else {
    load-bead-graph
  }

  if ($bead_list | is-empty) {
    print "[warn] No beads to process"
    return
  }

  print $"[ok] Loaded ($bead_list | length) beads"

  # 4. Compute dependency levels (topological sort)
  let levels = (compute-levels $bead_list)

  # 5. Create TDD15 jobs for each bead
  for level in $levels {
    print $"\n--- Level ($level.level): ($level.beads | length) beads ---"
    for bead in $level.beads {
      let job_def = (tdd15-job $bead.id --position $level.level)
      if $dry_run {
        print $"  [dry] Would create job: ($job_def.name) \(($job_def.tasks | length) tasks\)"
      } else {
        let job_id = (job-create $job_def)
        # Register outer-level job dependencies
        for dep_id in ($bead.deps? | default []) {
          let dep_job = $"tdd15-($dep_id)"
          sqlite3 ".oc-workflow/journal.db" $"INSERT OR IGNORE INTO job_deps \(job_id, depends_on\) VALUES \('($job_id)', '($dep_job)'\)"
        }
        print $"  [ok] Created job: ($job_id)"
      }
    }
  }

  if $dry_run {
    print "\n[dry-run] No jobs executed"
    return
  }

  # 6. Execute outer DAG level by level
  for level in $levels {
    print $"\n== Executing Level ($level.level) =="
    let job_ids = ($level.beads | each {|b| $"tdd15-($b.id)" })

    # Check outer deps are satisfied
    let ready_jobs = ($job_ids | where {|jid|
      let blocked = (sql $"SELECT depends_on FROM job_deps WHERE job_id = '($jid)'" )
      if ($blocked | is-empty) { true } else {
        let dep_statuses = ($blocked | each {|d|
          sql $"SELECT status FROM jobs WHERE id = '($d.depends_on)'"
        } | flatten)
        $dep_statuses | all {|s| $s.status == "COMPLETED" }
      }
    })

    # Execute ready jobs in parallel
    let results = ($ready_jobs | par-each {|jid|
      print $"  [>>] Executing ($jid)..."
      let result = (job-execute $jid)
      print $"  [($result.status | str downcase)] ($jid)"

      # Close bead on success
      if $result.status == "COMPLETED" {
        let bead_id = ($jid | str replace "tdd15-" "")
        try { ^bd close $bead_id } catch { }
      }
      $result
    })

    # Check for failures
    let failures = ($results | where status == "FAILED")
    if ($failures | is-not-empty) {
      print $"\n[!!] ($failures | length) jobs failed at level ($level.level)"
      print "     Run `nu scripts/oc-orchestrate.nu status` for details"
      print "     Run `nu scripts/oc-orchestrate.nu retry --job <id>` to retry"
    }
  }

  print "\n== Workflow Complete =="
  main status
}

# Resume from crash — re-execute from last checkpoint
export def "main resume" [
  --port: int = 4096
] {
  print "== Resuming Workflow =="
  db-init

  # Find incomplete jobs
  let incomplete = (sql "SELECT id, status FROM jobs WHERE status IN ('RUNNING', 'PENDING', 'FAILED') ORDER BY position")
  if ($incomplete | is-empty) {
    print "[ok] No incomplete jobs to resume"
    return
  }

  print $"[ok] Found ($incomplete | length) incomplete jobs"

  for job in $incomplete {
    print $"  [>>] Resuming ($job.id) \(was: ($job.status)\)..."
    let result = (job-resume $job.id)
    print $"  [($result.status | str downcase)] ($job.id)"
  }

  print "\n== Resume Complete =="
  main status
}

# Show status dashboard
export def "main status" [] {
  db-init
  let jobs = (job-list)
  if ($jobs | is-empty) {
    print "No jobs found"
    return
  }

  print "== Job Status =="
  print ""

  for job in $jobs {
    let tasks = (sql $"SELECT name, status, attempt, gate, error FROM tasks WHERE job_id = '($job.id)' ORDER BY rowid")
    let completed = ($tasks | where status == "COMPLETED" | length)
    let skipped = ($tasks | where status == "SKIPPED" | length)
    let failed = ($tasks | where status == "FAILED" | length)
    let total = ($tasks | length)
    let done = $completed + $skipped

    let indicator = match $job.status {
      "COMPLETED" => { "[ok]" }
      "FAILED" => { "[!!]" }
      "RUNNING" => { "[..]" }
      "CANCELLED" => { "[--]" }
      _ => { "[  ]" }
    }

    print $"($indicator) ($job.id) — ($job.status) \(($done)/($total) tasks, ($failed) failed\)"

    # Show task details for non-completed jobs
    if $job.status != "COMPLETED" {
      for task in $tasks {
        let t_icon = match $task.status {
          "COMPLETED" => { "  ✓" }
          "SKIPPED" => { "  ⊘" }
          "FAILED" => { "  ✗" }
          "RUNNING" => { "  ▶" }
          _ => { "  ·" }
        }
        let err = if ($task.error? | default "" | is-not-empty) { $" — ($task.error)" } else { "" }
        print $"($t_icon) ($task.name) [($task.status)]($err)"
      }
    }
  }
}

# Cancel a running job
export def "main cancel" [--job: string] {
  db-init
  job-cancel $job
  print $"[ok] Cancelled ($job)"
}

# Retry a failed job
export def "main retry" [--job: string] {
  db-init
  print $"[>>] Retrying ($job)..."
  let result = (job-retry $job)
  print $"[($result.status | str downcase)] ($job)"
}

# View event log
export def "main events" [--job: string, --limit: int = 50] {
  db-init
  let events = (event-log $job --limit $limit)
  if ($events | is-empty) {
    print "No events found"
    return
  }
  $events | table
}

# List all jobs
export def "main list" [] {
  db-init
  let jobs = (job-list)
  if ($jobs | is-empty) {
    print "No jobs found"
    return
  }
  $jobs | table
}

# ── Bead Graph Loading ───────────────────────────────────────────────────────

# Load bead dependency graph from beads CLI
def load-bead-graph []: nothing -> list<record> {
  # Get ready beads
  let ready_output = (try { ^bd ready --json | from json } catch { [] })
  let all_output = (try { ^bd list --status=open --json | from json } catch { [] })

  let beads = if ($all_output | is-not-empty) { $all_output } else { $ready_output }

  $beads | each {|b|
    let details = (try { ^bd show ($b.id) --json | from json } catch { $b })
    {
      id: $b.id
      title: ($b.title? | default "")
      type: ($b.type? | default "task")
      priority: ($b.priority? | default 2)
      deps: ($details.blocked_by? | default [] | each {|d| $d.id? | default $d })
    }
  }
}

# Compute topological levels from bead dependencies
def compute-levels [beads: list<record>]: nothing -> list<record> {
  let bead_ids = ($beads | get id)

  # Build in-degree map
  mut levels_out = []
  mut remaining = $beads
  mut level = 0

  loop {
    if ($remaining | is-empty) { break }

    let done_ids = ($levels_out | each {|l| $l.beads | get id } | flatten)

    let ready = ($remaining | where {|b|
      let deps = ($b.deps | where {|d| $d in $bead_ids })
      ($deps | all {|d| $d in $done_ids }) or ($deps | is-empty)
    })

    if ($ready | is-empty) {
      # Remaining beads have circular deps — force them into this level
      $levels_out = ($levels_out | append { level: $level, beads: $remaining })
      break
    }

    $levels_out = ($levels_out | append { level: $level, beads: $ready })
    let ready_ids = ($ready | get id)
    $remaining = ($remaining | where {|b| not ($b.id in $ready_ids) })
    $level = $level + 1
  }

  $levels_out
}

# ── SQL Helper (re-export for this module) ───────────────────────────────────

def sql [query: string] {
  sqlite3 -json ".oc-workflow/journal.db" $query | from json
}
