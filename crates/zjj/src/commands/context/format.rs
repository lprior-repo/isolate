//! Output formatting functions using functional patterns

use super::types::{DependencyInfo, EnvironmentContext};

/// Format human-readable context output with functional composition
pub fn format_human_readable(ctx: &EnvironmentContext) {
    println!("ZJJ Context");
    println!("===========");
    println!();

    // Working directory section
    println!("Working Directory: {}", ctx.cwd);
    println!();

    // JJ repository section using functional pattern
    format_jj_section(ctx);

    // ZJJ section
    format_zjj_section(ctx);

    // Environment section using functional transformation
    format_environment_section(ctx);

    // Dependencies section
    format_dependencies_section(ctx);
}

/// Format JJ repository section using functional pattern
fn format_jj_section(ctx: &EnvironmentContext) {
    println!("JJ Repository:");

    if ctx.jj_repo {
        println!("  Status: Yes");

        ctx.jj_repo_root
            .as_ref()
            .iter()
            .for_each(|root| println!("  Root: {root}"));

        ctx.jj_current_branch
            .as_ref()
            .iter()
            .for_each(|branch| println!("  Branch: {branch}"));
    } else {
        println!("  Status: No");
    }

    println!();
}

/// Format ZJJ initialization section
fn format_zjj_section(ctx: &EnvironmentContext) {
    println!("ZJJ:");

    if ctx.zjj_initialized {
        println!("  Initialized: Yes");

        ctx.zjj_data_dir
            .as_ref()
            .iter()
            .for_each(|dir| println!("  Data Dir: {dir}"));

        println!(
            "  Sessions: {} total, {} active",
            ctx.sessions.total, ctx.sessions.active
        );

        ctx.sessions
            .current
            .as_ref()
            .iter()
            .for_each(|current| println!("  Current Session: {current}"));
    } else {
        println!("  Initialized: No");
    }

    println!();
}

/// Format environment section with functional transformation
fn format_environment_section(ctx: &EnvironmentContext) {
    println!("Environment:");

    // Format Zellij status using functional pattern
    let zellij_status = if ctx.environment.zellij_running {
        "Running"
    } else {
        "Not running"
    };
    println!("  Zellij: {zellij_status}");

    ctx.environment
        .zellij_session
        .as_ref()
        .iter()
        .for_each(|session| println!("  Zellij Session: {session}"));

    ctx.environment
        .pager
        .as_ref()
        .iter()
        .for_each(|pager| println!("  Pager: {pager}"));

    ctx.environment
        .editor
        .as_ref()
        .iter()
        .for_each(|editor| println!("  Editor: {editor}"));

    println!();
}

/// Format dependencies section
fn format_dependencies_section(ctx: &EnvironmentContext) {
    println!("Dependencies:");
    format_dependency("JJ", &ctx.dependencies.jj);
    format_dependency("Zellij", &ctx.dependencies.zellij);
}

/// Format a single dependency using functional composition
fn format_dependency(name: &str, dep: &DependencyInfo) {
    let status = if dep.installed { "✓" } else { "✗" };

    let version_str = dep
        .version
        .as_ref()
        .map(|v| format!(" ({v})"))
        .unwrap_or_default();

    println!("  {status} {name}{version_str}");
}
