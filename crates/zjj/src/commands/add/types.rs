use zjj_core::OutputFormat;

/// Options for the add command
#[allow(clippy::struct_excessive_bools)]
pub struct AddOptions {
    /// Session name
    pub name: String,
    /// Optional bead/issue ID to associate with this session
    pub bead_id: Option<String>,
    /// Skip executing hooks
    pub no_hooks: bool,
    /// Template name to use for layout
    pub template: Option<String>,
    /// Create workspace but don't open Zellij tab
    pub no_open: bool,
    /// Skip Zellij integration entirely (for non-TTY environments)
    pub no_zellij: bool,
    /// Output format (JSON or Human-readable)
    pub format: OutputFormat,
    /// Succeed if session already exists (safe for retries)
    pub idempotent: bool,
    /// Preview without creating
    pub dry_run: bool,
}

impl AddOptions {
    /// Create new `AddOptions` with defaults
    #[allow(dead_code)]
    pub const fn new(name: String) -> Self {
        Self {
            name,
            bead_id: None,
            no_hooks: false,
            template: None,
            no_open: false,
            no_zellij: false,
            format: OutputFormat::Json,
            idempotent: false,
            dry_run: false,
        }
    }
}
