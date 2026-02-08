//! Progress indicators for long-running operations
//!
//! Provides progress bars and status indicators for operations that take >2 seconds.
//! Supports both terminal and Zellij environments, with --quiet flag support.
#![allow(dead_code)]

use std::{
    io::{self, Write},
    time::{Duration, Instant},
};

use thiserror::Error;

/// Configuration for progress reporting
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // Fields are part of public API
pub struct ProgressConfig {
    /// Minimum duration before showing progress (default: 2 seconds)
    pub min_duration: Duration,
    /// Whether progress is disabled (quiet mode)
    pub quiet: bool,
    /// Whether running in Zellij
    pub in_zellij: bool,
}

impl Default for ProgressConfig {
    fn default() -> Self {
        Self {
            min_duration: Duration::from_secs(2),
            quiet: false,
            in_zellij: false,
        }
    }
}

impl ProgressConfig {
    /// Create quiet configuration (no progress output)
    #[must_use]
    #[allow(dead_code)]
    // Constructor for quiet mode configuration
    pub const fn quiet() -> Self {
        Self {
            min_duration: Duration::ZERO,
            quiet: true,
            in_zellij: false,
        }
    }

    /// Create normal configuration
    #[must_use]
    #[allow(dead_code)]
    // Constructor for normal mode configuration
    pub const fn normal() -> Self {
        Self {
            min_duration: Duration::from_secs(2),
            quiet: false,
            in_zellij: false,
        }
    }

    /// Set Zellij mode
    #[must_use]
    #[allow(dead_code)]
    // Builder method for Zellij mode configuration
    pub const fn with_zellij(mut self, in_zellij: bool) -> Self {
        self.in_zellij = in_zellij;
        self
    }

    /// Set quiet mode
    #[must_use]
    #[allow(dead_code)]
    // Builder method for quiet mode configuration
    pub const fn with_quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        self
    }
}

/// Error type for progress operations
#[derive(Debug, Clone, Error)]
#[allow(dead_code)]
// Error type for progress tracking operations
pub enum ProgressError {
    /// Invalid phase transition
    #[error("invalid phase transition from {from:?} to {to:?}")]
    InvalidPhaseTransition {
        from: OperationPhase,
        to: OperationPhase,
    },

    /// Progress operation failed
    #[error("progress operation failed: {0}")]
    OperationFailed(String),
}

/// Result type for progress operations
#[allow(dead_code)]
// Type alias for progress operation results
pub type ProgressResult<T> = Result<T, ProgressError>;

/// Phases of a long-running operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(dead_code)] // Part of public API
pub enum OperationPhase {
    /// Operation is starting
    Initializing,
    /// Validating preconditions
    Validating,
    /// Executing main operation
    Executing,
    /// Cleaning up resources
    CleaningUp,
    /// Operation completed successfully
    Complete,
    /// Operation failed
    Failed,
}

impl OperationPhase {
    /// Get display name for the phase
    #[must_use]
    #[allow(dead_code)] // Part of public API
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Initializing => "Initializing",
            Self::Validating => "Validating",
            Self::Executing => "Executing",
            Self::CleaningUp => "Cleaning up",
            Self::Complete => "Complete",
            Self::Failed => "Failed",
        }
    }

    /// Check if this is a terminal phase (no transitions out)
    #[must_use]
    #[allow(dead_code)] // Part of public API
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Complete | Self::Failed)
    }

    /// Check if this is a starting phase
    #[must_use]
    #[allow(dead_code)] // Part of public API
    pub const fn is_starting(&self) -> bool {
        matches!(self, Self::Initializing)
    }
}

/// Progress tracking state
#[derive(Debug, Clone)]
struct ProgressState {
    /// Current phase
    phase: OperationPhase,
    /// When current phase started
    phase_start: Instant,
    /// When overall operation started
    operation_start: Instant,
    /// Total elapsed time (after completion)
    total_elapsed: Option<Duration>,
}

impl Default for ProgressState {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            phase: OperationPhase::Initializing,
            phase_start: now,
            operation_start: now,
            total_elapsed: None,
        }
    }
}

/// Progress indicator for long-running operations
///
/// # Example
///
/// ```no_run
/// use zjj::progress::{ProgressConfig, ProgressIndicator};
///
/// let config = ProgressConfig::normal();
/// let mut indicator = ProgressIndicator::new(config);
///
/// indicator.start();
/// indicator.complete()?;
/// # Ok::<(), zjj::progress::ProgressError>(())
/// ```
#[derive(Debug)]
pub struct ProgressIndicator {
    /// Configuration
    config: ProgressConfig,
    /// Current state
    state: ProgressState,
}

impl ProgressIndicator {
    /// Create a new progress indicator
    pub fn new(config: ProgressConfig) -> Self {
        let now = Instant::now();
        Self {
            config,
            state: ProgressState {
                phase: OperationPhase::Initializing,
                phase_start: now,
                operation_start: now,
                total_elapsed: None,
            },
        }
    }

    /// Start tracking progress
    pub fn start(&mut self) {
        self.state = ProgressState::default();
    }

    /// Transition to a new phase
    ///
    /// # Errors
    ///
    /// Returns error if transition is invalid (e.g., from terminal phase)
    pub fn transition_to(&mut self, phase: OperationPhase) -> ProgressResult<()> {
        // Validate transition
        if self.state.phase.is_terminal() {
            return Err(ProgressError::InvalidPhaseTransition {
                from: self.state.phase,
                to: phase,
            });
        }

        self.state.phase = phase;
        self.state.phase_start = Instant::now();
        Ok(())
    }

    /// Mark operation as complete
    pub fn complete(&mut self) -> ProgressResult<()> {
        self.transition_to(OperationPhase::Complete)?;
        self.state.total_elapsed = Some(self.state.operation_start.elapsed());
        Ok(())
    }

    /// Mark operation as failed
    pub fn fail(&mut self) -> ProgressResult<()> {
        self.transition_to(OperationPhase::Failed)?;
        self.state.total_elapsed = Some(self.state.operation_start.elapsed());
        Ok(())
    }

    /// Get current phase
    #[must_use]
    pub const fn phase(&self) -> OperationPhase {
        self.state.phase
    }

    /// Get elapsed time for current phase
    #[must_use]
    pub fn phase_elapsed(&self) -> Duration {
        self.state.phase_start.elapsed()
    }

    /// Get total elapsed time for operation
    #[must_use]
    #[allow(clippy::option_if_let_else)]
    // Match is clearer than map_or_else for explaining fallback logic
    pub fn total_elapsed(&self) -> Duration {
        match self.state.total_elapsed {
            Some(duration) => duration,
            None => self.state.operation_start.elapsed(),
        }
    }

    /// Check if progress should be displayed
    #[must_use]
    pub fn should_display(&self) -> bool {
        if self.config.quiet {
            return false;
        }

        self.total_elapsed() >= self.config.min_duration
    }

    /// Get configuration
    #[must_use]
    pub const fn config(&self) -> &ProgressConfig {
        &self.config
    }
}

/// Trait for reporting progress
pub trait ProgressReporter {
    /// Report the current progress state
    fn report(&mut self, indicator: &ProgressIndicator) -> io::Result<()>;

    /// Finalize the progress report (e.g., clear the progress bar)
    fn finalize(&mut self, indicator: &ProgressIndicator) -> io::Result<()>;
}

/// Terminal-based progress reporter using simple text output
pub struct TerminalReporter {
    stdout: io::Stdout,
    last_phase: Option<OperationPhase>,
}

impl TerminalReporter {
    /// Create a new terminal reporter
    #[must_use]
    pub fn new() -> Self {
        Self {
            stdout: io::stdout(),
            last_phase: None,
        }
    }
}

impl Default for TerminalReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressReporter for TerminalReporter {
    fn report(&mut self, indicator: &ProgressIndicator) -> io::Result<()> {
        // Only report if phase changed or it's been a while
        let current_phase = indicator.phase();

        if self.last_phase == Some(current_phase) && !current_phase.is_terminal() {
            return Ok(());
        }

        self.last_phase = Some(current_phase);

        let elapsed = indicator.total_elapsed();
        let phase_name = current_phase.display_name();

        writeln!(
            self.stdout.lock(),
            "[{:?}] {} - {:.2}s",
            current_phase,
            phase_name,
            elapsed.as_secs_f64()
        )
    }

    fn finalize(&mut self, indicator: &ProgressIndicator) -> io::Result<()> {
        let elapsed = indicator.total_elapsed();
        let status = if indicator.phase() == OperationPhase::Complete {
            "✓ Complete"
        } else {
            "✗ Failed"
        };

        writeln!(
            self.stdout.lock(),
            "{} in {:.2}s",
            status,
            elapsed.as_secs_f64()
        )
    }
}

/// Quiet reporter that produces no output
pub struct QuietReporter;

impl QuietReporter {
    /// Create a new quiet reporter
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for QuietReporter {
    fn default() -> Self {
        Self
    }
}

impl ProgressReporter for QuietReporter {
    fn report(&mut self, _indicator: &ProgressIndicator) -> io::Result<()> {
        Ok(())
    }

    fn finalize(&mut self, _indicator: &ProgressIndicator) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // Martin Fowler Style Behavior Tests
    // ============================================================================

    mod operation_phase_behavior {
        use super::*;

        /// GIVEN: Any operation phase
        /// WHEN: Display name is requested
        /// THEN: Should return human-readable name
        #[test]
        fn phases_have_display_names() {
            assert_eq!(OperationPhase::Initializing.display_name(), "Initializing");
            assert_eq!(OperationPhase::Validating.display_name(), "Validating");
            assert_eq!(OperationPhase::Executing.display_name(), "Executing");
            assert_eq!(OperationPhase::CleaningUp.display_name(), "Cleaning up");
            assert_eq!(OperationPhase::Complete.display_name(), "Complete");
            assert_eq!(OperationPhase::Failed.display_name(), "Failed");
        }

        /// GIVEN: Terminal phases (Complete, Failed)
        /// WHEN: Check if terminal
        /// THEN: Should return true
        #[test]
        fn terminal_phases_cannot_transition() {
            assert!(OperationPhase::Complete.is_terminal());
            assert!(OperationPhase::Failed.is_terminal());
            assert!(!OperationPhase::Executing.is_terminal());
        }

        /// GIVEN: Starting phase
        /// WHEN: Check if starting
        /// THEN: Should return true for Initializing
        #[test]
        fn initializing_is_starting_phase() {
            assert!(OperationPhase::Initializing.is_starting());
            assert!(!OperationPhase::Executing.is_starting());
        }
    }

    mod progress_indicator_behavior {
        use super::*;

        /// GIVEN: New progress indicator
        /// WHEN: Created with default config
        /// THEN: Should start in Initializing phase
        #[test]
        fn indicator_starts_in_initializing() {
            let indicator = ProgressIndicator::new(ProgressConfig::default());

            assert_eq!(indicator.phase(), OperationPhase::Initializing);
        }

        /// GIVEN: Progress indicator in active phase
        /// WHEN: Transition to new phase
        /// THEN: Phase should update
        #[test]
        fn can_transition_from_active_phases() {
            let mut indicator = ProgressIndicator::new(ProgressConfig::default());
            indicator.start();

            let result = indicator.transition_to(OperationPhase::Validating);

            assert!(result.is_ok(), "Should transition from Initializing");
            assert_eq!(indicator.phase(), OperationPhase::Validating);
        }

        /// GIVEN: Progress indicator in terminal phase
        /// WHEN: Attempt to transition
        /// THEN: Should return error
        #[test]
        fn cannot_transition_from_terminal_phases() {
            let mut indicator = ProgressIndicator::new(ProgressConfig::default());
            indicator.start();
            assert!(
                indicator.complete().is_ok(),
                "Setup: should complete to reach terminal phase"
            );

            let result = indicator.transition_to(OperationPhase::Executing);

            assert!(result.is_err(), "Should not transition from Complete");
            if let Err(ProgressError::InvalidPhaseTransition { from, to }) = result {
                assert_eq!(from, OperationPhase::Complete);
                assert_eq!(to, OperationPhase::Executing);
            } else {
                panic!("Expected InvalidPhaseTransition error, got: {result:?}");
            }
        }

        /// GIVEN: Progress indicator
        /// WHEN: Mark as complete
        /// THEN: Should be in Complete phase
        #[test]
        fn complete_moves_to_terminal_state() {
            let mut indicator = ProgressIndicator::new(ProgressConfig::default());
            indicator.start();

            assert!(indicator.complete().is_ok(), "Should complete successfully");

            assert_eq!(indicator.phase(), OperationPhase::Complete);
            assert!(indicator.phase().is_terminal());
        }

        /// GIVEN: Progress indicator
        /// WHEN: Mark as failed
        /// THEN: Should be in Failed phase
        #[test]
        fn fail_moves_to_terminal_state() {
            let mut indicator = ProgressIndicator::new(ProgressConfig::default());
            indicator.start();

            assert!(indicator.fail().is_ok(), "Should fail successfully");

            assert_eq!(indicator.phase(), OperationPhase::Failed);
            assert!(indicator.phase().is_terminal());
        }

        /// GIVEN: Progress indicator with 2 second min duration
        /// WHEN: Only 1 second has elapsed
        /// THEN: Should not display
        #[test]
        fn should_display_respects_min_duration() {
            let config = ProgressConfig {
                min_duration: Duration::from_secs(2),
                quiet: false,
                in_zellij: false,
            };
            let indicator = ProgressIndicator::new(config);

            // Less than min duration
            assert!(!indicator.should_display());
        }

        /// GIVEN: Progress indicator in quiet mode
        /// WHEN: Any time has elapsed
        /// THEN: Should never display
        #[test]
        fn quiet_mode_never_displays() {
            let config = ProgressConfig::quiet();
            let indicator = ProgressIndicator::new(config);

            assert!(!indicator.should_display());
        }

        /// GIVEN: Progress indicator
        /// WHEN: Elapsed time is queried
        /// THEN: Should return positive duration
        #[tokio::test]
        async fn elapsed_time_increases() {
            let mut indicator = ProgressIndicator::new(ProgressConfig::default());
            indicator.start();

            tokio::time::sleep(Duration::from_millis(10)).await;

            let elapsed = indicator.total_elapsed();
            assert!(elapsed >= Duration::from_millis(10));
        }

        /// GIVEN: Progress indicator completes
        /// WHEN: Total elapsed is queried
        /// THEN: Should return fixed duration
        #[tokio::test]
        async fn total_elapsed_is_fixed_after_completion() {
            let mut indicator = ProgressIndicator::new(ProgressConfig::default());
            indicator.start();

            tokio::time::sleep(Duration::from_millis(10)).await;
            assert!(
                indicator.complete().is_ok(),
                "Setup: should complete to measure elapsed time"
            );

            let elapsed1 = indicator.total_elapsed();
            tokio::time::sleep(Duration::from_millis(10)).await;
            let elapsed2 = indicator.total_elapsed();

            assert_eq!(
                elapsed1, elapsed2,
                "Total elapsed should be fixed after completion"
            );
        }
    }

    mod progress_config_behavior {
        use super::*;

        /// GIVEN: Default config
        /// WHEN: Created
        /// THEN: Should have 2 second min duration
        #[test]
        fn default_config_has_2_second_min() {
            let config = ProgressConfig::default();

            assert_eq!(config.min_duration, Duration::from_secs(2));
            assert!(!config.quiet);
            assert!(!config.in_zellij);
        }

        /// GIVEN: Quiet config
        /// WHEN: Created
        /// THEN: Should have quiet flag set
        #[test]
        fn quiet_config_is_quiet() {
            let config = ProgressConfig::quiet();

            assert!(config.quiet);
            assert_eq!(config.min_duration, Duration::ZERO);
        }

        /// GIVEN: Normal config
        /// WHEN: Created
        /// THEN: Should have 2 second min duration
        #[test]
        fn normal_config_has_standard_duration() {
            let config = ProgressConfig::normal();

            assert_eq!(config.min_duration, Duration::from_secs(2));
            assert!(!config.quiet);
        }

        /// GIVEN: Config
        /// WHEN: Zellij mode is set
        /// THEN: Should reflect zellij setting
        #[test]
        fn with_zellij_sets_zellij_flag() {
            let config = ProgressConfig::normal().with_zellij(true);

            assert!(config.in_zellij);
        }

        /// GIVEN: Config
        /// WHEN: Quiet mode is set
        /// THEN: Should reflect quiet setting
        #[test]
        fn with_quiet_sets_quiet_flag() {
            let config = ProgressConfig::normal().with_quiet(true);

            assert!(config.quiet);
        }
    }

    mod reporter_behavior {
        use super::*;

        /// GIVEN: Terminal reporter
        /// WHEN: Report is called
        /// THEN: Should write to stdout
        #[test]
        fn terminal_reporter_writes_to_stdout() {
            let config = ProgressConfig::normal();
            let mut indicator = ProgressIndicator::new(config);
            let mut reporter = TerminalReporter::new();

            indicator.start();

            let result = reporter.report(&indicator);

            assert!(result.is_ok());
        }

        /// GIVEN: Quiet reporter
        /// WHEN: Report is called
        /// THEN: Should produce no output
        #[test]
        fn quiet_reporter_produces_no_output() {
            let config = ProgressConfig::quiet();
            let indicator = ProgressIndicator::new(config);
            let mut reporter = QuietReporter::new();

            let result = reporter.report(&indicator);

            assert!(result.is_ok());
        }

        /// GIVEN: Terminal reporter
        /// WHEN: Finalize is called with complete indicator
        /// THEN: Should write completion message
        #[test]
        fn terminal_reporter_finalizes_complete() {
            let config = ProgressConfig::normal();
            let mut indicator = ProgressIndicator::new(config);
            let mut reporter = TerminalReporter::new();

            indicator.start();
            assert!(
                indicator.complete().is_ok(),
                "Setup: should complete indicator for test"
            );

            let result = reporter.finalize(&indicator);

            assert!(result.is_ok());
        }

        /// GIVEN: Terminal reporter
        /// WHEN: Finalize is called with failed indicator
        /// THEN: Should write failure message
        #[test]
        fn terminal_reporter_finalizes_failed() {
            let config = ProgressConfig::normal();
            let mut indicator = ProgressIndicator::new(config);
            let mut reporter = TerminalReporter::new();

            indicator.start();
            assert!(
                indicator.fail().is_ok(),
                "Setup: should fail indicator for test"
            );

            let result = reporter.finalize(&indicator);

            assert!(result.is_ok());
        }
    }
}
