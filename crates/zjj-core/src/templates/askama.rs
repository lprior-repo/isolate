//! Functional Askama Template Rendering
//!
//! Pure functional template rendering with zero-panic guarantees.
//! All operations return Result<T, TemplateError> with explicit error handling.
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

use thiserror::Error;

/// Template rendering errors
#[derive(Debug, Error, Clone, PartialEq)]
pub enum TemplateError {
    #[error("template not found: {name}")]
    TemplateNotFound { name: String },

    #[error("rendering failed for template '{name}': {reason}")]
    RenderFailed { name: String, reason: String },

    #[error("invalid context for template '{name}': {reason}")]
    InvalidContext { name: String, reason: String },

    #[error("missing required variable '{variable}' in template '{template}'")]
    MissingVariable {
        variable: String,
        template: String,
    },
}

/// Project configuration context for template rendering
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectContext {
    pub project_name: String,
    pub description: String,
    pub version: String,
    pub authors: Vec<String>,
    pub repository_url: Option<String>,
}

impl ProjectContext {
    /// Create a new project context with validation
    ///
    /// # Errors
    ///
    /// Returns `TemplateError::InvalidContext` if:
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
        ]
    }
}

/// Render a template with the given context (pure function)
///
/// This is a functional core operation - no I/O, pure transformation.
///
/// # Errors
///
/// Returns `TemplateError` if:
/// - Template variables are missing
/// - Rendering fails
pub fn render_template(
    template_type: TemplateType,
    context: &ProjectContext,
) -> Result<String, TemplateError> {
    // Pure function: input -> output, no side effects
    let template_content = get_template_content(template_type)?;

    // Simple string substitution (functional, no mutation)
    let result = template_content
        .replace("{{ project_name }}", &context.project_name)
        .replace("{{ description }}", &context.description)
        .replace("{{ version }}", &context.version)
        .replace("{{ authors }}", &context.authors.join(", "))
        .replace(
            "{{ repository_url }}",
            context.repository_url.as_deref().unwrap_or(""),
        );

    Ok(result)
}

/// Get template content by type (pure lookup)
///
/// # Errors
///
/// Returns `TemplateError::TemplateNotFound` if template type is invalid
fn get_template_content(template_type: TemplateType) -> Result<String, TemplateError> {
    match template_type {
        TemplateType::AgentsMd | TemplateType::ClaudeMd => {
            Ok(crate::templates::AI_INSTRUCTIONS.to_string())
        }
        TemplateType::ErrorHandling => Ok(crate::templates::DOC_01_ERROR_HANDLING.to_string()),
        TemplateType::RustStandards => Ok(crate::templates::DOC_05_RUST_STANDARDS.to_string()),
        TemplateType::Workflow => Ok(crate::templates::DOC_03_WORKFLOW.to_string()),
        TemplateType::Beads => Ok(crate::templates::DOC_08_BEADS.to_string()),
        TemplateType::Jujutsu => Ok(crate::templates::DOC_09_JUJUTSU.to_string()),
        TemplateType::MoonBuild => Ok(crate::templates::DOC_02_MOON_BUILD.to_string()),
    }
}

/// Render all templates and collect into a map (pure transformation)
///
/// Returns a HashMap of template type to rendered content.
///
/// # Errors
///
/// Returns `TemplateError` if any template fails to render
pub fn render_all_templates(
    context: &ProjectContext,
) -> Result<HashMap<TemplateType, String>, TemplateError> {
    TemplateType::all()
        .iter()
        .map(|&template_type| {
            render_template(template_type, context)
                .map(|content| (template_type, content))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_context_valid() {
        let context = ProjectContext::new(
            "test-project".to_string(),
            "A test project".to_string(),
            "0.1.0".to_string(),
            vec!["Author Name".to_string()],
            Some("https://github.com/test/repo".to_string()),
        );

        assert!(context.is_ok());
        let context = context.unwrap();
        assert_eq!(context.project_name, "test-project");
        assert_eq!(context.version, "0.1.0");
        assert_eq!(context.authors.len(), 1);
    }

    #[test]
    fn test_project_context_empty_name_rejected() {
        let result = ProjectContext::new(
            "".to_string(),
            "A test project".to_string(),
            "0.1.0".to_string(),
            vec!["Author".to_string()],
            None,
        );

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            TemplateError::InvalidContext {
                name: "ProjectContext".to_string(),
                reason: "project_name cannot be empty".to_string(),
            }
        );
    }

    #[test]
    fn test_project_context_empty_version_rejected() {
        let result = ProjectContext::new(
            "test".to_string(),
            "A test project".to_string(),
            "".to_string(),
            vec!["Author".to_string()],
            None,
        );

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            TemplateError::InvalidContext {
                name: "ProjectContext",
                reason: "version cannot be empty".to_string(),
            }
        );
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
        assert_eq!(
            result.unwrap_err(),
            TemplateError::InvalidContext {
                name: "ProjectContext",
                reason: "authors cannot be empty".to_string(),
            }
        );
    }

    #[test]
    fn test_render_template_success() {
        let context = ProjectContext::new(
            "my-project".to_string(),
            "My awesome project".to_string(),
            "1.0.0".to_string(),
            vec!["Alice".to_string(), "Bob".to_string()],
            None,
        )
        .unwrap();

        let result = render_template(TemplateType::AgentsMd, &context);

        assert!(result.is_ok());
        let content = result.unwrap();
        assert!(content.contains("my-project"));
        assert!(content.contains("My awesome project"));
    }

    #[test]
    fn test_render_all_templates() {
        let context = ProjectContext::new(
            "test".to_string(),
            "Test".to_string(),
            "0.1.0".to_string(),
            vec!["Author".to_string()],
            None,
        )
        .unwrap();

        let result = render_all_templates(&context);

        assert!(result.is_ok());
        let templates = result.unwrap();
        assert_eq!(templates.len(), TemplateType::all().len());
    }

    #[test]
    fn test_template_type_as_str() {
        assert_eq!(TemplateType::AgentsMd.as_str(), "agents.md");
        assert_eq!(TemplateType::ClaudeMd.as_str(), "claude.md");
        assert_eq!(TemplateType::MoonBuild.as_str(), "moon_build.md");
    }

    #[test]
    fn test_template_type_all() {
        let all = TemplateType::all();
        assert_eq!(all.len(), 8);
        assert!(all.contains(&TemplateType::AgentsMd));
        assert!(all.contains(&TemplateType::ClaudeMd));
    }

    // Property-based test: rendering never panics
    #[test]
    fn test_render_never_panics() {
        let contexts = vec![
            ProjectContext::new(
                "a".to_string(),
                "b".to_string(),
                "0.0.1".to_string(),
                vec!["x".to_string()],
                None,
            ),
            ProjectContext::new(
                "Long Project Name With Spaces 123".to_string(),
                "Desc".to_string(),
                "99.99.99".to_string(),
                vec!["Author1".to_string(), "Author2".to_string()],
                Some("https://example.com".to_string()),
            ),
        ];

        for context_result in contexts {
            let context = context_result.unwrap();
            for &template_type in TemplateType::all() {
                let _ = render_template(template_type, &context); // Never panics
            }
        }
    }
}
