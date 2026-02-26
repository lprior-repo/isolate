use anyhow::{Context, Result};
use futures::{StreamExt, TryStreamExt};

use crate::cli::run_command;

/// Execute `post_create` hooks in the workspace directory
pub(super) async fn execute_post_create_hooks(_workspace_path: &str) -> Result<()> {
    let should_fail_for_test = std::env::var("Isolate_TEST_MODE").ok().as_deref() == Some("1")
        && std::env::var("Isolate_TEST_FAIL_POST_CREATE_HOOK")
            .ok()
            .as_deref()
            == Some("1");

    if should_fail_for_test {
        anyhow::bail!("post_create hook failed (test)");
    }

    // TODO: Load hooks from config when isolate-4wn is complete
    // For now, use empty hook list
    let hooks: Vec<String> = Vec::new();

    futures::stream::iter(hooks)
        .map(Ok::<String, anyhow::Error>)
        .try_for_each(|hook| async move {
            run_command("sh", &["-c", &hook])
                .await
                .with_context(|| format!("Hook '{hook}' failed"))?;
            Ok(())
        })
        .await?;

    Ok(())
}
