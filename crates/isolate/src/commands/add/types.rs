use isolate_core::OutputFormat;

/// Options for the add command
#[allow(clippy::struct_excessive_bools)]
pub struct AddOptions {
    /// Session name
    pub name: String,
    /// Optional bead/issue ID to associate with this session
    pub bead_id: Option<String>,
    /// Optional template name
    pub template: Option<String>,
    /// Skip executing hooks
    pub no_hooks: bool,
    /// Create workspace but don't open
    pub no_open: bool,
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
            template: None,
            no_hooks: false,
            no_open: false,
            format: OutputFormat::Json,
            idempotent: false,
            dry_run: false,
        }
    }
}
