# Martin Fowler Test Plan: zjj-19n8

## Title
LOW-006: Document callback execution behavior

## Test Strategy
This is a documentation task. "Tests" are actually documentation review checkpoints.

## Documentation Review Checklist

### DR-1: Callback Overview Section
- [ ] Callbacks are defined clearly
- [ ] Use case is explained (why use callbacks?)
- [ ] Execution order is documented

### DR-2: Flag Documentation
- [ ] `--on-success` documented with behavior
- [ ] `--on-failure` documented with behavior
- [ ] `--on-complete` documented with behavior
- [ ] All three flags compared in a table

### DR-3: Execution Context
- [ ] Working directory behavior explained
- [ ] Available environment variables listed
- [ ] Exit code handling specified

### DR-4: Output Capture
- [ ] Clarifies callbacks don't receive command output
- [ ] Explains where callback output goes
- [ ] Documents any capture options if they exist

### DR-5: Examples
- [ ] At least 3 copy-paste runnable examples
- [ ] Examples cover common use cases
- [ ] Complex example (multiple callbacks) included

### DR-6: Edge Cases
- [ ] Callback failure behavior documented
- [ ] Missing callback command handling explained
- [ ] Multiple callbacks execution order specified
- [ ] Nested callbacks policy stated

### DR-7: CLI Help Text
- [ ] `zjj --help` mentions callback flags
- [ ] `zjj add --help` shows callback usage
- [ ] Help text matches user guide

## Verification Steps
```bash
# 1. Check help text includes callbacks
zjj --help | grep -E "(on-success|on-failure|on-complete)"

# 2. Verify documentation exists
ls docs/user-guide.md
# or
ls README.md

# 3. Try the examples from docs
# Copy each example and verify it works as documented
```

## Success Criteria
- User guide has callback section
- All 7 review checkpoints pass
- Examples are accurate
- `zjj --help` mentions callback flags

## Estimated Effort
30 minutes
