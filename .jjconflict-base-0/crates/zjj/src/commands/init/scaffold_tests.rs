use tempfile::TempDir;
use anyhow::Result;
use std::fs;
use crate::commands::init::{run_with_cwd_and_format, tests::setup_test_jj_repo};
use zjj_core::OutputFormat;

#[test]
fn test_init_scaffolds_ai_instructions() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo()? else {
        return Ok(());
    };

    run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::Human)?;

    // Verify AGENTS.md was created
    let agents_path = temp_dir.path().join("AGENTS.md");
    assert!(agents_path.exists(), "AGENTS.md was not created");
    
    // Verify CLAUDE.md was created
    let claude_path = temp_dir.path().join("CLAUDE.md");
    assert!(claude_path.exists(), "CLAUDE.md was not created");

    // Verify they are clones (identical content)
    let agents_content = fs::read_to_string(&agents_path)?;
    let claude_content = fs::read_to_string(&claude_path)?;
    assert_eq!(agents_content, claude_content, "AGENTS.md and CLAUDE.md are not identical");
    
    // Verify legacy .ai-instructions.md
    let legacy_path = temp_dir.path().join(".ai-instructions.md");
    assert!(legacy_path.exists(), ".ai-instructions.md was not created");
    let legacy_content = fs::read_to_string(&legacy_path)?;
    assert_eq!(agents_content, legacy_content, "AGENTS.md and .ai-instructions.md are not identical");

    // Verify core content exists
    assert!(agents_content.contains("# Agent Instructions"));
    assert!(agents_content.contains("Zero-Policy"));
    assert!(agents_content.contains("Moon"));

    Ok(())
}

#[test]
fn test_init_scaffolds_docs_directory() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo()? else {
        return Ok(());
    };

    run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::Human)?;

    let docs_dir = temp_dir.path().join("docs");
    assert!(docs_dir.exists(), "docs directory was not created");
    assert!(docs_dir.is_dir());

    // Verify key files exist in docs/
    let expected_files = vec![
        "01_ERROR_HANDLING.md",
        "02_MOON_BUILD.md",
        "03_WORKFLOW.md",
        "05_RUST_STANDARDS.md",
        "08_BEADS.md",
        "09_JUJUTSU.md",
    ];

    for file in expected_files {
        let path = docs_dir.join(file);
        assert!(path.exists(), "docs/{} was not created", file);
        
        let content = fs::read_to_string(&path)?;
        assert!(!content.is_empty(), "docs/{} is empty", file);
    }

    Ok(())
}

#[test]
fn test_init_scaffolds_moon_config() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo()? else {
        return Ok(());
    };

    // Determine project name from temp_dir
    let project_name = temp_dir
        .path()
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project");

    run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::Human)?;

    let moon_dir = temp_dir.path().join(".moon");
    assert!(moon_dir.exists(), ".moon directory was not created");
    assert!(moon_dir.is_dir());

    // Verify key Moon config files exist and contain project name
    let expected_files = vec![
        "workspace.yml",
        "toolchain.yml",
        "tasks.yml",
    ];

    for file in expected_files {
        let path = moon_dir.join(file);
        assert!(path.exists(), ".moon/{} was not created", file);
        
        let content = fs::read_to_string(&path)?;
        assert!(!content.is_empty(), ".moon/{} is empty", file);

        // Check for project name templating
        if file == "tasks.yml" {
            assert!(content.contains(&format!("target/release/{}", project_name)));
            assert!(content.contains(&format!("~/.local/bin/{}", project_name)));
        }
    }

    Ok(())
}

#[test]
fn test_init_scaffold_is_idempotent() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo()? else {
        return Ok(());
    };

    // Run first time
    run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::Human)?;
    
    let agents_path = temp_dir.path().join("AGENTS.md");
    let initial_content = fs::read_to_string(&agents_path)?;

    // Modify AGENTS.md
    fs::write(&agents_path, "modified content")?;

    // Run second time
    run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::Human)?;

    // Verify AGENTS.md was NOT overwritten (standard init behavior)
    let final_content = fs::read_to_string(&agents_path)?;
    assert_eq!(final_content, "modified content", "init should not overwrite existing AGENTS.md");

    Ok(())
}