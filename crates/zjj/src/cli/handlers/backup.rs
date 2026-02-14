//! Backup, export, and import handlers

use anyhow::Result;
use clap::ArgMatches;

use super::json_format::get_format;
use crate::commands::{backup, export_import};

pub async fn handle_backup(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);

    let create = sub_m.get_flag("create");
    let list = sub_m.get_flag("list");
    let restore = sub_m.get_one::<String>("restore");
    let status = sub_m.get_flag("status");
    let retention = sub_m.get_flag("retention");
    let timestamp = sub_m.get_one::<String>("timestamp").map(String::as_str);

    match (create, list, restore, status, retention) {
        (true, false, None, false, false) => backup::run_create(format).await,
        (false, true, None, false, false) => backup::run_list(format).await,
        (false, false, Some(database), false, false) => {
            if let Some(ts) = timestamp {
                if !is_valid_backup_timestamp(ts) {
                    anyhow::bail!(
                        "Invalid --timestamp '{ts}'. Expected format: YYYYMMDD-HHMMSS"
                    );
                }
            }
            backup::run_restore(database, timestamp, format).await
        }
        (false, false, None, true, false) => backup::run_status(format).await,
        (false, false, None, false, true) => backup::run_retention(format).await,
        _ => anyhow::bail!(
            "Unknown backup action. Use --create, --list, --restore <DATABASE>, --status, or --retention"
        ),
    }
}

fn is_valid_backup_timestamp(value: &str) -> bool {
    if value.len() != 15 {
        return false;
    }

    let bytes = value.as_bytes();
    if bytes.get(8).copied() != Some(b'-') {
        return false;
    }

    bytes
        .iter()
        .enumerate()
        .all(|(idx, b)| idx == 8 || b.is_ascii_digit())
}

pub async fn handle_export(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let session = sub_m.get_one::<String>("session").cloned();
    let output = sub_m.get_one::<String>("output").cloned();

    if let Some(ref session_name) = session {
        if looks_like_file_path(session_name) && output.is_none() {
            anyhow::bail!(
                "Ambiguous argument: '{session_name}' looks like a file path.\n\
                 \n\
                 If you meant to export TO a file, use the -o flag:\n\
                   zjj export -o {session_name}\n\
                 \n\
                 If '{session_name}' is actually a session name, please rename it\n\
                 or use the full path to disambiguate."
            );
        }
    }

    let options = export_import::ExportOptions {
        session,
        output,
        format,
    };
    export_import::run_export(&options).await
}

fn looks_like_file_path(s: &str) -> bool {
    let has_extension = s.contains('.')
        && s.split('.').next_back().is_some_and(|ext| {
            let ext_lower = ext.to_lowercase();
            matches!(
                ext_lower.as_str(),
                "json" | "yaml" | "yml" | "toml" | "txt" | "csv" | "xml"
            )
        });

    let has_path_sep = s.contains('/') || s.contains('\\');

    has_extension || has_path_sep
}

pub async fn handle_import(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let input = sub_m
        .get_one::<String>("file")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Input file is required"))?;
    let force = sub_m.get_flag("force");
    let skip_existing = sub_m.get_flag("skip-existing");
    let dry_run = sub_m.get_flag("dry-run");
    let options = export_import::ImportOptions {
        input,
        force,
        skip_existing,
        dry_run,
        format,
    };
    export_import::run_import(&options).await
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_looks_like_file_path_json_extension() {
        assert!(super::looks_like_file_path("export.json"));
        assert!(super::looks_like_file_path("data.JSON"));
        assert!(super::looks_like_file_path("backup.json"));
    }

    #[test]
    fn test_looks_like_file_path_other_extensions() {
        assert!(super::looks_like_file_path("config.yaml"));
        assert!(super::looks_like_file_path("data.yml"));
        assert!(super::looks_like_file_path("settings.toml"));
        assert!(super::looks_like_file_path("notes.txt"));
        assert!(super::looks_like_file_path("data.csv"));
        assert!(super::looks_like_file_path("config.xml"));
    }

    #[test]
    fn test_looks_like_file_path_with_path_separator() {
        assert!(super::looks_like_file_path("/tmp/export"));
        assert!(super::looks_like_file_path("./output"));
        assert!(super::looks_like_file_path("data/export"));
        assert!(super::looks_like_file_path("C:\\Users\\data"));
    }

    #[test]
    fn test_looks_like_file_path_valid_session_names() {
        assert!(!super::looks_like_file_path("feature-x"));
        assert!(!super::looks_like_file_path("main"));
        assert!(!super::looks_like_file_path("bugfix-123"));
        assert!(!super::looks_like_file_path("my-workspace"));
        assert!(!super::looks_like_file_path("dev"));
    }

    #[test]
    fn test_looks_like_file_path_edge_cases() {
        assert!(!super::looks_like_file_path("v1.2.3"));
        assert!(!super::looks_like_file_path("feature.test"));
        assert!(!super::looks_like_file_path("file.unknownext"));
    }

    #[test]
    fn test_valid_backup_timestamp_format() {
        assert!(super::is_valid_backup_timestamp("20250101-010101"));
        assert!(super::is_valid_backup_timestamp("19991231-235959"));
    }

    #[test]
    fn test_invalid_backup_timestamp_format() {
        assert!(!super::is_valid_backup_timestamp("20250101"));
        assert!(!super::is_valid_backup_timestamp("2025-0101-010101"));
        assert!(!super::is_valid_backup_timestamp("20250101_010101"));
        assert!(!super::is_valid_backup_timestamp("20250101-01010a"));
    }
}
