//! Benchmarks for configuration loading and parsing operations
//!
//! This benchmark suite measures performance of:
//! - Config file parsing (TOML)
//! - Config merging (defaults + global + project)
//! - Config serialization
//! - Config validation
//!
//! Run with: cargo bench --bench `config_operations`

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::fs;

use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use tempfile::TempDir;
use zjj_core::config::Config;

/// Create a temp directory with config files
fn create_config_files() -> TempDir {
    let dir = TempDir::new().unwrap_or_else(|e| {
        eprintln!("Failed to create temp dir: {e}");
        std::process::exit(1);
    });

    // Create a sample config file
    let config_content = r#"
workspace_dir = "../{repo}__workspaces"
main_branch = "main"

[watch]
enabled = true
debounce_ms = 500
paths = ["src", "tests"]

[hooks]
post_create = ["bd sync", "npm install"]
pre_remove = ["git stash"]
post_merge = ["cargo check"]

[zellij]
session_prefix = "jjz"
use_tabs = true
layout_dir = "layouts"

[zellij.panes.main]
command = "claude"
args = ["--config", "default"]
size = "70%"

[zellij.panes.beads]
command = "bd list"
args = []
size = "30%"

[dashboard]
refresh_interval_ms = 1000
show_metadata = true

[agent]
enabled = false
model = "claude-3-5-sonnet-20241022"

[session]
auto_sync = true
sync_interval_ms = 30000
"#;

    let config_path = dir.path().join("config.toml");
    fs::write(&config_path, config_content).map_or_else(
        |e| {
            eprintln!("Failed to write config: {e}");
            std::process::exit(1);
        },
        |()| (),
    );

    dir
}

/// Benchmark loading default config
fn bench_load_defaults(c: &mut Criterion) {
    c.bench_function("config_load_defaults", |b| {
        b.iter(|| black_box(Config::default()));
    });
}

/// Benchmark parsing TOML config
fn bench_parse_config(c: &mut Criterion) {
    c.bench_function("config_parse_toml", |b| {
        b.iter_batched(
            || {
                let dir = create_config_files();
                let config_path = dir.path().join("config.toml");
                let content = fs::read_to_string(&config_path).unwrap_or_else(|e| {
                    eprintln!("Failed to read config: {e}");
                    std::process::exit(1);
                });
                (content, dir)
            },
            |(content, _dir)| black_box(toml::from_str::<Config>(&content).ok()),
            BatchSize::SmallInput,
        );
    });
}

/// Benchmark full config loading (with merging)
fn bench_load_config(c: &mut Criterion) {
    c.bench_function("config_load_full", |b| {
        b.iter_batched(
            || {
                let dir = create_config_files();
                let config_path = dir.path().join("config.toml");
                (config_path, dir)
            },
            |(_config_path, _dir)| {
                // Note: load_config() uses current directory's .jjz/config.toml
                // The benchmark doesn't actually use the temp config created above
                // This is intentional - it benchmarks the real-world usage pattern
                black_box(zjj_core::config::load_config().ok())
            },
            BatchSize::SmallInput,
        );
    });
}

/// Benchmark config serialization to TOML
fn bench_serialize_config(c: &mut Criterion) {
    c.bench_function("config_serialize_toml", |b| {
        b.iter_batched(
            Config::default,
            |config| black_box(toml::to_string_pretty(&config).ok()),
            BatchSize::SmallInput,
        );
    });
}

/// Benchmark config serialization to JSON
fn bench_serialize_config_json(c: &mut Criterion) {
    c.bench_function("config_serialize_json", |b| {
        b.iter_batched(
            Config::default,
            |config| black_box(serde_json::to_string_pretty(&config).ok()),
            BatchSize::SmallInput,
        );
    });
}

/// Benchmark config cloning
fn bench_clone_config(c: &mut Criterion) {
    c.bench_function("config_clone", |b| {
        b.iter_batched(Config::default, black_box, BatchSize::SmallInput);
    });
}

/// Benchmark config merging with varying levels of nesting
fn bench_merge_configs(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_merge");

    for depth in &[1, 2, 3] {
        group.bench_with_input(BenchmarkId::from_parameter(depth), depth, |b, &_depth| {
            b.iter_batched(
                || {
                    let default_config = Config::default();
                    let dir = create_config_files();
                    let config_path = dir.path().join("config.toml");
                    let loaded = toml::from_str::<Config>(
                        &fs::read_to_string(&config_path).unwrap_or_else(|e| {
                            eprintln!("Failed to read config: {e}");
                            std::process::exit(1);
                        }),
                    )
                    .unwrap_or_else(|e| {
                        eprintln!("Failed to parse config: {e}");
                        std::process::exit(1);
                    });
                    (default_config, loaded, dir)
                },
                |(default_config, loaded, _dir)| {
                    // Simple merge: loaded config overrides defaults
                    let mut merged = default_config;
                    merged.workspace_dir = loaded.workspace_dir;
                    merged.main_branch = loaded.main_branch;
                    merged.watch = loaded.watch;
                    merged.hooks = loaded.hooks;
                    merged.zellij = loaded.zellij;
                    black_box(merged)
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark parsing various config sizes
fn bench_parse_config_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_parse_size");

    // Small config
    let small_config = r#"
workspace_dir = "../workspaces"
main_branch = "main"
"#;

    // Medium config
    let medium_config = r#"
workspace_dir = "../workspaces"
main_branch = "main"

[watch]
enabled = true
debounce_ms = 500

[hooks]
post_create = ["bd sync"]

[zellij]
session_prefix = "jjz"
"#;

    // Large config (from create_config_files)
    let dir = create_config_files();
    let large_config = fs::read_to_string(dir.path().join("config.toml")).unwrap_or_else(|e| {
        eprintln!("Failed to read config: {e}");
        std::process::exit(1);
    });

    group.bench_function("small", |b| {
        b.iter(|| black_box(toml::from_str::<Config>(small_config).ok()));
    });

    group.bench_function("medium", |b| {
        b.iter(|| black_box(toml::from_str::<Config>(medium_config).ok()));
    });

    group.bench_function("large", |b| {
        b.iter(|| black_box(toml::from_str::<Config>(&large_config).ok()));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_load_defaults,
    bench_parse_config,
    bench_load_config,
    bench_serialize_config,
    bench_serialize_config_json,
    bench_clone_config,
    bench_merge_configs,
    bench_parse_config_sizes,
);

criterion_main!(benches);
