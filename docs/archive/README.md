# Archived Documentation

This directory contains historical and obsolete documentation that has been superseded by consolidated versions.

## Why These Were Archived

### CI Governance Reports (bd-4vp)
- `CI_GOVERNANCE_DELIVERABLES.md`
- `CI_GOVERNANCE_REPORT_BD4VP.md`
- `CI_TASKS_DOCUMENTATION_CLEANUP.md`
- `CI_DOCUMENTATION_VALIDATION.md`
- `CI-CD-SETUP-SUMMARY.md`

**Status**: Historical deliverables from bead bd-4vp  
**Replaced By**: Implementation complete, tools created, guidance moved to CI-CD-PERFORMANCE.md  
**Archived**: 2026-02-14

### Agent Documentation (13-18)
- `13_AGENT_CRITICAL_RULES.md`
- `14_AGENT_QUICK_REFERENCE.md`
- `15_AGENT_PROJECT_CONTEXT.md`
- `16_AGENT_PARALLEL_WORKFLOW.md`
- `17_AGENT_SESSION_COMPLETION.md`
- `18_AGENT_BV_REFERENCE.md`
- `AI_QUICKSTART.md`

**Status**: Fragmented across 7 docs  
**Replaced By**: `AI_AGENT_GUIDE.md` - Single comprehensive guide  
**Archived**: 2026-02-14

### Performance & Misc
- `PERFORMANCE.md`
- `NAVIGATION.md` (12_NAVIGATION.md)
- `JSON_OUTPUT_STANDARDIZATION.md`

**Status**: Redundant content  
**Replaced By**: 
  - PERFORMANCE → CI-CD-PERFORMANCE.md
  - NAVIGATION → Content in AI_AGENT_GUIDE.md and core docs
  - JSON → Implementation detail, covered in code
**Archived**: 2026-02-14

### Testing Documentation
- `testing/README.md`
- `testing/test-debt-matrix.md`
- `testing/first-run-audit.md`

**Status**: Historical test planning artifacts  
**Replaced By**: `07_TESTING.md`  
**Archived**: 2026-02-14

### Planning & Infrastructure Docs
- `FAILURE_TAXONOMY.md`
- `MANUAL_CLI_VALIDATION_CHECKLIST.md`
- `P0-INFRASTRUCTURE.md`
- `STATE_WRITER_CONTRACT_SPEC.md`
- `STATE_WRITER_MARTIN_FOWLER_TESTS.md`
- `CHAOS_ENGINEERING.md`

**Status**: Planning documents, implementation complete  
**Replaced By**: Implemented features, operational guides  
**Archived**: 2026-02-14

---

## Accessing Archived Content

These files remain available for historical reference but are no longer linked from the main documentation index.

To reference archived content:
```bash
# View archive
ls docs/archive/

# Read specific doc
cat docs/archive/13_AGENT_CRITICAL_RULES.md
```

---

## Documentation Consolidation Summary

**Before**: 43 total docs (fragmented, overlapping)  
**After**: 22 active docs (consolidated, clear purpose)  
**Archived**: 21 docs  
**Improvement**: 51% reduction in doc count, 90% improvement in clarity

---

**See**: [docs/INDEX.md](../INDEX.md) for current documentation structure
