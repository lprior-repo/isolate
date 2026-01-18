//! Layout file generation - Imperative Shell combining pure logic with I/O
//!
//! This module handles file system operations for layout generation.
//! It combines the pure KDL generation from the kdl module with file I/O.

use std::path::{Path, PathBuf};

use super::config::LayoutConfig;
use super::kdl;
use crate::Result;

/// Generated layout information
#[derive(Debug, Clone)]
pub struct Layout {
    /// Generated KDL content
    pub kdl_content: String,
    /// Path where layout file is written
    pub file_path: PathBuf,
}

/// Generate a layout file for the given template
///
/// This function combines pure KDL generation with file I/O.
/// It calls the pure kdl module and then writes to disk.
///
/// # Errors
///
/// Returns error if:
/// - Unable to create layout directory
/// - Unable to write layout file
/// - Template generation fails
pub fn layout_generate(
    config: &LayoutConfig,
    template: super::config::LayoutTemplate,
    output_dir: &Path,
) -> Result<Layout> {
    // Create output directory
    std::fs::create_dir_all(output_dir)?;

    // Generate KDL content (pure function from kdl module)
    let kdl_content = kdl::generate_template_kdl(config, template)?;

    // Write to file
    let file_path = output_dir.join(format!("{}.kdl", config.session_name));
    std::fs::write(&file_path, &kdl_content)?;

    Ok(Layout {
        kdl_content,
        file_path,
    })
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::path::PathBuf;

    use super::*;

    fn test_config() -> LayoutConfig {
        LayoutConfig::new(
            "test-session".to_string(),
            PathBuf::from("/tmp/test-workspace"),
        )
    }

    // Additional test: Layout generation end-to-end
    #[test]
    fn test_layout_generate_creates_file() {
        let config = test_config();
        let output_dir = env::temp_dir().join("zjj-test-layouts");

        let result = layout_generate(&config, crate::zellij::LayoutTemplate::Minimal, &output_dir);
        assert!(result.is_ok());

        if let Ok(layout) = result {
            assert!(layout.file_path.exists());
            assert!(layout.kdl_content.contains("layout"));

            // Cleanup
            std::fs::remove_file(&layout.file_path).ok();
            std::fs::remove_dir(&output_dir).ok();
        }
    }
}
