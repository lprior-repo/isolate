use std::{
    cell::RefCell,
    future::Future,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use tokio::task_local;

#[derive(Debug, Clone)]
struct CommandState {
    base_id: String,
    sequence: u64,
}

impl CommandState {
    fn next_id(&mut self, action: &str, target: &str) -> String {
        self.sequence = self.sequence.saturating_add(1);
        format!("{}:{}:{}:{}", self.base_id, self.sequence, action, target)
    }
}

task_local! {
    static COMMAND_STATE: RefCell<CommandState>;
}

static COMMAND_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn resolve_base_command_id(explicit: Option<&str>) -> String {
    explicit
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map_or_else(default_base_command_id, std::string::ToString::to_string)
}

fn default_base_command_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0_u128, |duration| duration.as_nanos());
    let pid = std::process::id();
    let counter = COMMAND_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("cmd-{pid}-{now}-{counter}")
}

pub async fn with_command_context<T, F>(base_id: String, future: F) -> T
where
    F: Future<Output = T>,
{
    COMMAND_STATE
        .scope(
            RefCell::new(CommandState {
                base_id,
                sequence: 0,
            }),
            future,
        )
        .await
}

pub fn next_write_command_id(action: &str, target: &str) -> Option<String> {
    COMMAND_STATE
        .try_with(|state| state.borrow_mut().next_id(action, target))
        .ok()
}
