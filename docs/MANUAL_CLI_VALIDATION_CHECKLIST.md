# Manual CLI Validation Checklist

This document contains test cases for manual validation of zjj CLI commands.

## Test Categories

### 1. Basic Operations

#### 1.1 Repository Setup
- [ ] Test `zjj add <name>` with valid repository name
- [ ] Test `zjj add <name>` with invalid name (should fail)
- [ ] Test `zjj add <name>` with existing name (should fail)
- [ ] Test `zjj list` shows added repositories
- [ ] Test `zjj status` shows correct repository count

#### 1.2 Workspace Management
- [ ] Test `zjj add` creates workspace directory
- [ ] Test `zjj focus <name>` switches to workspace
- [ ] Test `zjj focus <name>` shows workspace info
- [ ] Test `zjj focus <missing>` shows error
- [ ] Test workspace isolation (commands in one workspace don't affect others)

### 2. Session Management

#### 2.1 Session Creation
- [ ] Test `zjj work` creates new session
- [ ] Test `zjj work --name <name>` creates named session
- [ ] Test `zjj work` in non-focused workspace shows error
- [ ] Test session name validation (no dashes, no special chars)

#### 2.2 Session Operations
- [ ] Test `zjj list` shows active sessions
- [ ] Test `zjj status` shows session count
- [ ] Test session persistence across terminal detach/reattach

### 3. Queue Management

#### 3.1 Queue Operations
- [ ] Test `zjj submit <command>` adds to queue
- [ ] Test `zjj submit --dry` shows what would be submitted
- [ ] Test `zjj queue list` shows queue contents
- [ ] Test `zjj queue clear` removes all queued items
- [ ] Test queue ordering (FIFO)

#### 3.2 Queue Constraints
- [ ] Test empty command submission (should fail)
- [ ] Test malformed JSON in command (should fail)
- [ ] Test duplicate submission handling

### 4. Checkout Operations

#### 4.1 Basic Checkout
- [ ] Test `zjj checkout <commit>` checks out commit
- [ ] Test `zjj checkout --abort` aborts current checkout
- [ ] Test `zjj checkout --abort` when no checkout active (should fail)
- [ ] Test checkout with uncommitted changes (should handle gracefully)

#### 4.2 Checkout Constraints
- [ ] Test checkout to invalid commit (should fail)
- [ ] Test checkout when workspace is busy (should wait or fail appropriately)

### 5. Done Operations

#### 5.1 Basic Done
- [ ] Test `zjj done` completes current work
- [ ] Test `zjj done --squash` squashes commits
- [ ] Test `zjj done --abort` aborts current work
- [ ] Test `zjj done --dry` shows what would be done

#### 5.2 Done Constraints
- [ ] Test done with uncommitted changes (should fail or warn)
- [ ] Test done with no active work (should fail appropriately)

### 6. Repository Management

#### 6.1 Repository Add/Remove
- [ ] Test `zjj add <name>` with git URL
- [ ] Test `zjj add <name>` with local path
- [ ] Test `zjj remove <name>` removes repository
- [ ] Test remove with active sessions (should fail or warn)

#### 6.2 Repository Status
- [ ] Test `zjj status` shows all repositories
- [ ] Test `zjj status --json` outputs valid JSON
- [ ] Test `zjj status` shows active/total counts

### 7. JSON Output Validation

#### 7.1 Output Format
- [ ] Test `zjj list --json` outputs valid JSON
- [ ] Test `zjj status --json` outputs valid JSON
- [ ] Test JSON structure matches schema
- [ ] Test error outputs use JSON envelope format

#### 7.2 Error Handling
- [ ] Test invalid command returns error JSON
- [ ] Test missing argument returns error JSON
- [ ] Test JSON errors have consistent structure

### 8. Error Handling

#### 8.1 User Errors
- [ ] Test invalid command shows helpful error
- [ ] Test missing dependency shows error
- [ ] Test permission denied shows error

#### 8.2 System Errors
- [ ] Test disk full handling
- [ ] Test git operation failure handling
- [ ] Test network failure handling

### 9. Performance

#### 9.1 Command Speed
- [ ] Test `zjj status` completes under 1 second
- [ ] Test `zjj list` completes under 1 second
- [ ] Test queue operations complete under expected time

#### 9.2 Concurrency
- [ ] Test parallel operations don't interfere
- [ ] Test lock contention handling
- [ ] Test timeout behavior

### 10. Zellij Integration

#### 10.1 Session Attachment
- [ ] Test `zjj attach` attaches to session
- [ ] Test attach shows correct panes
- [ ] Test detach/reattach preserves state

#### 10.2 Workspace Switching
- [ ] Test `zjj switch` switches workspaces
- [ ] Test switch preserves session state
- [ ] Test switch shows workspace info

## Validation Procedure

1. Run each test case manually
2. Record success/failure for each
3. Document any unexpected behavior
4. Update this checklist with results

## Notes

- All manual tests must complete without panics
- Error messages should be clear and actionable
- JSON output must be valid and parseable
