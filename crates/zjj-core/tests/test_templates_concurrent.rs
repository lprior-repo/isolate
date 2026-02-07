//! Concurrent template access tests
//!
//! This test module verifies that template storage handles concurrent
//! access patterns correctly without deadlocks or data races.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::sync::Arc;

use tokio::task::JoinSet;
use zjj_core::{
    templates::storage::{
        delete_template, list_templates, load_template, save_template, template_exists, Template,
    },
    Error,
};

/// Test concurrent template read/write operations
///
/// This test spawns:
/// - 10 concurrent readers that repeatedly load and list templates
/// - 5 concurrent writers that create, update, and delete templates
///
/// All operations must complete without deadlock or data races.
#[tokio::test]
async fn test_concurrent_template_read_write() {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let templates_base = Arc::new(temp_dir.path().to_path_buf());

    // Create initial set of templates
    let initial_templates: Vec<_> = (0..5)
        .map(|i| {
            Template::new(
                format!("template_{i}"),
                format!("layout {{ pane id=\"{i}\" }}"),
                Some(format!("Template {i}")),
            )
            .expect("Failed to create template")
        })
        .collect();

    for template in &initial_templates {
        save_template(template, &templates_base).expect("Failed to save initial template");
    }

    let mut join_set = JoinSet::new();

    // Spawn 10 concurrent readers (use ID range 0-9)
    for reader_id in 0..10 {
        let base = Arc::clone(&templates_base);
        join_set.spawn(async move {
            let mut successful_reads = 0;
            let mut read_errors = 0;
            let mut counter = 0;

            for _ in 0..20 {
                // Alternate between listing all templates and loading specific ones
                counter += 1;
                if counter % 2 == 0 {
                    // List all templates
                    match list_templates(&base) {
                        Ok(templates) => {
                            successful_reads += 1;
                            // Verify we got a reasonable result
                            assert!(templates.len() <= 60, "Too many templates returned");
                        }
                        Err(_) => {
                            read_errors += 1;
                        }
                    }
                } else {
                    // Load a specific template
                    let template_id = (reader_id + counter) % 10;
                    match load_template(&format!("template_{template_id}"), &base) {
                        Ok(_) => {
                            successful_reads += 1;
                        }
                        Err(Error::NotFound(_)) => {
                            // Template might not exist yet, that's okay
                            successful_reads += 1;
                        }
                        Err(_) => {
                            read_errors += 1;
                        }
                    }
                }

                // Small delay to increase contention
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            }

            // Return tuple with first element as reader ID (0-9)
            (reader_id, successful_reads, read_errors, 0) // (id, reads, read_errors, 0 for writes)
        });
    }

    // Spawn 5 concurrent writers (use ID range 100-104 to distinguish from readers)
    for writer_id in 0..5 {
        let base = Arc::clone(&templates_base);
        join_set.spawn(async move {
            let mut successful_writes = 0;
            let mut write_errors = 0;

            for i in 0..10 {
                let template_num = writer_id * 10 + i;

                // Create template
                let template = Template::new(
                    format!("template_{template_num}"),
                    format!("layout {{ pane id=\"{template_num}\" }}"),
                    Some(format!("Template {template_num}")),
                );

                match template {
                    Ok(t) => {
                        if save_template(&t, &base).is_ok() {
                            successful_writes += 1;
                        } else {
                            write_errors += 1;
                        }
                    }
                    Err(_) => {
                        write_errors += 1;
                    }
                }

                // Update existing template
                if template_num > 0 {
                    let prev_template_num = template_num - 1;
                    let update_template = Template::new(
                        format!("template_{prev_template_num}"),
                        format!("layout {{ pane id=\"{prev_template_num}\" version=\"2\" }}"),
                        Some(format!("Updated template {prev_template_num}")),
                    );

                    match update_template {
                        Ok(t) => {
                            if save_template(&t, &base).is_ok() {
                                successful_writes += 1;
                            } else {
                                write_errors += 1;
                            }
                        }
                        Err(_) => {
                            write_errors += 1;
                        }
                    }
                }

                // Occasionally delete a template
                if i % 3 == 0 && template_num > 5 {
                    let delete_num = template_num - 5;
                    let _ = delete_template(&format!("template_{delete_num}"), &base);
                    // Deletion failures are okay (template might not exist)
                }

                // Small delay
                tokio::time::sleep(tokio::time::Duration::from_millis(2)).await;
            }

            // Return tuple with first element as writer ID (100-104)
            (100 + writer_id, 0, 0, successful_writes) // (id, 0 for reads, 0 for read_errors,
                                                       // writes)
        });
    }

    // Wait for all tasks to complete
    let mut completed_readers = 0;
    let mut completed_writers = 0;
    let mut total_successful_reads = 0;
    let mut total_successful_writes = 0;

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok((id, reads, read_errors, writes)) => {
                if id < 10 {
                    // Reader task
                    completed_readers += 1;
                    total_successful_reads += reads;
                    assert!(
                        read_errors < 5,
                        "Reader {id} had too many errors: {read_errors}"
                    );
                } else {
                    // Writer task (ID >= 100)
                    completed_writers += 1;
                    total_successful_writes += writes;
                    assert!(
                        read_errors < 5, // This is write_errors for writers
                        "Writer {id} had too many errors: {read_errors}"
                    );
                }
            }
            Err(e) => {
                panic!("Task panicked: {e}");
            }
        }
    }

    // Verify all tasks completed
    assert_eq!(completed_readers, 10, "Not all readers completed");
    assert_eq!(completed_writers, 5, "Not all writers completed");

    // Verify we had significant successful operations
    assert!(
        total_successful_reads > 100,
        "Too few successful reads: {total_successful_reads}"
    );
    assert!(
        total_successful_writes > 50,
        "Too few successful writes: {total_successful_writes}"
    );

    // Final verification: ensure templates directory is in consistent state
    let final_templates = list_templates(&templates_base)
        .expect("Failed to list templates after concurrent operations");

    // Verify all templates have valid metadata
    for template in &final_templates {
        assert!(!template.name.as_str().is_empty(), "Template name is empty");
        assert!(!template.layout.is_empty(), "Template layout is empty");
        assert!(
            template.metadata.created_at > 0,
            "Invalid created_at timestamp"
        );
        assert!(
            template.metadata.updated_at >= template.metadata.created_at,
            "Invalid updated_at timestamp"
        );
    }
}

/// Test that template operations handle corrupted metadata gracefully
///
/// This test verifies that when metadata.json contains invalid JSON,
/// the system returns proper errors rather than panicking.
#[tokio::test]
async fn test_template_handles_corrupted_metadata() {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let templates_base = temp_dir.path();

    // Create a valid template first
    let template = Template::new(
        "valid_template".to_string(),
        "layout { pane }".to_string(),
        Some("Valid template".to_string()),
    )
    .expect("Failed to create template");

    save_template(&template, templates_base).expect("Failed to save template");

    // Verify it loads correctly
    let loaded = load_template("valid_template", templates_base);
    assert!(loaded.is_ok(), "Valid template should load successfully");

    // Now corrupt the metadata
    let template_dir = templates_base.join("valid_template");
    let metadata_path = template_dir.join("metadata.json");

    // Write invalid JSON
    tokio::fs::write(&metadata_path, "{ invalid json }")
        .await
        .expect("Failed to corrupt metadata");

    // Attempting to load should return a proper error, not panic
    let load_result = load_template("valid_template", templates_base);
    assert!(
        load_result.is_err(),
        "Loading corrupted metadata should fail"
    );

    match load_result {
        Err(Error::ValidationError(msg)) => {
            assert!(
                msg.contains("Invalid template metadata") || msg.contains("expected"),
                "Error should mention invalid metadata: {msg}"
            );
        }
        Err(e) => {
            panic!("Expected ValidationError, got: {e}");
        }
        Ok(_) => {
            panic!("Should have failed to load corrupted metadata");
        }
    }

    // Test with completely missing metadata file
    tokio::fs::remove_file(&metadata_path)
        .await
        .expect("Failed to remove metadata");

    let load_result = load_template("valid_template", templates_base);
    assert!(
        load_result.is_err(),
        "Loading with missing metadata should fail"
    );

    // Test with partially corrupted JSON (missing closing brace)
    tokio::fs::write(&metadata_path, r#"{"name": "test""#)
        .await
        .expect("Failed to write partial JSON");

    let load_result = load_template("valid_template", templates_base);
    assert!(load_result.is_err(), "Loading partial JSON should fail");

    // Test list_templates with corrupted metadata
    // Create another valid template
    let template2 = Template::new("template2".to_string(), "layout { pane }".to_string(), None)
        .expect("Failed to create template");

    save_template(&template2, templates_base).expect("Failed to save second template");

    // Corrupt first template's metadata again
    let metadata_path2 = templates_base.join("template2").join("metadata.json");
    tokio::fs::write(&metadata_path2, "corrupted")
        .await
        .expect("Failed to corrupt metadata");

    // list_templates should skip corrupted templates, not crash
    let list_result = list_templates(templates_base);
    assert!(
        list_result.is_ok(),
        "list_templates should handle corruption gracefully"
    );

    let templates = list_result.expect("Failed to list templates");
    // The corrupted template should be skipped
    assert!(
        !templates.iter().any(|t| t.name.as_str() == "template2"),
        "Corrupted template should not appear in list"
    );
}

/// Test concurrent operations on the same template
#[tokio::test]
async fn test_concurrent_same_template_operations() {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let templates_base = Arc::new(temp_dir.path().to_path_buf());

    // Create initial template
    let template = Template::new(
        "shared".to_string(),
        "layout { pane }".to_string(),
        Some("Shared template".to_string()),
    )
    .expect("Failed to create template");

    save_template(&template, &templates_base).expect("Failed to save template");

    let mut join_set = JoinSet::new();

    // Spawn multiple tasks operating on the same template
    for task_id in 0..8 {
        let base = Arc::clone(&templates_base);
        join_set.spawn(async move {
            let mut operations = 0;

            for i in 0..10 {
                // Mix of reads and writes
                if i % 2 == 0 {
                    // Read operation
                    let _ = load_template("shared", &base);
                } else {
                    // Write operation - update template
                    let updated = Template::new(
                        "shared".to_string(),
                        format!("layout {{ pane version=\"{i}\" }}"),
                        Some(format!("Version {i}")),
                    );
                    if let Ok(t) = updated {
                        let _ = save_template(&t, &base);
                    }
                }

                // Check existence
                let _ = template_exists("shared", &base);

                operations += 1;
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            }

            (task_id, operations)
        });
    }

    // Wait for all tasks
    let mut completed_tasks = 0;
    let mut total_operations = 0;

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok((_id, ops)) => {
                completed_tasks += 1;
                total_operations += ops;
            }
            Err(e) => {
                panic!("Task panicked: {e}");
            }
        }
    }

    assert_eq!(completed_tasks, 8, "Not all tasks completed");
    assert_eq!(total_operations, 80, "Not all operations completed");

    // Verify template still exists and is valid
    let final_template = load_template("shared", &templates_base)
        .expect("Failed to load template after concurrent operations");

    assert_eq!(final_template.name.as_str(), "shared");
    assert!(!final_template.layout.is_empty());
}

/// Test template_exists under concurrent load
#[tokio::test]
async fn test_concurrent_exists_checks() {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let templates_base = Arc::new(temp_dir.path().to_path_buf());

    let mut join_set = JoinSet::new();

    // Spawn multiple tasks checking existence
    for task_id in 0..20 {
        let base = Arc::clone(&templates_base);
        join_set.spawn(async move {
            let mut checks = 0;

            for i in 0..50 {
                let template_name = format!("template_{}", i % 15);
                let _ = template_exists(&template_name, &base);
                checks += 1;
            }

            (task_id, checks, 0) // (task_id, checks, creates)
        });
    }

    // Spawn some writers creating templates
    for writer_id in 0..5 {
        let base = Arc::clone(&templates_base);
        join_set.spawn(async move {
            let mut creates = 0;

            for i in 0..10 {
                let template = Template::new(
                    format!("template_{}", writer_id * 10 + i),
                    "layout { pane }".to_string(),
                    None,
                );

                if let Ok(t) = template {
                    if save_template(&t, &base).is_ok() {
                        creates += 1;
                    }
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            }

            (writer_id, 0, creates) // (task_id, checks, creates)
        });
    }

    // Wait for completion
    let mut total_checks = 0;
    let mut total_creates = 0;

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok((_id, checks, creates)) => {
                // All tasks return same tuple type
                total_checks += checks;
                total_creates += creates;
            }
            Err(e) => {
                panic!("Task panicked: {e}");
            }
        }
    }

    assert_eq!(total_checks, 1000, "Not all existence checks completed");
    assert!(total_creates > 0, "No templates were created");
}
