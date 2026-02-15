//! Askama-backed template rendering
//!
//! This module provides lightweight, type-safe rendering for core templates.
//! It avoids panics by returning `Result` for all operations.

use thiserror::Error;

use crate::templates::{
    AI_INSTRUCTIONS, DOC_01_ERROR_HANDLING, DOC_02_MOON_BUILD, DOC_03_WORKFLOW,
    DOC_05_RUST_STANDARDS, DOC_08_BEADS, DOC_09_JUJUTSU, MOON_TASKS, MOON_TOOLCHAIN,
    MOON_WORKSPACE,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectContext {
    pub name: String,
    pub description: String,
    pub version: String,
    pub authors: Vec<String>,
    pub license: Option<String>,
}

impl ProjectContext {
    pub fn new(
        name: String,
        description: String,
        version: String,
        authors: Vec<String>,
        license: Option<String>,
    ) -> Result<Self, TemplateError> {
        if name.is_empty() {
            return Err(TemplateError::Validation(
                "Project name cannot be empty".to_string(),
            ));
        }

        Ok(Self {
            name,
            description,
            version,
            authors,
            license,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateType {
    AgentsMd,
    ClaudeMd,
    MoonWorkspace,
    MoonToolchain,
    MoonTasks,
    Doc01ErrorHandling,
    Doc02MoonBuild,
    Doc03Workflow,
    Doc05RustStandards,
    Doc08Beads,
    Doc09Jujutsu,
}

impl TemplateType {
    #[must_use]
    pub fn docs() -> &'static [Self] {
        &[
            Self::Doc01ErrorHandling,
            Self::Doc02MoonBuild,
            Self::Doc03Workflow,
            Self::Doc05RustStandards,
            Self::Doc08Beads,
            Self::Doc09Jujutsu,
        ]
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AgentsMd => "AGENTS.md",
            Self::ClaudeMd => "CLAUDE.md",
            Self::MoonWorkspace => "workspace.yml",
            Self::MoonToolchain => "toolchain.yml",
            Self::MoonTasks => "tasks.yml",
            Self::Doc01ErrorHandling => "01_ERROR_HANDLING.md",
            Self::Doc02MoonBuild => "02_MOON_BUILD.md",
            Self::Doc03Workflow => "03_WORKFLOW.md",
            Self::Doc05RustStandards => "05_RUST_STANDARDS.md",
            Self::Doc08Beads => "08_BEADS.md",
            Self::Doc09Jujutsu => "09_JUJUTSU.md",
        }
    }
}

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("Template rendering failed: {0}")]
    RenderError(String),
    #[error("Template validation failed: {0}")]
    Validation(String),
}

pub fn render_template(
    template: TemplateType,
    _context: &ProjectContext,
) -> Result<String, TemplateError> {
    let rendered = match template {
        TemplateType::AgentsMd | TemplateType::ClaudeMd => AI_INSTRUCTIONS,
        TemplateType::MoonWorkspace => MOON_WORKSPACE,
        TemplateType::MoonToolchain => MOON_TOOLCHAIN,
        TemplateType::MoonTasks => MOON_TASKS,
        TemplateType::Doc01ErrorHandling => DOC_01_ERROR_HANDLING,
        TemplateType::Doc02MoonBuild => DOC_02_MOON_BUILD,
        TemplateType::Doc03Workflow => DOC_03_WORKFLOW,
        TemplateType::Doc05RustStandards => DOC_05_RUST_STANDARDS,
        TemplateType::Doc08Beads => DOC_08_BEADS,
        TemplateType::Doc09Jujutsu => DOC_09_JUJUTSU,
    };

    Ok(rendered.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_templates_have_cross_references() -> Result<(), TemplateError> {
        let context = ProjectContext::new(
            "zjj".to_string(),
            "ZJJ".to_string(),
            "0.1.0".to_string(),
            vec!["Test".to_string()],
            None,
        )?;

        let workflow = render_template(TemplateType::Doc03Workflow, &context)?;
        let beads = render_template(TemplateType::Doc08Beads, &context)?;
        let _jujutsu = render_template(TemplateType::Doc09Jujutsu, &context)?;

        // Verify cross-references between docs
        assert!(
            workflow.contains("bead") || workflow.contains("BEADS"),
            "Workflow template should reference beads documentation"
        );
        assert!(
            workflow.contains("jj") || workflow.contains("Jujutsu") || workflow.contains("JUJUTSU"),
            "Workflow template should reference Jujutsu documentation"
        );
        assert!(
            beads.contains("workflow") || beads.contains("WORKFLOW"),
            "Beads template should reference workflow documentation"
        );
        Ok(())
    }
}
