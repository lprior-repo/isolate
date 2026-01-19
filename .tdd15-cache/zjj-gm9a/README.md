# Bead Triage Analysis: zjj-gm9a

**Bead**: zjj-gm9a  
**Title**: Create CUE schema for JSON outputs  
**Complexity**: HIGH  
**Estimated Effort**: 11.5 hours  
**Analysis Date**: 2026-01-18

## Files in This Analysis

### 1. **triage.json** (Recommended - Start Here)
Comprehensive JSON analysis containing:
- Complete output structure inventory (27 types)
- Nested helper types (19 types)
- Field-by-field breakdown of every output type
- Complexity assessment per type
- Implementation roadmap (8 phases)
- Challenge identification and mitigations
- Detailed effort estimates

**Usage**: Parse this for detailed schema requirements and implementation planning.

### 2. **ANALYSIS.md** (Executive Summary)
Markdown format analysis with:
- Executive summary with quick facts
- Complexity breakdown by type category
- Key findings and schema patterns
- Implementation roadmap overview
- Challenge matrix with mitigations
- Output structure examples
- Files requiring schema definition
- Success criteria checklist

**Usage**: Read this first for understanding the full scope.

### 3. **SUMMARY.txt** (Quick Reference)
ASCII formatted summary with:
- Quick facts in tabular format
- Output type distribution breakdown
- Complexity breakdown with effort hours
- Key characteristics and challenges
- Validation requirements
- Implementation phases with time estimates
- Files affected list
- Success criteria
- Challenges and mitigations
- Recommendations

**Usage**: Print/display for quick reference during implementation.

## Quick Facts

| Metric | Value |
|--------|-------|
| **Total Output Types** | 27 |
| **Nested Helper Types** | 19 |
| **Files Affected** | 8 |
| **Estimated Effort** | 11.5 hours |
| **Confidence Level** | HIGH |
| **Complexity Rating** | HIGH |

## Output Types by Category

### Simple Types (13 types, 1 hour)
InitOutput, AddOutput, FocusOutput, ConfigViewAllOutput, ConfigGetOutput, ConfigSetOutput, BackupOutput, VerifyBackupOutput, AgentInfo, AgentListOutput, ExampleHelp, ExitCodeHelp, ParameterHelp

### Intermediate Types (10 types, 2 hours)
RemoveOutput, RemoveOperation, DiffOutput, DiffStat, FileDiffStat, SyncOutput, HelpOutput, SubcommandHelp, FileChanges, DiffStats, BeadStats, SessionBeadInfo, SessionAgentInfo, CommandRef, CommandCategories

### Complex Types (3 types, 2.5 hours)
- **PrimeOutput**: 6+ nested types (JjStatus, ZjjStatus, SessionInfo, CommandCategories, BeadsStatus, WorkflowSection)
- **DoctorOutput**: Health check system with DoctorCheck and CheckStatus enum
- **ContextOutput**: 5-level deep environment context structure

### Generic Types (1 type, 1 hour)
- **BatchOperationOutput<T>** and **BatchItemResult<T>**: Parametric schema needed

### Response Wrappers (2 types, 1 hour)
- **StatusResponse**: Wraps SessionStatusInfo
- **SessionListResponse**: Wraps SessionListItem

### Planning Structures (2 types, 1.5 hours)
- **RemoveDryRunPlan**: Remove operation planning
- **SyncDryRunPlan**: Sync operation planning

## Key Challenges

1. **Generic Types**: BatchOperationOutput<T> needs parametric CUE support
2. **Flattened Structures**: SessionStatusInfo uses #[serde(flatten)]
3. **Dynamic JSON**: Some fields use serde_json::Value
4. **Scattered Types**: Output types across 8 different files
5. **Deep Nesting**: PrimeOutput (6+ types), ContextOutput (5 levels)

## Implementation Roadmap

| Phase | Title | Time | Items |
|-------|-------|------|-------|
| 1 | Foundation & Enums | 1.0h | CheckStatus, ErrorDetail |
| 2 | Simple Output Types | 1.0h | All 13 simple types |
| 3 | Intermediate Nested | 2.0h | All 10 intermediate types |
| 4 | Complex Planning | 1.5h | RemoveDryRunPlan, SyncDryRunPlan |
| 5 | Generic & Batch | 1.0h | BatchOperationOutput<T> |
| 6 | Status & List | 1.0h | StatusResponse, SessionListResponse |
| 7 | Complex Composites | 2.5h | PrimeOutput, DoctorOutput, ContextOutput |
| 8 | Integration & Testing | 1.0h | Consolidate & validate |
| **Total** | | **11.5h** | |

## Files to Schema

1. `crates/zjj/src/json_output.rs` - 24 types (main)
2. `crates/zjj/src/commands/status/types.rs` - 5 types
3. `crates/zjj/src/commands/list/data/types.rs` - 5 types
4. `crates/zjj/src/commands/prime/output_types.rs` - 7 types
5. `crates/zjj-core/src/introspection/doctor_types.rs` - 4 types
6. `crates/zjj/src/commands/backup.rs` - 2 types
7. `crates/zjj/src/commands/context/types.rs` - 5 types
8. `schemas/json-outputs.cue` - NEW FILE (consolidation)

## Validation Requirements

- **Enum**: CheckStatus (Pass | Warn | Fail)
- **String Patterns**: Session names `^[a-zA-Z0-9_-]+$`
- **Numeric Bounds**: Non-negative counts and IDs
- **Path Validation**: PathBuf fields (backup_path, workspace_path)
- **Optional Fields**: All Option<T> with skip_serializing_if

## Success Criteria

- [ ] All 27 primary output types have CUE schemas
- [ ] All 19 helper types properly defined
- [ ] Generic BatchOperationOutput<T> supports parametric types
- [ ] All Option<T> marked as optional
- [ ] CheckStatus enum constrained
- [ ] Validation rules enforced (regex, bounds)
- [ ] Real JSON samples pass validation
- [ ] CUE schemas integrated with architecture.cue

## Next Steps

1. **For detailed analysis**: Read `triage.json` or `ANALYSIS.md`
2. **For quick reference**: View `SUMMARY.txt`
3. **For implementation**: Follow 8-phase roadmap in `ANALYSIS.md`
4. **For validation**: Collect real JSON outputs from each command

---

Generated: 2026-01-18 using Codanna semantic code search
