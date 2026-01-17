//! Benchmarks for validation operations
//!
//! This benchmark suite measures performance of:
//! - Session name validation
//! - Path validation
//! - Config validation
//! - Input sanitization
//!
//! Run with: cargo bench --bench validation

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use zjj::session::validate_session_name;

/// Benchmark valid session name validation
fn bench_validate_valid_names(c: &mut Criterion) {
    let max_length = "a".repeat(64);
    let valid_names = vec![
        "simple",
        "my-session",
        "my_session",
        "MySession",
        "session123",
        "a",
        "feature-branch-123",
        max_length.as_str(), // Max length
    ];

    c.bench_function("validate_session_name_valid", |b| {
        b.iter(|| {
            for name in &valid_names {
                black_box(validate_session_name(name).ok());
            }
        });
    });
}

/// Benchmark invalid session name validation
fn bench_validate_invalid_names(c: &mut Criterion) {
    let too_long = "a".repeat(65);
    let invalid_names = vec![
        "",
        "-session",
        "_session",
        "123session",
        "session name",
        "session!",
        too_long.as_str(), // Too long
        "🚀session",
        "session@domain",
        "../session",
    ];

    c.bench_function("validate_session_name_invalid", |b| {
        b.iter(|| {
            for name in &invalid_names {
                black_box(validate_session_name(name).ok());
            }
        });
    });
}

/// Benchmark validation with varying name lengths
fn bench_validate_name_lengths(c: &mut Criterion) {
    let mut group = c.benchmark_group("validate_name_length");

    for length in &[1, 10, 32, 64, 100] {
        let name = "a".repeat(*length);
        group.bench_with_input(BenchmarkId::from_parameter(length), &name, |b, name| {
            b.iter(|| black_box(validate_session_name(name).ok()));
        });
    }

    group.finish();
}

/// Benchmark validation with special character detection
fn bench_validate_special_chars(c: &mut Criterion) {
    let names_with_special_chars = vec![
        "session!",
        "session@domain",
        "session#tag",
        "session$var",
        "session%20",
        "session&more",
        "session*",
        "session(1)",
        "session[0]",
        "session{id}",
    ];

    c.bench_function("validate_special_chars", |b| {
        b.iter(|| {
            for name in &names_with_special_chars {
                black_box(validate_session_name(name).ok());
            }
        });
    });
}

/// Benchmark validation with unicode detection
fn bench_validate_unicode(c: &mut Criterion) {
    let unicode_names = vec![
        "中文名字",
        "日本語",
        "café",
        "Ñoño",
        "🚀rocket",
        "naïve",
        "résumé",
        "аsession",            // Cyrillic
        "αlpha",               // Greek
        "session\u{200B}name", // Zero-width space
    ];

    c.bench_function("validate_unicode", |b| {
        b.iter(|| {
            for name in &unicode_names {
                black_box(validate_session_name(name).ok());
            }
        });
    });
}

/// Benchmark validation with path traversal attempts
fn bench_validate_path_traversal(c: &mut Criterion) {
    let path_traversal_names = vec![
        "../session",
        "../../session",
        "../../../etc",
        "/etc/passwd",
        "/session",
        "./session",
        "sessions/active",
        "session\\name",
        "..\\..\\session",
    ];

    c.bench_function("validate_path_traversal", |b| {
        b.iter(|| {
            for name in &path_traversal_names {
                black_box(validate_session_name(name).ok());
            }
        });
    });
}

/// Benchmark validation with control characters
fn bench_validate_control_chars(c: &mut Criterion) {
    let control_char_names = vec![
        "session\0name",
        "session\tname",
        "session\nname",
        "session\rname",
        "session\x07name",
    ];

    c.bench_function("validate_control_chars", |b| {
        b.iter(|| {
            for name in &control_char_names {
                black_box(validate_session_name(name).ok());
            }
        });
    });
}

/// Benchmark validation with SQL injection attempts
fn bench_validate_sql_injection(c: &mut Criterion) {
    let sql_injection_names = vec![
        "'; DROP TABLE sessions; --",
        "session\"OR\"1\"=\"1",
        "session' OR '1'='1",
        "session'; DELETE FROM sessions; --",
    ];

    c.bench_function("validate_sql_injection", |b| {
        b.iter(|| {
            for name in &sql_injection_names {
                black_box(validate_session_name(name).ok());
            }
        });
    });
}

/// Benchmark validation with shell metacharacters
fn bench_validate_shell_metacharacters(c: &mut Criterion) {
    let shell_metachar_names = vec![
        "session|command",
        "session;command",
        "session`command`",
        "session$(command)",
        "session&background",
        "session>output",
        "session<input",
    ];

    c.bench_function("validate_shell_metacharacters", |b| {
        b.iter(|| {
            for name in &shell_metachar_names {
                black_box(validate_session_name(name).ok());
            }
        });
    });
}

/// Benchmark validation performance under high load
fn bench_validate_high_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("validate_high_load");

    for count in &[100, 1000, 10000] {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            let names: Vec<String> = (0..count).map(|i| format!("session-{i}")).collect();

            b.iter(|| {
                for name in &names {
                    black_box(validate_session_name(name).ok());
                }
            });
        });
    }

    group.finish();
}

/// Benchmark validation with mixed valid and invalid names
fn bench_validate_mixed(c: &mut Criterion) {
    let mixed_names = vec![
        ("valid", "my-session"),
        ("invalid", "my session"),
        ("valid", "session123"),
        ("invalid", "123session"),
        ("valid", "feature-branch"),
        ("invalid", "feature/branch"),
        ("valid", "MySession"),
        ("invalid", "My@Session"),
    ];

    c.bench_function("validate_mixed", |b| {
        b.iter(|| {
            for (_expected, name) in &mixed_names {
                black_box(validate_session_name(name).ok());
            }
        });
    });
}

/// Benchmark early rejection (first character check)
fn bench_validate_early_rejection(c: &mut Criterion) {
    let names_rejected_early = vec![
        "-session", "_session", "0session", "123", " session", "@session",
    ];

    c.bench_function("validate_early_rejection", |b| {
        b.iter(|| {
            for name in &names_rejected_early {
                black_box(validate_session_name(name).ok());
            }
        });
    });
}

criterion_group!(
    benches,
    bench_validate_valid_names,
    bench_validate_invalid_names,
    bench_validate_name_lengths,
    bench_validate_special_chars,
    bench_validate_unicode,
    bench_validate_path_traversal,
    bench_validate_control_chars,
    bench_validate_sql_injection,
    bench_validate_shell_metacharacters,
    bench_validate_high_load,
    bench_validate_mixed,
    bench_validate_early_rejection,
);

criterion_main!(benches);
