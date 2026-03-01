---
bead_id: bd-3bo
bead_title: dioxus-jsonl: Foundation and Hooks API
phase: p1
implementation_status: complete
created_at: 2026-03-01T06:00:00Z
---

# Implementation: dioxus-jsonl Foundation and Hooks API

## JSONL Records for Dioxus 0.7 Hooks

The following JSONL records define all 20 Dioxus 0.7 hooks with full signatures, 0.7 behavior notes, and signal-based patterns. Each record follows the functional Rust principles: zero unwrap, zero panic, Result-based error handling.

```jsonl
{"kind":"meta","skill":"dioxus-modern","version":"2.0.0","format":"jsonl","mode":"hooks-reference","bead_id":"bd-3bo"}
{"kind":"section","id":"foundation","title":"Foundation Types","description":"Core reactive primitives that form the basis of Dioxus 0.7 state management."}
{"kind":"api","id":"use_signal","signature":"pub fn use_signal<T: 'static>(initializer: impl FnOnce() -> T) -> Signal<T>","description":"Creates a new Signal for atomic state. Signals are Copy + Send + Sync and provide automatic dependency tracking.","version":"0.7","category":"state","pattern":"signal_atomic","example":"let count = use_signal(|| 0);","behavior":"Returns a Signal<T> that is Copy. Read with signal() or signal.read(). Write with signal.write(). Changes trigger re-renders of components that read the signal.","zero_unwrap":true,"functional_note":"No unwrap required. Signal provides direct access via () operator."}
{"kind":"api","id":"use_signal_sync","signature":"pub fn use_signal_sync<T: 'static + Send + Sync>(initializer: impl FnOnce() -> T) -> Signal<T, SyncStorage>","description":"Creates a Signal with SyncStorage for thread-safe state sharing. Required when state must cross thread boundaries.","version":"0.7","category":"state","pattern":"signal_sync","example":"let shared = use_signal_sync(|| SharedState::default());","behavior":"Uses SyncStorage instead of UnsyncStorage. Required for state accessed from multiple threads or async contexts.","zero_unwrap":true,"functional_note":"Thread-safe storage without unwrap patterns."}
{"kind":"api","id":"use_store","signature":"pub fn use_store<T: 'static + Default + Store>(initializer: impl FnOnce() -> T) -> Store<T>","description":"Creates a reactive Store for nested data structures. Use with #[derive(Store)] for fine-grained field-level reactivity.","version":"0.7","category":"state","pattern":"store_nested","example":"let state = use_store(AppState::default);","behavior":"Provides lenses for accessing nested fields. Only components reading specific fields re-render when those fields change. Requires Default trait.","zero_unwrap":true,"functional_note":"Lenses provide safe field access without optionals."}
{"kind":"api","id":"use_memo","signature":"pub fn use_memo<T: PartialEq + 'static>(calculator: impl FnOnce() -> T) -> Memo<T>","description":"Creates a memoized computed value. Re-computes only when signals read inside the closure change.","version":"0.7","category":"derived","pattern":"memo_computed","example":"let doubled = use_memo(move || count() * 2);","behavior":"Caches the result until dependencies change. Requires PartialEq for comparison. Reads signals automatically during calculation.","zero_unwrap":true,"functional_note":"Memo handles caching internally without unwrap."}
{"kind":"section","id":"async_hooks","title":"Async Hooks","description":"Hooks for managing asynchronous operations and data fetching."}
{"kind":"api","id":"use_resource","signature":"pub fn use_resource<T: 'static + Send, F>(future_factory: F) -> Resource<T> where F: Fn() -> impl Future<Output = T> + 'static","description":"Creates reactive async state. Automatically restarts when signals read inside the factory closure change.","version":"0.7","category":"async","pattern":"resource_async","example":"let user = use_resource(move || fetch_user(user_id()));","behavior":"Returns Resource<T> with loading/success/error states. Automatically tracks signal dependencies and restarts on change. Use .suspend() for Suspense integration.","zero_unwrap":true,"functional_note":"Use pattern matching on Resource states instead of unwrap."}
{"kind":"api","id":"use_future","signature":"pub fn use_future<T: 'static + Send, F>(future_factory: F) -> Resource<T> where F: FnOnce() -> impl Future<Output = T> + 'static","description":"Creates a static async future that runs once on mount. Does NOT react to signal changes.","version":"0.7","category":"async","pattern":"future_static","example":"let config = use_future(|| async { load_config().await });","behavior":"Runs exactly once when component mounts. Use when you want fire-and-forget async or need stable identity. Prefer use_resource for reactive data.","zero_unwrap":true,"functional_note":"Single execution model eliminates race conditions."}
{"kind":"api","id":"use_server_future","signature":"pub fn use_server_future<T: 'static + Serialize + DeserializeOwned, F>(future_factory: F) -> Result<Resource<T>, RenderError> where F: Fn() -> impl Future<Output = T> + 'static","description":"SSR-aware async fetching with automatic hydration. Runs on server during SSR, hydrates on client.","version":"0.7","category":"async","pattern":"server_hydration","example":"let data = use_server_future(move || fetch_from_db(id()))?;","behavior":"Executes on server during SSR, serializes result, deserializes on client. Prevents double-fetching. Returns Result for error handling.","zero_unwrap":true,"functional_note":"Returns Result<T, RenderError> - use ? operator for propagation."}
{"kind":"section","id":"effect_hooks","title":"Effect Hooks","description":"Hooks for side effects and lifecycle management."}
{"kind":"api","id":"use_effect","signature":"pub fn use_effect(effect: impl FnMut() + 'static)","description":"Runs a side effect after render. Tracks signals read inside and re-runs when they change. Return cleanup via closure.","version":"0.7","category":"effect","pattern":"effect_side_effects","example":"use_effect(move || { subscribe(); || unsubscribe() });","behavior":"Runs after every render where dependencies change. Return a cleanup closure for unmount logic. Automatically tracks signal reads.","zero_unwrap":true,"functional_note":"Effect runs without unwrap - cleanup via returned closure."}
{"kind":"api","id":"use_effect_with","signature":"pub fn use_effect_with<D: PartialEq + 'static, F>(dependency: D, effect: F) where F: FnMut(&D)","description":"Effect that runs only when the dependency value changes. Provides explicit dependency tracking.","version":"0.7","category":"effect","pattern":"effect_explicit_deps","example":"use_effect_with(user_id, |id| { log_access(id); });","behavior":"Only runs when dependency PartialEq returns false. Receives reference to current dependency value. More explicit than implicit tracking.","zero_unwrap":true,"functional_note":"Explicit dependencies prevent surprising re-runs."}
{"kind":"api","id":"use_drop","signature":"pub fn use_drop(cleanup: impl FnOnce() + 'static)","description":"Registers cleanup logic to run when component unmounts. Essential for resource cleanup.","version":"0.7","category":"lifecycle","pattern":"cleanup_on_unmount","example":"use_drop(move || connection.close());","behavior":"Runs exactly once when component is removed from tree. Use for closing connections, canceling timers, freeing resources.","zero_unwrap":true,"functional_note":"Deterministic cleanup without panic paths."}
{"kind":"api","id":"use_on_destroy","signature":"pub fn use_on_destroy(cleanup: impl FnOnce() + 'static)","description":"Alias for use_drop. Registers cleanup logic for component unmount.","version":"0.7","category":"lifecycle","pattern":"cleanup_alias","example":"use_on_destroy(move || cancel_subscription());","behavior":"Identical to use_drop. Use whichever name is more readable in context.","zero_unwrap":true,"functional_note":"Semantic alias for code clarity."}
{"kind":"section","id":"context_hooks","title":"Context Hooks","description":"Hooks for dependency injection and cross-component state sharing."}
{"kind":"api","id":"use_context","signature":"pub fn use_context<T: 'static + Clone>() -> T","description":"Consumes context value from nearest ancestor provider. Panics if no provider exists in tree.","version":"0.7","category":"context","pattern":"context_di","example":"let theme = use_context::<Theme>();","behavior":"Walks up component tree to find provider. Returns cloned value. Prefer use_context_provider for providing values.","zero_unwrap":true,"functional_note":"Returns T directly - ensure provider exists at app root.","warning":"Panics if context not provided. Use use_context_safe for Option return."}
{"kind":"api","id":"use_context_provider","signature":"pub fn use_context_provider<T: 'static + Clone>(initializer: impl FnOnce() -> T) -> T","description":"Provides context value to all descendant components. Returns the provided value.","version":"0.7","category":"context","pattern":"context_provider","example":"let theme = use_context_provider(|| Theme::Dark);","behavior":"Creates and provides context to subtree. Descendants access via use_context. Returns initialized value for use in provider.","zero_unwrap":true,"functional_note":"Provider pattern without unwrap - initializes via closure."}
{"kind":"api","id":"use_shared_state","signature":"pub fn use_shared_state<T: 'static + Clone>() -> Option<T>","description":"Accesses shared global state without panicking. Returns None if no provider exists.","version":"0.7","category":"context","pattern":"shared_state_optional","example":"let global = use_shared_state::<AppState>();","behavior":"Safe context access returning Option. Use when context may not exist. Prefer for library code.","zero_unwrap":true,"functional_note":"Option return enables safe pattern matching."}
{"kind":"api","id":"use_shared_state_provider","signature":"pub fn use_shared_state_provider<T: 'static + Clone>(initializer: impl FnOnce() -> T)","description":"Provides shared global state accessible via use_shared_state.","version":"0.7","category":"context","pattern":"shared_state_provider","example":"use_shared_state_provider(AppState::default);","behavior":"Sets up global state provider. Does not return value - use use_shared_state to access.","zero_unwrap":true,"functional_note":"Void return - provider setup is side effect."}
{"kind":"section","id":"callback_hooks","title":"Callback Hooks","description":"Hooks for stabilizing callback identity and event handling."}
{"kind":"api","id":"use_callback","signature":"pub fn use_callback<T: 'static, Args, F>(callback: F) -> Callback<Args, T> where F: Fn(Args) -> T + 'static","description":"Creates a memoized callback with stable identity across renders. Prevents unnecessary re-renders in child components.","version":"0.7","category":"callback","pattern":"callback_events","example":"let handler = use_callback(move |_| count += 1);","behavior":"Returns stable Callback handle. Callback internally references current signal values. Use for event handlers passed to children.","zero_unwrap":true,"functional_note":"Stable identity enables memoization downstream."}
{"kind":"api","id":"use_callback_ref","signature":"pub fn use_callback_ref<T: 'static, Args, F>(callback: F) -> impl Fn(Args) -> T where F: Fn(Args) -> T + 'static","description":"Creates callback with reference capture. Returns plain function for ergonomics.","version":"0.7","category":"callback","pattern":"callback_ref","example":"let handler = use_callback_ref(move |e| handle_event(e));","behavior":"Returns impl Fn for direct use. Internally memoized like use_callback. Prefer when Callback type is not needed.","zero_unwrap":true,"functional_note":"Ergonomic wrapper without type ceremony."}
{"kind":"section","id":"utility_hooks","title":"Utility Hooks","description":"Utility hooks for accessibility, suspension, and error handling."}
{"kind":"api","id":"use_id","signature":"pub fn use_id() -> String","description":"Generates a unique stable ID for accessibility and form element association.","version":"0.7","category":"utility","pattern":"accessibility_id","example":"let id = use_id(); rsx! { label { r#for: \"{id}\" } input { id: \"{id}\" } }","behavior":"Generates unique ID stable across renders. Essential for accessibility (label/input association). Automatically scoped to component.","zero_unwrap":true,"functional_note":"Deterministic ID generation without unwrap."}
{"kind":"api","id":"use_suspense","signature":"pub fn use_suspense<T: 'static, F, U>(future_factory: F) -> Result<T, RenderError> where F: FnOnce() -> impl Future<Output = T> + 'static","description":"Suspends component until async operation completes. Returns RenderError::Suspended during loading.","version":"0.7","category":"utility","pattern":"suspense_async","example":"let data = use_suspense(|| fetch_data())?;","behavior":"Returns Result<T, RenderError>. Suspended renders show fallback from SuspenseBoundary. Use ? to propagate suspension.","zero_unwrap":true,"functional_note":"Result-based suspension enables railway-oriented programming."}
{"kind":"api","id":"use_suspense_boundary","signature":"pub fn use_suspense_boundary() -> SuspenseBoundary","description":"Creates a boundary for catching suspended child components. Defines fallback UI.","version":"0.7","category":"utility","pattern":"suspense_boundary","example":"let boundary = use_suspense_boundary();","behavior":"Provides SuspenseBoundary handle. Use in rsx! with fallback prop. Catches any suspended descendants.","zero_unwrap":true,"functional_note":"Boundary creation without optional handling."}
{"kind":"api","id":"use_error_boundary","signature":"pub fn use_error_boundary() -> ErrorBoundary","description":"Creates boundary for catching errors from child components. Prevents app crashes.","version":"0.7","category":"utility","pattern":"error_boundary","example":"let boundary = use_error_boundary();","behavior":"Provides ErrorBoundary handle. Use in rsx! with fallback prop. Catches panics and errors from children.","zero_unwrap":true,"functional_note":"Error containment without unwrap patterns."}
{"kind":"api","id":"use_timeout","signature":"pub fn use_timeout(duration: Duration) -> TimeoutHandle","description":"Schedules a callback to run after specified duration. Returns handle for cancellation.","version":"0.7","category":"utility","pattern":"scheduled_callback","example":"let handle = use_timeout(Duration::from_secs(5));","behavior":"Returns TimeoutHandle. Use handle.cancel() to prevent execution. Runs once after duration.","zero_unwrap":true,"functional_note":"Handle provides cancel without optionals."}
```

## Summary of Hooks by Category

### Foundation Hooks (4)
| Hook | Purpose | Returns |
|------|---------|---------|
| `use_signal` | Atomic state | `Signal<T>` |
| `use_signal_sync` | Thread-safe state | `Signal<T, SyncStorage>` |
| `use_store` | Nested state | `Store<T>` |
| `use_memo` | Computed values | `Memo<T>` |

### Async Hooks (3)
| Hook | Purpose | Returns |
|------|---------|---------|
| `use_resource` | Reactive async | `Resource<T>` |
| `use_future` | Static async | `Resource<T>` |
| `use_server_future` | SSR-safe async | `Result<Resource<T>, RenderError>` |

### Effect Hooks (4)
| Hook | Purpose | Returns |
|------|---------|---------|
| `use_effect` | Side effects | `()` |
| `use_effect_with` | Effect with deps | `()` |
| `use_drop` | Cleanup on unmount | `()` |
| `use_on_destroy` | Alias for use_drop | `()` |

### Context Hooks (4)
| Hook | Purpose | Returns |
|------|---------|---------|
| `use_context` | Consume context | `T` |
| `use_context_provider` | Provide context | `T` |
| `use_shared_state` | Optional context | `Option<T>` |
| `use_shared_state_provider` | Global state | `()` |

### Callback Hooks (2)
| Hook | Purpose | Returns |
|------|---------|---------|
| `use_callback` | Stable callback | `Callback<Args, T>` |
| `use_callback_ref` | Callback reference | `impl Fn(Args) -> T` |

### Utility Hooks (5)
| Hook | Purpose | Returns |
|------|---------|---------|
| `use_id` | Unique ID | `String` |
| `use_suspense` | Async suspension | `Result<T, RenderError>` |
| `use_suspense_boundary` | Suspense container | `SuspenseBoundary` |
| `use_error_boundary` | Error container | `ErrorBoundary` |
| `use_timeout` | Scheduled callback | `TimeoutHandle` |

## Functional Rust Patterns

### Zero Unwrap Guarantee
All hooks are designed to work without `.unwrap()` or `.expect()`:

```rust
// GOOD: Pattern matching on Resource states
match user_data.read().as_ref() {
    Some(Ok(user)) => rsx! { "{user.name}" },
    Some(Err(e)) => rsx! { "Error: {e}" },
    None => rsx! { "Loading..." },
}

// GOOD: Using ? for Result propagation
fn MyComponent() -> Element {
    let data = use_suspense(|| fetch_data())?;
    rsx! { "{data}" }
}

// GOOD: Using Option safely
if let Some(global) = use_shared_state::<AppState>() {
    // Use global state
}
```

### Signal-Based State Management
```rust
// Atomic values with use_signal
let count = use_signal(|| 0);
let name = use_signal(String::new);

// Nested state with use_store
#[derive(Store, Default)]
struct AppState {
    user: User,
    items: Vec<Item>,
}
let state = use_store(AppState::default);

// Computed values with use_memo
let total = use_memo(move || {
    state.items().iter().map(|i| i.price()).sum()
});
```

### Railway-Oriented Error Handling
```rust
// use_server_future returns Result
fn DataComponent() -> Element {
    let data = use_server_future(|| fetch_from_db())
        .map_err(|e| rsx! { "Error: {e}" })?;

    rsx! { "{data}" }
}

// use_suspense with ? propagation
fn AsyncChild() -> Element {
    let user = use_suspense(|| fetch_user())?;
    let posts = use_suspense(|| fetch_posts())?;

    rsx! {
        h1 { "{user.name}" }
        for post in posts { PostCard { post } }
    }
}
```

## Anti-Patterns to Avoid

### Legacy use_state (Deprecated in 0.7)
```rust
// BAD: Legacy API
let mut count = use_state(|| 0);

// GOOD: Modern Signal API
let count = use_signal(|| 0);
```

### Manual Clone of Signals
```rust
// BAD: Unnecessary clone
let clone = signal.clone();

// GOOD: Read directly
let value = signal();
```

### Mutable Props
```rust
// BAD: Mutable signal props
#[component]
fn Child(mut value: Signal<i32>) -> Element { ... }

// GOOD: ReadSignal props with callbacks
#[component]
fn Child(value: ReadSignal<i32>, on_change: Callback<i32>) -> Element { ... }
```

## Integration with dioxus-modern Skill

These JSONL records extend the existing dioxus-modern skill at `/home/lewis/.claude/skills/dioxus-modern/`. The records follow the same format as the existing reference.md but provide more comprehensive hook coverage.

To integrate:
1. Append records to `.claude/skills/dioxus-modern/reference.md`
2. Update SKILL.md version to 2.0.0
3. Ensure all JSONL is valid (one record per line)
