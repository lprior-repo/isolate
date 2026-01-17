# Version Strategy and Stability Guarantees

## Current Version: 0.1.0 (Alpha)

ZJJ follows [Semantic Versioning 2.0.0](https://semver.org/) with clear stability commitments at each stage.

---

## Version Decision Rationale

### Why 0.x (Pre-1.0)?

ZJJ is currently in **alpha** stage with:

**Strengths:**
- Excellent architecture and code quality (A+ grade)
- Zero-unwrap enforcement at compiler level
- Comprehensive error handling with Railway-Oriented Programming
- Strong type safety and functional patterns
- 147+ test cases covering edge cases
- Complete developer documentation

**Not Yet Ready for 1.0:**
- Some documented features incomplete (merge functionality, hooks, templates)
- No end-to-end production testing with real users
- No security audit conducted
- No user-facing documentation
- No binary distribution setup
- No database migration strategy
- Unknown performance characteristics at scale

**Per SemVer 2.0 Section 4:**
> "Major version zero (0.y.z) is for initial development. Anything MAY change at any time. The public API SHOULD NOT be considered stable."

Releasing as 1.0.0 would be **irresponsible** given incomplete features and lack of production validation.

---

## Stability Guarantees

### 0.x Series (Alpha/Beta - Current)

**API Stability:** ❌ **NOT GUARANTEED**

- Breaking changes **MAY** occur in any 0.x release
- CLI commands may change arguments or behavior
- Database schema may change without migration support
- Configuration format may be modified
- Error types and messages may change
- Output formats (JSON, text) may change

**What IS Guaranteed:**
- ✅ Code quality standards (zero unwrap, zero panic)
- ✅ Functional programming patterns
- ✅ Security best practices
- ✅ Comprehensive error handling

**Use Cases:**
- ✅ Testing and evaluation
- ✅ Development and feedback
- ✅ Internal tooling (with caution)
- ❌ **NOT for production critical workflows**
- ❌ **NOT for commercial use without acceptance of risks**

**Upgrade Risk:**
- Updates may require manual intervention
- Session data may need recreation
- Configuration files may need updates
- Scripts may break

---

### 1.0+ Series (Stable - Future)

**API Stability:** ✅ **GUARANTEED per SemVer 2.0**

Once ZJJ reaches 1.0.0, the following guarantees apply:

#### SemVer Compliance

**MAJOR.MINOR.PATCH** (e.g., 1.2.3)

- **PATCH (1.2.x):** Bug fixes only
  - No breaking changes
  - No new features
  - Backward compatible
  - Safe to upgrade immediately

- **MINOR (1.x.0):** New features
  - Backward compatible additions
  - New commands or flags
  - Enhanced functionality
  - Safe to upgrade (may require config review)

- **MAJOR (x.0.0):** Breaking changes ONLY
  - API changes that break compatibility
  - Removed features or commands
  - Changed behavior
  - Requires migration planning

#### Specific Guarantees

**CLI Interface (1.x):**
- Command names won't change
- Existing flags continue to work
- Output formats remain compatible
- Exit codes stay consistent

**Database (1.x):**
- Migration tools provided for schema changes
- Automatic upgrades where possible
- Rollback procedures documented
- Data integrity guaranteed

**Configuration (1.x):**
- Backward compatible format changes
- Deprecated options supported for 1 major version
- Clear migration guides
- Validation tools provided

**Deprecation Policy:**
- Minimum 1 major version warning before removal
- Deprecation warnings in CLI output
- Documentation of alternatives
- Migration guides provided

#### Support Windows

- **Latest Major:** Full support (bug fixes, features)
- **Previous Major:** Security fixes only (12 months)
- **Older Majors:** Community support only

---

## Roadmap to 1.0

### 0.1.x (Alpha) - Current State

**Focus:** Complete documented MVP features

**Blockers:**
- [ ] Implement merge functionality (`jjz remove -m`)
- [ ] Implement hooks system (post_create, pre_remove, etc.)
- [ ] Implement template loading from config
- [ ] Implement change detection in status
- [ ] End-to-end testing with real JJ + Zellij
- [ ] Security audit (`cargo audit`)

**Timeline:** 2-3 weeks

---

### 0.2.0 (Alpha → Beta Transition)

**Focus:** Feature completeness and internal validation

**Goals:**
- ✅ All MVP features implemented (no TODO comments)
- ✅ All tests passing (100% success rate)
- ✅ Security audit clean (no known vulnerabilities)
- ✅ Internal dogfooding (developers using it daily)
- ✅ Performance benchmarks established
- ✅ Database migration strategy implemented

**Timeline:** 2-3 weeks after 0.1.x stable

---

### 0.5.0 (Beta)

**Focus:** External validation and user experience

**Goals:**
- ✅ User-facing documentation complete
  - Installation guide
  - Getting started tutorial
  - Command reference
  - Troubleshooting guide
- ✅ Binary distribution setup
  - GitHub releases
  - Pre-built binaries (Linux, macOS)
  - Homebrew formula
- ✅ Beta testing program (10-20 users)
- ✅ Bug fixes from real usage
- ✅ Performance validation at scale

**Timeline:** 3-4 weeks with user feedback cycles

---

### 0.9.0 (Release Candidate)

**Focus:** Production readiness

**Goals:**
- ✅ All P0-P2 audit issues resolved
- ✅ No known critical bugs
- ✅ Documentation comprehensive
- ✅ Shell completions (bash, zsh, fish)
- ✅ Support/maintenance plan defined
- ✅ Backward compatibility tested
- ✅ API frozen (no more breaking changes)

**Criteria for 1.0:**
- 2+ weeks of RC with no critical bugs
- Positive feedback from beta users
- All release criteria met
- Team confident in stability commitment

**Timeline:** 2-3 weeks validation period

---

### 1.0.0 (Stable)

**Release Criteria:**
- ✅ All features from roadmap complete
- ✅ Comprehensive test coverage (>80%)
- ✅ Zero known critical/high bugs
- ✅ Security audit passed
- ✅ Documentation complete (user + developer)
- ✅ Migration tools ready
- ✅ Support plan in place
- ✅ Performance benchmarks met
- ✅ Beta testing successful (3+ months)

**Commitment:**
- API stability per SemVer 2.0
- 12-month support for each major version
- Database migration support
- Deprecation policy enforcement
- Backward compatibility maintenance

**Timeline:** 8-12 weeks from now (optimistic)

---

## Breaking Change Policy

### During 0.x (Current)

**Allowed:**
- Any breaking change with notice in CHANGELOG
- Database schema changes
- Configuration format changes
- CLI interface changes

**Process:**
1. Document change in CHANGELOG under `[Unreleased]`
2. Tag as `[BREAKING]` in commit message
3. Update relevant documentation
4. Consider providing migration notes if feasible

**User Expectations:**
- Read CHANGELOG before upgrading
- Test in non-critical environment first
- Be prepared for manual migration

---

### During 1.x+ (Future)

**Breaking Changes ONLY in Major Versions**

**Process:**
1. **Deprecation (1.x.0):**
   - Mark feature/API as deprecated
   - Add warnings to CLI output
   - Document replacement in docs
   - Update CHANGELOG

2. **Support Window (1.x.0 → 2.0.0):**
   - Deprecated feature still works
   - Users have time to migrate
   - Migration guide published

3. **Removal (2.0.0):**
   - Feature removed in next major
   - CHANGELOG clearly documents breaking changes
   - Migration guide updated
   - Release notes highlight changes

**Exception:** Security vulnerabilities may require immediate breaking changes in patch versions with clear communication.

---

## Current Status Summary

**Version:** 0.1.0
**Stage:** Alpha
**API Stability:** Not guaranteed
**Production Ready:** No
**Breaking Changes:** Allowed

**Recommended Use:**
- Testing and feedback
- Development and experimentation
- Learning JJ + Zellij workflows

**NOT Recommended:**
- Production critical workflows
- Commercial applications
- Data you can't afford to lose

---

## Version Numbering Examples

### Pre-1.0 (Current)

```
0.1.0 → 0.1.1  (Bug fixes)
0.1.1 → 0.1.2  (More bug fixes)
0.1.2 → 0.2.0  (New features, may include breaking changes)
0.2.0 → 0.5.0  (Jump to beta, breaking changes allowed)
0.5.0 → 0.5.1  (Bug fixes)
0.5.1 → 0.9.0  (Jump to RC)
0.9.0 → 0.9.1  (Bug fixes)
0.9.1 → 1.0.0  (Stable release)
```

### Post-1.0 (Future)

```
1.0.0 → 1.0.1  (Bug fix, backward compatible)
1.0.1 → 1.1.0  (New feature, backward compatible)
1.1.0 → 1.1.1  (Bug fix)
1.1.1 → 2.0.0  (Breaking change, major version bump)
2.0.0 → 2.1.0  (New feature, backward compatible with 2.x)
```

---

## FAQ

### Why not start at 1.0.0?

Per SemVer 2.0 and industry best practices:
- 1.0.0 signals API stability commitment
- Current state has incomplete features and unvalidated production use
- Premature 1.0.0 erodes trust when breaking changes are needed
- 0.x gives flexibility to improve based on feedback

### When will 1.0.0 be ready?

**Realistic Timeline:** 8-12 weeks
- Assuming active development
- Dependent on user feedback
- May extend if critical issues discovered

**Criteria:** See "1.0.0 Release Criteria" above

### Can I use ZJJ in production now?

**Not recommended.**

Current 0.1.0 is alpha quality:
- Some features incomplete
- Breaking changes may occur
- No migration support guaranteed
- Untested at scale

**For Production:**
- Wait for 0.5.0+ (beta)
- Or accept risks and have rollback plan

### How do I stay updated on version changes?

1. **CHANGELOG.md** - All changes documented
2. **GitHub Releases** - Version announcements (when implemented)
3. **Breaking Changes** - Tagged clearly in CHANGELOG

### What if I need a feature not in 1.0?

Post-1.0 development continues via:
- **Minor versions (1.x.0)** - New features (backward compatible)
- **Major versions (2.0.0)** - Breaking changes if needed

Roadmap will be maintained separately.

---

## References

- [Semantic Versioning 2.0.0](https://semver.org/)
- [Keep a Changelog](https://keepachangelog.com/)
- [Production Readiness Audit](../PRODUCTION_READINESS_AUDIT.md)
- [Changelog](../CHANGELOG.md)

---

**Last Updated:** 2026-01-11
**Document Owner:** ZJJ Core Team
**Status:** Active
**Review Cycle:** Before each version bump
