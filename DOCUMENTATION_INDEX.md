# ZJJ Documentation Index

> **The ultimate navigation guide for all ZJJ documentation**

This index organizes all documentation files by purpose, audience, and topic. Each entry includes the file path, purpose, target audience, and key topics covered.

---

## Quick Navigation

| **I'm a...** | **Start Here** |
|-------------|---------------|
| New User | [README.md](#1-getting-started) |
| Developer | [CONTRIBUTING.md](#2-development-guides) |
| AI Agent | [AGENTS.md](#1-getting-started) |
| Architect | [ARCHITECTURE.md](#3-architecture-docs) |
| Migrating Code | [MIGRATION_GUIDE.md](#2-development-guides) |

---

## 1. Getting Started Documentation

**Audience:** New users, developers, and AI agents who need to understand what ZJJ is and how to use it.

### [README.md](/home/lewis/src/zjj/README.md)
**Purpose:** Project overview, quick start, and basic usage
**Audience:** Everyone
**Key Topics:**
- What ZJJ is and why it exists
- Mental model (sessions, queue, coordination)
- Quick start guide (60-second setup)
- Key commands reference
- Installation instructions
- Design philosophy (Functional Rust + DDD)
- Links to detailed documentation

### [AGENTS.md](/home/lewis/src/zjj/AGENTS.md)
**Purpose:** Single source of truth for AI agent development rules
**Audience:** AI agents, developers working with agents
**Key Topics:**
- Mandatory rules (NO_CLIPPY_EDITS, MOON_ONLY, ZERO_UNWRAP_PANIC)
- Workflow (IMPLEMENT → MANUAL_TEST → REVIEW → LAND)
- Functional Rust patterns
- Domain-Driven Design patterns
- Banned commands
- JSONL-formatted rules for programmatic access

---

## 2. Development Guides

**Audience:** Contributors and developers building ZJJ or extending it.

### Core Development

#### [CONTRIBUTING.md](/home/lewis/src/zjj/CONTRIBUTING.md)
**Purpose:** Contribution guidelines and development setup
**Audience:** Contributors
**Key Topics:**
- Development environment setup (automated and manual)
- Code style guidelines (Zero Unwrap Law, Functional Core, DDD)
- The Core 6 libraries (itertools, tap, rpds, thiserror, anyhow, futures-util)
- Testing requirements (unit, property, integration)
- Pull request process
- Common tasks (adding domain types, state machines, CLI commands)
- Code review checklist

### Domain-Driven Design

#### [DDD_QUICK_START.md](/home/lewis/src/zjj/DDD_QUICK_START.md)
**Purpose:** Quick reference for DDD patterns in ZJJ
**Audience:** Developers working with domain types
**Key Topics:**
- Semantic newtype wrappers
- Parse-once validation pattern
- Using domain types in function signatures
- Testing after migration

#### [DOMAIN_TYPES_GUIDE.md](/home/lewis/src/zjj/DOMAIN_TYPES_GUIDE.md)
**Purpose:** Comprehensive reference for all domain types
**Audience:** Developers working with the domain layer
**Key Topics:**
- Design principles (parse-at-boundaries, make illegal states unrepresentable)
- Identifier types (SessionName, AgentId, WorkspaceName, TaskId/BeadId, etc.)
- Value objects (DedupeKey, Priority, IssueId, Title, Description, etc.)
- State enums (AgentState, WorkspaceState, BranchState, ParentState, ClaimState, IssueState)
- Aggregates (Session, Workspace, Bead, QueueEntry, Agent)
- Domain events
- Repository interfaces
- Common patterns

#### [QUICK_REFERENCE.md](/home/lewis/src/zjj/QUICK_REFERENCE.md)
**Purpose:** Single-page cheat sheet for domain types
**Audience:** Developers (keep open while coding)
**Key Topics:**
- Parse pattern for all identifiers
- Constructor rules and validation
- State queries (active checks, terminal checks)
- Error helpers
- Key imports
- Common patterns (parse-validate-use, state transitions, builders)
- Serialization notes

#### [VALUE_OBJECTS.md](/home/lewis/src/zjj/VALUE_OBJECTS.md)
**Purpose:** Reference for value objects in the codebase
**Audience:** Developers creating or using value objects
**Key Topics:**
- What are value objects and why use them
- Text value objects (Message, PlanTitle, IssueTitle, Command)
- Action value objects (ActionVerb, ActionTarget)
- Warning value objects (WarningCode)
- State value objects (Outcome, RecoveryCapability, ExecutionMode, ActionResult)
- Metadata value objects (ValidatedMetadata)
- Priority value objects
- Deduplication value objects (DedupeKey)
- Creating new value objects
- Best practices

### Migration & Refactoring

#### [MIGRATION_GUIDE.md](/home/lewis/src/zjj/MIGRATION_GUIDE.md)
**Purpose:** Comprehensive guide for migrating to DDD architecture
**Audience:** Teams migrating existing code
**Key Topics:**
- Executive summary and breaking changes
- Migration patterns (parse-at-boundaries, use domain types, error conversion, state machines)
- Detailed migration steps (4 phases)
- Testing after migration
- Rollback strategies
- Common issues and solutions
- Complete type reference
- Quick reference cards

#### [MIGRATION_GUIDE_QUICK.md](/home/lewis/src/zjj/MIGRATION_GUIDE_QUICK.md)
**Purpose:** Quick reference for migration
**Audience:** Developers needing fast migration info
**Key Topics:**
- Essential migration steps
- Common patterns
- Troubleshooting

### Error Handling

#### [ERROR_HANDLING_GUIDE.md](/home/lewis/src/zjj/ERROR_HANDLING_GUIDE.md)
**Purpose:** Comprehensive guide to zero-panic error handling
**Audience:** All developers
**Key Topics:**
- Core principles (Zero-Panic, Zero-Unwrap, Railway-Oriented Programming)
- Error type hierarchy (IdentifierError → Aggregate Errors → RepositoryError → anyhow::Error)
- IdentifierError variants and usage
- Aggregate errors (SessionError, WorkspaceError, BeadError, QueueEntryError)
- RepositoryError and BuilderError
- Error conversion patterns
- Context preservation
- Recovery strategies
- Testing error cases
- Common pitfalls
- Best practices

### Testing

#### [TESTING_GUIDE.md](/home/lewis/src/zjj/TESTING_GUIDE.md)
**Purpose:** Comprehensive testing practices for the codebase
**Audience:** All developers
**Key Topics:**
- Testing philosophy (zero unwrap, property-based testing, test isolation)
- Test organization (directory structure, module-level tests)
- Writing unit tests (basic, async, table-driven)
- Property-based tests with proptest (custom strategies, invariant testing, JSON serialization)
- Integration tests (test harness, command execution, JSON output parsing, parallel execution)
- ATDD/BDD tests (Given-When-Then structure, step definitions)
- Benchmarks (template, running, categories)
- Test naming conventions
- Running tests (quick, property, integration, parallel)
- CI/CD integration
- Coverage requirements
- Common patterns
- Anti-patterns to avoid

---

## 3. Architecture Documentation

**Audience:** Architects and developers needing to understand system design.

### [ARCHITECTURE.md](/home/lewis/src/zjj/ARCHITECTURE.md)
**Purpose:** Complete system architecture overview
**Audience:** Architects, developers
**Key Topics:**
- System overview and core purpose
- Design philosophy (Functional Rust, DDD)
- Layer architecture (Shell vs Core, KIRK contracts)
- Module structure (crates, core modules)
- Data flow (command execution, error flow)
- Key design decisions (SQLite, async/await, semantic newtypes, JSONL output, KIRK contracts)
- Dependencies (core and shell)
- Extension points (adding commands, domain types, output types, quality gates, contracts)
- Architecture diagrams (high-level component, session lifecycle, queue processing, stack flow)
- Best practices for contributors

### [API_DOCUMENTATION.md](/home/lewis/src/zjj/API_DOCUMENTATION.md)
**Purpose:** API reference documentation
**Audience:** Developers using ZJJ as a library
**Key Topics:**
- Public API surface
- Module exports
- Function signatures
- Usage examples

---

## 4. Reports and Summaries

**Audience:** Project maintainers and contributors tracking progress and history.

### DDD Refactoring

#### [DDD_REFACTORING_REPORT.md](/home/lewis/src/zjj/DDD_REFACTORING_REPORT.md)
**Purpose:** Complete report on DDD refactoring effort
**Audience:** Maintainers, developers
**Key Topics:**
- Refactoring scope and goals
- Changes made (identifiers, state enums, error types)
- Migration guide
- Testing results
- Performance impact
- Lessons learned

#### [DDD_REFACTOR_SUMMARY.md](/home/lewis/src/zjj/DDD_REFACTOR_SUMMARY.md)
**Purpose:** Executive summary of DDD refactoring
**Audience:** Stakeholders
**Key Topics:**
- What changed
- Why it matters
- Impact on users

#### [DDD_FILES.md](/home/lewis/src/zjj/DDD_FILES.md)
**Purpose:** List of files affected by DDD refactoring
**Audience:** Developers
**Key Topics:**
- Modified files
- New files
- Deleted files

#### [DDD_AUDIT_REPORT.md](/home/lewis/src/zjj/DDD_AUDIT_REPORT.md)
**Purpose:** Audit of DDD implementation
**Audience:** Architects, maintainers
**Key Topics:**
- DDD principles compliance
- Gaps and issues
- Recommendations

#### [FINAL_REFACTOR_REPORT.md](/home/lewis/src/zjj/FINAL_REFACTOR_REPORT.md)
**Purpose:** Final report on all refactoring efforts
**Audience:** Maintainers
**Key Topics:**
- Complete refactoring summary
- All changes made
- Final state

### Code Examples

#### [CODE_EXAMPLES.md](/home/lewis/src/zjj/CODE_EXAMPLES.md)
**Purpose:** Example code patterns
**Audience:** Developers learning patterns
**Key Topics:**
- Common patterns
- Best practices
- Idiomatic code

#### [DDD_CODE_EXAMPLES.md](/home/lewis/src/zjj/DDD_CODE_EXAMPLES.md)
**Purpose:** DDD-specific code examples
**Audience:** Developers implementing DDD
**Key Topics:**
- Domain modeling examples
- Aggregate patterns
- Value object patterns

#### [EXAMPLES_DDD_REFACTOR.md](/home/lewis/src/zjj/EXAMPLES_DDD_REFACTOR.md)
**Purpose:** Before/after refactoring examples
**Audience:** Developers learning refactoring patterns
**Key Topics:**
- Before code (primitive obsession)
- After code (semantic types)
- Refactoring steps

#### [DOMAIN_EXAMPLES.md](/home/lewis/src/zjj/DOMAIN_EXAMPLES.md)
**Purpose:** Real-world domain examples
**Audience:** Developers
**Key Topics:**
- Session examples
- Queue examples
- Workspace examples

#### [CLI_EXAMPLES.md](/home/lewis/src/zjj/CLI_EXAMPLES.md)
**Purpose:** CLI usage examples
**Audience:** Users
**Key Topics:**
- Common workflows
- Command examples
- Output samples

### Error Handling Reports

#### [UNIFIED_ERROR_EXAMPLES.md](/home/lewis/src/zjj/UNIFIED_ERROR_EXAMPLES.md)
**Purpose:** Error handling examples
**Audience:** Developers
**Key Topics:**
- Error conversion examples
- Context preservation
- Recovery patterns

#### [UNIFIED_ERROR_TYPES_REPORT.md](/home/lewis/src/zjj/UNIFIED_ERROR_TYPES_REPORT.md)
**Purpose:** Report on unified error type system
**Audience:** Maintainers
**Key Topics:**
- Error type hierarchy
- Conversion patterns
- Implementation details

#### [ERROR_CONVERSION_GUIDE.md](/home/lewis/src/zjj/ERROR_CONVERSION_GUIDE.md)
**Purpose:** Guide for error conversion patterns
**Audience:** Developers
**Key Topics:**
- Converting between error types
- Adding context
- Recovery strategies

### CLI Contracts

#### [CLI_CONTRACTS_REFACTORING.md](/home/lewis/src/zjj/CLI_CONTRACTS_REFACTORING.md)
**Purpose:** CLI contracts refactoring documentation
**Audience:** Maintainers
**Key Topics:**
- KIRK design-by-contract pattern
- Contract implementation
- Refactoring details

#### [CLI_CONTRACTS_REFACTOR_SUMMARY.md](/home/lewis/src/zjj/CLI_CONTRACTS_REFACTOR_SUMMARY.md)
**Purpose:** Summary of CLI contracts refactoring
**Audience:** Stakeholders
**Key Topics:**
- What changed
- Why it matters

#### [CLI_CONTRACTS_HANDLER_EXAMPLES.md](/home/lewis/src/zjj/CLI_CONTRACTS_HANDLER_EXAMPLES.md)
**Purpose:** Handler examples with contracts
**Audience:** Developers
**Key Topics:**
- Contract usage in handlers
- Examples

#### [CLI_CONTRACTS_REFACTOR_FILES.md](/home/lewis/src/zjj/CLI_CONTRACTS_REFACTOR_FILES.md)
**Purpose:** Files changed in CLI contracts refactoring
**Audience:** Developers
**Key Topics:**
- Modified files
- New files

#### [CLI_CONTRACTS_REFACTOR_CHECKLIST.md](/home/lewis/src/zjj/CLI_CONTRACTS_REFACTOR_CHECKLIST.md)
**Purpose:** Checklist for CLI contracts refactoring
**Audience:** Developers
**Key Topics:**
- Migration steps
- Verification steps

#### [CLI_HANDLERS_REFACTOR_REPORT.md](/home/lewis/src/zjj/CLI_HANDLERS_REFACTOR_REPORT.md)
**Purpose:** Report on CLI handler refactoring
**Audience:** Maintainers
**Key Topics:**
- Changes made
- Patterns applied

### Beads Domain

#### [BEADS_DDD_SUMMARY.md](/home/lewis/src/zjj/BEADS_DDD_SUMMARY.md)
**Purpose:** Summary of beads DDD implementation
**Audience:** Stakeholders
**Key Topics:**
- Beads domain overview
- Key types and patterns

#### [BEADS_DDD_REFACTORING_REPORT.md](/home/lewis/src/zjj/BEADS_DDD_REFACTORING_REPORT.md)
**Purpose:** Detailed beads DDD refactoring report
**Audience:** Developers
**Key Topics:**
- Refactoring details
- Implementation patterns
- Examples

#### [BEADS_DDD_EXAMPLES.md](/home/lewis/src/zjj/BEADS_DDD_EXAMPLES.md)
**Purpose:** Beads domain examples
**Audience:** Developers
**Key Topics:**
- Bead lifecycle
- State transitions
- Validation

### Coordination Layer

#### [COORDINATION_REFACTOR_SUMMARY.md](/home/lewis/src/zjj/COORDINATION_REFACTOR_SUMMARY.md)
**Purpose:** Summary of coordination layer refactoring
**Audience:** Stakeholders
**Key Topics:**
- Queue refactoring
- Train processing changes

### Other Refactoring Reports

#### [FINAL_REFACTORING_REPORT.md](/home/lewis/src/zjj/FINAL_REFACTORING_REPORT.md)
**Purpose:** Final comprehensive refactoring report
**Audience:** Maintainers
**Key Topics:**
- All refactoring efforts
- Final state

#### [REFACTORING_SUMMARY.md](/home/lewis/src/zjj/REFACTORING_SUMMARY.md)
**Purpose:** Summary of all refactoring work
**Audience:** Stakeholders
**Key Topics:**
- High-level overview
- Impact assessment

#### [REFACTORING_AT_A_GLANCE.md](/home/lewis/src/zjj/REFACTORING_AT_A_GLANCE.md)
**Purpose:** Quick refactoring overview
**Audience:** Busy stakeholders
**Key Topics:**
- One-page summary
- Key changes

#### [REFACTORING_ARCHITECTURE.md](/home/lewis/src/zjj/REFACTORING_ARCHITECTURE.md)
**Purpose:** Architecture changes from refactoring
**Audience:** Architects
**Key Topics:**
- Before/after architecture
- Migration path

#### [REFACTORING_INDEX.md](/home/lewis/src/zjj/REFACTORING_INDEX.md)
**Purpose:** Index of refactoring documentation
**Audience:** Everyone
**Key Topics:**
- Links to all refactoring docs
- Organization

#### [REFACTORING_CHECKLIST.md](/home/lewis/src/zjj/REFACTORING_CHECKLIST.md)
**Purpose:** Checklist for refactoring tasks
**Audience:** Developers
**Key Topics:**
- Step-by-step tasks
- Verification steps

### Testing Reports

#### [TEST_REPORT.md](/home/lewis/src/zjj/TEST_REPORT.md)
**Purpose:** Comprehensive test report
**Audience:** Maintainers
**Key Topics:**
- Test coverage
- Test results
- Gaps and recommendations

#### [DOMAIN_TEST_COVERAGE_SUMMARY.md](/home/lewis/src/zjj/DOMAIN_TEST_COVERAGE_SUMMARY.md)
**Purpose:** Domain layer test coverage
**Audience:** Developers
**Key Topics:**
- Coverage by module
- Missing tests
- Recommendations

#### [TEST_UNWRAP_IMPROVEMENTS.md](/home/lewis/src/zjj/TEST_UNWRAP_IMPROVEMENTS.md)
**Purpose:** Report on removing unwrap from tests
**Audience:** Developers
**Key Topics:**
- Zero-unwrap test patterns
- Improvements made

#### [TEST_UNWRAP_REFACTORING_SUMMARY.md](/home/lewis/src/zjj/TEST_UNWRAP_REFACTORING_SUMMARY.md)
**Purpose:** Summary of test unwrap refactoring
**Audience:** Stakeholders
**Key Topics:**
- What changed
- Impact

#### [CLI_PROPERTY_TESTS_REPORT.md](/home/lewis/src/zjj/CLI_PROPERTY_TESTS_REPORT.md)
**Purpose:** CLI property test report
**Audience:** Developers
**Key Topics:**
- Property tests for CLI
- Invariants tested
- Results

#### [CLI_REGISTRATION_REPORT.md](/home/lewis/src/zjj/CLI_REGISTRATION_REPORT.md)
**Purpose:** CLI command registration report
**Audience:** Developers
**Key Topics:**
- Command registration
- Commands available

#### [CONFIG_OBJECT_BEAD_REPORT.md](/home/lewis/src/zjj/CONFIG_OBJECT_BEAD_REPORT.md)
**Purpose:** Config object/bead integration report
**Audience:** Developers
**Key Topics:**
- Config changes
- Bead integration

### Validation & Invariants

#### [VALIDATION_API_REFERENCE.md](/home/lewis/src/zjj/VALIDATION_API_REFERENCE.md)
**Purpose:** Validation API reference
**Audience:** Developers
**Key Topics:**
- Validation functions
- API usage
- Examples

#### [VALIDATION_REFACTOR_REPORT.md](/home/lewis/src/zjj/VALIDATION_REFACTOR_REPORT.md)
**Purpose:** Validation refactoring report
**Audience:** Developers
**Key Topics:**
- Validation changes
- Patterns applied

#### [INVARIANT_MACROS_REPORT.md](/home/lewis/src/zjj/INVARIANT_MACROS_REPORT.md)
**Purpose:** Invariant macros report
**Audience:** Developers
**Key Topics:**
- Macro usage
- Examples

#### [INVARIANT_MACROS_USAGE.md](/home/lewis/src/zjj/INVARIANT_MACROS_USAGE.md)
**Purpose:** Guide for using invariant macros
**Audience:** Developers
**Key Topics:**
- How to use macros
- Best practices

#### [BEADS_INVARIANT_TESTS_REPORT.md](/home/lewis/src/zjj/BEADS_INVARIANT_TESTS_REPORT.md)
**Purpose:** Beads invariant test report
**Audience:** Developers
**Key Topics:**
- Invariants tested
- Test coverage

### Build & Benchmarks

#### [CONST_FN_REPORT.md](/home/lewis/src/zjj/CONST_FN_REPORT.md)
**Purpose:** Const fn implementation report
**Audience:** Developers
**Key Topics:**
- Const fn changes
- Performance impact

#### [BENCHMARKS_REPORT.md](/home/lewis/src/zjj/BENCHMARKS_REPORT.md)
**Purpose:** Performance benchmark report
**Audience:** Maintainers
**Key Topics:**
- Benchmark results
- Performance analysis

#### [BENCHMARKS_IMPLEMENTATION_REPORT.md](/home/lewis/src/zjj/BENCHMARKS_IMPLEMENTATION_REPORT.md)
**Purpose:** Benchmark implementation details
**Audience:** Developers
**Key Topics:**
- How benchmarks work
- Adding new benchmarks

### Migration Completion

#### [MIGRATION_COMPLETE_REPORT.md](/home/lewis/src/zjj/MIGRATION_COMPLETE_REPORT.md)
**Purpose:** Report on completed migration
**Audience:** Stakeholders
**Key Topics:**
- Migration status
- What was completed

#### [DEV_SETUP_SCRIPT_SUMMARY.md](/home/lewis/src/zjj/DEV_SETUP_SCRIPT_SUMMARY.md)
**Purpose:** Dev setup script documentation
**Audience:** New contributors
**Key Topics:**
- Script usage
- Automated setup

### Consolidation Reports

#### [BEADID_CONSOLIDATION_REPORT.md](/home/lewis/src/zjj/BEADID_CONSOLIDATION_REPORT.md)
**Purpose:** BeadId consolidation report
**Audience:** Developers
**Key Topics:**
- Consolidation details
- Breaking changes

#### [BEADID_CONSOLIDATION_SUMMARY.md](/home/lewis/src/zjj/BEADID_CONSOLIDATION_SUMMARY.md)
**Purpose:** Summary of BeadId consolidation
**Audience:** Stakeholders
**Key Topics:**
- What changed
- Why it matters

#### [QUEUE_ENTRY_ID_CONSOLIDATION.md](/home/lewis/src/zjj/QUEUE_ENTRY_ID_CONSOLIDATION.md)
**Purpose:** Queue entry ID consolidation
**Audience:** Developers
**Key Topics:**
- ID consolidation
- Changes made

#### [SESSION_NAME_CONSOLIDATION.md](/home/lewis/src/zjj/SESSION_NAME_CONSOLIDATION.md)
**Purpose:** Session name consolidation
**Audience:** Developers
**Key Topics:**
- Consolidation details
- Breaking changes

#### [SESSIONNAME_MIGRATION_SUMMARY.md](/home/lewis/src/zjj/SESSIONNAME_MIGRATION_SUMMARY.md)
**Purpose:** SessionName migration summary
**Audience:** Stakeholders
**Key Topics:**
- Migration status
- Impact

#### [SESSIONNAME_QUICK_REFERENCE.md](/home/lewis/src/zjj/SESSIONNAME_QUICK_REFERENCE.md)
**Purpose:** SessionName quick reference
**Audience:** Developers
**Key Topics:**
- Usage patterns
- Common issues

#### [WORKSPACENAME_CONSOLIDATION_REPORT.md](/home/lewis/src/zjj/WORKSPACENAME_CONSOLIDATION_REPORT.md)
**Purpose:** Workspace name consolidation
**Audience:** Developers
**Key Topics:**
- Consolidation details
- Changes

### Session Name Migration

#### [SESSION_NAME_MIGRATION_PLAN.md](/home/lewis/src/zjj/SESSION_NAME_MIGRATION_PLAN.md)
**Purpose:** Session name migration plan
**Audience:** Maintainers
**Key Topics:**
- Migration strategy
- Timeline

#### [SESSION_NAME_MIGRATION_COMPLETE.md](/home/lewis/src/zjj/SESSION_NAME_MIGRATION_COMPLETE.md)
**Purpose:** Session name migration completion
**Audience:** Stakeholders
**Key Topics:**
- Completion status
- Results

#### [SESSION_NAME_PHASE1_REPORT.md](/home/lewis/src/zjj/SESSION_NAME_PHASE1_REPORT.md)
**Purpose:** Phase 1 migration report
**Audience:** Maintainers
**Key Topics:**
- Phase 1 changes
- Results

#### [SESSION_NAME_MIGRATION_ANALYSIS.md](/home/lewis/src/zjj/SESSION_NAME_MIGRATION_ANALYSIS.md)
**Purpose:** Session name migration analysis
**Audience:** Architects
**Key Topics:**
- Analysis of migration
- Recommendations

### Integration & JSONL

#### [INTEGRATION_TESTS_SUMMARY.md](/home/lewis/src/zjj/INTEGRATION_TESTS_SUMMARY.md)
**Purpose:** Integration test summary
**Audience:** Developers
**Key Topics:**
- Test coverage
- Test patterns

#### [JSONL_SCHEMA_VALIDATION_REPORT.md](/home/lewis/src/zjj/JSONL_SCHEMA_VALIDATION_REPORT.md)
**Purpose:** JSONL schema validation report
**Audience:** Developers
**Key Topics:**
- Schema validation
- Compliance

#### [SERDE_VALIDATION_TESTS_SUMMARY.md](/home/lewis/src/zjj/SERDE_VALIDATION_TESTS_SUMMARY.md)
**Purpose:** Serde validation test summary
**Audience:** Developers
**Key Topics:**
- Serialization tests
- Validation patterns

#### [BOUNDED_TYPES_GUIDE.md](/home/lewis/src/zjj/BOUNDED_TYPES_GUIDE.md)
**Purpose:** Guide for bounded types
**Audience:** Developers
**Key Topics:**
- Type boundaries
- Validation rules

### Status & Issues

#### [STATUS_RED_PHASE_REPORT.md](/home/lewis/src/zjj/STATUS_RED_PHASE_REPORT.md)
**Purpose:** Status red phase report
**Audience:** Maintainers
**Key Topics:**
- Issues found
- Resolution

#### [FINAL_REVIEW_CHECKLIST.md](/home/lewis/src/zjj/FINAL_REVIEW_CHECKLIST.md)
**Purpose:** Final review checklist
**Audience:** Maintainers
**Key Topics:**
- Review items
- Sign-off

### Additional Guides

#### [BUILDERS_DOCUMENTATION.md](/home/lewis/src/zjj/BUILDERS_DOCUMENTATION.md)
**Purpose:** Builder pattern documentation
**Audience:** Developers
**Key Topics:**
- Builder patterns
- Type-safe builders
- Examples

#### [CODE_REVIEW_CHECKLIST.md](/home/lewis/src/zjj/CODE_REVIEW_CHECKLIST.md)
**Purpose:** Code review checklist
**Audience:** Reviewers
**Key Topics:**
- Review items
- Quality gates

#### [STATE_MACHINES.md](/home/lewis/src/zjj/STATE_MACHINES.md)
**Purpose:** State machine reference
**Audience:** Developers
**Key Topics:**
- All state machines
- Transitions
- Validation

#### [TROUBLESHOOTING.md](/home/lewis/src/zjj/TROUBLESHOOTING.md)
**Purpose:** Troubleshooting guide
**Audience:** Users and developers
**Key Topics:**
- Common issues
- Solutions
- Debugging

#### [CHANGELOG_ENTRY.md](/home/lewis/src/zjj/CHANGELOG_ENTRY.md)
**Purpose:** Changelog entry template
**Audience:** Maintainers
**Key Topics:**
- Entry format
- Examples

#### [RELEASE_NOTES_TEMPLATE.md](/home/lewis/src/zjj/RELEASE_NOTES_TEMPLATE.md)
**Purpose:** Release notes template
**Audience:** Maintainers
**Key Topics:**
- Release notes format
- Sections

---

## 5. Historical Records

**Audience:** Historians and maintainers tracking project evolution.

### Complete Refactoring History
The following documents track the complete refactoring journey from primitive types to DDD architecture:

1. **[REFACTORING_INDEX.md](/home/lewis/src/zjj/REFACTORING_INDEX.md)** - Master index
2. **[REFACTORING_AT_A_GLANCE.md](/home/lewis/src/zjj/REFACTORING_AT_A_GLANCE.md)** - Quick overview
3. **[FINAL_REFACTORING_REPORT.md](/home/lewis/src/zjj/FINAL_REFACTORING_REPORT.md)** - Final state
4. **[DDD_REFACTORING_REPORT.md](/home/lewis/src/zjj/DDD_REFACTORING_REPORT.md)** - DDD refactoring
5. **[MIGRATION_COMPLETE_REPORT.md](/home/lewis/src/zjj/MIGRATION_COMPLETE_REPORT.md)** - Migration completion

### Consolidation History
- **[BEADID_CONSOLIDATION_REPORT.md](/home/lewis/src/zjj/BEADID_CONSOLIDATION_REPORT.md)** - BeadId consolidation
- **[QUEUE_ENTRY_ID_CONSOLIDATION.md](/home/lewis/src/zjj/QUEUE_ENTRY_ID_CONSOLIDATION.md)** - Queue entry ID
- **[SESSION_NAME_CONSOLIDATION.md](/home/lewis/src/zjj/SESSION_NAME_CONSOLIDATION.md)** - Session name
- **[WORKSPACENAME_CONSOLIDATION_REPORT.md](/home/lewis/src/zjj/WORKSPACENAME_CONSOLIDATION_REPORT.md)** - Workspace name

---

## 6. Documentation by Topic

### Domain Types
- **[DOMAIN_TYPES_GUIDE.md](/home/lewis/src/zjj/DOMAIN_TYPES_GUIDE.md)** - Complete reference
- **[QUICK_REFERENCE.md](/home/lewis/src/zjj/QUICK_REFERENCE.md)** - Cheat sheet
- **[VALUE_OBJECTS.md](/home/lewis/src/zjj/VALUE_OBJECTS.md)** - Value objects
- **[DDD_QUICK_START.md](/home/lewis/src/zjj/DDD_QUICK_START.md)** - Quick start

### Error Handling
- **[ERROR_HANDLING_GUIDE.md](/home/lewis/src/zjj/ERROR_HANDLING_GUIDE.md)** - Complete guide
- **[ERROR_CONVERSION_GUIDE.md](/home/lewis/src/zjj/ERROR_CONVERSION_GUIDE.md)** - Conversion patterns
- **[UNIFIED_ERROR_EXAMPLES.md](/home/lewis/src/zjj/UNIFIED_ERROR_EXAMPLES.md)** - Examples
- **[UNIFIED_ERROR_TYPES_REPORT.md](/home/lewis/src/zjj/UNIFIED_ERROR_TYPES_REPORT.md)** - Type system

### Testing
- **[TESTING_GUIDE.md](/home/lewis/src/zjj/TESTING_GUIDE.md)** - Complete guide
- **[TEST_REPORT.md](/home/lewis/src/zjj/TEST_REPORT.md)** - Test report
- **[DOMAIN_TEST_COVERAGE_SUMMARY.md](/home/lewis/src/zjj/DOMAIN_TEST_COVERAGE_SUMMARY.md)** - Coverage
- **[TEST_UNWRAP_IMPROVEMENTS.md](/home/lewis/src/zjj/TEST_UNWRAP_IMPROVEMENTS.md)** - Zero unwrap

### Migration
- **[MIGRATION_GUIDE.md](/home/lewis/src/zjj/MIGRATION_GUIDE.md)** - Complete guide
- **[MIGRATION_GUIDE_QUICK.md](/home/lewis/src/zjj/MIGRATION_GUIDE_QUICK.md)** - Quick reference
- **[MIGRATION_COMPLETE_REPORT.md](/home/lewis/src/zjj/MIGRATION_COMPLETE_REPORT.md)** - Completion report

### Architecture
- **[ARCHITECTURE.md](/home/lewis/src/zjj/ARCHITECTURE.md)** - Complete architecture
- **[REFACTORING_ARCHITECTURE.md](/home/lewis/src/zjj/REFACTORING_ARCHITECTURE.md)** - Refactoring impact

### State Machines
- **[STATE_MACHINES.md](/home/lewis/src/zjj/STATE_MACHINES.md)** - Reference
- **[DOMAIN_TYPES_GUIDE.md](/home/lewis/src/zjj/DOMAIN_TYPES_GUIDE.md)** - State enums section

### Examples
- **[CODE_EXAMPLES.md](/home/lewis/src/zjj/CODE_EXAMPLES.md)** - General examples
- **[DDD_CODE_EXAMPLES.md](/home/lewis/src/zjj/DDD_CODE_EXAMPLES.md)** - DDD examples
- **[DOMAIN_EXAMPLES.md](/home/lewis/src/zjj/DOMAIN_EXAMPLES.md)** - Domain examples
- **[CLI_EXAMPLES.md](/home/lewis/src/zjj/CLI_EXAMPLES.md)** - CLI examples
- **[EXAMPLES_DDD_REFACTOR.md](/home/lewis/src/zjj/EXAMPLES_DDD_REFACTOR.md)** - Refactoring examples

---

## 7. Documentation by Audience

### For New Users
1. **[README.md](/home/lewis/src/zjj/README.md)** - Start here
2. **[CLI_EXAMPLES.md](/home/lewis/src/zjj/CLI_EXAMPLES.md)** - Usage examples
3. **[TROUBLESHOOTING.md](/home/lewis/src/zjj/TROUBLESHOOTING.md)** - Common issues

### For Developers
1. **[CONTRIBUTING.md](/home/lewis/src/zjj/CONTRIBUTING.md)** - Development setup
2. **[DOMAIN_TYPES_GUIDE.md](/home/lewis/src/zjj/DOMAIN_TYPES_GUIDE.md)** - Domain types
3. **[QUICK_REFERENCE.md](/home/lewis/src/zjj/QUICK_REFERENCE.md)** - Cheat sheet
4. **[ERROR_HANDLING_GUIDE.md](/home/lewis/src/zjj/ERROR_HANDLING_GUIDE.md)** - Error handling
5. **[TESTING_GUIDE.md](/home/lewis/src/zjj/TESTING_GUIDE.md)** - Testing

### For AI Agents
1. **[AGENTS.md](/home/lewis/src/zjj/AGENTS.md)** - Single source of truth
2. **[CLAUDE.md](/home/lewis/src/zjj/CLAUDE.md)** - Functional Rust expert
3. **[QUICK_REFERENCE.md](/home/lewis/src/zjj/QUICK_REFERENCE.md)** - Quick lookup

### For Architects
1. **[ARCHITECTURE.md](/home/lewis/src/zjj/ARCHITECTURE.md)** - System design
2. **[DDD_QUICK_START.md](/home/lewis/src/zjj/DDD_QUICK_START.md)** - DDD patterns
3. **[REFACTORING_ARCHITECTURE.md](/home/lewis/src/zjj/REFACTORING_ARCHITECTURE.md)** - Architecture evolution

### For Maintainers
1. **[FINAL_REVIEW_CHECKLIST.md](/home/lewis/src/zjj/FINAL_REVIEW_CHECKLIST.md)** - Review
2. **[RELEASE_NOTES_TEMPLATE.md](/home/lewis/src/zjj/RELEASE_NOTES_TEMPLATE.md)** - Releases
3. **[REFACTORING_INDEX.md](/home/lewis/src/zjj/REFACTORING_INDEX.md)** - Refactoring history

---

## 8. Documentation by File Size

### Quick Reference (1-5 pages)
- **[QUICK_REFERENCE.md](/home/lewis/src/zjj/QUICK_REFERENCE.md)** - 1 page cheat sheet
- **[AGENTS.md](/home/lewis/src/zjj/AGENTS.md)** - 1 page rules
- **[DDD_QUICK_START.md](/home/lewis/src/zjj/DDD_QUICK_START.md)** - 2 pages
- **[REFACTORING_AT_A_GLANCE.md](/home/lewis/src/zjj/REFACTORING_AT_A_GLANCE.md)** - 1 page

### Medium Length (5-20 pages)
- **[CONTRIBUTING.md](/home/lewis/src/zjj/CONTRIBUTING.md)** - 15 pages
- **[ERROR_HANDLING_GUIDE.md](/home/lewis/src/zjj/ERROR_HANDLING_GUIDE.md)** - 20 pages
- **[VALUE_OBJECTS.md](/home/lewis/src/zjj/VALUE_OBJECTS.md)** - 15 pages
- **[TESTING_GUIDE.md](/home/lewis/src/zjj/TESTING_GUIDE.md)** - 20 pages

### Comprehensive (20+ pages)
- **[DOMAIN_TYPES_GUIDE.md](/home/lewis/src/zjj/DOMAIN_TYPES_GUIDE.md)** - 30 pages
- **[ARCHITECTURE.md](/home/lewis/src/zjj/ARCHITECTURE.md)** - 25 pages
- **[MIGRATION_GUIDE.md](/home/lewis/src/zjj/MIGRATION_GUIDE.md)** - 25 pages
- **[README.md](/home/lewis/src/zjj/README.md)** - 25 pages

---

## 9. Key Concept Index

### DDD Concepts
- **Aggregates**: [DOMAIN_TYPES_GUIDE.md](/home/lewis/src/zjj/DOMAIN_TYPES_GUIDE.md)
- **Value Objects**: [VALUE_OBJECTS.md](/home/lewis/src/zjj/VALUE_OBJECTS.md)
- **Domain Events**: [DOMAIN_TYPES_GUIDE.md](/home/lewis/src/zjj/DOMAIN_TYPES_GUIDE.md)
- **Repositories**: [DOMAIN_TYPES_GUIDE.md](/home/lewis/src/zjj/DOMAIN_TYPES_GUIDE.md)
- **Bounded Contexts**: [ARCHITECTURE.md](/home/lewis/src/zjj/ARCHITECTURE.md)

### Functional Rust
- **Zero Unwrap**: [ERROR_HANDLING_GUIDE.md](/home/lewis/src/zjj/ERROR_HANDLING_GUIDE.md)
- **Pure Functions**: [ARCHITECTURE.md](/home/lewis/src/zjj/ARCHITECTURE.md)
- **Iterator Pipelines**: [CONTRIBUTING.md](/home/lewis/src/zjj/CONTRIBUTING.md)
- **The Core 6**: [CONTRIBUTING.md](/home/lewis/src/zjj/CONTRIBUTING.md)

### State Machines
- **AgentState**: [DOMAIN_TYPES_GUIDE.md](/home/lewis/src/zjj/DOMAIN_TYPES_GUIDE.md)
- **WorkspaceState**: [DOMAIN_TYPES_GUIDE.md](/home/lewis/src/zjj/DOMAIN_TYPES_GUIDE.md)
- **ClaimState**: [DOMAIN_TYPES_GUIDE.md](/home/lewis/src/zjj/DOMAIN_TYPES_GUIDE.md)
- **IssueState**: [DOMAIN_TYPES_GUIDE.md](/home/lewis/src/zjj/DOMAIN_TYPES_GUIDE.md)
- **All State Machines**: [STATE_MACHINES.md](/home/lewis/src/zjj/STATE_MACHINES.md)

### Error Handling
- **IdentifierError**: [ERROR_HANDLING_GUIDE.md](/home/lewis/src/zjj/ERROR_HANDLING_GUIDE.md)
- **RepositoryError**: [ERROR_HANDLING_GUIDE.md](/home/lewis/src/zjj/ERROR_HANDLING_GUIDE.md)
- **Error Conversion**: [ERROR_CONVERSION_GUIDE.md](/home/lewis/src/zjj/ERROR_CONVERSION_GUIDE.md)

### Testing
- **Unit Tests**: [TESTING_GUIDE.md](/home/lewis/src/zjj/TESTING_GUIDE.md)
- **Property Tests**: [TESTING_GUIDE.md](/home/lewis/src/zjj/TESTING_GUIDE.md)
- **Integration Tests**: [TESTING_GUIDE.md](/home/lewis/src/zjj/TESTING_GUIDE.md)
- **Zero Unwrap Tests**: [TEST_UNWRAP_IMPROVEMENTS.md](/home/lewis/src/zjj/TEST_UNWRAP_IMPROVEMENTS.md)

---

## 10. Quick Start Paths

### Path A: I want to contribute code
1. Read [CONTRIBUTING.md](/home/lewis/src/zjj/CONTRIBUTING.md)
2. Read [QUICK_REFERENCE.md](/home/lewis/src/zjj/QUICK_REFERENCE.md)
3. Read [ERROR_HANDLING_GUIDE.md](/home/lewis/src/zjj/ERROR_HANDLING_GUIDE.md)
4. Read [TESTING_GUIDE.md](/home/lewis/src/zjj/TESTING_GUIDE.md)

### Path B: I want to understand the architecture
1. Read [README.md](/home/lewis/src/zjj/README.md)
2. Read [ARCHITECTURE.md](/home/lewis/src/zjj/ARCHITECTURE.md)
3. Read [DOMAIN_TYPES_GUIDE.md](/home/lewis/src/zjj/DOMAIN_TYPES_GUIDE.md)
4. Read [STATE_MACHINES.md](/home/lewis/src/zjj/STATE_MACHINES.md)

### Path C: I want to migrate existing code
1. Read [MIGRATION_GUIDE.md](/home/lewis/src/zjj/MIGRATION_GUIDE.md)
2. Read [DOMAIN_TYPES_GUIDE.md](/home/lewis/src/zjj/DOMAIN_TYPES_GUIDE.md)
3. Read [ERROR_CONVERSION_GUIDE.md](/home/lewis/src/zjj/ERROR_CONVERSION_GUIDE.md)
4. Read [EXAMPLES_DDD_REFACTOR.md](/home/lewis/src/zjj/EXAMPLES_DDD_REFACTOR.md)

### Path D: I'm an AI agent
1. Read [AGENTS.md](/home/lewis/src/zjj/AGENTS.md) (single source of truth)
2. Read [CLAUDE.md](/home/lewis/src/zjj/CLAUDE.md)
3. Keep [QUICK_REFERENCE.md](/home/lewis/src/zjj/QUICK_REFERENCE.md) open
4. Read [TESTING_GUIDE.md](/home/lewis/src/zjj/TESTING_GUIDE.md)

### Path E: I want to use ZJJ
1. Read [README.md](/home/lewis/src/zjj/README.md) (Quick Start section)
2. Read [CLI_EXAMPLES.md](/home/lewis/src/zjj/CLI_EXAMPLES.md)
3. Keep [TROUBLESHOOTING.md](/home/lewis/src/zjj/TROUBLESHOOTING.md) handy

---

## 11. External Documentation

### mdBook Site
- **Full Documentation**: https://lprior-repo.github.io/zjj/
- **AI Agent Guide**: [docs/AI_AGENT_GUIDE.md](/home/lewis/src/zjj/docs/AI_AGENT_GUIDE.md)
- **Docs Index**: [docs/INDEX.md](/home/lewis/src/zjj/docs/INDEX.md)

### Related Resources
- **Rust Book**: https://doc.rust-lang.org/book/
- **Rust by Example**: https://doc.rust-lang.org/rust-by-example/
- **DDD Reference**: https://www.domainlanguage.com/ddd/reference/
- **Proptest Book**: https://altsysrq.github.io/proptest-book/

---

## 12. Documentation Standards

All ZJJ documentation follows these standards:

### File Headers
Every documentation file should have:
- Clear purpose statement
- Target audience
- Last updated date
- Table of contents (for long files)

### Code Examples
All code examples must:
- Follow zero-unwrap principles
- Use Result<T, E> patterns
- Include error handling
- Be tested and working

### Structure
- Use consistent heading levels (H1, H2, H3)
- Include table of contents for files >10 pages
- Use bullet points for lists
- Include code blocks with syntax highlighting
- Add "See Also" sections for related docs

### Maintenance
- Update when code changes
- Keep examples synchronized with codebase
- Review quarterly for accuracy
- Archive historical docs appropriately

---

## 13. Contributing to Documentation

### Adding New Documentation

When adding new documentation:

1. **Check if it exists first** - Search this index
2. **Choose the right location** - Based on audience and purpose
3. **Follow standards** - File headers, structure, examples
4. **Update this index** - Add entry in appropriate section
5. **Link from related docs** - Cross-reference

### Documentation File Naming

Use these naming patterns:
- `*_GUIDE.md` - Comprehensive guides
- `*_QUICK.md` - Quick reference
- `*_EXAMPLES.md` - Code examples
- `*_REPORT.md` - Formal reports
- `*_SUMMARY.md` - Executive summaries
- `*_CHECKLIST.md` - Task checklists
- `*_TEMPLATE.md` - Document templates

### Documentation Review Checklist

Before submitting documentation:
- [ ] Purpose is clear
- [ ] Target audience is defined
- [ ] Table of contents (if >10 pages)
- [ ] Code examples are tested
- [ ] Cross-references are accurate
- [ ] Added to this index
- [ ] Follows markdown standards

---

## 14. Index Maintenance

This index is maintained as part of the documentation ecosystem:

### When to Update
- When new documentation is added
- When documentation is significantly updated
- When documentation is renamed or moved
- Quarterly review for accuracy

### How to Update
1. Add new entries to appropriate section
2. Update cross-references
3. Verify all links work
4. Update "Last Updated" date

### Last Updated
**Date**: 2026-02-23
**Maintainer**: Documentation team
**Review Frequency**: Quarterly

---

## Appendix A: Complete File Listing

For a complete alphabetical listing of all documentation files, see the repository root. Key documentation files include:

```
/home/lewis/src/zjj/
├── README.md
├── AGENTS.md
├── ARCHITECTURE.md
├── CONTRIBUTING.md
├── DOCUMENTATION_INDEX.md (this file)
├── QUICK_REFERENCE.md
├── DOMAIN_TYPES_GUIDE.md
├── ERROR_HANDLING_GUIDE.md
├── TESTING_GUIDE.md
├── MIGRATION_GUIDE.md
├── VALUE_OBJECTS.md
├── STATE_MACHINES.md
├── DDD_QUICK_START.md
├── CLI_EXAMPLES.md
├── CODE_EXAMPLES.md
├── TROUBLESHOOTING.md
└── [many more reports and guides...]
```

---

## Appendix B: Documentation Metrics

As of 2026-02-23:

- **Total Documentation Files**: 100+
- **Getting Started Docs**: 2 files
- **Development Guides**: 15 files
- **Architecture Docs**: 2 files
- **Reports & Summaries**: 80+ files
- **Historical Records**: 10+ files

---

**Need Help?**

- **Quick Question**: Check [QUICK_REFERENCE.md](/home/lewis/src/zjj/QUICK_REFERENCE.md)
- **Error?:** Check [TROUBLESHOOTING.md](/home/lewis/src/zjj/TROUBLESHOOTING.md)
- **Contributing?**: Check [CONTRIBUTING.md](/home/lewis/src/zjj/CONTRIBUTING.md)
- **Architecture?**: Check [ARCHITECTURE.md](/home/lewis/src/zjj/ARCHITECTURE.md)

---

*This index is maintained as part of the ZJJ documentation ecosystem. Last updated: 2026-02-23*
