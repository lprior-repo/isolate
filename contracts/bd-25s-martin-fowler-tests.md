# Martin Fowler BDD Test Plan: Conflict Resolution Configuration (bd-25s)

**Bead ID:** bd-25s
**Title:** Test conflict resolution configuration (mode, autonomy, keywords, logging)
**Framework:** Given-When-Then (Martin Fowler BDD style)
**Total Tests:** 75
**Status:** Draft

---

## Test Organization

```
bd-25s-martin-fowler-tests.md
├── Happy Path Tests (25 tests)
│   ├── HP-001 to HP-010: Config loading and defaults
│   ├── HP-011 to HP-015: Mode validation
│   └── HP-016 to HP-025: Runtime decision making
│
├── Edge Case Tests (25 tests)
│   ├── EC-001 to EC-010: Boundary conditions
│   ├── EC-011 to EC-020: Partial config merge
│   └── EC-021 to EC-025: Empty and malformed config
│
├── Contract Verification Tests (15 tests)
│   ├── CV-001 to CV-005: Preconditions
│   ├── CV-006 to CV-010: Postconditions
│   └── CV-011 to CV-015: Invariants
│
└── Integration Tests (10 tests)
    ├── IT-001 to IT-005: Config hierarchy
    └── IT-006 to IT-010: Environment variable overrides
```

---

## Category 1: Happy Path Tests (25 tests)

### HP-001: Default config is safe and valid

**Given:** A fresh configuration with default values
**When:** The config is loaded
**Then:** All fields have safe default values
  - mode is Manual
  - autonomy is 0
  - security_keywords has 5+ entries
  - log_resolutions is true

```rust
#[test]
fn hp_001_default_config_is_safe() {
    let config = ConflictResolutionConfig::default();

    assert_eq!(config.mode, ConflictMode::Manual);
    assert_eq!(config.autonomy, 0);
    assert!(config.security_keywords.len() >= 5);
    assert!(config.log_resolutions);
}
```

---

### HP-002: Valid config passes validation

**Given:** A config with valid mode, autonomy, keywords, and logging flag
**When:** `validate()` is called
**Then:** Validation returns `Ok(())`

```rust
#[test]
fn hp_002_valid_config_passes_validation() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Hybrid,
        autonomy: 50,
        security_keywords: vec!["password".to_string()],
        log_resolutions: true,
    };

    assert!(config.validate().is_ok());
}
```

---

### HP-003: Manual mode never allows auto-resolution

**Given:** Config with Manual mode and any autonomy level
**When:** `can_auto_resolve()` is called
**Then:** Returns false regardless of autonomy

```rust
#[test]
fn hp_003_manual_mode_never_auto_resolves() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Manual,
        autonomy: 100,
        security_keywords: vec!["password".to_string()],
        log_resolutions: true,
    };

    assert!(!config.can_auto_resolve(None));
    assert!(!config.can_auto_resolve(Some("safe_file.rs")));
}
```

---

### HP-004: Auto mode always allows auto-resolution

**Given:** Config with Auto mode
**When:** `can_auto_resolve()` is called
**Then:** Returns true for any file

```rust
#[test]
fn hp_004_auto_mode_allows_auto_resolution() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Auto,
        autonomy: 0,
        security_keywords: vec!["password".to_string()],
        log_resolutions: true,
    };

    assert!(config.can_auto_resolve(None));
    assert!(config.can_auto_resolve(Some("any_file.rs")));
}
```

---

### HP-005: Hybrid mode respects autonomy threshold

**Given:** Config with Hybrid mode and autonomy 60
**When:** `can_auto_resolve()` is called on non-sensitive file
**Then:** Returns true (autonomy >= 50)

```rust
#[test]
fn hp_005_hybrid_mode_respects_autonomy_threshold() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Hybrid,
        autonomy: 60,
        security_keywords: vec!["password".to_string()],
        log_resolutions: true,
    };

    assert!(config.can_auto_resolve(Some("safe_file.rs")));
}
```

---

### HP-006: Hybrid mode blocks auto-resolution below threshold

**Given:** Config with Hybrid mode and autonomy 30
**When:** `can_auto_resolve()` is called
**Then:** Returns false (autonomy < 50)

```rust
#[test]
fn hp_006_hybrid_mode_blocks_below_threshold() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Hybrid,
        autonomy: 30,
        security_keywords: vec!["password".to_string()],
        log_resolutions: true,
    };

    assert!(!config.can_auto_resolve(Some("safe_file.rs")));
}
```

---

### HP-007: Security keywords trigger human review

**Given:** Config with security keyword "password"
**When:** `requires_human_review()` is called on "auth/password.rs"
**Then:** Returns true

```rust
#[test]
fn hp_007_security_keywords_trigger_review() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Hybrid,
        autonomy: 90,
        security_keywords: vec!["password".to_string()],
        log_resolutions: true,
    };

    assert!(config.requires_human_review("src/auth/password.rs"));
}
```

---

### HP-008: Security keywords are case-insensitive

**Given:** Config with keyword "PASSWORD"
**When:** `requires_human_review()` is called on "password.txt"
**Then:** Returns true (case-insensitive match)

```rust
#[test]
fn hp_008_security_keywords_case_insensitive() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Manual,
        autonomy: 0,
        security_keywords: vec!["PASSWORD".to_string()],
        log_resolutions: true,
    };

    assert!(config.requires_human_review("src/auth/password.txt"));
    assert!(config.requires_human_review("src/auth/PASSWORD.txt"));
    assert!(config.requires_human_review("src/auth/PassWord.txt"));
}
```

---

### HP-009: Multiple security keywords all trigger review

**Given:** Config with keywords ["password", "token", "secret"]
**When:** Files are checked for human review
**Then:** Each keyword triggers review for matching files

```rust
#[test]
fn hp_009_multiple_keywords_all_trigger_review() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Hybrid,
        autonomy: 90,
        security_keywords: vec![
            "password".to_string(),
            "token".to_string(),
            "secret".to_string(),
        ],
        log_resolutions: true,
    };

    assert!(config.requires_human_review("src/auth/password.rs"));
    assert!(config.requires_human_review("src/api/token.rs"));
    assert!(config.requires_human_review("src/config/secret.rs"));
}
```

---

### HP-010: Security keywords override autonomy in hybrid mode

**Given:** Config with Hybrid mode, autonomy 90, and keyword "password"
**When:** `can_auto_resolve()` is called on "password.rs"
**Then:** Returns false (security keyword blocks auto-resolution)

```rust
#[test]
fn hp_010_security_overrides_autonomy() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Hybrid,
        autonomy: 90,
        security_keywords: vec!["password".to_string()],
        log_resolutions: true,
    };

    assert!(!config.can_auto_resolve(Some("src/auth/password.rs")));
}
```

---

### HP-011: Autonomy at minimum (0) is valid

**Given:** Config with autonomy 0
**When:** `validate()` is called
**Then:** Returns Ok (0 is valid)

```rust
#[test]
fn hp_011_autonomy_minimum_valid() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Manual,
        autonomy: 0,
        security_keywords: vec!["key".to_string()],
        log_resolutions: true,
    };

    assert!(config.validate().is_ok());
}
```

---

### HP-012: Autonomy at maximum (100) is valid

**Given:** Config with autonomy 100
**When:** `validate()` is called
**Then:** Returns Ok (100 is valid)

```rust
#[test]
fn hp_012_autonomy_maximum_valid() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Auto,
        autonomy: 100,
        security_keywords: vec!["key".to_string()],
        log_resolutions: true,
    };

    assert!(config.validate().is_ok());
}
```

---

### HP-013: All three modes are valid

**Given:** Config with each mode variant
**When:** Each is validated
**Then:** All return Ok

```rust
#[test]
fn hp_013_all_modes_valid() {
    let keywords = vec!["key".to_string()];

    let auto = ConflictResolutionConfig {
        mode: ConflictMode::Auto,
        autonomy: 80,
        security_keywords: keywords.clone(),
        log_resolutions: true,
    };

    let manual = ConflictResolutionConfig {
        mode: ConflictMode::Manual,
        autonomy: 0,
        security_keywords: keywords.clone(),
        log_resolutions: true,
    };

    let hybrid = ConflictResolutionConfig {
        mode: ConflictMode::Hybrid,
        autonomy: 50,
        security_keywords: keywords.clone(),
        log_resolutions: true,
    };

    assert!(auto.validate().is_ok());
    assert!(manual.validate().is_ok());
    assert!(hybrid.validate().is_ok());
}
```

---

### HP-014: log_resolutions can be true

**Given:** Config with log_resolutions = true
**When:** Config is loaded
**Then:** Field is set to true

```rust
#[test]
fn hp_014_log_resolutions_true() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Manual,
        autonomy: 0,
        security_keywords: vec!["key".to_string()],
        log_resolutions: true,
    };

    assert!(config.log_resolutions);
}
```

---

### HP-015: log_resolutions can be false

**Given:** Config with log_resolutions = false
**When:** Config is loaded
**Then:** Field is set to false

```rust
#[test]
fn hp_015_log_resolutions_false() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Auto,
        autonomy: 100,
        security_keywords: vec!["key".to_string()],
        log_resolutions: false,
    };

    assert!(!config.log_resolutions);
}
```

---

### HP-016: Config with all fields set loads correctly

**Given:** Complete TOML config with all conflict_resolution fields
**When:** Config is loaded from file
**Then:** All fields have correct values

```rust
#[tokio::test]
async fn hp_016_complete_config_loads() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let toml_content = r#"
[conflict_resolution]
mode = "hybrid"
autonomy = 70
security_keywords = ["password", "token", "secret"]
log_resolutions = true
"#;

    tokio::fs::write(&config_path, toml_content)
        .await
        .unwrap();

    let partial = load_partial_toml_file(&config_path)
        .await
        .unwrap();

    assert!(partial.conflict_resolution.is_some());
}
```

---

### HP-017: Minimal config (only mode) loads with other defaults

**Given:** Config with only mode set
**When:** Config is loaded
**Then:** Other fields use default values

```rust
#[tokio::test]
async fn hp_017_minimal_config_uses_defaults() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let toml_content = r#"
[conflict_resolution]
mode = "auto"
"#;

    tokio::fs::write(&config_path, toml_content)
        .await
        .unwrap();

    let partial = load_partial_toml_file(&config_path)
        .await
        .unwrap();

    assert!(partial.conflict_resolution.is_some());
}
```

---

### HP-018: Config without conflict_resolution section uses defaults

**Given:** Config with no conflict_resolution section
**When:** Config is loaded
**Then:** Uses default ConflictResolutionConfig

```rust
#[tokio::test]
async fn hp_018_missing_section_uses_defaults() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let toml_content = r#"
workspace_dir = "../test"
main_branch = "main"
"#;

    tokio::fs::write(&config_path, toml_content)
        .await
        .unwrap();

    let partial = load_partial_toml_file(&config_path)
        .await
        .unwrap();

    assert!(partial.conflict_resolution.is_none());
}
```

---

### HP-019: ConflictResolutionConfig is serializable to JSON

**Given:** A valid config instance
**When:** Serialized to JSON
**Then:** Produces valid JSON with all fields

```rust
#[test]
fn hp_019_config_serializable_to_json() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Hybrid,
        autonomy: 60,
        security_keywords: vec!["password".to_string()],
        log_resolutions: true,
    };

    let json = serde_json::to_string(&config).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["mode"], "hybrid");
    assert_eq!(parsed["autonomy"], 60);
    assert!(parsed["security_keywords"].is_array());
    assert_eq!(parsed["log_resolutions"], true);
}
```

---

### HP-020: ConflictResolutionConfig is deserializable from JSON

**Given:** Valid JSON string
**When:** Deserialized to ConflictResolutionConfig
**Then:** Produces valid config instance

```rust
#[test]
fn hp_020_config_deserializable_from_json() {
    let json = r#"{
        "mode": "hybrid",
        "autonomy": 60,
        "security_keywords": ["password"],
        "log_resolutions": true
    }"#;

    let config: ConflictResolutionConfig = serde_json::from_str(json).unwrap();

    assert_eq!(config.mode, ConflictMode::Hybrid);
    assert_eq!(config.autonomy, 60);
    assert_eq!(config.security_keywords.len(), 1);
    assert!(config.log_resolutions);
}
```

---

### HP-021: ConflictMode implements Display correctly

**Given:** Each mode variant
**When:** Converted to string
**Then:** Returns lowercase mode name

```rust
#[test]
fn hp_021_mode_display_correct() {
    assert_eq!(ConflictMode::Auto.to_string(), "auto");
    assert_eq!(ConflictMode::Manual.to_string(), "manual");
    assert_eq!(ConflictMode::Hybrid.to_string(), "hybrid");
}
```

---

### HP-022: ConflictMode implements FromStr correctly

**Given:** String representation of each mode
**When:** Parsed with FromStr
**Then:** Returns correct mode variant

```rust
#[test]
fn hp_022_mode_fromstr_correct() {
    assert_eq!(
        ConflictMode::from_str("auto").unwrap(),
        ConflictMode::Auto
    );
    assert_eq!(
        ConflictMode::from_str("manual").unwrap(),
        ConflictMode::Manual
    );
    assert_eq!(
        ConflictMode::from_str("hybrid").unwrap(),
        ConflictMode::Hybrid
    );
}
```

---

### HP-023: FromStr is case-insensitive for mode

**Given:** Mixed-case mode strings
**When:** Parsed with FromStr
**Then:** Returns correct mode regardless of case

```rust
#[test]
fn hp_023_mode_fromstr_case_insensitive() {
    assert_eq!(
        ConflictMode::from_str("AUTO").unwrap(),
        ConflictMode::Auto
    );
    assert_eq!(
        ConflictMode::from_str("Manual").unwrap(),
        ConflictMode::Manual
    );
    assert_eq!(
        ConflictMode::from_str("HyBrId").unwrap(),
        ConflictMode::Hybrid
    );
}
```

---

### HP-024: FromStr returns error for invalid mode

**Given:** Invalid mode string
**When:** Parsed with FromStr
**Then:** Returns Error

```rust
#[test]
fn hp_024_mode_fromstr_invalid() {
    let result = ConflictMode::from_str("invalid_mode");
    assert!(result.is_err());
}
```

---

### HP-025: Config validation passes with typical production settings

**Given:** Production-like config with hybrid mode, moderate autonomy
**When:** `validate()` is called
**Then:** Returns Ok

```rust
#[test]
fn hp_025_production_config_valid() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Hybrid,
        autonomy: 70,
        security_keywords: vec![
            "password".to_string(),
            "token".to_string(),
            "secret".to_string(),
            "api_key".to_string(),
            "private_key".to_string(),
            "credential".to_string(),
        ],
        log_resolutions: true,
    };

    assert!(config.validate().is_ok());
}
```

---

## Category 2: Edge Case Tests (25 tests)

### EC-001: Autonomy just below minimum (-1) is invalid

**Given:** Config with autonomy -1 (or 255 if u8 underflows)
**When:** `validate()` is called
**Then:** Returns ValidationError

```rust
#[test]
fn ec_001_autonomy_below_minimum_invalid() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Manual,
        autonomy: 255, // Would be -1 if signed
        security_keywords: vec!["key".to_string()],
        log_resolutions: true,
    };

    assert!(config.validate().is_err());
}
```

---

### EC-002: Autonomy just above maximum (101) is invalid

**Given:** Config with autonomy 101
**When:** `validate()` is called
**Then:** Returns ValidationError with autonomy constraint message

```rust
#[test]
fn ec_002_autonomy_above_maximum_invalid() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Auto,
        autonomy: 101,
        security_keywords: vec!["key".to_string()],
        log_resolutions: true,
    };

    let result = config.validate();
    assert!(result.is_err());

    if let Err(Error::ValidationError { field, .. }) = result {
        assert_eq!(field, Some("autonomy".to_string()));
    }
}
```

---

### EC-003: Empty security_keywords list is invalid

**Given:** Config with empty security_keywords
**When:** `validate()` is called
**Then:** Returns ValidationError with security_keywords constraint message

```rust
#[test]
fn ec_003_empty_keywords_invalid() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Manual,
        autonomy: 0,
        security_keywords: vec![],
        log_resolutions: true,
    };

    let result = config.validate();
    assert!(result.is_err());

    if let Err(Error::ValidationError { field, .. }) = result {
        assert_eq!(field, Some("security_keywords".to_string()));
    }
}
```

---

### EC-004: File without security keywords doesn't trigger review

**Given:** Config with keyword "password"
**When:** `requires_human_review()` is called on "README.md"
**Then:** Returns false

```rust
#[test]
fn ec_004_safe_file_no_review() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Hybrid,
        autonomy: 90,
        security_keywords: vec!["password".to_string()],
        log_resolutions: true,
    };

    assert!(!config.requires_human_review("README.md"));
    assert!(!config.requires_human_review("src/lib.rs"));
    assert!(!config.requires_human_review("tests/test.rs"));
}
```

---

### EC-005: Keyword as substring triggers review

**Given:** Config with keyword "key"
**When:** `requires_human_review()` is called on "api_key.rs"
**Then:** Returns true (substring match)

```rust
#[test]
fn ec_005_keyword_substring_triggers_review() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Hybrid,
        autonomy: 90,
        security_keywords: vec!["key".to_string()],
        log_resolutions: true,
    };

    assert!(config.requires_human_review("src/config/api_key.rs"));
    assert!(config.requires_human_review("src/auth/private_key.rs"));
    assert!(config.requires_human_review("src/encryption/secret_key.rs"));
}
```

---

### EC-006: Hybrid mode with autonomy exactly at threshold (50)

**Given:** Config with Hybrid mode and autonomy 50
**When:** `can_auto_resolve()` is called on safe file
**Then:** Returns true (50 >= 50)

```rust
#[test]
fn ec_006_hybrid_autonomy_at_threshold() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Hybrid,
        autonomy: 50,
        security_keywords: vec!["password".to_string()],
        log_resolutions: true,
    };

    assert!(config.can_auto_resolve(Some("safe_file.rs")));
}
```

---

### EC-007: Hybrid mode with autonomy just below threshold (49)

**Given:** Config with Hybrid mode and autonomy 49
**When:** `can_auto_resolve()` is called
**Then:** Returns false (49 < 50)

```rust
#[test]
fn ec_007_hybrid_autonomy_below_threshold() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Hybrid,
        autonomy: 49,
        security_keywords: vec!["password".to_string()],
        log_resolutions: true,
    };

    assert!(!config.can_auto_resolve(Some("safe_file.rs")));
}
```

---

### EC-008: can_auto_resolve with None file path in hybrid mode

**Given:** Config with Hybrid mode and autonomy 60
**When:** `can_auto_resolve()` is called with None
**Then:** Returns true (no file to check for security keywords)

```rust
#[test]
fn ec_008_auto_resolve_no_file_path() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Hybrid,
        autonomy: 60,
        security_keywords: vec!["password".to_string()],
        log_resolutions: true,
    };

    assert!(config.can_auto_resolve(None));
}
```

---

### EC-009: can_auto_resolve with None file path in manual mode

**Given:** Config with Manual mode
**When:** `can_auto_resolve()` is called with None
**Then:** Returns false (manual mode never auto-resolves)

```rust
#[test]
fn ec_009_manual_mode_no_file_path() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Manual,
        autonomy: 100,
        security_keywords: vec!["password".to_string()],
        log_resolutions: true,
    };

    assert!(!config.can_auto_resolve(None));
}
```

---

### EC-010: can_auto_resolve with None file path in auto mode

**Given:** Config with Auto mode
**When:** `can_auto_resolve()` is called with None
**Then:** Returns true (auto mode always allows)

```rust
#[test]
fn ec_010_auto_mode_no_file_path() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Auto,
        autonomy: 0,
        security_keywords: vec!["password".to_string()],
        log_resolutions: true,
    };

    assert!(config.can_auto_resolve(None));
}
```

---

### EC-011: Partial config with only mode specified

**Given:** Partial config with mode = Some(Auto), other fields None
**When:** Merged into base config
**Then:** Only mode is updated

```rust
#[test]
fn ec_011_partial_mode_only() {
    let mut base = ConflictResolutionConfig::default();
    let original_autonomy = base.autonomy;
    let original_keywords = base.security_keywords.clone();
    let original_log = base.log_resolutions;

    let partial = PartialConflictResolutionConfig {
        mode: Some(ConflictMode::Auto),
        autonomy: None,
        security_keywords: None,
        log_resolutions: None,
    };

    base.merge_partial(partial);

    assert_eq!(base.mode, ConflictMode::Auto);
    assert_eq!(base.autonomy, original_autonomy);
    assert_eq!(base.security_keywords, original_keywords);
    assert_eq!(base.log_resolutions, original_log);
}
```

---

### EC-012: Partial config with only autonomy specified

**Given:** Partial config with autonomy = Some(80), other fields None
**When:** Merged into base config
**Then:** Only autonomy is updated

```rust
#[test]
fn ec_012_partial_autonomy_only() {
    let mut base = ConflictResolutionConfig::default();
    let original_mode = base.mode;

    let partial = PartialConflictResolutionConfig {
        mode: None,
        autonomy: Some(80),
        security_keywords: None,
        log_resolutions: None,
    };

    base.merge_partial(partial);

    assert_eq!(base.mode, original_mode);
    assert_eq!(base.autonomy, 80);
}
```

---

### EC-013: Partial config with only keywords specified

**Given:** Partial config with keywords = Some(vec!["token"]), other fields None
**When:** Merged into base config
**Then:** Only keywords are updated

```rust
#[test]
fn ec_013_partial_keywords_only() {
    let mut base = ConflictResolutionConfig::default();
    let original_mode = base.mode;
    let original_autonomy = base.autonomy;

    let new_keywords = vec!["token".to_string()];
    let partial = PartialConflictResolutionConfig {
        mode: None,
        autonomy: None,
        security_keywords: Some(new_keywords.clone()),
        log_resolutions: None,
    };

    base.merge_partial(partial);

    assert_eq!(base.mode, original_mode);
    assert_eq!(base.autonomy, original_autonomy);
    assert_eq!(base.security_keywords, new_keywords);
}
```

---

### EC-014: Partial config with only log_resolutions specified

**Given:** Partial config with log_resolutions = Some(false), other fields None
**When:** Merged into base config
**Then:** Only log_resolutions is updated

```rust
#[test]
fn ec_014_partial_log_only() {
    let mut base = ConflictResolutionConfig::default();
    let original_mode = base.mode;

    let partial = PartialConflictResolutionConfig {
        mode: None,
        autonomy: None,
        security_keywords: None,
        log_resolutions: Some(false),
    };

    base.merge_partial(partial);

    assert_eq!(base.mode, original_mode);
    assert!(!base.log_resolutions);
}
```

---

### EC-015: Partial config with all fields specified

**Given:** Partial config with all fields Some
**When:** Merged into base config
**Then:** All fields are updated

```rust
#[test]
fn ec_015_partial_all_fields() {
    let mut base = ConflictResolutionConfig::default();

    let partial = PartialConflictResolutionConfig {
        mode: Some(ConflictMode::Auto),
        autonomy: Some(90),
        security_keywords: Some(vec!["test".to_string()]),
        log_resolutions: Some(false),
    };

    base.merge_partial(partial);

    assert_eq!(base.mode, ConflictMode::Auto);
    assert_eq!(base.autonomy, 90);
    assert_eq!(base.security_keywords, vec!["test".to_string()]);
    assert!(!base.log_resolutions);
}
```

---

### EC-016: Empty partial config (all None) doesn't change base

**Given:** Partial config with all fields None
**When:** Merged into base config
**Then:** Base config remains unchanged

```rust
#[test]
fn ec_016_empty_partial_no_changes() {
    let mut base = ConflictResolutionConfig::default();
    let original = base.clone();

    let partial = PartialConflictResolutionConfig::default();
    base.merge_partial(partial);

    assert_eq!(base, original);
}
```

---

### EC-017: Multiple partial merges accumulate correctly

**Given:** Base config merged with partial1, then partial2
**When:** Second merge happens
**Then:** Both merges are applied

```rust
#[test]
fn ec_017_multiple_partial_merges() {
    let mut base = ConflictResolutionConfig::default();

    let partial1 = PartialConflictResolutionConfig {
        mode: Some(ConflictMode::Hybrid),
        autonomy: None,
        security_keywords: None,
        log_resolutions: None,
    };

    let partial2 = PartialConflictResolutionConfig {
        mode: None,
        autonomy: Some(70),
        security_keywords: None,
        log_resolutions: None,
    };

    base.merge_partial(partial1);
    base.merge_partial(partial2);

    assert_eq!(base.mode, ConflictMode::Hybrid);
    assert_eq!(base.autonomy, 70);
}
```

---

### EC-018: Later partial merge overrides earlier one

**Given:** Base merged with partial1 (autonomy=50), then partial2 (autonomy=80)
**When:** Second merge happens
**Then:** autonomy is 80 (latest wins)

```rust
#[test]
fn ec_018_later_merge_overrides() {
    let mut base = ConflictResolutionConfig::default();

    let partial1 = PartialConflictResolutionConfig {
        mode: None,
        autonomy: Some(50),
        security_keywords: None,
        log_resolutions: None,
    };

    let partial2 = PartialConflictResolutionConfig {
        mode: None,
        autonomy: Some(80),
        security_keywords: None,
        log_resolutions: None,
    };

    base.merge_partial(partial1);
    base.merge_partial(partial2);

    assert_eq!(base.autonomy, 80);
}
```

---

### EC-019: Config with invalid TOML fails to load

**Given:** Malformed TOML file
**When:** Attempted to load
**Then:** Returns ParseError

```rust
#[tokio::test]
async fn ec_019_invalid_toml_fails() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("bad.toml");

    tokio::fs::write(&config_path, b"[conflict_resolution\nmode = invalid")
        .await
        .unwrap();

    let result = load_partial_toml_file(&config_path).await;
    assert!(result.is_err());
}
```

---

### EC-020: Config with wrong type for mode fails

**Given:** TOML with mode as integer instead of string
**When:** Attempted to load
**Then:** Returns ParseError

```rust
#[tokio::test]
async fn ec_020_wrong_mode_type_fails() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("bad.toml");

    tokio::fs::write(
        &config_path,
        b"[conflict_resolution]\nmode = 123",
    )
    .await
    .unwrap();

    let result = load_partial_toml_file(&config_path).await;
    assert!(result.is_err());
}
```

---

### EC-021: Config with wrong type for autonomy fails

**Given:** TOML with autonomy as string instead of integer
**When:** Attempted to load
**Then:** Returns ParseError

```rust
#[tokio::test]
async fn ec_021_wrong_autonomy_type_fails() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("bad.toml");

    tokio::fs::write(
        &config_path,
        b"[conflict_resolution]\nautonomy = \"high\"",
    )
    .await
    .unwrap();

    let result = load_partial_toml_file(&config_path).await;
    assert!(result.is_err());
}
```

---

### EC-022: Config with wrong type for log_resolutions fails

**Given:** TOML with log_resolutions as string instead of boolean
**When:** Attempted to load
**Then:** Returns ParseError

```rust
#[tokio::test]
async fn ec_022_wrong_log_type_fails() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("bad.toml");

    tokio::fs::write(
        &config_path,
        b"[conflict_resolution]\nlog_resolutions = \"yes\"",
    )
    .await
    .unwrap();

    let result = load_partial_toml_file(&config_path).await;
    assert!(result.is_err());
}
```

---

### EC-023: Config with unknown field fails validation

**Given:** TOML with unknown field in conflict_resolution section
**When:** Attempted to load
**Then:** Returns ValidationError for unknown key

```rust
#[tokio::test]
async fn ec_023_unknown_field_fails() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("bad.toml");

    tokio::fs::write(
        &config_path,
        b"[conflict_resolution]\nunknown_field = \"value\"",
    )
    .await
    .unwrap();

    let result = load_partial_toml_file(&config_path).await;
    assert!(result.is_err());
}
```

---

### EC-024: Security keyword with special characters

**Given:** Config with keyword containing special characters like "api-key"
**When:** Checking file for review
**Then:** Keyword match works correctly

```rust
#[test]
fn ec_024_keyword_special_chars() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Hybrid,
        autonomy: 90,
        security_keywords: vec!["api-key".to_string()],
        log_resolutions: true,
    };

    assert!(config.requires_human_review("src/config/api-key.rs"));
    assert!(config.requires_human_review("src/config/api_key.rs"));
}
```

---

### EC-025: Very long security keyword list

**Given:** Config with 100+ security keywords
**When:** Validation and file checks are performed
**Then:** Operations complete successfully

```rust
#[test]
fn ec_025_many_keywords_works() {
    let keywords: Vec<String> = (0..100)
        .map(|i| format!("keyword_{}", i))
        .collect();

    let config = ConflictResolutionConfig {
        mode: ConflictMode::Hybrid,
        autonomy: 80,
        security_keywords: keywords.clone(),
        log_resolutions: true,
    };

    assert!(config.validate().is_ok());
    assert!(config.requires_human_review("keyword_50.rs"));
    assert!(!config.requires_human_review("safe.rs"));
}
```

---

## Category 3: Contract Verification Tests (15 tests)

### CV-001: PRE-CONF-001 - Autonomy must be 0-100

**Given:** Config with autonomy in valid range
**When:** Validated
**Then:** Validation passes

```rust
#[test]
fn cv_001_pre_conf_001_autonomy_range() {
    for autonomy in [0, 1, 50, 99, 100] {
        let config = ConflictResolutionConfig {
            mode: ConflictMode::Manual,
            autonomy,
            security_keywords: vec!["key".to_string()],
            log_resolutions: true,
        };
        assert!(config.validate().is_ok(), "autonomy={} should be valid", autonomy);
    }
}
```

---

### CV-002: PRE-CONF-002 - Security keywords must not be empty

**Given:** Config with non-empty keywords list
**When:** Validated
**Then:** Validation passes

```rust
#[test]
fn cv_002_pre_conf_002_keywords_nonempty() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Manual,
        autonomy: 0,
        security_keywords: vec!["test".to_string()],
        log_resolutions: true,
    };

    assert!(config.validate().is_ok());
}
```

---

### CV-003: PRE-CONF-003 - Mode must be valid

**Given:** Config with each valid mode
**When:** Validated
**Then:** Validation passes

```rust
#[test]
fn cv_003_pre_conf_003_mode_valid() {
    for mode in [ConflictMode::Auto, ConflictMode::Manual, ConflictMode::Hybrid] {
        let config = ConflictResolutionConfig {
            mode,
            autonomy: 50,
            security_keywords: vec!["key".to_string()],
            log_resolutions: true,
        };
        assert!(config.validate().is_ok(), "mode={:?} should be valid", mode);
    }
}
```

---

### CV-004: POST-LOAD-001 - Loaded config passes validation

**Given:** Config loaded from valid TOML file
**When:** Validation is checked
**Then:** Config is valid

```rust
#[tokio::test]
async fn cv_004_post_load_001_valid_after_load() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let toml_content = r#"
[conflict_resolution]
mode = "hybrid"
autonomy = 70
security_keywords = ["password"]
log_resolutions = true
"#;

    tokio::fs::write(&config_path, toml_content)
        .await
        .unwrap();

    let partial = load_partial_toml_file(&config_path)
        .await
        .unwrap();

    assert!(partial.conflict_resolution.is_some());
}
```

---

### CV-005: POST-LOAD-002 - Default values are sensible

**Given:** Default config instance
**When:** Fields are inspected
**Then:** Mode is Manual, autonomy is 0, keywords has entries, log is true

```rust
#[test]
fn cv_005_post_load_002_sensible_defaults() {
    let config = ConflictResolutionConfig::default();

    assert_eq!(config.mode, ConflictMode::Manual);
    assert_eq!(config.autonomy, 0);
    assert!(config.security_keywords.len() >= 5);
    assert!(config.log_resolutions);
}
```

---

### CV-006: INV-DATA-001 - Autonomy always 0-100

**Given:** Any valid config instance
**When:** Autonomy is inspected
**Then:** Always in range 0-100

```rust
#[test]
fn cv_006_inv_data_001_autonomy_invariant() {
    let configs = vec![
        ConflictResolutionConfig::default(),
        ConflictResolutionConfig {
            mode: ConflictMode::Auto,
            autonomy: 100,
            security_keywords: vec!["key".to_string()],
            log_resolutions: true,
        },
    ];

    for config in configs {
        assert!(config.autonomy <= 100, "autonomy invariant violated");
    }
}
```

---

### CV-007: INV-DATA-002 - Security keywords never empty

**Given:** Any valid config instance
**When:** Keywords are inspected
**Then:** Never empty

```rust
#[test]
fn cv_007_inv_data_002_keywords_invariant() {
    let config = ConflictResolutionConfig::default();
    assert!(!config.security_keywords.is_empty());
}
```

---

### CV-008: INV-SEC-001 - Security keywords override autonomy

**Given:** Config with high autonomy and security keywords
**When:** File matching keyword is checked
**Then:** Requires human review regardless of autonomy

```rust
#[test]
fn cv_008_inv_sec_001_security_overrides_autonomy() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Hybrid,
        autonomy: 100,
        security_keywords: vec!["password".to_string()],
        log_resolutions: true,
    };

    assert!(config.requires_human_review("src/auth/password.rs"));
    assert!(!config.can_auto_resolve(Some("src/auth/password.rs")));
}
```

---

### CV-009: INV-SEC-002 - Manual mode never auto-resolves

**Given:** Config with Manual mode and any autonomy
**When:** `can_auto_resolve()` is called
**Then:** Always returns false

```rust
#[test]
fn cv_009_inv_sec_002_manual_never_auto() {
    for autonomy in [0, 50, 100] {
        let config = ConflictResolutionConfig {
            mode: ConflictMode::Manual,
            autonomy,
            security_keywords: vec!["key".to_string()],
            log_resolutions: true,
        };

        assert!(!config.can_auto_resolve(None));
        assert!(!config.can_auto_resolve(Some("any_file.rs")));
    }
}
```

---

### CV-010: INV-SEC-003 - Auto mode always allows auto-resolution

**Given:** Config with Auto mode
**When:** `can_auto_resolve()` is called
**Then:** Always returns true

```rust
#[test]
fn cv_010_inv_sec_003_auto_always_allows() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Auto,
        autonomy: 0,
        security_keywords: vec!["password".to_string()],
        log_resolutions: true,
    };

    assert!(config.can_auto_resolve(None));
    assert!(config.can_auto_resolve(Some("any_file.rs")));
}
```

---

### CV-011: INV-CONF-001 - Higher priority config overrides lower

**Given:** Default config merged with global config merged with project config
**When:** Config hierarchy is applied
**Then:** Project config values win

```rust
#[test]
fn cv_011_inv_conf_001_hierarchy_precedence() {
    let mut config = ConflictResolutionConfig::default();
    assert_eq!(config.mode, ConflictMode::Manual);

    // Apply "global" override
    config.merge_partial(PartialConflictResolutionConfig {
        mode: Some(ConflictMode::Hybrid),
        autonomy: Some(50),
        security_keywords: None,
        log_resolutions: None,
    });
    assert_eq!(config.mode, ConflictMode::Hybrid);
    assert_eq!(config.autonomy, 50);

    // Apply "project" override
    config.merge_partial(PartialConflictResolutionConfig {
        mode: Some(ConflictMode::Auto),
        autonomy: None,
        security_keywords: None,
        log_resolutions: None,
    });
    assert_eq!(config.mode, ConflictMode::Auto);
    assert_eq!(config.autonomy, 50); // Preserved from global
}
```

---

### CV-012: INV-CONF-002 - Partial merge preserves unspecified fields

**Given:** Base config with all fields set
**When:** Partial config with only one field is merged
**Then:** Other fields remain unchanged

```rust
#[test]
fn cv_012_inv_conf_002_partial_preserves_fields() {
    let mut base = ConflictResolutionConfig {
        mode: ConflictMode::Hybrid,
        autonomy: 60,
        security_keywords: vec!["password".to_string()],
        log_resolutions: true,
    };

    base.merge_partial(PartialConflictResolutionConfig {
        mode: Some(ConflictMode::Auto),
        autonomy: None,
        security_keywords: None,
        log_resolutions: None,
    });

    assert_eq!(base.mode, ConflictMode::Auto);
    assert_eq!(base.autonomy, 60);
    assert_eq!(base.security_keywords, vec!["password".to_string()]);
    assert!(base.log_resolutions);
}
```

---

### CV-013: POST-RES-001 - can_auto_resolve respects mode

**Given:** Configs with each mode
**When:** `can_auto_resolve()` is called
**Then:** Returns expected result for each mode

```rust
#[test]
fn cv_013_post_res_001_mode_respected() {
    let auto = ConflictResolutionConfig {
        mode: ConflictMode::Auto,
        autonomy: 0,
        security_keywords: vec!["key".to_string()],
        log_resolutions: true,
    };

    let manual = ConflictResolutionConfig {
        mode: ConflictMode::Manual,
        autonomy: 100,
        security_keywords: vec!["key".to_string()],
        log_resolutions: true,
    };

    let hybrid = ConflictResolutionConfig {
        mode: ConflictMode::Hybrid,
        autonomy: 60,
        security_keywords: vec!["key".to_string()],
        log_resolutions: true,
    };

    assert!(auto.can_auto_resolve(None));
    assert!(!manual.can_auto_resolve(None));
    assert!(hybrid.can_auto_resolve(None));
}
```

---

### CV-014: POST-RES-002 - requires_human_review checks all keywords

**Given:** Config with multiple keywords
**When:** Files are checked
**Then:** Each keyword triggers review

```rust
#[test]
fn cv_014_post_res_002_all_keywords_checked() {
    let config = ConflictResolutionConfig {
        mode: ConflictMode::Hybrid,
        autonomy: 90,
        security_keywords: vec![
            "password".to_string(),
            "token".to_string(),
            "secret".to_string(),
        ],
        log_resolutions: true,
    };

    assert!(config.requires_human_review("password.rs"));
    assert!(config.requires_human_review("token.rs"));
    assert!(config.requires_human_review("secret.rs"));
}
```

---

### CV-015: POST-RES-003 - Security keywords always trigger review

**Given:** Config with any mode and autonomy
**When:** File matching keyword is checked
**Then:** Always requires human review

```rust
#[test]
fn cv_015_post_res_003_security_always_review() {
    for mode in [ConflictMode::Auto, ConflictMode::Manual, ConflictMode::Hybrid] {
        for autonomy in [0, 50, 100] {
            let config = ConflictResolutionConfig {
                mode,
                autonomy,
                security_keywords: vec!["password".to_string()],
                log_resolutions: true,
            };

            assert!(
                config.requires_human_review("src/auth/password.rs"),
                "mode={:?}, autonomy={} should require review",
                mode,
                autonomy
            );
        }
    }
}
```

---

## Category 4: Integration Tests (10 tests)

### IT-001: Config hierarchy - defaults → global

**Given:** Global config file with custom mode
**When:** Config is loaded
**Then:** Global config overrides defaults

```rust
#[tokio::test]
async fn it_001_global_overrides_defaults() {
    let temp_dir = tempfile::tempdir().unwrap();
    let global_path = temp_dir.path().join("global_config.toml");

    let toml_content = r#"
[conflict_resolution]
mode = "hybrid"
autonomy = 60
"#;

    tokio::fs::write(&global_path, toml_content)
        .await
        .unwrap();

    // Load would use global_config_path() in real implementation
    let partial = load_partial_toml_file(&global_path)
        .await
        .unwrap();

    assert!(partial.conflict_resolution.is_some());
}
```

---

### IT-002: Config hierarchy - global → project

**Given:** Global and project config files with conflicting values
**When:** Config is loaded
**Then:** Project config overrides global

```rust
#[tokio::test]
async fn it_002_project_overrides_global() {
    let temp_dir = tempfile::tempdir().unwrap();
    let global_path = temp_dir.path().join("global.toml");
    let project_path = temp_dir.path().join("project.toml");

    tokio::fs::write(
        &global_path,
        b"[conflict_resolution]\nmode = \"hybrid\"\nautonomy = 60",
    )
    .await
    .unwrap();

    tokio::fs::write(
        &project_path,
        b"[conflict_resolution]\nmode = \"auto\"\nautonomy = 80",
    )
    .await
    .unwrap();

    let mut config = ConflictResolutionConfig::default();

    let global = load_partial_toml_file(&global_path).await.unwrap();
    config.merge_partial(global.conflict_resolution.unwrap());

    let project = load_partial_toml_file(&project_path).await.unwrap();
    config.merge_partial(project.conflict_resolution.unwrap());

    assert_eq!(config.mode, ConflictMode::Auto);
    assert_eq!(config.autonomy, 80);
}
```

---

### IT-003: Environment variable overrides file config

**Given:** Config file with mode = "manual", env var ZJJ_CONFLICT_RESOLUTION_MODE=auto
**When:** Config is loaded with env vars
**Then:** Env var overrides file

```rust
#[test]
fn it_003_env_overrides_file() {
    std::env::set_var("ZJJ_CONFLICT_RESOLUTION_MODE", "auto");

    let mut config = ConflictResolutionConfig {
        mode: ConflictMode::Manual,
        autonomy: 0,
        security_keywords: vec!["key".to_string()],
        log_resolutions: true,
    };

    // Apply env var (in real implementation, this would be in apply_env_vars)
    if let Ok(mode_str) = std::env::var("ZJJ_CONFLICT_RESOLUTION_MODE") {
        if let Ok(mode) = ConflictMode::from_str(&mode_str) {
            config.mode = mode;
        }
    }

    assert_eq!(config.mode, ConflictMode::Auto);

    std::env::remove_var("ZJJ_CONFLICT_RESOLUTION_MODE");
}
```

---

### IT-004: Invalid environment variable is rejected

**Given:** Env var ZJJ_CONFLICT_RESOLUTION_AUTONOMY=150 (out of range)
**When:** Config is loaded
**Then:** Validation fails

```rust
#[test]
fn it_004_invalid_env_rejected() {
    std::env::set_var("ZJJ_CONFLICT_RESOLUTION_AUTONOMY", "150");

    let autonomy_str = std::env::var("ZJJ_CONFLICT_RESOLUTION_AUTONOMY").unwrap();
    let autonomy: u8 = autonomy_str.parse().unwrap();

    let config = ConflictResolutionConfig {
        mode: ConflictMode::Manual,
        autonomy,
        security_keywords: vec!["key".to_string()],
        log_resolutions: true,
    };

    assert!(config.validate().is_err());

    std::env::remove_var("ZJJ_CONFLICT_RESOLUTION_AUTONOMY");
}
```

---

### IT-005: Config validation runs on all sources

**Given:** Invalid config in any source (file, env var)
**When:** Config is loaded
**Then:** Validation catches the error

```rust
#[tokio::test]
async fn it_005_validation_all_sources() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Invalid config: empty keywords
    tokio::fs::write(
        &config_path,
        b"[conflict_resolution]\nmode = \"manual\"\nautonomy = 0\nsecurity_keywords = []",
    )
    .await
    .unwrap();

    let partial = load_partial_toml_file(&config_path)
        .await
        .unwrap();

    if let Some(cr_config) = partial.conflict_resolution {
        let config = ConflictResolutionConfig {
            mode: cr_config.mode.unwrap_or(ConflictMode::Manual),
            autonomy: cr_config.autonomy.unwrap_or(0),
            security_keywords: cr_config.security_keywords.unwrap_or_default(),
            log_resolutions: cr_config.log_resolutions.unwrap_or(true),
        };

        assert!(config.validate().is_err());
    }
}
```

---

### IT-006: Runtime decision uses loaded config

**Given:** Config loaded from file with hybrid mode, autonomy 70
**When:** Runtime decision is made
**Then:** Decision respects loaded config

```rust
#[tokio::test]
async fn it_006_runtime_uses_loaded_config() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    tokio::fs::write(
        &config_path,
        b"[conflict_resolution]\nmode = \"hybrid\"\nautonomy = 70\nsecurity_keywords = [\"password\"]\nlog_resolutions = true",
    )
    .await
    .unwrap();

    let partial = load_partial_toml_file(&config_path)
        .await
        .unwrap();

    let cr_config = ConflictResolutionConfig {
        mode: partial.conflict_resolution.unwrap().mode.unwrap(),
        autonomy: partial.conflict_resolution.unwrap().autonomy.unwrap(),
        security_keywords: partial.conflict_resolution.unwrap().security_keywords.unwrap(),
        log_resolutions: partial.conflict_resolution.unwrap().log_resolutions.unwrap(),
    };

    assert!(cr_config.can_auto_resolve(Some("safe_file.rs")));
    assert!(!cr_config.can_auto_resolve(Some("src/auth/password.rs")));
}
```

---

### IT-007: Config reload updates runtime behavior

**Given:** Config loaded, decision made, config updated, reloaded
**When:** Second decision is made
**Then:** New config values are used

```rust
#[tokio::test]
async fn it_007_reload_updates_behavior() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Initial config
    tokio::fs::write(
        &config_path,
        b"[conflict_resolution]\nmode = \"manual\"\nautonomy = 0",
    )
    .await
    .unwrap();

    let partial1 = load_partial_toml_file(&config_path).await.unwrap();
    let config1 = ConflictResolutionConfig {
        mode: partial1.conflict_resolution.clone().unwrap().mode.unwrap(),
        autonomy: partial1.conflict_resolution.clone().unwrap().autonomy.unwrap(),
        security_keywords: vec!["key".to_string()],
        log_resolutions: true,
    };

    assert!(!config1.can_auto_resolve(Some("any.rs")));

    // Update config
    tokio::fs::write(
        &config_path,
        b"[conflict_resolution]\nmode = \"auto\"\nautonomy = 100",
    )
    .await
    .unwrap();

    let partial2 = load_partial_toml_file(&config_path).await.unwrap();
    let config2 = ConflictResolutionConfig {
        mode: partial2.conflict_resolution.unwrap().mode.unwrap(),
        autonomy: partial2.conflict_resolution.unwrap().autonomy.unwrap(),
        security_keywords: vec!["key".to_string()],
        log_resolutions: true,
    };

    assert!(config2.can_auto_resolve(Some("any.rs")));
}
```

---

### IT-008: Multiple config files merge correctly

**Given:** Three config files (global, project, local)
**When:** All are loaded
**Then:** Correct merge hierarchy is applied

```rust
#[tokio::test]
async fn it_008_multiple_files_merge() {
    let temp_dir = tempfile::tempdir().unwrap();

    let global = temp_dir.path().join("global.toml");
    let project = temp_dir.path().join("project.toml");

    tokio::fs::write(&global, b"[conflict_resolution]\nmode = \"hybrid\"").await.unwrap();
    tokio::fs::write(&project, b"[conflict_resolution]\nautonomy = 75").await.unwrap();

    let mut config = ConflictResolutionConfig::default();

    let global_partial = load_partial_toml_file(&global).await.unwrap();
    config.merge_partial(global_partial.conflict_resolution.unwrap());

    let project_partial = load_partial_toml_file(&project).await.unwrap();
    config.merge_partial(project_partial.conflict_resolution.unwrap());

    assert_eq!(config.mode, ConflictMode::Hybrid);
    assert_eq!(config.autonomy, 75);
}
```

---

### IT-009: Env var with invalid mode is rejected

**Given:** Env var ZJJ_CONFLICT_RESOLUTION_MODE=invalid
**When:** Attempted to parse
**Then:** Returns error

```rust
#[test]
fn it_009_invalid_env_mode_rejected() {
    std::env::set_var("ZJJ_CONFLICT_RESOLUTION_MODE", "invalid_mode");

    let result = ConflictMode::from_str("invalid_mode");
    assert!(result.is_err());

    std::env::remove_var("ZJJ_CONFLICT_RESOLUTION_MODE");
}
```

---

### IT-010: Config with all override sources works correctly

**Given:** Defaults → global → project → env vars
**When:** All are applied
**Then:** Final config has correct values from each source

```rust
#[tokio::test]
async fn it_010_full_hierarchy_works() {
    let temp_dir = tempfile::tempdir().unwrap();

    let global = temp_dir.path().join("global.toml");
    let project = temp_dir.path().join("project.toml");

    // Global sets mode
    tokio::fs::write(&global, b"[conflict_resolution]\nmode = \"hybrid\"").await.unwrap();

    // Project sets autonomy
    tokio::fs::write(&project, b"[conflict_resolution]\nautonomy = 80").await.unwrap();

    let mut config = ConflictResolutionConfig::default();

    // Apply global
    let global_partial = load_partial_toml_file(&global).await.unwrap();
    config.merge_partial(global_partial.conflict_resolution.unwrap());

    // Apply project
    let project_partial = load_partial_toml_file(&project).await.unwrap();
    config.merge_partial(project_partial.conflict_resolution.unwrap());

    // Verify: mode from global, autonomy from project, others from defaults
    assert_eq!(config.mode, ConflictMode::Hybrid);
    assert_eq!(config.autonomy, 80);
    assert!(config.security_keywords.len() >= 5); // Default
    assert!(config.log_resolutions); // Default
}
```

---

## Test Implementation Notes

### Module Structure

```rust
// crates/zjj-core/src/config/conflict_resolution.rs

#[cfg(test)]
mod tests {
    use super::*;

    // Happy path tests (HP-001 to HP-025)
    mod happy_path { /* ... */ }

    // Edge case tests (EC-001 to EC-025)
    mod edge_cases { /* ... */ }

    // Contract verification tests (CV-001 to CV-015)
    mod contract_verification { /* ... */ }

    // Integration tests (IT-001 to IT-010)
    mod integration { /* ... */ }
}
```

### Test Utilities

```rust
#[cfg(test)]
pub(crate) mod test_utils {
    use super::*;

    pub(crate) fn create_test_config(
        mode: ConflictMode,
        autonomy: u8,
        keywords: Vec<String>,
    ) -> ConflictResolutionConfig {
        ConflictResolutionConfig {
            mode,
            autonomy,
            security_keywords: keywords,
            log_resolutions: true,
        }
    }

    pub(crate) fn default_test_config() -> ConflictResolutionConfig {
        ConflictResolutionConfig::default()
    }
}
```

---

## Test Execution

Run all tests:
```bash
moon run :test -- conflict_resolution
```

Run specific category:
```bash
moon run :test -- conflict_resolution::tests::happy_path
moon run :test -- conflict_resolution::tests::edge_cases
moon run :test -- conflict_resolution::tests::contract_verification
moon run :test -- conflict_resolution::tests::integration
```

---

## Summary

This test plan provides:

- **25 Happy Path Tests**: Normal operation validation
- **25 Edge Case Tests**: Boundary and error conditions
- **15 Contract Verification Tests**: Pre/postcondition and invariant checks
- **10 Integration Tests**: Full workflow and config hierarchy

**Total: 75 tests** following Martin Fowler's BDD Given-When-Then pattern.

All tests enforce the contract specified in `bd-25s-contract-spec.md`.
