# Conflict Resolution Configuration

This document describes the conflict resolution configuration system implemented for bead bd-25s.

## Overview

The conflict resolution configuration provides flexible, environment-specific conflict resolution behavior with security safeguards.

## Configuration Structure

```toml
[conflict_resolution]
mode = "hybrid"           # auto, manual, or hybrid
autonomy = 60             # 0-100 (0=manual, 100=fully autonomous)
security_keywords = [     # Files containing these require human review
    "password",
    "token",
    "secret",
    "api_key",
    "private_key",
    "credential",
]
log_resolutions = true    # Enable audit logging
```

## Modes

### Auto Mode
Fully automatic resolution by AI without human intervention.
- **Use case**: CI environments with comprehensive tests
- **Risk**: High - can cause data loss if tests are insufficient
- **Recommendation**: Only use with strong test coverage

### Manual Mode
All conflicts require human intervention. AI may suggest resolutions, but humans must approve them.
- **Use case**: Development, production
- **Risk**: Low - humans review all changes
- **Recommendation**: Default mode for safety

### Hybrid Mode
AI auto-resolves safe conflicts based on autonomy level and security keywords. Risky conflicts require human review.
- **Use case**: CI with human oversight
- **Risk**: Medium - configurable balance
- **Recommendation**: Best for most CI environments

## Autonomy Levels

| Range | Behavior |
|-------|----------|
| 0 | Fully manual - all conflicts require human approval |
| 1-49 | Conservative AI - suggest resolutions, require approval |
| 50-89 | Balanced AI - auto-resolve safe conflicts, prompt for risky ones |
| 90-99 | Aggressive AI - auto-resolve most conflicts, prompt only for security |
| 100 | Fully autonomous - AI resolves all conflicts without prompting |

## Environment Variables

Configuration can be overridden via environment variables:

```bash
# Override mode
export ZJJ_CONFLICT_RESOLUTION_MODE=auto

# Override autonomy
export ZJJ_CONFLICT_RESOLUTION_AUTONOMY=80

# Override logging
export ZJJ_CONFLICT_RESOLUTION_LOG_RESOLUTIONS=false

# Override security keywords (comma-separated)
export ZJJ_CONFLICT_RESOLUTION_SECURITY_KEYWORDS="password,token,secret,api_key"
```

## Security Keywords

Files containing any of these keywords (case-insensitive) require human review, regardless of autonomy level:

- `password`
- `token`
- `secret`
- `key`
- `credential`
- `api_key`
- `private_key`
- `auth`

## Examples

### Development Configuration
```toml
[conflict_resolution]
mode = "manual"
autonomy = 0
security_keywords = ["password", "token", "secret", "key", "credential"]
log_resolutions = true
```

### CI Configuration (Balanced)
```toml
[conflict_resolution]
mode = "hybrid"
autonomy = 60
security_keywords = [
    "password",
    "token",
    "secret",
    "api_key",
    "private_key",
    "credential",
    "auth",
]
log_resolutions = true
```

### Trusted CI Configuration (Aggressive)
```toml
[conflict_resolution]
mode = "auto"
autonomy = 90
security_keywords = [
    "password",
    "token",
    "secret",
    "api_key",
    "private_key",
]
log_resolutions = true
```

## API Usage

```rust
use zjj_core::config::conflict_resolution::{
    ConflictMode, ConflictResolutionConfig, PartialConflictResolutionConfig,
};

// Create config with defaults
let config = ConflictResolutionConfig::default();

// Check if a file requires human review
if config.requires_human_review("src/auth/password.rs") {
    println!("Security-sensitive file detected!");
}

// Check if auto-resolution is allowed
if config.can_auto_resolve(Some("src/lib.rs")) {
    println!("Can auto-resolve this conflict");
}

// Validate configuration
if let Err(e) = config.validate() {
    eprintln!("Invalid config: {e}");
}

// Merge partial config (e.g., from project config)
let mut config = ConflictResolutionConfig::default();
let partial = PartialConflictResolutionConfig {
    mode: Some(ConflictMode::Hybrid),
    autonomy: Some(70),
    ..Default::default()
};
config.merge_partial(partial);
```

## Implementation Details

- **File**: `/home/lewis/src/zjj/crates/zjj-core/src/config/conflict_resolution.rs`
- **Tests**: 75 tests covering happy paths, edge cases, contract verification, and integration
- **Safety**: Zero-panic, zero-unwrap functional Rust patterns
- **Validation**: Autonomy range (0-100), non-empty security keywords
- **Security**: Case-insensitive keyword matching, security overrides autonomy

## Contract Compliance

Implements contract bd-25s:
- ✅ ConflictMode enum (Auto/Manual/Hybrid)
- ✅ ConflictResolutionConfig with all fields
- ✅ PartialConflictResolutionConfig for merge semantics
- ✅ Validation (autonomy range, non-empty keywords)
- ✅ requires_human_review() method
- ✅ can_auto_resolve() method
- ✅ Integration with main Config hierarchy
- ✅ Environment variable overrides
- ✅ Safe defaults (Manual mode, autonomy 0)
- ✅ Comprehensive test coverage
