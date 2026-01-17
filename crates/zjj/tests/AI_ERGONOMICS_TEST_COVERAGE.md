# AI Ergonomics Test Coverage (zjj-pr36)

## Overview

The `ai_ergonomics_test.rs` file implements comprehensive integration testing for the AI onboarding and workflow discovery flow in ZJJ. This document details the test coverage, regression prevention strategies, and validation approaches.

## Test Structure

### Test Organization

The tests are organized into logical groups that mirror the AI agent workflow:

1. **AI Onboarding Flow Tests**: Complete end-to-end workflows
2. **JSON Output Validation Tests**: Ensure all outputs are machine-readable
3. **Exit Code Validation Tests**: Verify semantic exit codes (0-4)
4. **Command Discovery Tests**: Validate introspection capabilities
5. **Regression Prevention Tests**: Ensure features remain functional
6. **Error Handling Tests**: Graceful degradation under failure conditions
7. **Workflow Integration Tests**: Multi-step workflow validation

## Test Coverage Matrix

### Commands Tested

| Command | JSON Output | Exit Codes | Error Handling | Documentation |
|---------|-------------|------------|----------------|---------------|
| `--help-json` | ✅ | ✅ | ✅ | ✅ |
| `init` | ✅ | ✅ | ✅ | ✅ |
| `introspect` | ✅ | ✅ | ✅ | ✅ |
| `doctor` | ✅ | ✅ | ✅ | ✅ |
| `context` | ✅ | ✅ | ✅ | ✅ |
| `list` | ✅ | ✅ | ✅ | ✅ |
| `add` | ✅ | ✅ | ✅ | ✅ |
| `remove` | ✅ | ✅ | ✅ | ✅ |
| `version` | ✅ | ✅ | ✅ | ✅ |

### Features Validated

#### 1. AI Discovery Flow
- **Test**: `test_ai_agent_complete_onboarding_flow`
- **Coverage**:
  - `--help-json` produces valid JSON with commands list
  - `doctor --json` includes AI guidance
  - `introspect --json` shows dependencies and system state
  - `context --json` provides workflow context
  - Commands execute successfully with `--json` flag

#### 2. JSON Output Quality
- **Test**: `test_all_ai_commands_support_json`
- **Coverage**:
  - All AI-focused commands accept `--json` flag
  - JSON output is parseable
  - No panics or crashes on JSON serialization

- **Test**: `test_json_outputs_include_required_fields`
- **Coverage**:
  - `introspect` includes: version, dependencies, system_state
  - `doctor` includes: success, checks, ai_guidance
  - `list` produces array or object structure

#### 3. Exit Code Semantics
- **Test**: `test_semantic_exit_codes`
- **Coverage**:
  - Exit code 0: Success (init, successful operations)
  - Exit code 1: User error (invalid session name)
  - Exit code 2: System error (dependency issues)
  - Exit code 3: Not found (nonexistent session)
  - Exit code 4: Invalid state (not initialized)

#### 4. Command Discovery
- **Test**: `test_command_discovery_via_introspect`
- **Coverage**:
  - `introspect --json` exposes version for compatibility
  - System state shows initialization status
  - Dependencies are listed and checked

- **Test**: `test_help_json_provides_complete_docs`
- **Coverage**:
  - `--help-json` works without jj installed
  - Complete command documentation available
  - Machine-readable format for AI parsing

#### 5. Regression Prevention
- **Test**: `test_doctor_includes_ai_guidance`
- **Coverage**:
  - AI guidance field is present in JSON
  - Guidance array is non-empty
  - Mentions key AI commands (introspect, context)

- **Test**: `test_introspect_shows_dependencies`
- **Coverage**:
  - Dependency status for jj is shown
  - Dependency status for zellij is shown
  - Versions and availability are reported

- **Test**: `test_context_provides_workflow_state`
- **Coverage**:
  - Context returns environment information
  - Repository state is included
  - JSON structure is consistent

#### 6. Error Handling
- **Test**: `test_json_errors_are_well_formed`
- **Coverage**:
  - Errors produce valid JSON when `--json` requested
  - Error JSON includes `success: false`
  - Error messages are included

- **Test**: `test_graceful_failure_without_prerequisites`
- **Coverage**:
  - Commands fail gracefully without init
  - Helpful error messages provided
  - No panics or crashes

#### 7. Complete Workflow
- **Test**: `test_complete_ai_workflow_cycle`
- **Coverage**:
  - Discovery → Health Check → Context → Execute → Verify → Cleanup
  - All phases produce valid JSON
  - State changes are verifiable
  - Cleanup is complete and verifiable

## Regression Checks

### JSON Schema Validation

Each test validates that JSON outputs:
1. **Parse correctly** - `serde_json::from_str` succeeds
2. **Include required fields** - Expected keys are present
3. **Use correct types** - Arrays are arrays, objects are objects
4. **Are consistent** - Same command always produces same structure

### Exit Code Validation

Tests ensure:
1. **Success returns 0** - All successful operations
2. **User errors return 1** - Invalid input, validation failures
3. **System errors return 2** - Missing dependencies, IO failures
4. **Not found returns 3** - Nonexistent resources
5. **Invalid state returns 4** - Not initialized, corrupt state

### AI Guidance Validation

Tests check that:
1. **doctor output includes ai_guidance** - Field is present and populated
2. **Guidance mentions key commands** - introspect, context, help-json
3. **Guidance is actionable** - Suggests next steps for AI agents
4. **Format is consistent** - Array of strings

## Running the Tests

### Local Development

```bash
# Run all AI ergonomics tests
moon run :test -- ai_ergonomics

# Run specific test
moon run :test -- test_ai_agent_complete_onboarding_flow

# Run with verbose output
moon run :test -- --nocapture ai_ergonomics
```

### CI Pipeline

Tests run automatically in CI via `moon run :ci` which includes:
- Format checking (`cargo fmt`)
- Linting (`cargo clippy`)
- **All tests** (`cargo test`)
- Build verification

### Prerequisites

Tests require:
- **jj** installed and in PATH
- **jjz** built (via `moon run :build`)
- Write permissions in test directory
- Sufficient disk space for temporary repos

Tests **gracefully skip** if prerequisites are missing.

## Success Criteria

### Definition of Success (from zjj-pr36)

✅ Test runs complete AI workflow:
- Onboard (help-json) → Check (doctor) → Introspect → Context → Command execution

✅ Validates all JSON outputs:
- All JSON parsing succeeds
- Required fields are present
- Types are correct

✅ Checks exit codes:
- Semantic codes (0-4) are used correctly
- Failures have appropriate codes

✅ Runs in CI pipeline:
- Tests execute on every commit
- Failures block merges
- Graceful degradation when tools missing

## Future Enhancements

### Potential Additions

1. **JSON Schema Generation**
   - Generate JSON Schema files from actual outputs
   - Validate against schemas in tests
   - Provide schemas to AI agents for validation

2. **Performance Testing**
   - Measure command response times
   - Ensure discovery commands are fast (<100ms)
   - Test with large numbers of sessions

3. **AI Agent Simulation**
   - Full LLM integration test
   - Measure success rate of AI completing tasks
   - Identify usability issues

4. **Documentation Generation**
   - Auto-generate AI_GUIDE.md from introspect output
   - Keep documentation in sync with code
   - Version-specific documentation

## Maintenance

### When to Update Tests

1. **New AI-focused commands added**
   - Add to test coverage matrix
   - Add JSON validation tests
   - Add to discovery flow test

2. **JSON output structure changes**
   - Update field validation tests
   - Update regression tests
   - Document breaking changes

3. **Exit code semantics change**
   - Update exit code validation tests
   - Update documentation
   - Communicate to AI agent developers

4. **AI guidance content changes**
   - Update regression tests
   - Verify guidance is still actionable
   - Ensure consistency across commands

### Review Checklist

When modifying AI ergonomics features:

- [ ] All existing tests still pass
- [ ] New features have test coverage
- [ ] JSON outputs validated
- [ ] Exit codes checked
- [ ] AI guidance updated if needed
- [ ] Documentation reflects changes
- [ ] Regression tests protect new features

## Known Limitations

1. **Zellij Integration**
   - Tests run without TTY, can't test full Zellij integration
   - Focus command tests are limited

2. **External Dependencies**
   - Tests skip if jj not installed
   - Can't test all error conditions without mocking

3. **Timing**
   - No performance benchmarks yet
   - No load testing

4. **AI Feedback Loop**
   - No actual AI agent testing
   - Feedback is based on manual validation

## Related Documentation

- [AI_GUIDE.md](../../../docs/12_AI_GUIDE.md) - AI agent user guide
- [AI_ERGONOMICS_PLAN.md](../../../AI_ERGONOMICS_PLAN.md) - Enhancement plan
- [CONTRIBUTING.md](../../../CONTRIBUTING.md) - Development guidelines
- [ARCHITECTURE.md](../../../docs/11_ARCHITECTURE.md) - System architecture

## Contact

For questions or issues with AI ergonomics tests:
1. Check existing tests for patterns
2. Review AI_ERGONOMICS_PLAN.md for context
3. Run tests locally to reproduce issues
4. Report bugs via beads: `bd create --title="AI test issue" --type=bug`
