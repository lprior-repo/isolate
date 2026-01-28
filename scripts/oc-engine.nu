#!/usr/bin/env nu
# oc-engine.nu — Tork-Inspired DAG Workflow Engine
# Pure nushell + SQLite journal for durable execution with replay + regression

const DB_DIR = ".oc-workflow"
const DB_PATH = $"($DB_DIR)/journal.db"

# ── Database Initialization ──────────────────────────────────────────────────

export def db-init [] {
  mkdir $DB_DIR
  sqlite3 $DB_PATH "
    CREATE TABLE IF NOT EXISTS jobs (
      id TEXT PRIMARY KEY,
      name TEXT NOT NULL,
      bead_id TEXT,
      inputs TEXT,
      defaults TEXT,
      status TEXT NOT NULL DEFAULT 'PENDING',
      position INTEGER DEFAULT 0,
      created_at TEXT NOT NULL DEFAULT (datetime('now')),
      started_at TEXT,
      completed_at TEXT,
      error TEXT,
      result TEXT
    );

    CREATE TABLE IF NOT EXISTS tasks (
      id TEXT PRIMARY KEY,
      job_id TEXT NOT NULL,
      name TEXT NOT NULL,
      var TEXT,
      status TEXT NOT NULL DEFAULT 'PENDING',
      run_cmd TEXT,
      agent_type TEXT,
      agent_model TEXT,
      gate TEXT,
      condition TEXT,
      on_fail_regress TEXT,
      priority INTEGER DEFAULT 0,
      timeout_sec INTEGER,
      input TEXT,
      output TEXT,
      error TEXT,
      attempt INTEGER DEFAULT 0,
      max_attempts INTEGER DEFAULT 3,
      retry_delay_sec INTEGER DEFAULT 1,
      retry_scaling INTEGER DEFAULT 2,
      started_at TEXT,
      completed_at TEXT,
      duration_ms INTEGER,
      FOREIGN KEY (job_id) REFERENCES jobs(id),
      UNIQUE (job_id, name)
    );

    CREATE TABLE IF NOT EXISTS task_deps (
      job_id TEXT NOT NULL,
      task_name TEXT NOT NULL,
      depends_on TEXT NOT NULL,
      PRIMARY KEY (job_id, task_name, depends_on)
    );

    CREATE TABLE IF NOT EXISTS job_deps (
      job_id TEXT NOT NULL,
      depends_on TEXT NOT NULL,
      PRIMARY KEY (job_id, depends_on)
    );

    CREATE TABLE IF NOT EXISTS events (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      job_id TEXT NOT NULL,
      task_name TEXT,
      event_type TEXT NOT NULL,
      old_state TEXT,
      new_state TEXT,
      payload TEXT,
      created_at TEXT NOT NULL DEFAULT (datetime('now'))
    );

    CREATE TABLE IF NOT EXISTS webhooks (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      job_id TEXT NOT NULL,
      url TEXT NOT NULL,
      event TEXT NOT NULL,
      headers TEXT,
      condition TEXT
    );
  "
}

# ── Helpers ──────────────────────────────────────────────────────────────────

def sql [query: string] {
  sqlite3 -json $DB_PATH $query | from json
}

def sql-exec [query: string] {
  sqlite3 $DB_PATH $query
}

def sql-escape [val: string]: nothing -> string {
  $val | str replace --all "'" "''"
}

def emit-event [job_id: string, task_name: string, event_type: string, old_state: string, new_state: string, payload: string] {
  let tn = if ($task_name | is-empty) { "NULL" } else { $"'($task_name)'" }
  let pl = if ($payload | is-empty) { "NULL" } else { $"'(sql-escape $payload)'" }
  sql-exec $"INSERT INTO events \(job_id, task_name, event_type, old_state, new_state, payload\) VALUES \('($job_id)', ($tn), '($event_type)', '($old_state)', '($new_state)', ($pl)\)"
}

# ── Job Operations ───────────────────────────────────────────────────────────

# Create a job from a Tork-style record definition
export def job-create [job_def: record] {
  let job_id = $job_def.name
  let bead_id = ($job_def.inputs?.bead_id? | default "")
  let inputs = ($job_def.inputs? | default {} | to json -r)
  let defaults = ($job_def.defaults? | default {} | to json -r)
  let position = ($job_def.position? | default 0)

  let inputs_esc = (sql-escape $inputs)
  let defaults_esc = (sql-escape $defaults)
  sql-exec $"INSERT OR REPLACE INTO jobs \(id, name, bead_id, inputs, defaults, status, position\) VALUES \('($job_id)', '($job_def.name)', '($bead_id)', '($inputs_esc)', '($defaults_esc)', 'PENDING', ($position)\)"

  # Insert tasks
  for task in ($job_def.tasks? | default []) {
    let task_id = $"($job_id):($task.name)"
    let var = ($task.var? | default "")
    let run_cmd = ($task.run? | default "")
    let agent_type = ($task.agent?.type? | default "")
    let agent_model = ($task.agent?.model? | default "")
    let gate = ($task.gate? | default "")
    let condition = ($task.if? | default "")
    let on_fail = ($task.on_fail?.regress_to? | default "")
    let priority = ($task.priority? | default 0)
    let timeout = ($task.timeout_sec? | default 600)
    let max_attempts = ($task.retry?.limit? | default ($job_def.defaults?.retry?.limit? | default 3))

    let cond_esc = (sql-escape $condition)
    sql-exec $"INSERT OR REPLACE INTO tasks \(id, job_id, name, var, status, run_cmd, agent_type, agent_model, gate, condition, on_fail_regress, priority, timeout_sec, max_attempts\) VALUES \('($task_id)', '($job_id)', '($task.name)', '($var)', 'PENDING', '($run_cmd)', '($agent_type)', '($agent_model)', '($gate)', '($cond_esc)', '($on_fail)', ($priority), ($timeout), ($max_attempts)\)"

    # Insert dependencies
    for dep in ($task.needs? | default []) {
      sql-exec $"INSERT OR IGNORE INTO task_deps \(job_id, task_name, depends_on\) VALUES \('($job_id)', '($task.name)', '($dep)'\)"
    }
  }

  emit-event $job_id "" "job.StateChange" "" "PENDING" ""
  $job_id
}

# Execute a job using Kahn's BFS with replay + regression
export def job-execute [job_id: string] {
  # Mark job running
  sql-exec $"UPDATE jobs SET status = 'RUNNING', started_at = datetime\('now'\) WHERE id = '($job_id)'"
  emit-event $job_id "" "job.StateChange" "PENDING" "RUNNING" ""

  # Load all tasks
  let all_tasks = (sql $"SELECT name, status, condition, priority, on_fail_regress FROM tasks WHERE job_id = '($job_id)'" )

  # Build in-degree map
  let deps = (sql $"SELECT task_name, depends_on FROM task_deps WHERE job_id = '($job_id)'" )

  # Main execution loop
  loop {
    # Refresh task statuses
    let tasks = (sql $"SELECT name, status, condition, priority, on_fail_regress FROM tasks WHERE job_id = '($job_id)'" )

    # Find tasks with all deps satisfied (COMPLETED or SKIPPED)
    let pending = ($tasks | where status == "PENDING")
    let ready = ($pending | each {|t|
      let task_deps = ($deps | where task_name == $t.name | get depends_on)
      let deps_met = if ($task_deps | is-empty) {
        true
      } else {
        let dep_statuses = (sql $"SELECT name, status FROM tasks WHERE job_id = '($job_id)' AND name IN \(($task_deps | each {|d| $"'($d)'" } | str join ",")\)" )
        ($dep_statuses | all {|d| $d.status in ["COMPLETED", "SKIPPED"] })
      }
      if $deps_met { $t } else { null }
    } | compact)

    if ($ready | is-empty) { break }

    # Sort by priority descending
    let sorted = ($ready | sort-by priority -r)

    # Execute ready tasks in parallel
    let results = ($sorted | par-each {|t|
      task-execute $job_id $t.name
    })

    # Check for regressions
    for result in $results {
      if ($result.regression? | default "" | is-not-empty) {
        task-regress $job_id $result.regression
        # Loop will restart and re-evaluate
      }
    }

    # Check if any task failed without regression
    let failed = (sql $"SELECT name FROM tasks WHERE job_id = '($job_id)' AND status = 'FAILED'" )
    if ($failed | is-not-empty) {
      sql-exec $"UPDATE jobs SET status = 'FAILED', completed_at = datetime\('now'\), error = 'Task failed: ($failed.0.name)' WHERE id = '($job_id)'"
      emit-event $job_id "" "job.StateChange" "RUNNING" "FAILED" $"Task failed: ($failed.0.name)"
      return { status: "FAILED", failed_task: $failed.0.name }
    }
  }

  # Check final state
  let final_tasks = (sql $"SELECT status FROM tasks WHERE job_id = '($job_id)'" )
  let all_done = ($final_tasks | all {|t| $t.status in ["COMPLETED", "SKIPPED"] })

  if $all_done {
    sql-exec $"UPDATE jobs SET status = 'COMPLETED', completed_at = datetime\('now'\) WHERE id = '($job_id)'"
    emit-event $job_id "" "job.StateChange" "RUNNING" "COMPLETED" ""
    { status: "COMPLETED" }
  } else {
    sql-exec $"UPDATE jobs SET status = 'FAILED', completed_at = datetime\('now'\) WHERE id = '($job_id)'"
    emit-event $job_id "" "job.StateChange" "RUNNING" "FAILED" ""
    { status: "FAILED" }
  }
}

# Resume a job from its last checkpoint (skip completed tasks)
export def job-resume [job_id: string] {
  let job = (sql $"SELECT status FROM jobs WHERE id = '($job_id)'" )
  if ($job | is-empty) {
    error make { msg: $"Job not found: ($job_id)" }
  }
  # Reset RUNNING tasks back to PENDING (they were interrupted)
  sql-exec $"UPDATE tasks SET status = 'PENDING' WHERE job_id = '($job_id)' AND status = 'RUNNING'"
  # Reset job to RUNNING if it was FAILED
  sql-exec $"UPDATE jobs SET status = 'RUNNING' WHERE id = '($job_id)'"
  emit-event $job_id "" "job.StateChange" $job.0.status "RUNNING" "resumed"
  job-execute $job_id
}

# Execute a single task with retry + gate evaluation
export def task-execute [job_id: string, task_name: string]: nothing -> record {
  let task = (sql $"SELECT * FROM tasks WHERE job_id = '($job_id)' AND name = '($task_name)'" ).0

  # Replay check
  if $task.status == "COMPLETED" {
    return { name: $task_name, status: "COMPLETED", output: $task.output }
  }

  # Condition check — skip if `if` evaluates false
  if ($task.condition | is-not-empty) {
    let should_run = (eval-condition $job_id $task.condition)
    if not $should_run {
      sql-exec $"UPDATE tasks SET status = 'SKIPPED', completed_at = datetime\('now'\) WHERE job_id = '($job_id)' AND name = '($task_name)'"
      emit-event $job_id $task_name "task.StateChange" $task.status "SKIPPED" "condition false"
      return { name: $task_name, status: "SKIPPED" }
    }
  }

  # Mark RUNNING
  sql-exec $"UPDATE tasks SET status = 'RUNNING', started_at = datetime\('now'\), attempt = attempt + 1 WHERE job_id = '($job_id)' AND name = '($task_name)'"
  emit-event $job_id $task_name "task.StateChange" $task.status "RUNNING" ""

  # Execute with retry loop
  let max = ($task.max_attempts | into int)
  mut attempt = 0
  mut last_error = ""
  mut result = { status: "FAILED" }

  while $attempt < $max {
    $attempt = $attempt + 1
    if $attempt > 1 {
      # Exponential backoff
      let delay = ($task.retry_delay_sec | into int) * (($task.retry_scaling | into int) ** ($attempt - 1))
      sleep ($delay | into duration)
    }

    sql-exec $"UPDATE tasks SET attempt = ($attempt) WHERE job_id = '($job_id)' AND name = '($task_name)'"

    # Execute the task
    let exec_result = (try {
      run-task $job_id $task
    } catch {|e|
      { status: "FAILED", error: ($e | get msg? | default "unknown error") }
    })

    if $exec_result.status == "COMPLETED" {
      let output = (sql-escape ($exec_result.output? | default ""))
      let start = (sql $"SELECT started_at FROM tasks WHERE job_id = '($job_id)' AND name = '($task_name)'" ).0.started_at
      sql-exec $"UPDATE tasks SET status = 'COMPLETED', output = '($output)', completed_at = datetime\('now'\) WHERE job_id = '($job_id)' AND name = '($task_name)'"
      emit-event $job_id $task_name "task.StateChange" "RUNNING" "COMPLETED" ""
      $result = { name: $task_name, status: "COMPLETED", output: ($exec_result.output? | default "") }
      break
    }

    $last_error = ($exec_result.error? | default "gate failed")
  }

  # All retries exhausted — check regression
  if $result.status == "FAILED" {
    let err = (sql-escape $last_error)
    sql-exec $"UPDATE tasks SET status = 'FAILED', error = '($err)', completed_at = datetime\('now'\) WHERE job_id = '($job_id)' AND name = '($task_name)'"
    emit-event $job_id $task_name "task.StateChange" "RUNNING" "FAILED" $last_error

    if ($task.on_fail_regress | is-not-empty) {
      emit-event $job_id $task_name "task.Regression" "" $task.on_fail_regress $last_error
      $result = { name: $task_name, status: "FAILED", regression: $task.on_fail_regress }
    } else {
      $result = { name: $task_name, status: "FAILED", error: $last_error }
    }
  }

  $result
}

# Run a task — dispatch to agent or inline execution
def run-task [job_id: string, task: record]: nothing -> record {
  # Gather prior task outputs for context
  let prior_outputs = (gather-task-outputs $job_id)

  if ($task.agent_type | is-not-empty) {
    # Agent execution via opencode
    use oc-agent.nu *
    let session = (oc-session-create $"($job_id):($task.name)")
    let prompt = (build-prompt $job_id $task $prior_outputs)
    oc-prompt $session.id $prompt
    let response = (oc-wait-idle $session.id ($task.timeout_sec | into int))
    let output = $response.content

    # Evaluate gate
    if ($task.gate | is-not-empty) {
      let gate_result = (gate-check $task.gate $output $job_id)
      if $gate_result.pass {
        { status: "COMPLETED", output: $output }
      } else {
        { status: "FAILED", error: $gate_result.reason, output: $output }
      }
    } else {
      { status: "COMPLETED", output: $output }
    }
  } else {
    # Inline execution
    let output = (try {
      if ($task.run_cmd | is-not-empty) {
        ^nu -c $task.run_cmd | complete
      } else {
        { stdout: "", exit_code: 0 }
      }
    } catch {|e|
      { stdout: "", exit_code: 1, stderr: ($e | get msg? | default "inline execution failed") }
    })

    if ($output.exit_code? | default 0) == 0 {
      { status: "COMPLETED", output: ($output.stdout? | default "") }
    } else {
      { status: "FAILED", error: ($output.stderr? | default "non-zero exit") }
    }
  }
}

# Build prompt for agent task using prior outputs
def build-prompt [job_id: string, task: record, prior_outputs: record]: nothing -> string {
  let bead_id = (sql $"SELECT bead_id FROM jobs WHERE id = '($job_id)'" ).0.bead_id
  let context_lines = ($prior_outputs | transpose key value | each {|kv|
    $"## Prior output: ($kv.key)\n($kv.value)"
  } | str join "\n\n")

  $"# Task: ($task.name) for bead ($bead_id)\n\nPhase: ($task.run_cmd)\nGate: ($task.gate)\n\n($context_lines)"
}

# Gather completed task outputs as a record keyed by var name
def gather-task-outputs [job_id: string]: nothing -> record {
  let completed = (sql $"SELECT var, output FROM tasks WHERE job_id = '($job_id)' AND status = 'COMPLETED' AND var IS NOT NULL AND var != ''" )
  mut outputs = {}
  for row in $completed {
    $outputs = ($outputs | insert $row.var $row.output)
  }
  $outputs
}

# ── Phase Regression ─────────────────────────────────────────────────────────

# Reset target task + all downstream tasks to PENDING
export def task-regress [job_id: string, target_task: string] {
  # Find all tasks transitively depending on target
  let all_downstream = (find-downstream $job_id $target_task)
  let to_reset = ([$target_task] | append $all_downstream | uniq)

  for task_name in $to_reset {
    sql-exec $"UPDATE tasks SET status = 'PENDING', output = NULL, error = NULL, attempt = 0, started_at = NULL, completed_at = NULL, duration_ms = NULL WHERE job_id = '($job_id)' AND name = '($task_name)'"
    emit-event $job_id $task_name "task.Regression" "" "PENDING" $"regressed from ($target_task)"
  }
}

# Find all tasks transitively downstream of a given task
def find-downstream [job_id: string, task_name: string]: nothing -> list<string> {
  let direct = (sql $"SELECT task_name FROM task_deps WHERE job_id = '($job_id)' AND depends_on = '($task_name)'" | get task_name)
  if ($direct | is-empty) { return [] }
  let indirect = ($direct | each {|d| find-downstream $job_id $d } | flatten)
  $direct | append $indirect | uniq
}

# ── Condition Evaluation ─────────────────────────────────────────────────────

# Evaluate a Tork-style condition like '{{ tasks.triage.route contains 1 }}'
def eval-condition [job_id: string, condition: string]: nothing -> bool {
  # Parse {{ tasks.<name>.route contains <phase> }}
  let match = ($condition | parse "{{ tasks.{task_ref}.route contains {phase} }}" | get -o 0)
  if ($match | is-empty) { return true }

  let task_ref = $match.task_ref
  let phase = ($match.phase | str trim | into int)
  let output = (sql $"SELECT output FROM tasks WHERE job_id = '($job_id)' AND name = '($task_ref)' AND status = 'COMPLETED'" )

  if ($output | is-empty) { return false }

  # Parse the route from the triage output (expect JSON array or comma list)
  let route_str = $output.0.output
  try {
    let route = ($route_str | from json)
    $phase in $route
  } catch {
    # Fallback: check if phase number appears in output
    ($route_str | str contains ($phase | into string))
  }
}

# ── Gate Evaluation ──────────────────────────────────────────────────────────

# Evaluate a gate condition against task output
export def gate-check [gate_name: string, output: string, job_id: string]: nothing -> record {
  # Gate dispatch — each gate has specific pass criteria
  match $gate_name {
    "complexity_assessed" => {
      let has_route = ($output | str contains "route")
      { pass: $has_route, reason: (if $has_route { "ok" } else { "no route in output" }) }
    }
    "sufficient_context" => {
      let ok = ($output | str length) > 100
      { pass: $ok, reason: (if $ok { "ok" } else { "insufficient context" }) }
    }
    "plan_verified" => {
      let ok = ($output | str contains "PLAN") or ($output | str contains "plan")
      { pass: $ok, reason: (if $ok { "ok" } else { "no plan found" }) }
    }
    "user_approval" => {
      # Auto-approve in automated mode
      { pass: true, reason: "auto-approved" }
    }
    "tests_fail" => {
      # RED phase: tests should fail
      let result = (try { ^moon run :test | complete } catch { { exit_code: 1 } })
      let ok = ($result.exit_code? | default 1) != 0
      { pass: $ok, reason: (if $ok { "tests fail as expected" } else { "tests pass — RED phase needs failing tests" }) }
    }
    "tests_pass" | "tests_green" => {
      let result = (try { ^moon run :test | complete } catch { { exit_code: 1 } })
      let ok = ($result.exit_code? | default 1) == 0
      { pass: $ok, reason: (if $ok { "tests pass" } else { "tests failing" }) }
    }
    "martin_fowler_1" | "martin_fowler_2" => {
      # Check agent output for PASS/FAIL verdict
      let pass = ($output | str downcase | str contains "pass") and (not ($output | str downcase | str contains "fail"))
      { pass: $pass, reason: (if $pass { "review passed" } else { "review found issues" }) }
    }
    "implementation_complete" => {
      let ok = ($output | str contains "DONE") or ($output | str contains "complete") or ($output | str contains "implemented")
      { pass: $ok, reason: (if $ok { "ok" } else { "implementation incomplete" }) }
    }
    "criteria_met" => {
      let ok = ($output | str downcase | str contains "met") or ($output | str downcase | str contains "pass")
      { pass: $ok, reason: (if $ok { "ok" } else { "criteria not met" }) }
    }
    "no_critical_issues" => {
      let ok = not ($output | str downcase | str contains "critical")
      { pass: $ok, reason: (if $ok { "ok" } else { "critical issues found" }) }
    }
    "qa_pass" => {
      let ok = ($output | str downcase | str contains "pass")
      { pass: $ok, reason: (if $ok { "ok" } else { "QA failed" }) }
    }
    "standards_met" => {
      let ok = ($output | str downcase | str contains "pass") or ($output | str downcase | str contains "consistent")
      { pass: $ok, reason: (if $ok { "ok" } else { "standards not met" }) }
    }
    "minimized" => {
      let result = (try { ^moon run :test | complete } catch { { exit_code: 1 } })
      let ok = ($result.exit_code? | default 1) == 0
      { pass: $ok, reason: (if $ok { "ok" } else { "final validation failed" }) }
    }
    "push_succeeded" => {
      # Landing gate — check tests + lint pass
      let test_result = (try { ^moon run :ci | complete } catch { { exit_code: 1 } })
      let ok = ($test_result.exit_code? | default 1) == 0
      { pass: $ok, reason: (if $ok { "CI passed" } else { "CI failed" }) }
    }
    _ => {
      # Unknown gate — pass by default
      { pass: true, reason: $"unknown gate: ($gate_name)" }
    }
  }
}

# ── Task Output Retrieval ────────────────────────────────────────────────────

# Retrieve cached output by var name (Tork's {{ tasks.X }})
export def task-output [job_id: string, var_name: string]: nothing -> string {
  let result = (sql $"SELECT output FROM tasks WHERE job_id = '($job_id)' AND var = '($var_name)' AND status = 'COMPLETED'" )
  if ($result | is-empty) { "" } else { $result.0.output }
}

# ── Job Management ───────────────────────────────────────────────────────────

# Show status of a job and all its tasks
export def job-status [job_id: string]: nothing -> record {
  let job = (sql $"SELECT * FROM jobs WHERE id = '($job_id)'" )
  if ($job | is-empty) {
    error make { msg: $"Job not found: ($job_id)" }
  }
  let tasks = (sql $"SELECT name, status, attempt, gate, error, started_at, completed_at FROM tasks WHERE job_id = '($job_id)' ORDER BY rowid" )
  { job: $job.0, tasks: $tasks }
}

# Cancel a running job
export def job-cancel [job_id: string] {
  sql-exec $"UPDATE jobs SET status = 'CANCELLED', completed_at = datetime\('now'\) WHERE id = '($job_id)'"
  sql-exec $"UPDATE tasks SET status = 'CANCELLED' WHERE job_id = '($job_id)' AND status IN \('PENDING', 'RUNNING', 'SCHEDULED'\)"
  emit-event $job_id "" "job.StateChange" "RUNNING" "CANCELLED" ""
}

# List all jobs
export def job-list []: nothing -> table {
  sql "SELECT id, name, bead_id, status, position, created_at, completed_at FROM jobs ORDER BY position, created_at"
}

# Retry a failed job — reset failed tasks and re-execute
export def job-retry [job_id: string] {
  sql-exec $"UPDATE tasks SET status = 'PENDING', error = NULL, attempt = 0 WHERE job_id = '($job_id)' AND status = 'FAILED'"
  sql-exec $"UPDATE jobs SET status = 'PENDING', error = NULL WHERE id = '($job_id)'"
  emit-event $job_id "" "job.StateChange" "FAILED" "PENDING" "retry"
  job-execute $job_id
}

# ── Event Log ────────────────────────────────────────────────────────────────

export def event-log [job_id: string, --limit: int = 50]: nothing -> table {
  sql $"SELECT * FROM events WHERE job_id = '($job_id)' ORDER BY created_at DESC LIMIT ($limit)"
}
