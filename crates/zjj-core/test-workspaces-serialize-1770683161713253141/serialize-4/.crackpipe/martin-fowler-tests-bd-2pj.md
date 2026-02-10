# Martin Fowler Test Plan for Bead bd-2pj

## Description
Web: web-017: Settings UI

## Test Strategy

### Overview
This test plan follows Martin Fowler's testing philosophy, emphasizing:
- **Unit tests** for isolated component behavior
- **Integration tests** for component interaction
- **Edge case testing** for boundary conditions
- **Performance tests** for non-functional requirements
- **Test doubles** for external dependencies

### Test Pyramid
```
        E2E Tests (5%)
       /             \
      /               \
     /    Integration  \
    /       Tests (15%) \
   /                     \
  /      Unit Tests (80%) \
 /_________________________\
```

## Unit Tests

### Test Suite: Settings Types

#### Test: Settings Construction
```rust
#[test]
fn test_settings_default_construction() {
    let settings = Settings::default();
    assert_eq!(settings.theme, Theme::System);
    assert_eq!(settings.language, "en".to_string());
    assert!(settings.notifications.enabled);
}

#[test]
fn test_settings_clone() {
    let settings = create_test_settings();
    let cloned = settings.clone();
    assert_eq!(settings, cloned);
}

#[test]
fn test_settings_serialization() {
    let settings = create_test_settings();
    let json = serde_json::to_string(&settings).unwrap();
    let deserialized: Settings = serde_json::from_str(&json).unwrap();
    assert_eq!(settings, deserialized);
}
```

### Test Suite: Validation

#### Test: Theme Validation
```rust
#[test]
fn test_validate_theme_valid() {
    let settings = Settings {
        theme: Theme::Dark,
        ..Default::default()
    };
    assert!(validate_settings(&settings).is_ok());
}

#[test]
fn test_validate_font_size_valid_range() {
    let settings = Settings {
        display: DisplaySettings {
            font_size: 14,
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(validate_settings(&settings).is_ok());
}

#[test]
fn test_validate_font_size_too_small() {
    let settings = Settings {
        display: DisplaySettings {
            font_size: 5,
            ..Default::default()
        },
        ..Default::default()
    };
    let result = validate_settings(&settings);
    assert!(result.is_err());
    match result {
        Err(SettingsError::InvalidFontSize(5)) => (),
        _ => panic!("Expected InvalidFontSize error"),
    }
}

#[test]
fn test_validate_font_size_too_large() {
    let settings = Settings {
        display: DisplaySettings {
            font_size: 30,
            ..Default::default()
        },
        ..Default::default()
    };
    let result = validate_settings(&settings);
    assert!(result.is_err());
}

#[test]
fn test_validate_language_code_valid() {
    let settings = Settings {
        language: "en".to_string(),
        ..Default::default()
    };
    assert!(validate_settings(&settings).is_ok());
}

#[test]
fn test_validate_language_code_invalid() {
    let settings = Settings {
        language: "xx".to_string(),
        ..Default::default()
    };
    let result = validate_settings(&settings);
    assert!(result.is_err());
}
```

### Test Suite: Persistence

#### Test: Load Settings Success
```rust
#[test]
fn test_load_settings_success() {
    let test_settings = create_test_settings();
    let mock_storage = MockStorage::with_data(test_settings.clone());
    
    let loaded = load_settings_from_storage(&mock_storage).unwrap();
    assert_eq!(loaded, test_settings);
}

#[test]
fn test_load_settings_file_not_found() {
    let mock_storage = MockStorage::empty();
    let result = load_settings_from_storage(&mock_storage);
    
    assert!(result.is_err());
    match result {
        Err(SettingsError::PersistenceError(_)) => (),
        _ => panic!("Expected PersistenceError"),
    }
}

#[test]
fn test_load_settings_corrupted_data() {
    let mock_storage = MockStorage::with_corrupted_data();
    let result = load_settings_from_storage(&mock_storage);
    
    assert!(result.is_err());
    match result {
        Err(SettingsError::CorruptedSettings(_)) => (),
        _ => panic!("Expected CorruptedSettings error"),
    }
}

#[test]
fn test_save_settings_success() {
    let settings = create_test_settings();
    let mock_storage = MockStorage::new();
    
    let result = save_settings_to_storage(&mock_storage, &settings);
    assert!(result.is_ok());
    assert!(mock_storage.was_called_with("save", &settings));
}

#[test]
fn test_save_settings_persistence_failure() {
    let settings = create_test_settings();
    let mock_storage = MockStorage::failing();
    
    let result = save_settings_to_storage(&mock_storage, &settings);
    assert!(result.is_err());
}
```

### Test Suite: UI Event Handling

#### Test: Theme Change Event
```rust
#[test]
fn test_handle_theme_change_event() {
    let current = Settings::default();
    let event = UiEvent::ThemeChanged(Theme::Dark);
    
    let result = handle_settings_event(event, &current);
    assert!(result.is_ok());
    
    match result {
        Ok(Some(new_settings)) => {
            assert_eq!(new_settings.theme, Theme::Dark);
        },
        _ => panic!("Expected Some(Settings)"),
    }
}

#[test]
fn test_handle_font_size_event_invalid() {
    let current = Settings::default();
    let event = UiEvent::FontSizeChanged(50); // Too large
    
    let result = handle_settings_event(event, &current);
    assert!(result.is_err());
    
    match result {
        Err(SettingsError::InvalidFontSize(50)) => (),
        _ => panic!("Expected InvalidFontSize error"),
    }
}

#[test]
fn test_handle_noop_event() {
    let current = Settings::default();
    let event = UiEvent::Noop;
    
    let result = handle_settings_event(event, &current);
    assert!(result.is_ok());
    
    match result {
        Ok(None) => (), // Expected
        _ => panic!("Expected None for Noop event"),
    }
}
```

## Integration Tests

### Test Suite: Settings Workflow

#### Test: Complete Settings Load-Modify-Save Cycle
```rust
#[tokio::test]
async fn test_complete_settings_workflow() {
    // Setup
    let storage = InMemoryStorage::new();
    let initial_settings = create_test_settings();
    save_settings_to_storage(&storage, &initial_settings).unwrap();
    
    // Load
    let loaded = load_settings_from_storage(&storage).unwrap();
    assert_eq!(loaded, initial_settings);
    
    // Modify
    let mut modified = loaded;
    modified.theme = Theme::Light;
    modified.display.font_size = 18;
    
    // Validate
    assert!(validate_settings(&modified).is_ok());
    
    // Save
    save_settings_to_storage(&storage, &modified).unwrap();
    
    // Verify persistence
    let reloaded = load_settings_from_storage(&storage).unwrap();
    assert_eq!(reloaded.theme, Theme::Light);
    assert_eq!(reloaded.display.font_size, 18);
}
```

#### Test: Error Recovery Workflow
```rust
#[tokio::test]
async fn test_error_recovery_with_defaults() {
    // Setup: Corrupted settings file
    let storage = InMemoryStorage::with_corrupted_data();
    
    // Attempt load (should fail)
    let load_result = load_settings_from_storage(&storage);
    assert!(load_result.is_err());
    
    // Reset to defaults
    let defaults = reset_to_defaults().unwrap();
    
    // Save defaults
    save_settings_to_storage(&storage, &defaults).unwrap();
    
    // Verify recovery
    let recovered = load_settings_from_storage(&storage).unwrap();
    assert_eq!(recovered, defaults);
}
```

### Test Suite: UI Integration

#### Test: Settings UI Rendering and Interaction
```rust
#[test]
fn test_settings_ui_rendering() {
    let settings = create_test_settings();
    let ui = render_settings_ui(&settings);
    
    // Verify UI contains expected elements
    assert!(ui.contains_element("theme-selector"));
    assert!(ui.contains_element("language-selector"));
    assert!(ui.contains_element("font-size-slider"));
}

#[test]
fn test_settings_ui_interaction_flow() {
    let settings = Settings::default();
    let ui = render_settings_ui(&settings);
    
    // Simulate user interaction
    let event = ui.simulate_click("theme-dark-button");
    let result = handle_settings_event(event, &settings);
    
    assert!(result.is_ok());
    match result {
        Ok(Some(new_settings)) => {
            assert_eq!(new_settings.theme, Theme::Dark);
        },
        _ => panic!("Expected updated settings"),
    }
}
```

## Edge Cases

### Test Suite: Boundary Conditions

#### Test: Font Size Boundaries
```rust
#[test]
fn test_font_size_minimum_boundary() {
    let settings = Settings {
        display: DisplaySettings {
            font_size: 10, // Minimum valid
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(validate_settings(&settings).is_ok());
}

#[test]
fn test_font_size_below_minimum() {
    let settings = Settings {
        display: DisplaySettings {
            font_size: 9, // Invalid
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(validate_settings(&settings).is_err());
}

#[test]
fn test_font_size_maximum_boundary() {
    let settings = Settings {
        display: DisplaySettings {
            font_size: 24, // Maximum valid
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(validate_settings(&settings).is_ok());
}

#[test]
fn test_font_size_above_maximum() {
    let settings = Settings {
        display: DisplaySettings {
            font_size: 25, // Invalid
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(validate_settings(&settings).is_err());
}
```

#### Test: Language Code Edge Cases
```rust
#[test]
fn test_language_code_empty() {
    let settings = Settings {
        language: "".to_string(),
        ..Default::default()
    };
    assert!(validate_settings(&settings).is_err());
}

#[test]
fn test_language_code_with_region() {
    let settings = Settings {
        language: "en-US".to_string(),
        ..Default::default()
    };
    assert!(validate_settings(&settings).is_ok());
}

#[test]
fn test_language_code_case_sensitivity() {
    let settings = Settings {
        language: "EN".to_string(), // Should be normalized
        ..Default::default()
    };
    // Should either accept or normalize
    assert!(validate_settings(&settings).is_ok());
}
```

#### Test: Concurrent Access
```rust
#[tokio::test]
async fn test_concurrent_settings_access() {
    let storage = Arc::new(InMemoryStorage::new());
    let settings = create_test_settings();
    save_settings_to_storage(&storage, &settings).unwrap();
    
    // Spawn concurrent tasks
    let tasks: Vec<_> = (0..10).map(|_| {
        let storage = Arc::clone(&storage);
        tokio::spawn(async move {
            load_settings_from_storage(&storage)
        })
    }).collect();
    
    // All tasks should succeed
    for task in tasks {
        let result = task.await.unwrap();
        assert!(result.is_ok());
    }
}
```

## Performance Tests

### Test Suite: Non-Functional Requirements

#### Test: Load Performance
```rust
#[tokio::test]
async fn test_load_settings_performance() {
    let storage = InMemoryStorage::with_large_dataset();
    let start = Instant::now();
    
    let _ = load_settings_from_storage(&storage).unwrap();
    
    let duration = start.elapsed();
    assert!(duration < Duration::from_millis(50), 
            "Load took {:?}", duration);
}
```

#### Test: Save Performance
```rust
#[tokio::test]
async fn test_save_settings_performance() {
    let storage = InMemoryStorage::new();
    let settings = create_test_settings();
    let start = Instant::now();
    
    save_settings_to_storage(&storage, &settings).unwrap();
    
    let duration = start.elapsed();
    assert!(duration < Duration::from_millis(100), 
            "Save took {:?}", duration);
}
```

#### Test: Validation Performance
```rust
#[test]
fn test_validate_settings_performance() {
    let settings = create_test_settings();
    let iterations = 1000;
    let start = Instant::now();
    
    for _ in 0..iterations {
        validate_settings(&settings).unwrap();
    }
    
    let duration = start.elapsed();
    let avg = duration / iterations;
    assert!(avg < Duration::from_micros(10), 
            "Average validation took {:?}", avg);
}
```

## Test Doubles

### Mocks

#### MockStorage
```rust
pub struct MockStorage {
    save_called: AtomicBool,
    load_called: AtomicBool,
    data: Arc<Mutex<Option<Settings>>>,
    should_fail: AtomicBool,
}

impl MockStorage {
    pub fn new() -> Self {
        Self {
            save_called: AtomicBool::new(false),
            load_called: AtomicBool::new(false),
            data: Arc::new(Mutex::new(None)),
            should_fail: AtomicBool::new(false),
        }
    }
    
    pub fn with_data(settings: Settings) -> Self {
        let mut mock = Self::new();
        *mock.data.lock().unwrap() = Some(settings);
        mock
    }
    
    pub fn failing() -> Self {
        let mut mock = Self::new();
        mock.should_fail.store(true, Ordering::SeqCst);
        mock
    }
    
    pub fn was_called_with(&self, operation: &str, data: &Settings) -> bool {
        match operation {
            "save" => self.save_called.load(Ordering::SeqCst),
            "load" => self.load_called.load(Ordering::SeqCst),
            _ => false,
        }
    }
}

impl Storage for MockStorage {
    fn load(&self) -> Result<Option<Settings>, StorageError> {
        self.load_called.store(true, Ordering::SeqCst);
        if self.should_fail.load(Ordering::SeqCst) {
            return Err(StorageError::IoError("Mock failure".into()));
        }
        Ok(self.data.lock().unwrap().clone())
    }
    
    fn save(&self, settings: &Settings) -> Result<(), StorageError> {
        self.save_called.store(true, Ordering::SeqCst);
        if self.should_fail.load(Ordering::SeqCst) {
            return Err(StorageError::IoError("Mock failure".into()));
        }
        *self.data.lock().unwrap() = Some(settings.clone());
        Ok(())
    }
}
```

### Fakes

#### InMemoryStorage
```rust
pub struct InMemoryStorage {
    data: Arc<RwLock<Option<Settings>>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(None)),
        }
    }
    
    pub fn with_data(settings: Settings) -> Self {
        let storage = Self::new();
        *storage.data.write().unwrap() = Some(settings);
        storage
    }
    
    pub fn with_corrupted_data() -> Self {
        let storage = Self::new();
        // Simulate corrupted data
        *storage.data.write().unwrap() = Some(Settings::default());
        storage
    }
}

impl Storage for InMemoryStorage {
    fn load(&self) -> Result<Option<Settings>, StorageError> {
        Ok(self.data.read().unwrap().clone())
    }
    
    fn save(&self, settings: &Settings) -> Result<(), StorageError> {
        *self.data.write().unwrap() = Some(settings.clone());
        Ok(())
    }
}
```

### Stubs

#### StubSettingsProvider
```rust
pub struct StubSettingsProvider {
    settings: Settings,
}

impl StubSettingsProvider {
    pub fn new() -> Self {
        Self {
            settings: Settings::default(),
        }
    }
    
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.settings.theme = theme;
        self
    }
    
    pub fn with_language(mut self, language: &str) -> Self {
        self.settings.language = language.to_string();
        self
    }
}

impl SettingsProvider for StubSettingsProvider {
    fn get_settings(&self) -> &Settings {
        &self.settings
    }
}
```

## Test Data Scenarios

### Scenario 1: First-Time User
- **Setup**: No existing settings file
- **Action**: Load settings
- **Expected**: Default settings created and returned
- **Test**: `test_load_settings_creates_defaults`

### Scenario 2: Upgrading User
- **Setup**: Old settings format file
- **Action**: Load settings
- **Expected**: Settings migrated to new format
- **Test**: `test_load_settings_migrates_old_format`

### Scenario 3: Invalid Settings Recovery
- **Setup**: Corrupted settings file
- **Action**: Load settings
- **Expected**: Error returned, defaults loaded on retry
- **Test**: `test_corrupted_settings_recovery`

### Scenario 4: Rapid Settings Changes
- **Setup**: Valid settings loaded
- **Action**: Change theme 10 times rapidly
- **Expected**: All changes validated, last change persisted
- **Test**: `test_rapid_settings_changes`

## Coverage Requirements

### Minimum Coverage Targets
- **Line Coverage**: 90%
- **Branch Coverage**: 85%
- **Function Coverage**: 100%

### Critical Path Coverage
- Settings load/save MUST be covered
- All validation error paths MUST be covered
- UI event handling MUST be covered
- Error recovery paths MUST be covered

### Coverage Exclusions
- Generated code (serde derives)
- Test helpers
- Debug/trace statements

## Acceptance Criteria Mapping

### AC1: Settings UI Display
- ✅ `test_settings_default_construction`
- ✅ `test_settings_ui_rendering`
- ✅ `test_complete_settings_workflow`

### AC2: User Interaction
- ✅ `test_handle_theme_change_event`
- ✅ `test_settings_ui_interaction_flow`
- ✅ `test_rapid_settings_changes`

### AC3: Error Handling
- ✅ `test_validate_font_size_too_small`
- ✅ `test_validate_font_size_too_large`
- ✅ `test_error_recovery_with_defaults`
- ✅ `test_corrupted_settings_recovery`

### AC4: Performance
- ✅ `test_load_settings_performance`
- ✅ `test_save_settings_performance`
- ✅ `test_validate_settings_performance`

## Test Organization

### Directory Structure
```
tests/
├── unit/
│   ├── settings_types_test.rs
│   ├── validation_test.rs
│   ├── persistence_test.rs
│   └── ui_events_test.rs
├── integration/
│   ├── workflow_test.rs
│   ├── ui_integration_test.rs
│   └── error_recovery_test.rs
├── edge_cases/
│   ├── boundaries_test.rs
│   └── concurrency_test.rs
├── performance/
│   └── benchmarks_test.rs
└── helpers/
    ├── mocks.rs
    ├── fakes.rs
    └── fixtures.rs
```

## Running Tests

### Unit Tests
```bash
cargo test --lib unit::
```

### Integration Tests
```bash
cargo test --test integration
```

### All Tests
```bash
moon run :test
```

### With Coverage
```bash
cargo tarpaulin --out Html
```
