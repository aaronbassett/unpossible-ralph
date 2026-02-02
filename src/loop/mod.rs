//! Core orchestration loop for ralph.
//!
//! This module provides the state machine and types for managing the AI agent loop.
//! The loop runs development agents, verifies work via review agents, and routes
//! to the next task based on `RESULT: CONTINUE|REPEAT|DONE` signals.
//!
//! # Components
//!
//! - [`LoopState`] - Current state of the orchestration loop
//! - [`LoopOutcome`] - Final outcome when the loop terminates
//! - [`IterationSummary`] - Summary of a single agent execution (for display)
//! - [`LoopExecutor`] - Executor that manages loop state and configuration
//!
//! # State Machine
//!
//! The loop follows this state machine:
//!
//! 1. Start with `iteration_count = 1`, `retry_count = 0`
//! 2. Run dev agent with starting prompt (iteration 1) or continuation prompt (iteration 2+)
//! 3. Run review agent to evaluate output
//! 4. Based on RESULT:
//!    - `CONTINUE`: Run next-action agent, increment iteration, reset retry
//!    - `REPEAT`: Increment retry, check max_retries limit
//!    - `DONE`: Exit with success
//! 5. Loop until `DONE`, `max_iterations`, or `max_retries` exceeded
//!
//! # Example
//!
//! ```ignore
//! use ralph::config::Config;
//! use ralph::r#loop::{LoopExecutor, LoopState, LoopOutcome};
//!
//! let executor = LoopExecutor::new(config);
//! // Full run loop will be implemented in Phase 9
//! ```

mod executor;

pub use executor::LoopExecutor;

use std::fmt;

/// Current state of the orchestration loop.
///
/// This struct tracks:
/// - `iteration_count`: 1-indexed counter incremented after each successful task completion
/// - `retry_count`: Consecutive failures, reset to 0 on `RESULT: CONTINUE`
/// - `next_prompt`: Output from the next-action agent for the continuation prompt
///
/// # Invariants
///
/// - `iteration_count >= 1` (starts at 1, incremented on CONTINUE)
/// - `retry_count >= 0` (reset on CONTINUE, incremented on REPEAT or dev failure)
/// - `next_prompt` is empty for iteration 1, populated for iteration 2+
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopState {
    /// Current iteration number (1-indexed).
    ///
    /// Incremented after each successful task completion (RESULT: CONTINUE).
    pub iteration_count: u32,

    /// Consecutive failure count.
    ///
    /// Incremented on dev agent failure (non-zero exit) or `RESULT: REPEAT`.
    /// Reset to 0 on `RESULT: CONTINUE`.
    pub retry_count: u32,

    /// Next task prompt from the next-action agent.
    ///
    /// Empty for the first iteration. Populated with the next-action agent's
    /// output after each `RESULT: CONTINUE`.
    pub next_prompt: String,
}

impl Default for LoopState {
    fn default() -> Self {
        Self::new()
    }
}

impl LoopState {
    /// Creates a new `LoopState` with initial values.
    ///
    /// - `iteration_count = 1`
    /// - `retry_count = 0`
    /// - `next_prompt = ""` (empty)
    ///
    /// # Example
    ///
    /// ```
    /// use ralph::r#loop::LoopState;
    ///
    /// let state = LoopState::new();
    /// assert_eq!(state.iteration_count, 1);
    /// assert_eq!(state.retry_count, 0);
    /// assert!(state.next_prompt.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            iteration_count: 1,
            retry_count: 0,
            next_prompt: String::new(),
        }
    }

    /// Increments the retry count by 1.
    ///
    /// Called when:
    /// - Dev agent exits with non-zero code
    /// - Review agent returns `RESULT: REPEAT`
    ///
    /// # Example
    ///
    /// ```
    /// use ralph::r#loop::LoopState;
    ///
    /// let mut state = LoopState::new();
    /// assert_eq!(state.retry_count, 0);
    ///
    /// state.increment_retry();
    /// assert_eq!(state.retry_count, 1);
    ///
    /// state.increment_retry();
    /// assert_eq!(state.retry_count, 2);
    /// ```
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }

    /// Resets the retry count to 0.
    ///
    /// Called when `RESULT: CONTINUE` is received, indicating successful
    /// task completion. The retry counter starts fresh for the next iteration.
    ///
    /// # Example
    ///
    /// ```
    /// use ralph::r#loop::LoopState;
    ///
    /// let mut state = LoopState::new();
    /// state.increment_retry();
    /// state.increment_retry();
    /// assert_eq!(state.retry_count, 2);
    ///
    /// state.reset_retry();
    /// assert_eq!(state.retry_count, 0);
    /// ```
    pub fn reset_retry(&mut self) {
        self.retry_count = 0;
    }

    /// Transitions to the next iteration.
    ///
    /// Called when `RESULT: CONTINUE` is received. This method:
    /// - Increments `iteration_count` by 1
    /// - Resets `retry_count` to 0
    /// - Stores the `next_prompt` for the continuation template
    ///
    /// # Arguments
    ///
    /// * `next_prompt` - The output from the next-action agent that will be
    ///   used in the continuation prompt template via `{{next_prompt}}`.
    ///
    /// # Example
    ///
    /// ```
    /// use ralph::r#loop::LoopState;
    ///
    /// let mut state = LoopState::new();
    /// state.increment_retry(); // Simulate a failed attempt
    /// assert_eq!(state.iteration_count, 1);
    /// assert_eq!(state.retry_count, 1);
    ///
    /// state.next_iteration("Fix the failing tests".to_string());
    /// assert_eq!(state.iteration_count, 2);
    /// assert_eq!(state.retry_count, 0);
    /// assert_eq!(state.next_prompt, "Fix the failing tests");
    /// ```
    pub fn next_iteration(&mut self, next_prompt: String) {
        self.iteration_count += 1;
        self.retry_count = 0;
        self.next_prompt = next_prompt;
    }

    /// Returns `true` if this is the first iteration.
    ///
    /// The first iteration uses the `starting` prompt template instead of
    /// the `continuation` prompt template.
    ///
    /// # Example
    ///
    /// ```
    /// use ralph::r#loop::LoopState;
    ///
    /// let mut state = LoopState::new();
    /// assert!(state.is_first_iteration());
    ///
    /// state.next_iteration("next task".to_string());
    /// assert!(!state.is_first_iteration());
    /// ```
    #[must_use]
    pub fn is_first_iteration(&self) -> bool {
        self.iteration_count == 1
    }
}

/// Summary of a single agent execution.
///
/// This struct captures information needed for displaying progress after
/// each agent runs (FR17). It provides a snapshot of what happened during
/// the agent execution without storing the full output.
///
/// # Display Format (FR17)
///
/// ```text
/// [iter 3] dev: exit=0, stdout=1234 bytes, stderr=0 bytes
/// ```
#[derive(Debug, Clone)]
pub struct IterationSummary {
    /// Current iteration number when this agent was run.
    pub iteration: u32,

    /// Which agent was executed.
    ///
    /// One of: `"dev"`, `"review"`, `"next-action"`
    pub agent: &'static str,

    /// Exit code of the agent process.
    ///
    /// `None` if the process was killed by a signal.
    pub exit_code: Option<i32>,

    /// Number of bytes captured from stdout.
    pub stdout_bytes: usize,

    /// Number of bytes captured from stderr.
    pub stderr_bytes: usize,
}

impl IterationSummary {
    /// Creates a new `IterationSummary`.
    ///
    /// # Arguments
    ///
    /// * `iteration` - Current iteration number
    /// * `agent` - Agent name ("dev", "review", or "next-action")
    /// * `exit_code` - Process exit code (None if killed by signal)
    /// * `stdout_bytes` - Bytes captured from stdout
    /// * `stderr_bytes` - Bytes captured from stderr
    #[must_use]
    pub fn new(
        iteration: u32,
        agent: &'static str,
        exit_code: Option<i32>,
        stdout_bytes: usize,
        stderr_bytes: usize,
    ) -> Self {
        Self {
            iteration,
            agent,
            exit_code,
            stdout_bytes,
            stderr_bytes,
        }
    }

    /// Returns `true` if the agent exited successfully (exit code 0).
    #[must_use]
    pub fn success(&self) -> bool {
        self.exit_code == Some(0)
    }
}

impl fmt::Display for IterationSummary {
    /// Formats the summary for display (FR17).
    ///
    /// Format: `[iter N] agent: exit=CODE, stdout=X bytes, stderr=Y bytes`
    ///
    /// If the process was killed by a signal, shows `exit=signal` instead.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let exit_str = match self.exit_code {
            Some(code) => code.to_string(),
            None => "signal".to_string(),
        };

        write!(
            f,
            "[iter {}] {}: exit={}, stdout={} bytes, stderr={} bytes",
            self.iteration, self.agent, exit_str, self.stdout_bytes, self.stderr_bytes
        )
    }
}

/// Final outcome when the orchestration loop terminates.
///
/// The loop can terminate in several ways, each with different exit codes:
///
/// | Outcome | Exit Code | Meaning |
/// |---------|-----------|---------|
/// | `Done` | 0 | `RESULT: DONE` received |
/// | `MaxIterations` | 0 | `max_iterations` limit reached |
/// | `MaxRetries` | 2 | `retry_count` exceeded `max_retries` |
/// | `Error` | 1 | Fatal error occurred |
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopOutcome {
    /// The review agent returned `RESULT: DONE`.
    ///
    /// All work is complete. Exit code: 0
    Done,

    /// The `max_iterations` limit was reached.
    ///
    /// This is a successful termination. Exit code: 0
    MaxIterations,

    /// The `max_retries` limit was exceeded.
    ///
    /// Contains the retry count at the time of failure.
    /// Exit code: 2
    MaxRetries {
        /// The retry count when the limit was exceeded.
        count: u32,
    },

    /// A fatal error occurred during loop execution.
    ///
    /// Exit code: 1
    Error(String),
}

impl LoopOutcome {
    /// Returns the exit code for this outcome.
    ///
    /// | Outcome | Exit Code |
    /// |---------|-----------|
    /// | `Done` | 0 |
    /// | `MaxIterations` | 0 |
    /// | `MaxRetries` | 2 |
    /// | `Error` | 1 |
    ///
    /// # Example
    ///
    /// ```
    /// use ralph::r#loop::LoopOutcome;
    ///
    /// assert_eq!(LoopOutcome::Done.exit_code(), 0);
    /// assert_eq!(LoopOutcome::MaxIterations.exit_code(), 0);
    /// assert_eq!(LoopOutcome::MaxRetries { count: 3 }.exit_code(), 2);
    /// assert_eq!(LoopOutcome::Error("failed".into()).exit_code(), 1);
    /// ```
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        match self {
            LoopOutcome::Done | LoopOutcome::MaxIterations => 0,
            LoopOutcome::MaxRetries { .. } => 2,
            LoopOutcome::Error(_) => 1,
        }
    }

    /// Returns `true` if this is a successful outcome.
    ///
    /// Both `Done` and `MaxIterations` are considered successful.
    ///
    /// # Example
    ///
    /// ```
    /// use ralph::r#loop::LoopOutcome;
    ///
    /// assert!(LoopOutcome::Done.is_success());
    /// assert!(LoopOutcome::MaxIterations.is_success());
    /// assert!(!LoopOutcome::MaxRetries { count: 3 }.is_success());
    /// assert!(!LoopOutcome::Error("failed".into()).is_success());
    /// ```
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, LoopOutcome::Done | LoopOutcome::MaxIterations)
    }
}

impl fmt::Display for LoopOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoopOutcome::Done => write!(f, "Completed: RESULT: DONE received"),
            LoopOutcome::MaxIterations => write!(f, "Completed: max_iterations limit reached"),
            LoopOutcome::MaxRetries { count } => {
                write!(f, "Failed: max_retries exceeded (retry count: {count})")
            }
            LoopOutcome::Error(msg) => write!(f, "Error: {msg}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // LoopState tests
    // =========================================================================

    #[test]
    fn test_loop_state_new() {
        let state = LoopState::new();
        assert_eq!(state.iteration_count, 1);
        assert_eq!(state.retry_count, 0);
        assert!(state.next_prompt.is_empty());
    }

    #[test]
    fn test_loop_state_default() {
        let state = LoopState::default();
        assert_eq!(state, LoopState::new());
    }

    #[test]
    fn test_loop_state_increment_retry() {
        let mut state = LoopState::new();
        assert_eq!(state.retry_count, 0);

        state.increment_retry();
        assert_eq!(state.retry_count, 1);

        state.increment_retry();
        assert_eq!(state.retry_count, 2);

        state.increment_retry();
        assert_eq!(state.retry_count, 3);
    }

    #[test]
    fn test_loop_state_reset_retry() {
        let mut state = LoopState::new();
        state.increment_retry();
        state.increment_retry();
        state.increment_retry();
        assert_eq!(state.retry_count, 3);

        state.reset_retry();
        assert_eq!(state.retry_count, 0);
    }

    #[test]
    fn test_loop_state_reset_retry_when_zero() {
        let mut state = LoopState::new();
        assert_eq!(state.retry_count, 0);

        state.reset_retry();
        assert_eq!(state.retry_count, 0);
    }

    #[test]
    fn test_loop_state_next_iteration() {
        let mut state = LoopState::new();
        state.increment_retry(); // Simulate a failed attempt
        assert_eq!(state.iteration_count, 1);
        assert_eq!(state.retry_count, 1);
        assert!(state.next_prompt.is_empty());

        state.next_iteration("Fix the tests".to_string());
        assert_eq!(state.iteration_count, 2);
        assert_eq!(state.retry_count, 0);
        assert_eq!(state.next_prompt, "Fix the tests");
    }

    #[test]
    fn test_loop_state_multiple_iterations() {
        let mut state = LoopState::new();

        // First iteration complete
        state.next_iteration("Task 2".to_string());
        assert_eq!(state.iteration_count, 2);
        assert_eq!(state.next_prompt, "Task 2");

        // Second iteration with retries
        state.increment_retry();
        state.increment_retry();
        assert_eq!(state.retry_count, 2);

        // Second iteration complete
        state.next_iteration("Task 3".to_string());
        assert_eq!(state.iteration_count, 3);
        assert_eq!(state.retry_count, 0);
        assert_eq!(state.next_prompt, "Task 3");
    }

    #[test]
    fn test_loop_state_is_first_iteration() {
        let mut state = LoopState::new();
        assert!(state.is_first_iteration());

        state.next_iteration("next".to_string());
        assert!(!state.is_first_iteration());

        state.next_iteration("another".to_string());
        assert!(!state.is_first_iteration());
    }

    #[test]
    fn test_loop_state_next_prompt_overwrites() {
        let mut state = LoopState::new();

        state.next_iteration("First task".to_string());
        assert_eq!(state.next_prompt, "First task");

        state.next_iteration("Second task".to_string());
        assert_eq!(state.next_prompt, "Second task");
    }

    #[test]
    fn test_loop_state_empty_next_prompt() {
        let mut state = LoopState::new();
        state.next_iteration(String::new());

        assert_eq!(state.iteration_count, 2);
        assert_eq!(state.retry_count, 0);
        assert!(state.next_prompt.is_empty());
    }

    // =========================================================================
    // IterationSummary tests
    // =========================================================================

    #[test]
    fn test_iteration_summary_new() {
        let summary = IterationSummary::new(1, "dev", Some(0), 1024, 0);

        assert_eq!(summary.iteration, 1);
        assert_eq!(summary.agent, "dev");
        assert_eq!(summary.exit_code, Some(0));
        assert_eq!(summary.stdout_bytes, 1024);
        assert_eq!(summary.stderr_bytes, 0);
    }

    #[test]
    fn test_iteration_summary_success() {
        let success = IterationSummary::new(1, "dev", Some(0), 100, 0);
        assert!(success.success());

        let failure = IterationSummary::new(1, "dev", Some(1), 100, 50);
        assert!(!failure.success());

        let signal = IterationSummary::new(1, "dev", None, 100, 0);
        assert!(!signal.success());
    }

    #[test]
    fn test_iteration_summary_display_success() {
        let summary = IterationSummary::new(3, "dev", Some(0), 1234, 0);
        let display = summary.to_string();

        assert_eq!(
            display,
            "[iter 3] dev: exit=0, stdout=1234 bytes, stderr=0 bytes"
        );
    }

    #[test]
    fn test_iteration_summary_display_failure() {
        let summary = IterationSummary::new(2, "review", Some(1), 500, 250);
        let display = summary.to_string();

        assert_eq!(
            display,
            "[iter 2] review: exit=1, stdout=500 bytes, stderr=250 bytes"
        );
    }

    #[test]
    fn test_iteration_summary_display_signal() {
        let summary = IterationSummary::new(1, "next-action", None, 0, 100);
        let display = summary.to_string();

        assert_eq!(
            display,
            "[iter 1] next-action: exit=signal, stdout=0 bytes, stderr=100 bytes"
        );
    }

    #[test]
    fn test_iteration_summary_all_agents() {
        let dev = IterationSummary::new(1, "dev", Some(0), 100, 0);
        assert_eq!(dev.agent, "dev");

        let review = IterationSummary::new(1, "review", Some(0), 100, 0);
        assert_eq!(review.agent, "review");

        let next_action = IterationSummary::new(1, "next-action", Some(0), 100, 0);
        assert_eq!(next_action.agent, "next-action");
    }

    // =========================================================================
    // LoopOutcome tests
    // =========================================================================

    #[test]
    fn test_loop_outcome_done() {
        let outcome = LoopOutcome::Done;
        assert_eq!(outcome.exit_code(), 0);
        assert!(outcome.is_success());
    }

    #[test]
    fn test_loop_outcome_max_iterations() {
        let outcome = LoopOutcome::MaxIterations;
        assert_eq!(outcome.exit_code(), 0);
        assert!(outcome.is_success());
    }

    #[test]
    fn test_loop_outcome_max_retries() {
        let outcome = LoopOutcome::MaxRetries { count: 5 };
        assert_eq!(outcome.exit_code(), 2);
        assert!(!outcome.is_success());
    }

    #[test]
    fn test_loop_outcome_error() {
        let outcome = LoopOutcome::Error("something went wrong".to_string());
        assert_eq!(outcome.exit_code(), 1);
        assert!(!outcome.is_success());
    }

    #[test]
    fn test_loop_outcome_display_done() {
        let outcome = LoopOutcome::Done;
        assert_eq!(outcome.to_string(), "Completed: RESULT: DONE received");
    }

    #[test]
    fn test_loop_outcome_display_max_iterations() {
        let outcome = LoopOutcome::MaxIterations;
        assert_eq!(
            outcome.to_string(),
            "Completed: max_iterations limit reached"
        );
    }

    #[test]
    fn test_loop_outcome_display_max_retries() {
        let outcome = LoopOutcome::MaxRetries { count: 3 };
        assert_eq!(
            outcome.to_string(),
            "Failed: max_retries exceeded (retry count: 3)"
        );
    }

    #[test]
    fn test_loop_outcome_display_error() {
        let outcome = LoopOutcome::Error("connection timeout".to_string());
        assert_eq!(outcome.to_string(), "Error: connection timeout");
    }

    #[test]
    fn test_loop_outcome_equality() {
        assert_eq!(LoopOutcome::Done, LoopOutcome::Done);
        assert_eq!(LoopOutcome::MaxIterations, LoopOutcome::MaxIterations);
        assert_eq!(
            LoopOutcome::MaxRetries { count: 3 },
            LoopOutcome::MaxRetries { count: 3 }
        );
        assert_ne!(
            LoopOutcome::MaxRetries { count: 3 },
            LoopOutcome::MaxRetries { count: 4 }
        );
        assert_eq!(
            LoopOutcome::Error("a".to_string()),
            LoopOutcome::Error("a".to_string())
        );
        assert_ne!(
            LoopOutcome::Error("a".to_string()),
            LoopOutcome::Error("b".to_string())
        );
    }
}
