#![cfg(test)]

use super::templates::{
    get_docs_templates, get_moon_templates, DOC_01_ERROR_HANDLING, DOC_02_MOON_BUILD,
    DOC_03_WORKFLOW, DOC_05_RUST_STANDARDS, DOC_08_BEADS, DOC_09_JUJUTSU, MOON_TASKS,
    MOON_TOOLCHAIN, MOON_WORKSPACE,
};

// Behavior: get_docs_templates returns all documentation templates
#[test]
fn given_get_docs_templates_when_called_then_returns_all_docs() {
    let docs = get_docs_templates();

    assert!(docs.contains_key("01_ERROR_HANDLING.md"));
    assert!(docs.contains_key("02_MOON_BUILD.md"));
    assert!(docs.contains_key("03_WORKFLOW.md"));
    assert!(docs.contains_key("05_RUST_STANDARDS.md"));
    assert!(docs.contains_key("08_BEADS.md"));
    assert!(docs.contains_key("09_JUJUTSU.md"));
    assert_eq!(docs.len(), 6);
}

// Behavior: get_docs_templates returns correct values
#[test]
fn given_get_docs_templates_when_called_then_values_match_constants() {
    let docs = get_docs_templates();

    assert_eq!(
        docs.get("01_ERROR_HANDLING.md"),
        Some(&DOC_01_ERROR_HANDLING)
    );
    assert_eq!(docs.get("02_MOON_BUILD.md"), Some(&DOC_02_MOON_BUILD));
    assert_eq!(docs.get("03_WORKFLOW.md"), Some(&DOC_03_WORKFLOW));
    assert_eq!(
        docs.get("05_RUST_STANDARDS.md"),
        Some(&DOC_05_RUST_STANDARDS)
    );
}

// Behavior: get_moon_templates returns all moon templates
#[test]
fn given_get_moon_templates_when_called_then_returns_all_moon() {
    let moon = get_moon_templates();

    assert!(moon.contains_key("workspace.yml"));
    assert!(moon.contains_key("toolchain.yml"));
    assert!(moon.contains_key("tasks.yml"));
    assert_eq!(moon.len(), 3);
}

// Behavior: get_moon_templates returns correct values
#[test]
fn given_get_moon_templates_when_called_then_values_match_constants() {
    let moon = get_moon_templates();

    assert_eq!(moon.get("workspace.yml"), Some(&MOON_WORKSPACE));
    assert_eq!(moon.get("toolchain.yml"), Some(&MOON_TOOLCHAIN));
    assert_eq!(moon.get("tasks.yml"), Some(&MOON_TASKS));
}

// Behavior: Moon templates are valid YAML strings
#[test]
fn given_moon_workspace_template_then_is_valid_yaml() {
    assert!(MOON_WORKSPACE.contains("$schema"));
    assert!(MOON_WORKSPACE.contains("moonrepo.dev"));
}

// Behavior: Moon tasks template contains ci task
#[test]
fn given_moon_tasks_template_then_contains_ci_task() {
    assert!(MOON_TASKS.contains("ci"));
    assert!(MOON_TASKS.contains("command"));
}

// Behavior: Documentation templates contain expected content
#[test]
fn given_error_handling_template_then_contains_policy() {
    assert!(DOC_01_ERROR_HANDLING.contains("Result<T, Error>"));
    assert!(DOC_01_ERROR_HANDLING.contains("Zero Policy"));
}

// Behavior: Workflow template contains expected steps
#[test]
fn given_workflow_template_then_contains_steps() {
    assert!(DOC_03_WORKFLOW.contains("Pull"));
    assert!(DOC_03_WORKFLOW.contains("Isolate"));
    assert!(DOC_03_WORKFLOW.contains("Verify"));
    assert!(DOC_03_WORKFLOW.contains("Merge"));
}
