# API Documentation Guide

This guide explains how to write clear, consistent, and useful API documentation for the zjj project using rustdoc.

## Table of Contents

1. [Documentation Principles](#documentation-principles)
2. [Rustdoc Conventions](#rustdoc-conventions)
3. [Documenting Public APIs](#documenting-public-apis)
4. [Documenting Result-Returning Functions](#documenting-result-returning-functions)
5. [Documenting Generic Types](#documenting-generic-types)
6. [Documenting Trait Implementations](#documenting-trait-implementations)
7. [Writing Documentation Examples](#writing-documentation-examples)
8. [Common Documentation Mistakes](#common-documentation-mistakes)
9. [Best Practices](#best-practices)
10. [Running cargo doc](#running-cargo-doc)
11. [Documentation Templates](#documentation-templates)

---

## Documentation Principles

### Core Values

1. **User-Centric**: Write for the person using the API, not the person who wrote it
2. **Complete**: Document all public items (types, functions, methods, traits)
3. **Accurate**: Examples must compile and pass `cargo test --doc`
4. **Clear**: Avoid jargon, explain concepts plainly
5. **Functional**: Show typical usage patterns and edge cases

### The "Why" Before the "What"

Good documentation explains:
- **What** the item does (brief description)
- **Why** you would use it (use cases)
- **When** to use it (context and alternatives)
- **How** to use it (examples)

---

## Rustdoc Conventions

### Basic Syntax

Rust uses `///` for item documentation and `//!` for module/module-level documentation:

```rust
//! # My Module
//!
//! This module provides useful functionality.

/// A brief description of a struct.
///
/// A more detailed explanation spanning multiple lines.
pub struct MyStruct;
```

### Sections and Headings

Use standard Markdown headings within documentation:

```rust
/// # Heading Level 1
///
/// ## Heading Level 2
///
/// Regular paragraph text.
///
/// - List item 1
/// - List item 2
///
/// 1. Numbered item
/// 2. Another numbered item
```

### Code Formatting

Use backticks for inline code and triple backticks for blocks:

```rust
/// Use `SessionName::parse()` to create a validated session name.
///
/// ```rust
/// use zjj_core::domain::SessionName;
///
/// let name = SessionName::parse("my-session")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
```

---

## Documenting Public APIs

### Structs

**Before (insufficient):**
```rust
/// A queue entry
pub struct QueueEntry {
    pub id: String,
    pub status: Status,
}
```

**After (complete):**
```rust
/// An entry in the work queue.
///
/// A queue entry represents a workspace waiting to be processed.
/// Each entry has a unique ID, a status tracking its lifecycle, and
/// metadata for deduplication and priority handling.
///
/// # Lifecycle
///
/// Entries progress through these states:
/// - `Pending`: Waiting to be claimed
/// - `Claimed`: Being processed by an agent
/// - `Completed`: Successfully processed
/// - `Failed`: Processing failed (retryable)
///
/// # Example
///
/// ```rust
/// use zjj_core::coordination::{PureEntry, QueueStatus};
///
/// let entry = PureEntry::new("workspace-1".to_string(), 0, 0);
/// assert!(entry.is_claimable());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueEntry {
    /// Unique identifier for this entry
    pub id: String,

    /// Current status in the queue lifecycle
    pub status: QueueStatus,

    /// Priority (lower = higher priority)
    pub priority: i32,

    /// Agent ID that claimed this entry (if claimed)
    pub claimed_by: Option<String>,

    /// Deduplication key to prevent duplicate work
    pub dedupe_key: Option<String>,
}
```

### Enums

**Before (minimal):**
```rust
pub enum QueueStatus {
    Pending,
    Claimed,
    Completed,
    Failed,
}
```

**After (complete):**
```rust
/// Status of a queue entry in its lifecycle.
///
/// Queue entries follow a state machine that ensures proper ordering
/// and prevents invalid transitions.
///
/// # State Transitions
///
/// ```text
///     Pending
///        │
///        ├───────────────► Claimed
///        │                    │
///        │                    ├───────► Completed
///        │                    │
///        │                    └───────► Failed ──┐
///        │                                     │
///        └─────────────────────────────────────┘
///                     (retry)
/// ```
///
/// # Variants
///
/// - `Pending`: Entry is waiting to be claimed
/// - `Claimed`: Entry is being processed by an agent
/// - `Completed`: Entry finished successfully
/// - `Failed`: Entry processing failed (may be retried)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueStatus {
    /// Entry is waiting to be claimed by an agent
    Pending,

    /// Entry is currently being processed
    Claimed,

    /// Entry completed successfully
    Completed,

    /// Entry failed during processing
    Failed,
}
```

### Functions and Methods

**Before (missing details):**
```rust
pub fn parse(name: &str) -> Result<SessionName, Error> {
    // ...
}
```

**After (complete):**
```rust
/// Parse and validate a session name.
///
/// This function follows the "parse at boundaries" DDD principle:
/// - Trims whitespace from input
/// - Validates according to naming rules
/// - Cannot represent invalid states
///
/// # Validation Rules
///
/// - Must start with a letter
/// - Can contain letters, numbers, hyphens, underscores
/// - Must be 1-63 characters
///
/// # Errors
///
/// Returns [`IdentifierError`] if the name violates validation rules:
/// - [`Empty`] if name is empty or whitespace-only
/// - [`TooLong`] if name exceeds 63 characters
/// - [`InvalidStart`] if name doesn't start with a letter
/// - [`InvalidCharacters`] if name contains disallowed characters
///
/// # Examples
///
/// ```rust
/// use zjj_core::domain::SessionName;
///
/// // Valid names
/// assert!(SessionName::parse("my-session").is_ok());
/// assert!(SessionName::parse("my_session_123").is_ok());
///
/// // Invalid names
/// assert!(SessionName::parse("").is_err());
/// assert!(SessionName::parse("123-session").is_err());
/// assert!(SessionName::parse("my.session").is_err());
/// ```
pub fn parse(name: &str) -> Result<SessionName, IdentifierError> {
    // ...
}
```

---

## Documenting Result-Returning Functions

### Always Document Errors

Every `Result<T, E>` must document:

1. **When** each error variant is returned
2. **Why** the error occurs
3. **How** to handle it

**Template:**

```rust
/// [Brief description]
///
/// [Detailed explanation]
///
/// # Errors
///
/// Returns [`ErrorType`] if:
/// - [`Variant1`]: [when this happens]
/// - [`Variant2`]: [when this happens]
/// - [`Variant3`]: [when this happens]
///
/// # Examples
///
/// ```rust
/// // Show both success and error cases
/// ```
```

### Example from zjj

```rust
/// Create a new invalid input error.
///
/// This is a convenience constructor for creating typed errors
/// with proper field validation.
///
/// # Errors
///
/// This function itself never returns an error, but the returned
/// [`ContractError::InvalidInput`] represents an error condition.
///
/// # Examples
///
/// ```rust
/// use zjj_core::cli_contracts::ContractError;
///
/// let error = ContractError::invalid_input("name", "cannot be empty");
/// assert!(matches!(error, ContractError::InvalidInput { .. }));
/// ```
#[must_use]
pub fn invalid_input(field: &'static str, reason: impl Into<String>) -> Self {
    Self::InvalidInput {
        field,
        reason: reason.into(),
    }
}
```

---

## Documenting Generic Types

### Type Parameters

Always document:
- What each type parameter represents
- Any constraints (trait bounds)
- Typical usage patterns

**Before (unclear):**
```rust
pub struct Container<T> {
    inner: Vec<T>,
}
```

**After (clear):**
```rust
/// A container for items of type `T`.
///
/// This container maintains insertion order and provides
/// O(1) access by index.
///
/// # Type Parameters
///
/// - `T`: The type of items stored in the container. Must implement
///   [`Clone`] to support internal operations, and [`Debug`] for
///   diagnostic output.
///
/// # Examples
///
/// ```rust
/// use mycrate::Container;
///
/// let mut container = Container::new();
/// container.push("item");
/// ```
pub struct Container<T: Clone + std::fmt::Debug> {
    /// The underlying storage
    inner: Vec<T>,
}
```

### Lifetime Parameters

**Example:**
```rust
/// A view into a slice of bytes.
///
/// This type provides a read-only window into existing byte data
/// without taking ownership.
///
/// # Lifetime
///
/// The `'a` lifetime represents the duration of the reference to
/// the underlying bytes. The `ByteView` cannot outlive the data
/// it references.
///
/// # Examples
///
/// ```rust
/// use mycrate::ByteView;
///
/// let data = b"hello world";
/// let view = ByteView::new(data);
/// assert_eq!(view.as_slice(), b"hello");
/// ```
pub struct ByteView<'a> {
    slice: &'a [u8],
}
```

---

## Documenting Trait Implementations

### Trait Definitions

**Example:**
```rust
/// Trait for types that can be validated and converted from strings.
///
/// This trait encodes the "parse, don't validate" principle: validation
/// happens at the boundary during parsing, and the resulting type
/// guarantees invariants.
///
/// # Required Methods
///
/// Implementers must provide [`parse`](Self::parse), which validates
/// input and returns the validated type or an error.
///
/// # Examples
///
/// ```rust
/// use mycrate::ParseValidated;
///
/// #[derive(Debug)]
/// struct Username(String);
///
/// impl ParseValidated for Username {
///     type Error = ParseError;
///
///     fn parse(s: &str) -> Result<Self, Self::Error> {
///         if s.len() < 3 {
///             return Err(ParseError::TooShort);
///         }
///         Ok(Username(s.to_string()))
///     }
/// }
/// ```
pub trait ParseValidated: Sized {
    /// Error type returned when parsing fails
    type Error: std::error::Error;

    /// Parse and validate from a string
    ///
    /// # Errors
    ///
    /// Returns [`Self::Error`] if validation fails
    fn parse(s: &str) -> Result<Self, Self::Error>;
}
```

### Impl Blocks

Document trait implementations when they add specific behavior:

```rust
impl TryFrom<String> for SessionName {
    type Error = IdentifierError;

    /// Attempt to convert a `String` into a `SessionName`.
    ///
    /// This delegates to [`SessionName::parse`] and performs
    /// full validation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zjj_core::domain::SessionName;
    /// use std::convert::TryFrom;
    ///
    /// let name = SessionName::try_from("my-session".to_string())?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}
```

---

## Writing Documentation Examples

### The Three Levels of Examples

Good documentation includes:

1. **Basic Usage**: Simple, common case
2. **Error Handling**: Show failure modes
3. **Advanced Patterns**: Real-world usage

### Complete Example Template

```rust
/// [Function summary]
///
/// [Detailed explanation]
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust
/// // Simple, happy-path example
/// ```
///
/// ## Error Handling
///
/// ```rust
/// // Show how to handle errors
/// ```
///
/// ## Advanced Usage
///
/// ```rust
/// // Show integration with other features
/// ```
///
/// # Errors
///
/// [Document all error cases]
///
/// # Panics
///
/// [If applicable, document when it panics (prefer Result)]
///
/// # Safety
///
/// [If unsafe, document invariants and preconditions]
```

### Making Examples Testable

Use `#` to hide setup code from docs but test it:

```rust
/// ```rust
/// use zjj_core::domain::SessionName;
///
/// let name = SessionName::parse("my-session")?;
/// assert_eq!(name.as_str(), "my-session");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```

The `# Ok::<(), ...>(())` line makes the example compile as a test
 but doesn't show in the rendered documentation.
```

---

## Common Documentation Mistakes

### 1. Missing Error Documentation

**Bad:**
```rust
/// Parse a session name
pub fn parse(s: &str) -> Result<SessionName, Error>
```

**Good:**
```rust
/// Parse and validate a session name.
///
/// # Errors
///
/// Returns [`Error::InvalidName`] if:
/// - Name is empty
/// - Name exceeds 63 characters
/// - Name contains invalid characters
pub fn parse(s: &str) -> Result<SessionName, Error>
```

### 2. Vague Descriptions

**Bad:**
```rust
/// Process the data
pub fn process(data: &Data) -> Result<Output>
```

**Good:**
```rust
/// Validates, transforms, and persists the provided data.
///
/// This performs three steps:
/// 1. Schema validation
/// 2. Business rule application
/// 3. Database persistence
pub fn process(data: &Data) -> Result<Output>
```

### 3. Untestable Examples

**Bad (assumes external state):**
```rust
/// ```rust
/// let config = load_config();
/// ```
```

**Good (self-contained):**
```rust
/// ```rust
/// let config = Config::builder()
///     .with_max_connections(10)
///     .build();
/// ```
```

### 4. Missing Generic Constraints

**Bad:**
```rust
/// Container for T
pub struct Container<T>
```

**Good:**
```rust
/// Container for items of type `T`
///
/// # Type Parameters
///
/// - `T`: Item type. Must implement [`Clone`] for internal operations
pub struct Container<T: Clone>
```

### 5. Copying Implementation Details

**Bad:**
```rust
/// Checks if name length > 0 and returns Ok(name)
pub fn parse(name: &str) -> Result<Name>
```

**Good:**
```rust
/// Parse and validate a name.
///
/// Names must be non-empty and contain only alphanumeric characters.
pub fn parse(name: &str) -> Result<Name>
```

---

## Best Practices

### 1. Use Intra-Doc Links

Link to related items using brackets:

```rust
/// Parse a [`SessionName`] from a string.
///
/// See [`IdentifierError`] for all possible validation errors.
///
/// This is equivalent to calling [`SessionName::parse`].
```

### 2. Follow Rust API Guidelines

- Use `///` for item docs
- Use `//!` for module docs
- Start with a one-sentence summary
- Use present tense ("returns" not "returned")
- Use imperative mood for examples

### 3. Document Invariants

For types that maintain guarantees:

```rust
/// A validated session name.
///
/// # Guarantees
///
/// - Non-empty
/// - Starts with a letter
/// - Contains only alphanumeric, hyphen, underscore
/// - 1-63 characters
///
/// These guarantees are enforced at construction time via
/// [`SessionName::parse`], making invalid states unrepresentable.
#[derive(Debug, Clone)]
pub struct SessionName(String);
```

### 4. Document Performance Characteristics

For performance-sensitive APIs:

```rust
/// Remove an entry from the queue.
///
/// # Performance
///
/// - **Time**: O(n) where n is the queue length
/// - **Space**: O(1)
///
/// For bulk removals, use [`remove_many`] which is O(n) total.
pub fn remove(&mut self, id: &str) -> Result<()>
```

### 5. Document Thread Safety

For concurrent code:

```rust
/// Get the current configuration.
///
/// # Thread Safety
///
/// This method uses internal locking and may be called concurrently
/// from multiple threads. The lock is held for the duration of the
/// call, which is O(1).
pub fn get_config(&self) -> Config
```

### 6. Use `#[must_use]`

Tag functions that return values that should be used:

```rust
/// Create a new queue entry.
///
/// # Note
///
/// This entry is not added to the queue. Use [`Queue::push`] to
/// add it.
#[must_use]
pub fn entry(workspace: String, priority: i32) -> Entry
```

---

## Running cargo doc

### Generate Documentation

```bash
# Generate documentation for all packages
cargo doc --all

# Open documentation in browser
cargo doc --open

# Include documentation for private items
cargo doc --document-private-items

# Generate for specific package
cargo doc -p zjj-core
```

### Test Documentation Examples

```bash
# Run documentation tests
cargo test --doc

# Run with verbose output
cargo test --doc -- --nocapture

# Run only doc tests for a specific module
cargo test --doc zjj_core::domain::identifiers
```

### Continuous Integration

Add to CI pipeline:

```yaml
# .github/workflows/doc.yml
name: Documentation
on: [push, pull_request]

jobs:
  doc:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: stable
      - run: cargo doc --all --document-private-items
      - run: cargo test --doc
```

---

## Documentation Templates

### Template 1: Public Function

```rust
/// [One-line summary]
///
/// [Detailed paragraph explaining what the function does and when to use it]
///
/// # Arguments
///
/// * `arg1` - [Description]
/// * `arg2` - [Description]
///
/// # Returns
///
/// [Description of return value]
///
/// # Errors
///
/// Returns [`ErrorType`] if:
/// - [`Variant1`]: [When this happens]
/// - [`Variant2`]: [When this happens]
///
/// # Examples
///
/// ```
/// use crate_name::Type;
///
/// let result = Type::function(arg1, arg2)?;
/// # Ok::<(), ErrorType>(())
/// ```
pub fn function_name(arg1: Type1, arg2: Type2) -> Result<ReturnType, ErrorType>
```

### Template 2: Public Struct

```rust
/// [One-line summary of what the struct represents]
///
/// [Detailed explanation of the struct's purpose and usage]
///
/// # Fields
///
/// - [`field1`]: [Description]
/// - [`field2`]: [Description]
///
/// # Examples
///
/// ```
/// use crate_name::StructName;
///
/// let instance = StructName {
///     field1: value1,
///     field2: value2,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct StructName {
    /// [Field description]
    pub field1: Type1,

    /// [Field description]
    pub field2: Type2,
}
```

### Template 3: Domain Type (Newtype)

```rust
/// A validated [what this represents].
///
/// # Construction
///
/// ```
/// use crate_name::TypeName;
///
/// let instance = TypeName::parse("input")?;
/// # Ok::<(), ErrorType>(())
/// ```
///
/// # Guarantees
///
/// - [Guarantee 1]
/// - [Guarantee 2]
/// - [Guarantee 3]
///
/// These guarantees are enforced at construction time, making
/// invalid states unrepresentable.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeName(String);

impl TypeName {
    /// Parse and validate [type name].
    ///
    /// # Errors
    ///
    /// Returns [`ErrorType`] if:
    /// - [When validation fails]
    pub fn parse(s: impl Into<String>) -> Result<Self, ErrorType> {
        // ...
    }

    /// Get the inner value
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

### Template 4: Error Enum

```rust
/// Errors that can occur when [doing what].
///
/// These errors represent expected domain failures that callers
/// should handle explicitly.
///
/// # Error Categories
///
/// 1. **[Category1]**: [Description]
/// 2. **[Category2]**: [Description]
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ErrorType {
    /// [Brief description]
    #[error("[message]")]
    Variant1,
    // ... more variants
}
```

### Template 5: Trait

```rust
/// Trait for [what the trait abstracts].
///
/// [Detailed explanation of when to implement this trait
///  and what behavior it guarantees]
///
/// # Required Methods
///
/// Implementers must provide:
/// - [`method1`]: [What it must do]
/// - [`method2`]: [What it must do]
///
/// # Examples
///
/// ```
/// use crate_name::TraitName;
///
/// struct MyImpl;
///
/// impl TraitName for MyImpl {
///     fn method1(&self) -> Result<()> {
///         // ...
///     }
/// }
/// ```
pub trait TraitName {
    /// [Method summary]
    ///
    /// [Detailed explanation]
    ///
    /// # Errors
    ///
    /// [When it returns errors]
    fn method_name(&self) -> Result<()>;
}
```

---

## Quick Reference Checklist

Use this checklist when reviewing API documentation:

### Structure
- [ ] One-sentence summary at the top
- [ ] Detailed explanation below summary
- [ ] Examples section
- [ ] Errors section (for `Result`-returning functions)
- [ ] Panics section (if applicable)

### Content
- [ ] Explains "what" (what it does)
- [ ] Explains "why" (why use it)
- [ ] Explains "when" (when to use it vs alternatives)
- [ ] Explains "how" (examples)
- [ ] All public types are linked with backticks

### Examples
- [ ] Compiles without external dependencies
- [ ] Shows typical usage
- [ ] Shows error handling
- [ ] Uses `#` for setup code that shouldn't be shown
- [ ] Tested with `cargo test --doc`

### Types
- [ ] Generic parameters documented
- [ ] Lifetime parameters explained
- [ ] Trait bounds justified
- [ ] Field documentation on public structs

### Errors
- [ ] All error variants listed
- [ ] When each error occurs
- [ ] How to recover/handle each error

---

## Additional Resources

- [The Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [How to write documentation](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html)
- [The rustdoc book](https://doc.rust-lang.org/rustdoc/)
- [Rust by Example - Documentation](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html)

---

## Real Example from zjj

Here's a complete example following all guidelines:

```rust
//! Semantic newtypes for domain identifiers
//!
//! # Parse-at-Boundaries Pattern
//!
//! Each identifier type:
//! - Validates its input on construction (parse-once pattern)
//! - Trims whitespace before validation (boundary sanitization)
//! - Cannot represent invalid states
//! - Provides safe access to the underlying value
//! - Implements serde serialization/deserialization with validation

/// A validated session name
///
/// # Construction
///
/// ```rust
/// use zjj_core::domain::SessionName;
///
/// let name = SessionName::parse("my-session")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Guarantees
///
/// - Non-empty
/// - Starts with a letter
/// - Contains only alphanumeric, hyphen, underscore
/// - 1-63 characters
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct SessionName(String);

impl SessionName {
    /// Maximum allowed length for a session name
    pub const MAX_LENGTH: usize = 63;

    /// Parse and validate a session name (trims whitespace first)
    ///
    /// This follows the "parse at boundaries" DDD principle:
    /// - Trims whitespace from input
    /// - Validates once at construction
    /// - Cannot represent invalid states
    ///
    /// # Errors
    ///
    /// Returns [`IdentifierError`] if the name is invalid:
    /// - [`Empty`] if name is empty or whitespace-only
    /// - [`TooLong`] if name exceeds 63 characters
    /// - [`InvalidStart`] if name doesn't start with a letter
    /// - [`InvalidCharacters`] if name contains disallowed characters
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        let trimmed = s.trim();
        validate_session_name(trimmed)?;
        Ok(Self(trimmed.to_string()))
    }

    /// Get the session name as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

---

## Summary

Good API documentation is part of the API itself. Following these guidelines ensures that:

1. **Users can discover** functionality through rustdoc
2. **Examples stay valid** through `cargo test --doc`
3. **Intent is clear** from reading the docs
4. **Edge cases are documented** and expected
5. **The codebase is maintainable** by future contributors

Remember: Documentation that isn't tested will break. Documentation that isn't read won't help. Write docs that are tested and worth reading.
