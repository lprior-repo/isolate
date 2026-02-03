//! Functional Askama Template Rendering
//!
//! Pure functional template rendering with zero-panic guarantees.
//! All operations return Result<T, `TemplateError`> with explicit error handling.
//!
//! # Architecture
//!
//! - **Functional Core**: Pure template rendering functions
//! - **Imperative Shell**: File I/O operations pushed to edges
//! - **Railway-Oriented**: All fallible operations chain via combinators

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

use std::collections::HashMap;

use askama::Template;
use thiserror::Error;

/// Template rendering errors
#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("template not found: {name}")]
    TemplateNotFound { name: String },

    #[error("rendering failed for template '{name}': {reason}")]
    RenderFailed { name: String, reason: String },

    #[error("invalid context for template '{name}': {reason}")]
    InvalidContext { name: String, reason: String },

    #[error("missing required variable '{variable}' in template '{template}'")]
    MissingVariable { variable: String, template: String },

    #[error("askama error: {0}")]
    Askama(#[from] askama::Error),
}

/// Project configuration context for template rendering
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectContext {
    pub project_name: String,
    pub description: String,
    pub version: String,
    pub authors: Vec<String>,
    pub repository_url: Option<String>,
}

#[derive(Template)]
#[template(path = "agents.md.j2")]
struct AgentsMdTemplate<'a> {
    project_name: &'a str,
    description: &'a str,
    version: &'a str,
    authors: &'a [String],
    repository_url: Option<&'a str>,
}

#[derive(Template)]
#[template(path = "claude.md.j2")]
struct ClaudeMdTemplate<'a> {
    project_name: &'a str,
    description: &'a str,
    version: &'a str,
    authors: &'a [String],
    repository_url: Option<&'a str>,
}

#[derive(Template)]
#[template(path = "error_handling.md.j2")]
struct ErrorHandlingTemplate;

#[derive(Template)]
#[template(path = "rust_standards.md.j2")]
struct RustStandardsTemplate;

#[derive(Template)]
#[template(path = "workflow.md.j2")]
struct WorkflowTemplate;

#[derive(Template)]
#[template(path = "beads.md.j2")]
struct BeadsTemplate;

#[derive(Template)]
#[template(path = "jujutsu.md.j2")]
struct JujutsuTemplate;

#[derive(Template)]
#[template(path = "moon_build.md.j2")]
struct MoonBuildTemplate;

#[derive(Template)]
#[template(path = "moon_workspace.yml.j2")]
struct MoonWorkspaceTemplate<'a> {
    project_name: &'a str,
    version: &'a str,
}

#[derive(Template)]
#[template(path = "moon_toolchain.yml.j2")]
struct MoonToolchainTemplate<'a> {
    project_name: &'a str,
    version: &'a str,
}

#[derive(Template)]
#[template(path = "moon_tasks.yml.j2")]
struct MoonTasksTemplate<'a> {
    project_name: &'a str,
}

impl ProjectContext {
    /// Create a new project context with validation
    ///
    /// # Errors
    ///
    /// Returns [`TemplateError::InvalidContext`] if:
    /// - `project_name` is empty
    /// - `version` is empty
    /// - `authors` is empty
    pub fn new(
        project_name: String,
        description: String,
        version: String,
        authors: Vec<String>,
        repository_url: Option<String>,
    ) -> Result<Self, TemplateError> {
        if project_name.trim().is_empty() {
            return Err(TemplateError::InvalidContext {
                name: "ProjectContext".to_string(),
                reason: "project_name cannot be empty".to_string(),
            });
        }

        if version.trim().is_empty() {
            return Err(TemplateError::InvalidContext {
                name: "ProjectContext".to_string(),
                reason: "version cannot be empty".to_string(),
            });
        }

        if authors.is_empty() {
            return Err(TemplateError::InvalidContext {
                name: "ProjectContext".to_string(),
                reason: "authors cannot be empty".to_string(),
            });
        }

        Ok(Self {
            project_name,
            description,
            version,
            authors,
            repository_url,
        })
    }
}

/// Template type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TemplateType {
    AgentsMd,
    ClaudeMd,
    ErrorHandling,
    RustStandards,
    Workflow,
    Beads,
    Jujutsu,
    MoonBuild,
    MoonWorkspace,
    MoonToolchain,
    MoonTasks,
}

impl TemplateType {
    /// Get template name as string
    #[must_use]
    pub const fn as_str(&self) -> &str {
        match self {
            Self::AgentsMd => "agents.md",
            Self::ClaudeMd => "claude.md",
            Self::ErrorHandling => "error_handling.md",
            Self::RustStandards => "rust_standards.md",
            Self::Workflow => "workflow.md",
            Self::Beads => "beads.md",
            Self::Jujutsu => "jujutsu.md",
            Self::MoonBuild => "moon_build.md",
            Self::MoonWorkspace => "workspace.yml",
            Self::MoonToolchain => "toolchain.yml",
            Self::MoonTasks => "tasks.yml",
        }
    }

    /// Get all template types
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::AgentsMd,
            Self::ClaudeMd,
            Self::ErrorHandling,
            Self::RustStandards,
            Self::Workflow,
            Self::Beads,
            Self::Jujutsu,
            Self::MoonBuild,
            Self::MoonWorkspace,
            Self::MoonToolchain,
            Self::MoonTasks,
        ]
    }

    /// Get all documentation template types
    #[must_use]
    pub const fn docs() -> &'static [Self] {
        &[
            Self::ErrorHandling,
            Self::RustStandards,
            Self::Workflow,
            Self::Beads,
            Self::Jujutsu,
            Self::MoonBuild,
        ]
    }

    /// Get all Moon config template types
    #[must_use]
    pub const fn moon_configs() -> &'static [Self] {
        &[Self::MoonWorkspace, Self::MoonToolchain, Self::MoonTasks]
    }
}

/// Render a template with the given context (pure function)
///
/// This is a functional core operation - no I/O, pure transformation.
///
/// # Errors
///
/// Returns [`TemplateError`] if:
/// - Template variables are missing
/// - Rendering fails
pub fn render_template(
    template_type: TemplateType,
    context: &ProjectContext,
) -> Result<String, TemplateError> {
    match template_type {
        TemplateType::AgentsMd => {
            let template = AgentsMdTemplate {
                project_name: &context.project_name,
                description: &context.description,
                version: &context.version,
                authors: &context.authors,
                repository_url: context.repository_url.as_deref(),
            };
            Ok(template.render()?)
        }
        TemplateType::ClaudeMd => {
            let template = ClaudeMdTemplate {
                project_name: &context.project_name,
                description: &context.description,
                version: &context.version,
                authors: &context.authors,
                repository_url: context.repository_url.as_deref(),
            };
            Ok(template.render()?)
        }
        TemplateType::ErrorHandling => Ok(ErrorHandlingTemplate.render()?),
        TemplateType::RustStandards => Ok(RustStandardsTemplate.render()?),
        TemplateType::Workflow => Ok(WorkflowTemplate.render()?),
        TemplateType::Beads => Ok(BeadsTemplate.render()?),
        TemplateType::Jujutsu => Ok(JujutsuTemplate.render()?),
        TemplateType::MoonBuild => Ok(MoonBuildTemplate.render()?),
        TemplateType::MoonWorkspace => {
            let template = MoonWorkspaceTemplate {
                project_name: &context.project_name,
                version: &context.version,
            };
            Ok(template.render()?)
        }
        TemplateType::MoonToolchain => {
            let template = MoonToolchainTemplate {
                project_name: &context.project_name,
                version: &context.version,
            };
            Ok(template.render()?)
        }
        TemplateType::MoonTasks => {
            let template = MoonTasksTemplate {
                project_name: &context.project_name,
            };
            Ok(template.render()?)
        }
    }
}

/// Render all templates and collect into a map (pure transformation)
///
/// Returns a [`HashMap`] of template type to rendered content.
///
/// # Errors
///
/// Returns [`TemplateError`] if any template fails to render
pub fn render_all_templates(
    context: &ProjectContext,
) -> Result<HashMap<TemplateType, String>, TemplateError> {
    TemplateType::all()
        .iter()
        .map(|&template_type| {
            render_template(template_type, context).map(|content| (template_type, content))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // A helper to create a valid context for tests
    fn valid_context() -> Result<ProjectContext, TemplateError> {
        ProjectContext::new(
            "test-project".to_string(),
            "A test project".to_string(),
            "0.1.0".to_string(),
            vec!["Author Name".to_string()],
            Some("https://github.com/test/repo".to_string()),
        )
    }

    #[test]
    fn test_project_context_valid() -> Result<(), TemplateError> {
        let context = valid_context()?;
        assert_eq!(context.project_name, "test-project");
        assert_eq!(context.version, "0.1.0");
        assert_eq!(context.authors.len(), 1);
        Ok(())
    }

    #[test]
    fn test_project_context_empty_name_rejected() {
        let result = ProjectContext::new(
            String::new(),
            "A test project".to_string(),
            "0.1.0".to_string(),
            vec!["Author".to_string()],
            None,
        );

        assert!(result.is_err());
        assert!(matches!(result, Err(TemplateError::InvalidContext { .. })));
    }

    #[test]
    fn test_project_context_empty_version_rejected() {
        let result = ProjectContext::new(
            "test".to_string(),
            "A test project".to_string(),
            String::new(),
            vec!["Author".to_string()],
            None,
        );
        assert!(result.is_err());
        assert!(matches!(result, Err(TemplateError::InvalidContext { .. })));
    }

    #[test]
    fn test_project_context_empty_authors_rejected() {
        let result = ProjectContext::new(
            "test".to_string(),
            "A test project".to_string(),
            "0.1.0".to_string(),
            vec![],
            None,
        );
        assert!(result.is_err());
        assert!(matches!(result, Err(TemplateError::InvalidContext { .. })));
    }

    #[test]
    fn test_render_template_success() -> Result<(), TemplateError> {
        let context = valid_context()?;
        let result = render_template(TemplateType::AgentsMd, &context);

        assert!(result.is_ok());
        if let Ok(rendered) = result {
            assert!(rendered.contains("test-project"));
            assert!(rendered.contains("A test project"));
            assert!(rendered.contains("Author Name"));
            assert!(rendered.contains("https://github.com/test/repo"));
        }
        Ok(())
    }

    #[test]
    fn test_render_all_templates() -> Result<(), TemplateError> {
        let context = valid_context()?;
        let result = render_all_templates(&context);

        assert!(result.is_ok());
        if let Ok(templates) = result {
            assert_eq!(templates.len(), TemplateType::all().len());
        }
        Ok(())
    }

    #[test]
    fn test_template_type_as_str() {
        assert_eq!(TemplateType::AgentsMd.as_str(), "agents.md");
        assert_eq!(TemplateType::ClaudeMd.as_str(), "claude.md");
        assert_eq!(TemplateType::MoonBuild.as_str(), "moon_build.md");
        assert_eq!(TemplateType::MoonWorkspace.as_str(), "workspace.yml");
        assert_eq!(TemplateType::MoonToolchain.as_str(), "toolchain.yml");
        assert_eq!(TemplateType::MoonTasks.as_str(), "tasks.yml");
    }

    #[test]
    fn test_template_type_all() {
        let all = TemplateType::all();
        assert_eq!(all.len(), 11);
        assert!(all.contains(&TemplateType::AgentsMd));
        assert!(all.contains(&TemplateType::ClaudeMd));
        assert!(all.contains(&TemplateType::MoonWorkspace));
        assert!(all.contains(&TemplateType::MoonToolchain));
        assert!(all.contains(&TemplateType::MoonTasks));
    }

    #[test]
    fn test_template_type_moon_configs() {
        let moon = TemplateType::moon_configs();
        assert_eq!(moon.len(), 3);
        assert!(moon.contains(&TemplateType::MoonWorkspace));
        assert!(moon.contains(&TemplateType::MoonToolchain));
        assert!(moon.contains(&TemplateType::MoonTasks));
    }

    #[test]
    fn test_render_moon_workspace_template() -> Result<(), TemplateError> {
        let context = valid_context()?;
        let result = render_template(TemplateType::MoonWorkspace, &context);

        assert!(result.is_ok());
        if let Ok(rendered) = result {
            assert!(rendered.contains("test-project"));
            assert!(rendered.contains("0.1.0"));
        }
        Ok(())
    }

    #[test]
    fn test_render_moon_tasks_template() -> Result<(), TemplateError> {
        let context = valid_context()?;
        let result = render_template(TemplateType::MoonTasks, &context);

        assert!(result.is_ok());
        if let Ok(rendered) = result {
            assert!(rendered.contains("test-project"));
            assert!(rendered.contains("target/release/test-project"));
            assert!(rendered.contains("~/.local/bin/test-project"));
        }
        Ok(())
    }
}
