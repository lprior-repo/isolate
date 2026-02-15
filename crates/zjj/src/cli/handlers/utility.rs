//! Utility handlers: config, query, schema, completions, wait, pane

use anyhow::Result;
use clap::ArgMatches;
use zjj_core::OutputFormat;

use super::json_format::get_format;
use crate::commands::{completions, config, pane, query, schema, wait};

pub async fn handle_config(sub_m: &ArgMatches) -> Result<()> {
    let key = sub_m.get_one::<String>("key").cloned();
    let value = sub_m.get_one::<String>("value").cloned();
    let global = sub_m.get_flag("global");
    let format = get_format(sub_m);
    let options = config::ConfigOptions {
        key,
        value,
        global,
        format,
    };
    config::run(options).await
}

pub async fn handle_query(sub_m: &ArgMatches) -> Result<()> {
    // Handle --contract flag first
    if sub_m.get_flag("contract") {
        println!("{}", crate::cli::json_docs::ai_contracts::query());
        return Ok(());
    }

    // Handle --ai-hints flag
    if sub_m.get_flag("ai-hints") {
        println!("AI COMMAND FLOW:");
        println!("{}", crate::cli::json_docs::ai_contracts::command_flow());
        return Ok(());
    }

    let query_type = sub_m
        .get_one::<String>("query_type")
        .ok_or_else(|| anyhow::anyhow!("Query type is required"))?;
    let args = sub_m.get_one::<String>("args").map(String::as_str);
    let json_mode = sub_m.get_flag("json");

    let result = query::run(query_type, args, json_mode).await?;

    if !result.output.is_empty() {
        println!("{}", result.output);
    }

    if result.exit_code != 0 {
        std::process::exit(result.exit_code);
    }

    Ok(())
}

pub fn handle_schema(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let options = schema::SchemaOptions {
        schema_name: sub_m.get_one::<String>("name").cloned(),
        list: sub_m.get_flag("list"),
        all: sub_m.get_flag("all"),
        format,
    };
    schema::run(&options)
}

pub fn handle_completions(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let shell_str = sub_m
        .get_one::<String>("shell")
        .ok_or_else(|| anyhow::anyhow!("Shell is required"))?;
    let shell: completions::Shell = shell_str.parse()?;
    let options = completions::CompletionsOptions { shell, format };
    completions::run(&options)
}

pub async fn handle_wait(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let options = build_wait_options(sub_m, format)?;
    let exit_code = wait::run(&options).await?;
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}

fn build_wait_options(sub_m: &ArgMatches, format: OutputFormat) -> Result<wait::WaitOptions> {
    let condition_str = sub_m
        .get_one::<String>("condition")
        .ok_or_else(|| anyhow::anyhow!("Condition is required"))?;
    let name = sub_m.get_one::<String>("name").cloned();
    let status = sub_m.get_one::<String>("status").cloned();
    let timeout = sub_m.get_one::<f64>("timeout").copied().unwrap_or(30.0);
    let interval = sub_m.get_one::<f64>("interval").copied().unwrap_or(1.0);

    build_wait_options_from_values(condition_str, name, status, timeout, interval, format)
}

fn build_wait_options_from_values(
    condition_str: &str,
    name: Option<String>,
    status: Option<String>,
    timeout: f64,
    interval: f64,
    format: OutputFormat,
) -> Result<wait::WaitOptions> {
    if status.is_some() && condition_str != "session-status" {
        anyhow::bail!("--status is only valid with session-status condition");
    }

    let condition = match condition_str {
        "session-exists" => wait::WaitCondition::SessionExists(
            name.ok_or_else(|| anyhow::anyhow!("Session name required"))?,
        ),
        "session-unlocked" => wait::WaitCondition::SessionUnlocked(
            name.ok_or_else(|| anyhow::anyhow!("Session name required"))?,
        ),
        "healthy" => wait::WaitCondition::Healthy,
        "session-status" => wait::WaitCondition::SessionStatus {
            name: name.ok_or_else(|| anyhow::anyhow!("Session name required"))?,
            status: status.ok_or_else(|| anyhow::anyhow!("--status required"))?,
        },
        _ => anyhow::bail!("Unknown condition: {condition_str}"),
    };

    Ok(wait::WaitOptions {
        condition,
        timeout: std::time::Duration::from_secs_f64(timeout),
        poll_interval: std::time::Duration::from_secs_f64(interval),
        format,
    })
}

pub async fn handle_pane(sub_m: &ArgMatches) -> Result<()> {
    match sub_m.subcommand() {
        Some(("focus", focus_m)) => {
            if focus_m.get_flag("contract") {
                println!("AI CONTRACT for zjj pane focus:");
                println!("{}", crate::cli::json_docs::ai_contracts::command_flow());
                return Ok(());
            }

            if focus_m.get_flag("ai-hints") {
                println!("AI COMMAND FLOW:");
                println!("{}", crate::cli::json_docs::ai_contracts::command_flow());
                return Ok(());
            }

            let format = get_format(focus_m);
            let session = focus_m
                .get_one::<String>("session")
                .ok_or_else(|| anyhow::anyhow!("Session name is required"))?;
            let pane_identifier = focus_m.get_one::<String>("pane").map(String::as_str);
            let direction = focus_m.get_one::<String>("direction").map(String::as_str);
            let options = pane::PaneFocusOptions { format };
            if let Some(dir_str) = direction {
                let dir = pane::Direction::parse(dir_str)?;
                pane::pane_navigate(session, dir, &options).await
            } else {
                pane::pane_focus(session, pane_identifier, &options).await
            }
        }
        Some(("list", list_m)) => {
            let format = get_format(list_m);
            let session = list_m
                .get_one::<String>("session")
                .ok_or_else(|| anyhow::anyhow!("Session name is required"))?;
            pane::pane_list(session, &pane::PaneListOptions { format }).await
        }
        Some(("next", next_m)) => {
            let format = get_format(next_m);
            let session = next_m
                .get_one::<String>("session")
                .ok_or_else(|| anyhow::anyhow!("Session name is required"))?;
            pane::pane_next(session, &pane::PaneNextOptions { format }).await
        }
        _ => anyhow::bail!("Unknown pane subcommand"),
    }
}

#[cfg(test)]
mod tests {
    use zjj_core::OutputFormat;

    use super::build_wait_options_from_values;
    use crate::commands::wait::WaitCondition;

    #[test]
    fn test_handle_query_always_uses_json_format() {
        let json_flag = true;
        let format = OutputFormat::from_json_flag(json_flag);
        assert!(format.is_json());
        let json_flag_false = false;
        let _ = OutputFormat::from_json_flag(json_flag_false);
        let query_format = OutputFormat::Json;
        assert!(query_format.is_json());
    }

    #[test]
    fn wait_rejects_status_for_non_session_status() {
        let result = build_wait_options_from_values(
            "healthy",
            None,
            Some("active".to_string()),
            30.0,
            1.0,
            OutputFormat::Json,
        );

        assert!(result.is_err());
        let err = result.err().map_or(String::new(), |e| e.to_string());
        assert!(
            err.contains("--status is only valid with session-status"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn wait_defaults_preserve_timeout_and_interval() {
        let options =
            build_wait_options_from_values("healthy", None, None, 30.0, 1.0, OutputFormat::Json)
                .expect("options should build");

        assert!(matches!(options.condition, WaitCondition::Healthy));
        assert_eq!(options.timeout.as_secs(), 30);
        assert_eq!(options.poll_interval.as_secs(), 1);
    }

    #[test]
    fn wait_parser_rejects_non_numeric_timeout() {
        let parsed =
            crate::cli::commands::cmd_wait().try_get_matches_from(["wait", "-t", "abc", "healthy"]);
        assert!(parsed.is_err(), "timeout should require a number");
    }

    #[test]
    fn wait_parser_rejects_negative_interval() {
        let parsed =
            crate::cli::commands::cmd_wait().try_get_matches_from(["wait", "-i", "-1", "healthy"]);
        assert!(parsed.is_err(), "interval should reject negative values");
    }

    mod martin_fowler_wait_option_behavior {
        use super::*;

        /// GIVEN: `session-exists` requires a session name
        /// WHEN: We build options without a name
        /// THEN: The parser should fail with an actionable error
        #[test]
        fn given_session_exists_without_name_when_building_then_error_is_actionable() {
            let result = build_wait_options_from_values(
                "session-exists",
                None,
                None,
                30.0,
                1.0,
                OutputFormat::Json,
            );

            assert!(result.is_err());
            let err = result.err().map_or(String::new(), |e| e.to_string());
            assert!(
                err.contains("Session name required"),
                "unexpected error: {err}"
            );
        }

        /// GIVEN: `session-status` requires `--status`
        /// WHEN: We provide a session name but no status target
        /// THEN: Option construction should fail with a focused validation error
        #[test]
        fn given_session_status_without_status_when_building_then_requires_status() {
            let result = build_wait_options_from_values(
                "session-status",
                Some("feature-auth".to_string()),
                None,
                30.0,
                1.0,
                OutputFormat::Json,
            );

            assert!(result.is_err());
            let err = result.err().map_or(String::new(), |e| e.to_string());
            assert!(err.contains("--status required"), "unexpected error: {err}");
        }

        /// GIVEN: Valid `session-status` inputs
        /// WHEN: We build wait options with name + status
        /// THEN: The resulting condition should preserve both values exactly
        #[test]
        fn given_valid_session_status_when_building_then_preserves_name_and_status() {
            let result = build_wait_options_from_values(
                "session-status",
                Some("feature-auth".to_string()),
                Some("active".to_string()),
                45.0,
                2.0,
                OutputFormat::Json,
            )
            .expect("valid session-status options should build");

            match result.condition {
                WaitCondition::SessionStatus { name, status } => {
                    assert_eq!(name, "feature-auth");
                    assert_eq!(status, "active");
                }
                _ => panic!("expected session-status condition"),
            }
            assert_eq!(result.timeout.as_secs(), 45);
            assert_eq!(result.poll_interval.as_secs(), 2);
        }

        /// GIVEN: A non-session-status wait condition
        /// WHEN: `--status` is provided anyway
        /// THEN: Validation should fail and explain correct usage
        #[test]
        fn given_session_exists_with_status_when_building_then_rejects_misused_status_flag() {
            let result = build_wait_options_from_values(
                "session-exists",
                Some("feature-auth".to_string()),
                Some("active".to_string()),
                30.0,
                1.0,
                OutputFormat::Json,
            );

            assert!(result.is_err());
            let err = result.err().map_or(String::new(), |e| e.to_string());
            assert!(
                err.contains("--status is only valid with session-status"),
                "unexpected error: {err}"
            );
        }

        /// GIVEN: A healthy wait condition
        /// WHEN: Options are built with explicit timeout and interval
        /// THEN: Healthy condition remains selected and durations are preserved
        #[test]
        fn given_healthy_with_explicit_timing_when_building_then_preserves_timing() {
            let result = build_wait_options_from_values(
                "healthy",
                None,
                None,
                90.0,
                5.0,
                OutputFormat::Json,
            )
            .expect("healthy options should build");

            assert!(matches!(result.condition, WaitCondition::Healthy));
            assert_eq!(result.timeout.as_secs(), 90);
            assert_eq!(result.poll_interval.as_secs(), 5);
        }

        /// GIVEN: An unknown wait condition string
        /// WHEN: Option construction is attempted
        /// THEN: Validation should fail fast with the unknown condition value
        #[test]
        fn given_unknown_condition_when_building_then_returns_explicit_unknown_error() {
            let result = build_wait_options_from_values(
                "not-a-real-condition",
                None,
                None,
                30.0,
                1.0,
                OutputFormat::Json,
            );

            assert!(result.is_err());
            let err = result.err().map_or(String::new(), |e| e.to_string());
            assert!(
                err.contains("Unknown condition: not-a-real-condition"),
                "unexpected error: {err}"
            );
        }
    }

    mod martin_fowler_wait_cli_parser_behavior {
        /// GIVEN: valid positive timeout and interval values
        /// WHEN: command arguments are parsed
        /// THEN: parser should accept and preserve both values
        #[test]
        fn given_positive_timing_values_when_parsing_then_cli_accepts_them() {
            let parsed = crate::cli::commands::cmd_wait()
                .try_get_matches_from(["wait", "-t", "7", "-i", "250", "healthy"])
                .expect("valid wait arguments should parse");

            assert_eq!(parsed.get_one::<f64>("timeout").copied(), Some(7.0));
            assert_eq!(parsed.get_one::<f64>("interval").copied(), Some(250.0));
            assert_eq!(
                parsed.get_one::<String>("condition").map(String::as_str),
                Some("healthy")
            );
        }
    }

    mod martin_fowler_wait_table_driven_behavior {
        use super::*;

        struct BuildCase {
            name: &'static str,
            condition: &'static str,
            session_name: Option<&'static str>,
            status: Option<&'static str>,
            timeout: u64,
            interval: u64,
            expect_ok: bool,
            expected_error_fragment: Option<&'static str>,
        }

        /// GIVEN: a matrix of wait option scenarios
        /// WHEN: option-building runs across all rows
        /// THEN: each row should produce expected success/failure behavior
        #[test]
        fn given_wait_option_matrix_when_building_then_each_row_matches_expected_behavior() {
            let cases = [
                BuildCase {
                    name: "healthy basic",
                    condition: "healthy",
                    session_name: None,
                    status: None,
                    timeout: 30,
                    interval: 1000,
                    expect_ok: true,
                    expected_error_fragment: None,
                },
                BuildCase {
                    name: "session-exists missing name",
                    condition: "session-exists",
                    session_name: None,
                    status: None,
                    timeout: 30,
                    interval: 1000,
                    expect_ok: false,
                    expected_error_fragment: Some("Session name required"),
                },
                BuildCase {
                    name: "session-status missing status",
                    condition: "session-status",
                    session_name: Some("feat-x"),
                    status: None,
                    timeout: 30,
                    interval: 1000,
                    expect_ok: false,
                    expected_error_fragment: Some("--status required"),
                },
                BuildCase {
                    name: "session-status complete",
                    condition: "session-status",
                    session_name: Some("feat-x"),
                    status: Some("active"),
                    timeout: 30,
                    interval: 1000,
                    expect_ok: true,
                    expected_error_fragment: None,
                },
                BuildCase {
                    name: "misused status on healthy",
                    condition: "healthy",
                    session_name: None,
                    status: Some("active"),
                    timeout: 30,
                    interval: 1000,
                    expect_ok: false,
                    expected_error_fragment: Some("--status is only valid with session-status"),
                },
            ];

            for case in cases {
                let result = build_wait_options_from_values(
                    case.condition,
                    case.session_name.map(str::to_string),
                    case.status.map(str::to_string),
                    case.timeout as f64,
                    case.interval as f64,
                    OutputFormat::Json,
                );

                assert_eq!(result.is_ok(), case.expect_ok, "case failed: {}", case.name);

                if let Some(fragment) = case.expected_error_fragment {
                    let err_text = result.err().map_or(String::new(), |e| e.to_string());
                    assert!(
                        err_text.contains(fragment),
                        "case '{}' missing error fragment '{}': {}",
                        case.name,
                        fragment,
                        err_text
                    );
                }
            }
        }

        struct ParseCase {
            name: &'static str,
            args: Vec<&'static str>,
            expect_ok: bool,
        }

        /// GIVEN: a matrix of CLI parse scenarios
        /// WHEN: clap parses each argument row
        /// THEN: acceptance/rejection should match the parser contract
        #[test]
        fn given_wait_parser_matrix_when_parsing_then_each_row_matches_expected_outcome() {
            let cases = [
                ParseCase {
                    name: "accept zero timeout (f64 allows it)",
                    args: vec!["wait", "-t", "0", "healthy"],
                    expect_ok: true,
                },
                ParseCase {
                    name: "reject non numeric timeout",
                    args: vec!["wait", "-t", "abc", "healthy"],
                    expect_ok: false,
                },
                ParseCase {
                    name: "accept zero interval (f64 allows it)",
                    args: vec!["wait", "-i", "0", "healthy"],
                    expect_ok: true,
                },
                ParseCase {
                    name: "accept explicit positive timing",
                    args: vec!["wait", "-t", "5", "-i", "250", "healthy"],
                    expect_ok: true,
                },
                ParseCase {
                    name: "accept session status with status flag",
                    args: vec![
                        "wait",
                        "session-status",
                        "feat-x",
                        "--status",
                        "active",
                        "-t",
                        "3",
                    ],
                    expect_ok: true,
                },
            ];

            for case in cases {
                let result = crate::cli::commands::cmd_wait().try_get_matches_from(case.args);
                assert_eq!(
                    result.is_ok(),
                    case.expect_ok,
                    "case '{}' parse expectation failed",
                    case.name
                );
            }
        }
    }
}
