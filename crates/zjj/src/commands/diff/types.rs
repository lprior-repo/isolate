//! Types for diff command

/// Options for the diff command (zjj-1d2)
#[derive(Debug, Clone, Copy, Default)]
pub struct DiffOptions {
    /// Show diffstat only (summary of changes)
    pub stat: bool,
    /// Output as JSON
    pub json: bool,
    /// Minimal output for pipes
    pub silent: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_options_default() {
        let options = DiffOptions::default();
        assert!(!options.stat);
        assert!(!options.json);
        assert!(!options.silent);
    }
}
