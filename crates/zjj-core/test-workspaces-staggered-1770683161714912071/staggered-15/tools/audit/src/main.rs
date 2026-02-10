//! ZJJ Codebase Auditor
//!
//! Systematic detection of forbidden patterns in Rust codebase.
//! Searches for: unwrap, expect, panic, todo, unimplemented, unsafe
//!
//! Usage: cargo run --manifest-path tools/audit/Cargo.toml

use std::{fs, path::Path};

use colored::Colorize;
use regex::Regex;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
struct Violation {
    file: String,
    line: usize,
    pattern: String,
    context: String,
}

#[derive(Debug)]
struct AuditReport {
    production_violations: Vec<Violation>,
    test_violations: Vec<Violation>,
}

impl AuditReport {
    fn total_violations(&self) -> usize {
        self.production_violations.len() + self.test_violations.len()
    }

    fn is_clean(&self) -> bool {
        self.production_violations.is_empty()
    }
}

fn is_comment_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!")
}

fn audit_codebase(root: &Path) -> Result<AuditReport, Box<dyn std::error::Error>> {
    let mut all_violations = Vec::new();

    // Forbidden patterns and their names
    let forbidden_patterns = vec![
        (r"\.unwrap\(\)", "unwrap"),
        (r"\.expect\(", "expect"),
        (r"panic!\(", "panic!"),
        (r"\btodo!\(", "todo!"),
        (r"\bunimplemented!\(", "unimplemented!"),
        (r"unsafe\s*\{", "unsafe"),
    ];

    let patterns: Vec<_> = forbidden_patterns
        .iter()
        .map(|(pat, name)| (Regex::new(pat).map(|r| (r, *name))))
        .collect::<Result<Vec<_>, _>>()?;

    // Walk all .rs files in crates/ directory
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        let path = entry.path();
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for (line_num, line) in content.lines().enumerate() {
            // Skip comment-only lines
            if is_comment_line(line) {
                continue;
            }

            for (regex, pattern_name) in &patterns {
                if regex.is_match(line) {
                    all_violations.push(Violation {
                        file: path
                            .strip_prefix(root)
                            .unwrap_or(path)
                            .display()
                            .to_string(),
                        line: line_num + 1,
                        pattern: pattern_name.to_string(),
                        context: line.trim().to_string(),
                    });
                }
            }
        }
    }

    // Separate production and test violations
    let (test_violations, production_violations): (Vec<_>, Vec<_>) = all_violations
        .into_iter()
        .partition(|v| v.file.contains("/tests/") || v.file.contains("/test_"));

    Ok(AuditReport {
        production_violations,
        test_violations,
    })
}

fn print_report(report: &AuditReport) {
    println!("\n{}", "=== ZJJ CODEBASE AUDIT REPORT ===".bold().cyan());
    println!();

    // Production violations (critical)
    if report.production_violations.is_empty() {
        println!(
            "{} {}",
            "✅".green(),
            "Production Code: CLEAN (0 violations)".green().bold()
        );
    } else {
        println!(
            "{} {}",
            "🔴".red(),
            format!(
                "Production Code: {} VIOLATIONS",
                report.production_violations.len()
            )
            .red()
            .bold()
        );
        for v in &report.production_violations {
            println!(
                "  {} {}:{} {} {}",
                "❌".red(),
                v.file.yellow(),
                v.line.to_string().yellow(),
                format!("[{}]", v.pattern).red(),
                v.context.dimmed()
            );
        }
    }

    println!();

    // Test violations (warnings)
    if report.test_violations.is_empty() {
        println!(
            "{} {}",
            "✅".green(),
            "Test Code: CLEAN (0 violations)".green().bold()
        );
    } else {
        println!(
            "{} {}",
            "⚠️ ".yellow(),
            format!("Test Code: {} violations", report.test_violations.len())
                .yellow()
                .bold()
        );

        // Group by pattern
        let mut by_pattern: std::collections::HashMap<String, Vec<&Violation>> =
            std::collections::HashMap::new();
        for v in &report.test_violations {
            by_pattern.entry(v.pattern.clone()).or_default().push(v);
        }

        for (pattern, violations) in by_pattern {
            println!(
                "  {} {} ({} occurrences)",
                "⚠️ ".yellow(),
                pattern.yellow(),
                violations.len()
            );
            // Show first 5 examples
            for v in violations.iter().take(5) {
                println!(
                    "    {} {}:{}",
                    "→".dimmed(),
                    v.file.dimmed(),
                    v.line.to_string().dimmed()
                );
            }
            if violations.len() > 5 {
                println!(
                    "    {} ... and {} more",
                    "→".dimmed(),
                    (violations.len() - 5).to_string().dimmed()
                );
            }
        }
    }

    println!();
    println!("{}", "=== SUMMARY ===".bold().cyan());
    println!();

    if report.is_clean() {
        println!(
            "{} {}",
            "🎯".green(),
            "Production code passes all checks!".green().bold()
        );
    } else {
        println!(
            "{} {}",
            "❌".red(),
            format!(
                "Production code has {} critical violations",
                report.production_violations.len()
            )
            .red()
            .bold()
        );
    }

    if report.test_violations.is_empty() {
        println!(
            "{} {}",
            "✅".green(),
            "Test code passes all checks!".green().bold()
        );
    } else {
        println!(
            "{} {}",
            "⚠️ ".yellow(),
            format!(
                "Test code has {} violations (low priority)",
                report.test_violations.len()
            )
            .yellow()
        );
    }

    println!();
    println!(
        "{} {} {}",
        "📊".cyan(),
        "Total violations:".bold(),
        report.total_violations()
    );
    println!();

    // Grade
    let grade = if report.is_clean() && report.test_violations.is_empty() {
        "A+".green().bold()
    } else if report.is_clean() {
        "A".green()
    } else if report.production_violations.len() < 5 {
        "B".yellow()
    } else {
        "C".red()
    };

    println!("{} {}", "🎓 Overall Grade:".bold(), grade);
    println!();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let crates_dir = Path::new("./crates");

    if !crates_dir.exists() {
        eprintln!(
            "{} Crates directory not found. Run from project root.",
            "❌".red()
        );
        std::process::exit(1);
    }

    println!();
    println!("{}", "🔍 Scanning codebase...".cyan().bold());
    println!();

    let report = audit_codebase(crates_dir)?;
    print_report(&report);

    // Exit with non-zero if production violations found
    if !report.is_clean() {
        std::process::exit(1);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_comment_line() {
        assert!(is_comment_line("// This is a comment"));
        assert!(is_comment_line("/// Doc comment"));
        assert!(is_comment_line("//! Module doc"));
        assert!(!is_comment_line("let x = 5; // comment"));
        assert!(!is_comment_line("    // comment"));
    }
}
