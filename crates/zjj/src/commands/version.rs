//! Version command - detailed version information with JSON output (zjj-t6e)
//!
//! This command provides structured version information for AI agents
//! to perform compatibility checks.

use anyhow::Result;
use im;
use serde::Serialize;

/// Version output structure
#[derive(Debug, Serialize)]
pub struct VersionOutput {
    pub success: bool,
    pub version: VersionInfo,
}

/// Detailed version information
#[derive(Debug, Serialize)]
pub struct VersionInfo {
    /// Semantic version string
    pub semver: String,
    /// Major version number
    pub major: u32,
    /// Minor version number
    pub minor: u32,
    /// Patch version number
    pub patch: u32,
    /// Prerelease tag if any
    pub prerelease: Option<String>,
    /// Git commit hash (if available at build time)
    pub git_commit: Option<String>,
    /// Git branch (if available at build time)
    pub git_branch: Option<String>,
    /// Whether working directory was dirty at build time
    pub git_dirty: Option<bool>,
    /// Build timestamp
    pub build_date: Option<String>,
    /// Rust version used for compilation
    pub rust_version: Option<String>,
    /// Target triple
    pub target: String,
}

/// Run the version command
pub async fn run(json: bool) -> Result<()> {
    // Yield to make function legitimately async
    tokio::task::yield_now().await;

    let info = gather_version_info();
    let output = VersionOutput {
        success: true,
        version: info,
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_human_readable(&output.version);
    }

    Ok(())
}

/// Gather all version information
fn gather_version_info() -> VersionInfo {
    let semver = env!("CARGO_PKG_VERSION").to_string();

    // Parse major.minor.patch from semver
    let parts: im::Vector<&str> = semver.split('.').collect();
    let major = parts
        .iter()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let minor = parts
        .iter()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let patch_str = parts.iter().nth(2).unwrap_or(&"0");
    // Handle prerelease suffix (e.g., "0-alpha")
    let (patch, prerelease) = patch_str.find('-').map_or_else(
        || (patch_str.parse().unwrap_or(0), None),
        |idx| {
            let (p, pre) = patch_str.split_at(idx);
            (
                p.parse().unwrap_or(0),
                Some(pre[1..].to_string()), // Skip the '-'
            )
        },
    );

    VersionInfo {
        semver: semver.clone(),
        major,
        minor,
        patch,
        prerelease,
        git_commit: option_env!("GIT_COMMIT").map(String::from),
        git_branch: option_env!("GIT_BRANCH").map(String::from),
        git_dirty: option_env!("GIT_DIRTY").and_then(|s| s.parse().ok()),
        build_date: option_env!("BUILD_DATE").map(String::from),
        rust_version: option_env!("RUSTC_VERSION").map(String::from),
        target: get_target_triple(),
    }
}

/// Get target triple from environment
fn get_target_triple() -> String {
    // Use cfg attributes to construct target triple
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;
    format!("{arch}-{os}")
}

/// Print human-readable version
fn print_human_readable(info: &VersionInfo) {
    println!("jjz {}", info.semver);

    if let Some(ref commit) = info.git_commit {
        let dirty = info
            .git_dirty
            .map(|d| if d { " (dirty)" } else { "" })
            .unwrap_or("");
        println!("git: {commit}{dirty}");
    }

    if let Some(ref branch) = info.git_branch {
        println!("branch: {branch}");
    }

    if let Some(ref date) = info.build_date {
        println!("built: {date}");
    }

    if let Some(ref rust) = info.rust_version {
        println!("rust: {rust}");
    }

    println!("target: {}", info.target);
}
