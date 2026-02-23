# Validation API Reference

## Quick Import

```rust
// Import all validation functions
use zjj_core::validation::{
    validate_session_name,
    validate_agent_id,
    validate_workspace_name,
    validate_task_id,
    validate_bead_id,
    validate_session_id,
    validate_absolute_path,
    validate_path_exists,
    validate_session_workspace_exists,
    IdentifierError,
};

// Or import submodules
use zjj_core::validation::domain::*;
use zjj_core::validation::infrastructure::*;
use zjj_core::validation::validators::*;
```

## Domain Validation Functions

### validate_session_name

```rust
pub fn validate_session_name(s: &str) -> Result<(), IdentifierError>
```

Validates a session name according to domain rules:
- Must start with a letter (a-z, A-Z)
- Can contain letters, numbers, hyphens, underscores
- Maximum length: 63 characters
- Leading/trailing whitespace is trimmed

**Example**:
```rust
validate_session_name("my-session")?;
validate_session_name("my_session")?;
```

### validate_agent_id

```rust
pub fn validate_agent_id(s: &str) -> Result<(), IdentifierError>
```

Validates an agent ID:
- Can contain alphanumeric, hyphen, underscore, dot, colon
- Maximum length: 128 characters
- Non-empty

**Example**:
```rust
validate_agent_id("agent-123")?;
validate_agent_id("agent.example")?;
```

### validate_workspace_name

```rust
pub fn validate_workspace_name(s: &str) -> Result<(), IdentifierError>
```

Validates a workspace name:
- Cannot contain path separators (/ or \)
- Cannot contain null bytes
- Maximum length: 255 characters
- Non-empty

**Example**:
```rust
validate_workspace_name("my-workspace")?;
validate_workspace_name("my_workspace")?;
```

### validate_task_id

```rust
pub fn validate_task_id(s: &str) -> Result<(), IdentifierError>
```

Validates a task ID:
- Must start with "bd-" prefix
- Followed by hexadecimal characters (0-9, a-f, A-F)
- Non-empty after prefix

**Example**:
```rust
validate_task_id("bd-abc123")?;
validate_task_id("bd-ABC123DEF456")?;
```

### validate_bead_id

```rust
pub fn validate_bead_id(s: &str) -> Result<(), IdentifierError>
```

Alias for `validate_task_id` (beads and tasks use the same format).

### validate_session_id

```rust
pub fn validate_session_id(s: &str) -> Result<(), IdentifierError>
```

Validates a session ID:
- Must be ASCII-only
- Non-empty

**Example**:
```rust
validate_session_id("session-abc123")?;
validate_session_id("SESSION_ABC")?;
```

### validate_absolute_path

```rust
pub fn validate_absolute_path(s: &str) -> Result<(), IdentifierError>
```

Validates an absolute path:
- Must be absolute (starts with / on Unix)
- Cannot contain null bytes
- Non-empty

**Example**:
```rust
validate_absolute_path("/home/user")?;
validate_absolute_path("/tmp/workspace")?;
```

## Infrastructure Validation Functions

### validate_path_exists

```rust
pub fn validate_path_exists(path: &Path) -> Result<(), Error>
```

Checks if a path exists on the filesystem (I/O operation).

**Example**:
```rust
use std::path::Path;

validate_path_exists(Path::new("/tmp"))?;
```

### validate_is_directory

```rust
pub fn validate_is_directory(path: &Path) -> Result<(), Error>
```

Checks if a path is a directory (I/O operation).

**Example**:
```rust
validate_is_directory(Path::new("/tmp"))?;
```

### validate_is_file

```rust
pub fn validate_is_file(path: &Path) -> Result<(), Error>
```

Checks if a path is a file (I/O operation).

**Example**:
```rust
validate_is_file(Path::new("/etc/hosts"))?;
```

### validate_workspace_path

```rust
pub fn validate_workspace_path(path: &Path) -> Result<(), Error>
```

Combined validation: path exists AND is a directory.

**Example**:
```rust
validate_workspace_path(Path::new("/home/user/project"))?;
```

### validate_session_workspace_exists

```rust
pub fn validate_session_workspace_exists(session: &Session) -> Result<(), Error>
```

Validates that a session's workspace path exists (I/O operation).
Skipped if session status is `Creating`.

**Example**:
```rust
validate_session_workspace_exists(&session)?;
```

## Composable Validators

### Common Validators

```rust
// Not empty
let validator = not_empty::<String>();

// Alphanumeric only
let validator = is_alphanumeric::<String>();

// Pattern matching
let validator = matches_pattern::<String>(r"^[a-z]+$");

// Range check
let validator = in_range(1..=100);

// Min/max length
let validator = min_length(3);
let validator = max_length(10);

// One of allowed values
let validator = one_of(&["red", "green", "blue"]);
```

### Composition

```rust
// AND: both validators must pass
let valid = not_empty.and(min_length::<String>(3));

// Map error
let validator = not_empty.map_err(|e| ValidationError {
    message: format!("Custom: {}", e.message),
    field: Some("username".to_string()),
    value: None,
});
```

### Collection Validation

```rust
// All items must pass
validate_all(&items, validator)?;

// At least one item must pass
validate_any(&items, validator)?;

// No items should pass
validate_none(&items, validator)?;
```

## Error Types

### IdentifierError

Domain validation error with variants:
- `Empty` - Identifier is empty or whitespace-only
- `TooLong { max, actual }` - Exceeds maximum length
- `InvalidCharacters { details }` - Contains invalid characters
- `InvalidFormat { details }` - Generic format error
- `InvalidStart { expected }` - Wrong starting character
- `InvalidPrefix { prefix, value }` - Missing required prefix
- `InvalidHex { value }` - Invalid hexadecimal
- `NotAbsolutePath { value }` - Path not absolute
- `NullBytesInPath` - Path contains null bytes
- `NotAscii { value }` - Non-ASCII characters
- `ContainsPathSeparators` - Contains / or \

### ValidationError

Simplified error for validator composition:
```rust
pub struct ValidationError {
    pub message: String,
    pub field: Option<String>,
    pub value: Option<String>,
}
```

## Usage Patterns

### Parse at Boundaries

```rust
// Use validated newtypes for domain entities
let name = SessionName::parse("my-session")?;
let agent = AgentId::parse("agent-123")?;
let workspace = WorkspaceName::parse("my-workspace")?;

// These are guaranteed valid after construction
```

### Pure Validation

```rust
// For temporary/derived values
validate_session_name(temp_name)?;

// Compose validators
validate_session_and_agent("my-session", "agent-123")?;
```

### Infrastructure Validation

```rust
// For I/O operations
validate_session_workspace_exists(&session)?;
validate_workspace_path(Path::new("/home/user/project"))?;
```

## Testing

```bash
# Run all validation tests
cargo test --package zjj-core --lib validation

# Run specific module
cargo test --package zjj-core --lib validation::domain
cargo test --package zjj-core --lib validation::infrastructure
cargo test --package zjj-core --lib validation::validators
```

## Files

- `/home/lewis/src/zjj/crates/zjj-core/src/validation.rs` - Module root
- `/home/lewis/src/zjj/crates/zjj-core/src/validation/domain.rs` - Pure validation
- `/home/lewis/src/zjj/crates/zjj-core/src/validation/infrastructure.rs` - I/O validation
- `/home/lewis/src/zjj/crates/zjj-core/src/validation/validators.rs` - Composable patterns
