import { Effect } from "../lib/effect"
import { tool } from "@opencode-ai/plugin"

type ToolContext = {
  directory?: string
  worktree?: string
}

type ToolError = {
  code:
    | "BINARY_NOT_FOUND"
    | "TIMEOUT"
    | "PROCESS_ERROR"
    | "INVALID_CONFIRMATION"
    | "INVALID_SUBCOMMAND"
  message: string
}

type CommandResult = {
  ok: boolean
  command: string[]
  cwd: string
  exitCode: number
  stdout: string
  stderr: string
  parsed: unknown | null
  durationMs: number
  error?: ToolError
}

type VerifiedResponse = {
  ok: boolean
  action: string
  result: CommandResult
  verification?: CommandResult
}

const DEFAULT_TIMEOUT_MS = 600_000

const ALLOWED_SUBCOMMANDS = new Set([
  "status",
  "list",
  "add",
  "work",
  "spawn",
  "sync",
  "focus",
  "done",
  "abort",
  "whereami",
  "whoami",
])

const STATUS_VERIFICATION_ACTIONS = new Set(["add", "work", "spawn", "sync", "done", "abort"])

const SENSITIVE_PATTERNS = [
  /(DATABASE_URL\s*[=:]\s*)(\S+)/gi,
  /(TOKEN\s*[=:]\s*)(\S+)/gi,
  /(API_KEY\s*[=:]\s*)(\S+)/gi,
  /(SECRET\s*[=:]\s*)(\S+)/gi,
  /(PASSWORD\s*[=:]\s*)(\S+)/gi,
  /(Authorization:\s*Bearer\s+)(\S+)/gi,
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

function withJson(args: string[]): string[] {
  return args.includes("--json") ? args : [...args, "--json"]
}

function extractSessionName(parsed: unknown): string | null {
  if (!parsed || typeof parsed !== "object") return null

  const data = parsed as Record<string, unknown>
  const direct = data.name ?? data.session_name
  if (typeof direct === "string" && direct.length > 0) return direct

  const sessions = data.sessions
  if (Array.isArray(sessions) && sessions.length === 1) {
    const first = sessions[0]
    if (first && typeof first === "object") {
      const named = (first as Record<string, unknown>).name
      if (typeof named === "string" && named.length > 0) return named
    }
  }

  return null
}

function confirmationError(action: string): ToolError {
  return {
    code: "INVALID_CONFIRMATION",
    message: `Refusing to run zjj ${action} without confirm=true.`,
  }
}

function makeErrorResult(
  cwd: string,
  command: string[],
  exitCode: number,
  stderr: string,
  error: ToolError,
  durationMs = 0,
): CommandResult {
  return {
    ok: false,
    command,
    cwd,
    exitCode,
    stdout: "",
    stderr,
    parsed: null,
    durationMs,
    error,
  }
}

function formatToolOutput(value: unknown): string {
  return JSON.stringify(value, null, 2)
}

function parsePossiblyMixedJson(stdout: string): unknown | null {
  const trimmed = stdout.trim()
  if (trimmed.length === 0) return null

  try {
    return JSON.parse(trimmed)
  } catch {
    // Fall through and try extracting trailing JSON payload.
  }

  for (let index = trimmed.lastIndexOf("{"); index >= 0; index = trimmed.lastIndexOf("{", index - 1)) {
    const candidate = trimmed.slice(index)
    try {
      return JSON.parse(candidate)
    } catch {
      // Continue scanning for a valid JSON object boundary.
    }
  }

  return null
}

function executeProcessEffect(
  binary: string,
  subcommand: string,
  args: string[],
  cwd: string,
  timeoutMs: number,
) {
  return Effect.tryPromise({
    try: () =>
      new Promise<{ stdoutRaw: string; stderrRaw: string; exitCode: number; timedOut: boolean }>(
        (resolve, reject) => {
          const process = Bun.spawn([binary, subcommand, ...args], {
            cwd,
            stdout: "pipe",
            stderr: "pipe",
          })

          let timedOut = false
          const timeoutHandle = setTimeout(() => {
            timedOut = true
            process.kill()
          }, timeoutMs)

          const settle = <T>(fn: (value: T) => void) => (value: T) => {
            clearTimeout(timeoutHandle)
            fn(value)
          }

          Promise.all([new Response(process.stdout).text(), new Response(process.stderr).text()])
            .then(([stdoutRaw, stderrRaw]) =>
              process.exited.then((exitCode) => ({ stdoutRaw, stderrRaw, exitCode, timedOut })),
            )
            .then(settle(resolve))
            .catch(settle(reject))
        },
      ),
    catch: (err) => err,
  })
}

function runZjjEffect(
  subcommand: string,
  args: string[],
  context: ToolContext,
  timeoutMs?: number,
) {
  return Effect.gen(function* () {
    const started = Date.now()
    const cwd = getCwd(context)

    if (!ALLOWED_SUBCOMMANDS.has(subcommand)) {
      return makeErrorResult(
        cwd,
        ["zjj", subcommand, ...args],
        2,
        `Subcommand '${subcommand}' is not allowlisted.`,
        {
          code: "INVALID_SUBCOMMAND",
          message: `Subcommand '${subcommand}' is not allowed.`,
        },
        Date.now() - started,
      )
    }

    const binary = Bun.which("zjj")
    if (!binary) {
      return makeErrorResult(
        cwd,
        ["zjj", subcommand, ...args],
        127,
        "zjj binary not found on PATH.",
        {
          code: "BINARY_NOT_FOUND",
          message: "zjj binary not found on PATH.",
        },
        Date.now() - started,
      )
    }

    const timeout = timeoutMs ?? DEFAULT_TIMEOUT_MS
    const processResultOrError = yield* executeProcessEffect(binary, subcommand, args, cwd, timeout).pipe(
      Effect.map((data) => ({ tag: "ok" as const, data })),
      Effect.catchAll((err) => Effect.succeed({ tag: "err" as const, err })),
    )

    if (processResultOrError.tag === "err") {
      const message = redactSecrets(
        processResultOrError.err instanceof Error
          ? processResultOrError.err.message
          : "Unknown process execution error",
      )
      return makeErrorResult(
        cwd,
        ["zjj", subcommand, ...args],
        1,
        message,
        {
          code: "PROCESS_ERROR",
          message,
        },
        Date.now() - started,
      )
    }

    const processResult = processResultOrError.data

    const stdout = redactSecrets(processResult.stdoutRaw.trim())
    const stderr = redactSecrets(processResult.stderrRaw.trim())

    const parsed = parsePossiblyMixedJson(stdout)

    return {
      ok: processResult.exitCode === 0,
      command: ["zjj", subcommand, ...args],
      cwd,
      exitCode: processResult.exitCode,
      stdout,
      stderr,
      parsed,
      durationMs: Date.now() - started,
      ...(processResult.timedOut
        ? {
            error: {
              code: "TIMEOUT" as const,
              message: `zjj ${subcommand} timed out after ${timeout}ms.`,
            },
          }
        : {}),
    }
  })
}

function runWithVerificationEffect(
  action: string,
  args: string[],
  context: ToolContext,
  verificationName?: string,
  timeoutMs?: number,
) {
  return Effect.gen(function* () {
    const result = yield* runZjjEffect(action, withJson(args), context, timeoutMs)
    if (!result.ok || !STATUS_VERIFICATION_ACTIONS.has(action)) {
      return { ok: result.ok, action, result }
    }

    const inferred = extractSessionName(result.parsed)
    const session = verificationName ?? inferred
    const verificationArgs = session ? [session] : []
    const verification = yield* runZjjEffect("status", withJson(verificationArgs), context, timeoutMs)

    return {
      ok: result.ok && verification.ok,
      action,
      result,
      verification,
    }
  })
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
    const cmd = args.name ? [args.name] : []
    const result = await Effect.runPromise(runZjjEffect("status", withJson(cmd), context))
    return formatToolOutput({ ok: result.ok, action: "status", result })
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
    const cmd = [] as string[]
    if (args.all) cmd.push("--all")
    if (args.verbose) cmd.push("--verbose")
    if (args.agent) cmd.push("--agent", args.agent)
    if (args.bead) cmd.push("--bead", args.bead)
    if (args.state) cmd.push("--state", args.state)
    const result = await Effect.runPromise(runZjjEffect("list", withJson(cmd), context))
    return formatToolOutput({ ok: result.ok, action: "list", result })
  },
})

export const add = tool({
  description: "Create a manual zjj workspace session",
  args: {
    name: tool.schema
      .string()
      .optional()
      .describe("Session name. Omit to let zjj choose interactively where supported."),
    beadId: tool.schema
      .string()
      .optional()
      .describe("Optional bead ID to associate with the session"),
    template: tool.schema
      .enum(["minimal", "standard", "full"])
      .optional()
      .describe("Optional zellij layout template"),
    noOpen: tool.schema
      .boolean()
      .default(true)
      .describe("Create session without opening zellij tab"),
    noZellij: tool.schema
      .boolean()
      .default(true)
      .describe("Skip zellij integration in web/non-TTY contexts"),
    idempotent: tool.schema
      .boolean()
      .default(true)
      .describe("Succeed if session already exists"),
    dryRun: tool.schema
      .boolean()
      .default(false)
      .describe("Preview without creating"),
  },
  async execute(args, context) {
    const cmd = [] as string[]
    if (args.name) cmd.push(args.name)
    if (args.beadId) cmd.push("--bead", args.beadId)
    if (args.template) cmd.push("--template", args.template)
    if (args.noOpen) cmd.push("--no-open")
    if (args.noZellij) cmd.push("--no-zellij")
    if (args.idempotent) cmd.push("--idempotent")
    if (args.dryRun) cmd.push("--dry-run")
    return formatToolOutput(
      await Effect.runPromise(runWithVerificationEffect("add", cmd, context, args.name)),
    )
  },
})

export const work = tool({
  description: "Unified AI/manual workflow start for zjj sessions",
  args: {
    name: tool.schema.string().optional().describe("Session name to create/use"),
    beadId: tool.schema
      .string()
      .optional()
      .describe("Optional bead id to associate with this work"),
    noZellij: tool.schema
      .boolean()
      .default(true)
      .describe("Skip zellij integration in web/non-TTY contexts"),
    noAgent: tool.schema
      .boolean()
      .default(false)
      .describe("Do not register as an agent"),
    idempotent: tool.schema
      .boolean()
      .default(true)
      .describe("Succeed if session already exists"),
    dryRun: tool.schema
      .boolean()
      .default(false)
      .describe("Preview without creating"),
  },
  async execute(args, context) {
    const cmd = [] as string[]
    if (args.name) cmd.push(args.name)
    if (args.beadId) cmd.push("--bead", args.beadId)
    if (args.noZellij) cmd.push("--no-zellij")
    if (args.noAgent) cmd.push("--no-agent")
    if (args.idempotent) cmd.push("--idempotent")
    if (args.dryRun) cmd.push("--dry-run")
    return formatToolOutput(
      await Effect.runPromise(runWithVerificationEffect("work", cmd, context, args.name)),
    )
  },
})

export const spawn = tool({
  description: "Spawn a zjj workspace for autonomous agent bead work",
  args: {
    beadId: tool.schema.string().describe("Bead id, for example zjj-ab123"),
    background: tool.schema
      .boolean()
      .default(false)
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
    idempotent: tool.schema
      .boolean()
      .default(true)
      .describe("Succeed if workspace already exists"),
    dryRun: tool.schema
      .boolean()
      .default(false)
      .describe("Preview spawn without executing"),
  },
  async execute(args, context) {
    const cmd = [args.beadId]
    if (args.background) cmd.push("--background")
    if (args.noAutoMerge) cmd.push("--no-auto-merge")
    if (args.noAutoCleanup) cmd.push("--no-auto-cleanup")
    if (args.timeoutSeconds) cmd.push("--timeout", String(args.timeoutSeconds))
    if (args.idempotent) cmd.push("--idempotent")
    if (args.dryRun) cmd.push("--dry-run")

    const timeoutMs = args.timeoutSeconds ? args.timeoutSeconds * 1000 : undefined
    return formatToolOutput(
      await Effect.runPromise(runWithVerificationEffect("spawn", cmd, context, undefined, timeoutMs)),
    )
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
    const cmd = [] as string[]
    if (args.name) cmd.push(args.name)
    if (args.all) cmd.push("--all")
    if (args.dryRun) cmd.push("--dry-run")
    return formatToolOutput(
      await Effect.runPromise(runWithVerificationEffect("sync", cmd, context, args.name)),
    )
  },
})

export const focus = tool({
  description: "Focus a zjj session tab (non-interactive safe)",
  args: {
    name: tool.schema.string().describe("Session name to focus"),
    noZellij: tool.schema
      .boolean()
      .default(true)
      .describe("Skip zellij integration in non-TTY/web contexts"),
  },
  async execute(args, context) {
    const cmd = [args.name]
    if (args.noZellij) cmd.push("--no-zellij")
    const result = await Effect.runPromise(runZjjEffect("focus", withJson(cmd), context))
    return formatToolOutput({ ok: result.ok, action: "focus", result })
  },
})

export const done = tool({
  description: "Complete zjj workspace and merge to main (guarded)",
  args: {
    confirm: tool.schema
      .boolean()
      .default(false)
      .describe("Must be true to execute merge completion"),
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
    if (!args.confirm) {
      const err = confirmationError("done")
      return formatToolOutput({
        ok: false,
        action: "done",
        result: makeErrorResult(getCwd(context), ["zjj", "done"], 2, err.message, err),
      })
    }

    const cmd = [] as string[]
    if (args.workspace) cmd.push("--workspace", args.workspace)
    if (args.message) cmd.push("--message", args.message)
    if (args.squash) cmd.push("--squash")
    if (args.keepWorkspace) cmd.push("--keep-workspace")
    if (args.dryRun) cmd.push("--dry-run")

    return formatToolOutput(await Effect.runPromise(runWithVerificationEffect("done", cmd, context)))
  },
})

export const abort = tool({
  description: "Abort a zjj workspace and abandon changes (guarded)",
  args: {
    confirm: tool.schema
      .boolean()
      .default(false)
      .describe("Must be true to execute abort"),
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
    if (!args.confirm) {
      const err = confirmationError("abort")
      return formatToolOutput({
        ok: false,
        action: "abort",
        result: makeErrorResult(getCwd(context), ["zjj", "abort"], 2, err.message, err),
      })
    }

    const cmd = [] as string[]
    if (args.workspace) cmd.push("--workspace", args.workspace)
    if (args.keepWorkspace) cmd.push("--keep-workspace")
    if (args.noBeadUpdate) cmd.push("--no-bead-update")
    if (args.dryRun) cmd.push("--dry-run")

    return formatToolOutput(await Effect.runPromise(runWithVerificationEffect("abort", cmd, context)))
  },
})

export const whereami = tool({
  description: "Get current zjj location context",
  args: {},
  async execute(_, context) {
    const result = await Effect.runPromise(runZjjEffect("whereami", withJson([]), context))
    return formatToolOutput({ ok: result.ok, action: "whereami", result })
  },
})

export const whoami = tool({
  description: "Get current zjj agent identity context",
  args: {},
  async execute(_, context) {
    const result = await Effect.runPromise(runZjjEffect("whoami", withJson([]), context))
    return formatToolOutput({ ok: result.ok, action: "whoami", result })
  },
})
