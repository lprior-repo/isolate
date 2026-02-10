# ZJJ - EARS Requirements Document

## Core Purpose

THE SYSTEM SHALL create and manage isolated development workspaces combining JJ version control with Zellij terminal multiplexing for focused parallel task execution

## Session Management

WHEN user invokes `zjj init` THE SYSTEM SHALL initialize zjj in a JJ repository

WHEN user invokes `zjj add <name>` THE SYSTEM SHALL create a new session with a JJ workspace and dedicated Zellij tab

WHEN user invokes `zjj list` THE SYSTEM SHALL display all sessions with their status

WHEN user invokes `zjj remove <name>` THE SYSTEM SHALL remove the session and cleanup its JJ workspace

WHEN user invokes `zjj focus <name>` THE SYSTEM SHALL switch to the session's Zellij tab

WHEN user invokes `zjj status [name]` THE SYSTEM SHALL show detailed session status

WHEN user invokes `zjj sync [name]` THE SYSTEM SHALL sync session workspace with main via rebase

WHEN user invokes `zjj diff <name>` THE SYSTEM SHALL show diff between session and main

WHEN user invokes `zjj attach <name>` THE SYSTEM SHALL attach to an existing Zellij session

WHEN user invokes `zjj clean` THE SYSTEM SHALL remove stale sessions

## Agent Workflow

WHEN user invokes `zjj spawn <bead-id>` THE SYSTEM SHALL spawn isolated workspace for a bead and run agent

WHEN user invokes `zjj done` THE SYSTEM SHALL complete work and merge workspace to main

## System & Diagnostics

WHEN user invokes `zjj config [key] [value]` THE SYSTEM SHALL view or modify configuration

WHEN user invokes `zjj doctor` THE SYSTEM SHALL run system health checks

WHEN user invokes `zjj introspect [cmd]` THE SYSTEM SHALL discover zjj capabilities and command details

WHEN user invokes `zjj query <type>` THE SYSTEM SHALL query system state programmatically

WHEN user invokes `zjj context` THE SYSTEM SHALL show complete environment context

WHEN user invokes `zjj dashboard` THE SYSTEM SHALL launch interactive TUI dashboard

## Error Handling

THE SYSTEM SHALL NOT use unwrap() or expect() for error handling

THE SYSTEM SHALL NOT panic under any conditions

THE SYSTEM SHALL return Result<T, Error> for all fallible operations

THE SYSTEM SHALL propagate errors using the ? operator

## Output Format

WHEN user provides --json flag THE SYSTEM SHALL output machine-readable JSON format

## State Management

THE SYSTEM SHALL persist session state in SQLite database

THE SYSTEM SHALL integrate with .beads/beads.db for issue tracking

## Naming Conventions

THE SYSTEM SHALL name Zellij tabs with format "zjj:<session-name>"

THE SYSTEM SHALL use rebase strategy for workspace sync (jj rebase -d main)

## Prerequisites

THE SYSTEM SHALL require Moon build system

THE SYSTEM SHALL require JJ (Jujutsu) version control

THE SYSTEM SHALL require Zellij terminal multiplexer

THE SYSTEM SHALL require Rust 1.80 or later
