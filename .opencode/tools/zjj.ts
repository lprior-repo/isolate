import { tool } from "@opencode-ai/plugin"

type ToolContext = {
  directory?: string
  worktree?: string
}

type CommandResult = {
  ok: boolean
  command: string[]
  cwd: string
  exitCode: number
  stdout: string
  stderr: string
  parsed: unknown | null
}

const SENSITIVE_PATTERNS = [
  /(DATABASE_URL\s*[=:]\s*)(\S+)/gi,
  /(TOKEN\s*[=:]\s*)(\S+)/gi,
  /(API_KEY\s*[=:]\s*)(\S+)/gi,
  /(SECRET\s*[=:]\s*)(\S+)/gi,
]

function redactSecrets(input: string): string {
  return SENSITIVE_PATTERNS.reduce(
    (value, pattern) => value.replace(pattern, "$1********"),
    input,
  )
}

function getCwd(context: ToolContext): string {
  return context.worktree ?? context.directory ?? process.cwd()
}

async function runZjj(args: string[], context: ToolContext): Promise<CommandResult> {
  const cwd = getCwd(context)
  const command = ["zjj", ...args]
  const process = Bun.spawn(command, {
    cwd,
    stdout: "pipe",
    stderr: "pipe",
  })

  const [stdoutRaw, stderrRaw] = await Promise.all([
    new Response(process.stdout).text(),
    new Response(process.stderr).text(),
  ])
  const exitCode = await process.exited
  const stdout = redactSecrets(stdoutRaw.trim())
  const stderr = redactSecrets(stderrRaw.trim())

  let parsed: unknown | null = null
  if (stdout.length > 0) {
    try {
      parsed = JSON.parse(stdout)
    } catch {
      parsed = null
    }
  }

  return {
    ok: exitCode === 0,
    command,
    cwd,
    exitCode,
    stdout,
    stderr,
    parsed,
  }
}

function withJson(args: string[]): string[] {
  return args.includes("--json") ? args : [...args, "--json"]
}

export const status = tool({
  description: "Get zjj status for all sessions or one session",
  args: {
    name: tool.schema
      .string()
      .optional()
      .describe("Optional session name. Omit for all sessions."),
  },
  async execute(args, context) {
    const cmd = ["status", ...(args.name ? [args.name] : [])]
    return runZjj(withJson(cmd), context)
  },
})

export const list = tool({
  description: "List zjj sessions with optional filters",
  args: {
    all: tool.schema
      .boolean()
      .default(false)
      .describe("Include completed and failed sessions"),
    verbose: tool.schema
      .boolean()
      .default(false)
      .describe("Include verbose details"),
    agent: tool.schema
      .string()
      .optional()
      .describe("Filter by agent owner name"),
    bead: tool.schema
      .string()
      .optional()
      .describe("Filter by bead id"),
    state: tool.schema
      .string()
      .optional()
      .describe("Filter by state (working, ready, merged, etc.)"),
  },
  async execute(args, context) {
    const cmd = ["list"]
    if (args.all) cmd.push("--all")
    if (args.verbose) cmd.push("--verbose")
    if (args.agent) cmd.push("--agent", args.agent)
    if (args.bead) cmd.push("--bead", args.bead)
    if (args.state) cmd.push("--state", args.state)
    return runZjj(withJson(cmd), context)
  },
})

export const spawn = tool({
  description: "Spawn a zjj workspace for a bead",
  args: {
    beadId: tool.schema.string().describe("Bead id, for example zjj-ab123"),
    background: tool.schema
      .boolean()
      .default(true)
      .describe("Run in background mode"),
    noAutoMerge: tool.schema
      .boolean()
      .default(false)
      .describe("Disable auto-merge on success"),
    noAutoCleanup: tool.schema
      .boolean()
      .default(false)
      .describe("Disable auto-cleanup on failure"),
    timeoutSeconds: tool.schema
      .number()
      .int()
      .positive()
      .optional()
      .describe("Optional timeout seconds"),
    dryRun: tool.schema
      .boolean()
      .default(false)
      .describe("Preview spawn without executing"),
  },
  async execute(args, context) {
    const cmd = ["spawn", args.beadId]
    if (args.background) cmd.push("--background")
    if (args.noAutoMerge) cmd.push("--no-auto-merge")
    if (args.noAutoCleanup) cmd.push("--no-auto-cleanup")
    if (args.timeoutSeconds) cmd.push("--timeout", String(args.timeoutSeconds))
    if (args.dryRun) cmd.push("--dry-run")
    return runZjj(withJson(cmd), context)
  },
})

export const sync = tool({
  description: "Sync zjj workspace(s) with main",
  args: {
    name: tool.schema
      .string()
      .optional()
      .describe("Session name to sync. Omit for current workspace."),
    all: tool.schema
      .boolean()
      .default(false)
      .describe("Sync all active sessions"),
    dryRun: tool.schema
      .boolean()
      .default(false)
      .describe("Preview sync without applying"),
  },
  async execute(args, context) {
    const cmd = ["sync"]
    if (args.name) cmd.push(args.name)
    if (args.all) cmd.push("--all")
    if (args.dryRun) cmd.push("--dry-run")
    return runZjj(withJson(cmd), context)
  },
})

export const focus = tool({
  description: "Focus a zjj session tab",
  args: {
    name: tool.schema
      .string()
      .optional()
      .describe("Session name. Omit for interactive pick."),
    noZellij: tool.schema
      .boolean()
      .default(false)
      .describe("Skip zellij integration in non-TTY environments"),
  },
  async execute(args, context) {
    const cmd = ["focus"]
    if (args.name) cmd.push(args.name)
    if (args.noZellij) cmd.push("--no-zellij")
    return runZjj(withJson(cmd), context)
  },
})

export const done = tool({
  description: "Complete zjj workspace and merge to main",
  args: {
    workspace: tool.schema
      .string()
      .optional()
      .describe("Workspace name. Omit for current workspace."),
    message: tool.schema
      .string()
      .optional()
      .describe("Optional commit message"),
    squash: tool.schema
      .boolean()
      .default(false)
      .describe("Squash commits before merge"),
    keepWorkspace: tool.schema
      .boolean()
      .default(false)
      .describe("Keep workspace after completion"),
    dryRun: tool.schema
      .boolean()
      .default(false)
      .describe("Preview done operation"),
  },
  async execute(args, context) {
    const cmd = ["done"]
    if (args.workspace) cmd.push("--workspace", args.workspace)
    if (args.message) cmd.push("--message", args.message)
    if (args.squash) cmd.push("--squash")
    if (args.keepWorkspace) cmd.push("--keep-workspace")
    if (args.dryRun) cmd.push("--dry-run")
    return runZjj(withJson(cmd), context)
  },
})

export const abort = tool({
  description: "Abort a zjj workspace and abandon changes",
  args: {
    workspace: tool.schema
      .string()
      .optional()
      .describe("Workspace/session name. Omit for current workspace."),
    keepWorkspace: tool.schema
      .boolean()
      .default(false)
      .describe("Keep workspace files while removing zjj tracking"),
    noBeadUpdate: tool.schema
      .boolean()
      .default(false)
      .describe("Skip bead status update"),
    dryRun: tool.schema
      .boolean()
      .default(false)
      .describe("Preview abort without executing"),
  },
  async execute(args, context) {
    const cmd = ["abort"]
    if (args.workspace) cmd.push("--workspace", args.workspace)
    if (args.keepWorkspace) cmd.push("--keep-workspace")
    if (args.noBeadUpdate) cmd.push("--no-bead-update")
    if (args.dryRun) cmd.push("--dry-run")
    return runZjj(withJson(cmd), context)
  },
})
