# Bead Analysis: zjj-gm9a - Create CUE schema for JSON outputs

## Executive Summary

**Status**: HIGH COMPLEXITY task  
**Date Analyzed**: 2026-01-18  
**Output Types Found**: 27  
**Nested Helper Types**: 19  
**Files Affected**: 8  
**Estimated Effort**: 11.5 hours

## Overview

The zjj codebase has **27 distinct JSON output types** distributed across multiple files. These range from simple single-level structs to deeply nested composite structures. Creating comprehensive CUE schemas for all of these will standardize JSON output validation and enable better tooling integration.

### Distribution

- **24 types** in `crates/zjj/src/json_output.rs` (main output definitions)
- **3+ types** spread across command-specific modules:
  - `SessionStatusInfo` in status module
  - `SessionListItem` in list module  
  - `PrimeOutput` in prime module
  - `DoctorOutput` in core introspection
  - `BackupOutput` and `VerifyBackupOutput` in backup module
  - `ContextOutput` in context module

## Complexity Breakdown

### Simple Types (13 - 1 hour)
Single-level structs with primitive fields only:
- InitOutput, AddOutput, FocusOutput
- ConfigViewAllOutput, ConfigGetOutput, ConfigSetOutput
- BackupOutput, VerifyBackupOutput
- AgentInfo, AgentListOutput
- ExampleHelp, ExitCodeHelp, ParameterHelp

### Intermediate Types (10 - 2 hours)
Structs with 1-2 levels of nesting:
- RemoveOutput (+ RemoveOperation)
- DiffOutput (+ DiffStat, FileDiffStat)
- SyncOutput
- HelpOutput (+ SubcommandHelp)
- FileChanges, DiffStats, BeadStats
- SessionBeadInfo, SessionAgentInfo
- CommandRef, CommandCategories

### Complex Types (3 - 2.5 hours)
Heavily nested structures (3+ levels):
- **PrimeOutput**: 6+ nested types (JjStatus, ZjjStatus, SessionInfo, CommandCategories, BeadsStatus, WorkflowSection)
- **DoctorOutput**: Multiple check types with enums
- **ContextOutput**: 5 nested levels (EnvironmentContext, SessionStats, EnvironmentInfo, DependencyStatus, DependencyInfo)

### Generic Types (1 - 1 hour)
- **BatchOperationOutput<T>**: Parametric schema for batch operations
- **BatchItemResult<T>**: Generic result wrapper

### Response Wrappers (2 - 1 hour)
- **StatusResponse**: Wraps SessionStatusInfo with metadata
- **SessionListResponse**: Wraps SessionListItem with metadata

### Planning Structures (2 - 1.5 hours)
- **RemoveDryRunPlan** (+ PlannedRemoveOperation)
- **SyncDryRunPlan** (+ SyncSessionPlan)

## Key Findings

### Schema Patterns
All outputs follow a consistent pattern:
```rust
{
  success: bool,
  ... optional/required fields,
  #[serde(skip_serializing_if = "Option::is_none")]
  optional_field: Option<T>,
  error: Option<ErrorDetail>  // for error cases
}
```

### Common Traits
- **Success indicators**: All command outputs include `success: bool`
- **Optional error handling**: Most include `Option<ErrorDetail>`
- **Nested statistics**: File changes, diff stats, bead stats appear repeatedly
- **Flattening**: SessionStatusInfo flattens Session type with `#[serde(flatten)]`
- **Dynamic JSON**: Some fields use `serde_json::Value` (ConfigViewAllOutput)

### Validation Requirements
1. **Enum validation**: CheckStatus (Pass | Warn | Fail)
2. **String constraints**: Session names `^[a-zA-Z0-9_-]+$`
3. **Numeric bounds**: ID ranges, count validations
4. **Path validations**: PathBuf fields (backup_path, workspace_path)
5. **Conditional fields**: Many `Option<T>` fields with `skip_serializing_if`

## Implementation Roadmap

### Phase 1: Foundation & Enums (1 hour)
- Define CheckStatus enum
- Define ErrorDetail common structure
- Create base schema patterns

### Phase 2: Simple Output Types (1 hour)
- All 13 simple types

### Phase 3: Intermediate Nested Types (2 hours)
- All 10 intermediate complexity types

### Phase 4: Complex Planning Structures (1.5 hours)
- RemoveDryRunPlan + PlannedRemoveOperation
- SyncDryRunPlan + SyncSessionPlan

### Phase 5: Generic & Batch Operations (1 hour)
- BatchOperationOutput<T> parametric schema
- BatchItemResult<T>

### Phase 6: Status & List Responses (1 hour)
- StatusResponse + SessionStatusInfo
- SessionListResponse + SessionListItem

### Phase 7: Complex Composite Types (2.5 hours)
- PrimeOutput (6+ nested types)
- DoctorOutput with check system
- ContextOutput (5 levels deep)

### Phase 8: Integration & Testing (1 hour)
- Organize into schemas/json-outputs.cue
- Validate against real outputs
- Documentation

## Challenges & Mitigations

| Challenge | Mitigation |
|-----------|-----------|
| Generic BatchOperationOutput<T> | Use CUE parameter syntax or define concrete types |
| Flattened Session type | Define inline expansion in schema |
| Dynamic serde_json::Value fields | Schema as 'any' or document expected structure |
| Types spread across 8 files | Consolidate into single schemas/json-outputs.cue |
| Deep nesting (3-4 levels) | Clear hierarchical organization in schema |

## Output Structure Examples

### Simple (InitOutput)
```json
{
  "success": true,
  "message": "...",
  "zjj_dir": "...",
  "config_file": "...",
  "state_db": "...",
  "layouts_dir": "..."
}
```

### Intermediate (RemoveOutput)
```json
{
  "success": true,
  "session_name": "feature-x",
  "operations": [
    {"action": "delete_workspace", "path": "..."},
    {"action": "delete_zellij_tab", "tab": "..."}
  ],
  "closed_bead": "zjj-1234",
  "message": "..."
}
```

### Complex (PrimeOutput)
```json
{
  "jj_status": {...},
  "zjj_status": {...},
  "sessions": [...],
  "commands": {
    "session_lifecycle": [...],
    "workspace_sync": [...]
  },
  "beads_status": {...},
  "workflows": [...]
}
```

## Files Requiring Schema Definition

1. `crates/zjj/src/json_output.rs` - 24 types
2. `crates/zjj/src/commands/status/types.rs` - StatusInfo, helpers
3. `crates/zjj/src/commands/list/data/types.rs` - ListItem, helpers
4. `crates/zjj/src/commands/prime/output_types.rs` - PrimeOutput tree
5. `crates/zjj-core/src/introspection/doctor_types.rs` - DoctorOutput
6. `crates/zjj/src/commands/backup.rs` - BackupOutput
7. `crates/zjj/src/commands/context/types.rs` - ContextOutput tree
8. `schemas/json-outputs.cue` - NEW FILE (consolidation)

## Success Criteria

- [ ] All 27 primary output types have CUE schema definitions
- [ ] All 19 helper types are properly defined
- [ ] Generic BatchOperationOutput<T> supports parametric types
- [ ] All Option<T> fields correctly marked as optional
- [ ] Enums (CheckStatus) properly constrained
- [ ] Validation rules enforced (regex, bounds)
- [ ] Real JSON samples pass schema validation
- [ ] Documentation explains each output type's purpose
- [ ] CUE schemas integrate with existing architecture.cue
- [ ] No manual validation needed (CUE enforces all constraints)

## Recommendations

1. **Start with foundations**: Enums and simple types build confidence
2. **Group by complexity**: Tackle intermediate before complex
3. **Generic handling**: Decide on CUE parametric approach early
4. **Testing**: Collect real JSON outputs for validation
5. **Documentation**: Add bead/command metadata to schema comments
6. **Consolidation**: Move all to single schemas/json-outputs.cue for consistency
