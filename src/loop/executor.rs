//! Loop executor for managing agent orchestration.
//!
//! This module provides [`LoopExecutor`], which manages the state and configuration
//! for the orchestration loop. It handles:
//!
//! - State management via [`LoopState`]
//! - Building template contexts for variable interpolation
//! - Displaying iteration summaries
//! - Checking termination conditions
//!
//! The full run loop will be implemented in Phase 9 when all components are ready.

use crate::agent::AgentRunner;
use crate::config::Config;
use crate::template::{shell_escape_single_quotes, TemplateContext};
use tracing::info;

use super::{IterationSummary, LoopState};

/// Executor for the orchestration loop.
///
/// The `LoopExecutor` manages the configuration, state, and agent runner for
/// the AI coding agent loop. It provides methods for:
///
/// - Building template contexts with current state variables
/// - Checking iteration and retry limits
/// - Displaying iteration summaries (FR17)
///
/// # Example
///
/// ```ignore
/// use ralph::config::Config;
/// use ralph::r#loop::LoopExecutor;
///
/// let executor = LoopExecutor::new(config);
///
/// // Check iteration count
/// println!("Current iteration: {}", executor.iteration());
///
/// // Build context for template rendering
/// let ctx = executor.build_context();
/// ```
#[derive(Debug)]
pub struct LoopExecutor {
    /// The loaded configuration.
    config: Config,

    /// Current loop state.
    state: LoopState,

    /// Agent runner for executing shell commands.
    runner: AgentRunner,
}

impl LoopExecutor {
    /// Creates a new `LoopExecutor` with the given configuration.
    ///
    /// Initializes:
    /// - `state` with `iteration_count = 1`, `retry_count = 0`
    /// - `runner` with the default timeout from config
    ///
    /// # Arguments
    ///
    /// * `config` - The loaded and validated configuration
    ///
    /// # Example
    ///
    /// ```ignore
    /// use ralph::config;
    /// use ralph::r#loop::LoopExecutor;
    ///
    /// let config = config::load(Path::new("ralph.toml"))?;
    /// let executor = LoopExecutor::new(config);
    /// ```
    #[must_use]
    pub fn new(config: Config) -> Self {
        let timeout = config.general.timeout;
        Self {
            config,
            state: LoopState::new(),
            runner: AgentRunner::new(timeout),
        }
    }

    /// Returns the current iteration count (1-indexed).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let executor = LoopExecutor::new(config);
    /// assert_eq!(executor.iteration(), 1);
    /// ```
    #[must_use]
    pub fn iteration(&self) -> u32 {
        self.state.iteration_count
    }

    /// Returns the current retry count.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let executor = LoopExecutor::new(config);
    /// assert_eq!(executor.retry_count(), 0);
    /// ```
    #[must_use]
    pub fn retry_count(&self) -> u32 {
        self.state.retry_count
    }

    /// Returns `true` if this is the first iteration.
    ///
    /// The first iteration uses the `starting` prompt template instead of
    /// the `continuation` prompt template.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let executor = LoopExecutor::new(config);
    /// assert!(executor.is_first_iteration());
    /// ```
    #[must_use]
    pub fn is_first_iteration(&self) -> bool {
        self.state.is_first_iteration()
    }

    /// Returns a reference to the current loop state.
    ///
    /// Useful for inspecting state without modification.
    #[must_use]
    pub fn state(&self) -> &LoopState {
        &self.state
    }

    /// Returns a mutable reference to the current loop state.
    ///
    /// Allows direct state manipulation for the run loop.
    pub fn state_mut(&mut self) -> &mut LoopState {
        &mut self.state
    }

    /// Returns a reference to the configuration.
    #[must_use]
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Returns a reference to the agent runner.
    #[must_use]
    pub fn runner(&self) -> &AgentRunner {
        &self.runner
    }

    /// Builds a template context with current state variables.
    ///
    /// The context includes:
    /// - `iteration_count` - Current iteration number (as string)
    /// - `retry_count` - Current retry count (as string)
    /// - `next_prompt` - Output from previous next-action agent (empty for iter 1)
    ///
    /// Additional variables (like `dev_response`, `dev_errors`, etc.) should be
    /// added by the caller after agent execution.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let executor = LoopExecutor::new(config);
    /// let mut ctx = executor.build_context();
    ///
    /// assert_eq!(ctx.get("iteration_count"), Some("1"));
    /// assert_eq!(ctx.get("retry_count"), Some("0"));
    /// assert_eq!(ctx.get("next_prompt"), Some(""));
    ///
    /// // Add agent output after execution
    /// ctx.set("dev_response", agent_result.stdout);
    /// ctx.set("dev_errors", agent_result.stderr);
    /// ```
    #[must_use]
    pub fn build_context(&self) -> TemplateContext {
        let mut ctx = TemplateContext::new();

        // Add state variables
        ctx.set("iteration_count", self.state.iteration_count.to_string());
        ctx.set("retry_count", self.state.retry_count.to_string());
        ctx.set("next_prompt", self.state.next_prompt.clone());

        ctx
    }

    /// Displays an iteration summary to the log (FR17).
    ///
    /// Logs the summary at INFO level using tracing. The summary includes:
    /// - Iteration number
    /// - Agent name
    /// - Exit code (or "signal" if killed)
    /// - Stdout/stderr byte counts
    ///
    /// # Arguments
    ///
    /// * `summary` - The iteration summary to display
    ///
    /// # Example
    ///
    /// ```ignore
    /// let summary = IterationSummary::new(1, "dev", Some(0), 1024, 0);
    /// executor.display_summary(&summary);
    /// // Logs: [iter 1] dev: exit=0, stdout=1024 bytes, stderr=0 bytes
    /// ```
    pub fn display_summary(&self, summary: &IterationSummary) {
        info!("{}", summary);
    }

    /// Returns the maximum number of retries allowed.
    #[must_use]
    pub fn max_retries(&self) -> u32 {
        self.config.general.max_retries
    }

    /// Returns the maximum number of iterations (0 = unlimited).
    #[must_use]
    pub fn max_iterations(&self) -> u32 {
        self.config.general.max_iterations
    }

    /// Checks if the retry limit has been exceeded.
    ///
    /// Returns `true` if `retry_count > max_retries`.
    ///
    /// # Boundary Conditions
    ///
    /// - `max_retries = 0`: First failure triggers exit (retry_count > 0)
    /// - `max_retries = 3`: Up to 3 retries allowed, 4th retry exceeds limit
    ///
    /// # Example
    ///
    /// ```ignore
    /// // With max_retries = 3
    /// executor.state_mut().retry_count = 3;
    /// assert!(!executor.retry_limit_exceeded()); // 3 <= 3
    ///
    /// executor.state_mut().retry_count = 4;
    /// assert!(executor.retry_limit_exceeded()); // 4 > 3
    /// ```
    #[must_use]
    pub fn retry_limit_exceeded(&self) -> bool {
        self.state.retry_count > self.config.general.max_retries
    }

    /// Checks if the iteration limit has been reached.
    ///
    /// Returns `true` if `max_iterations > 0` and `iteration_count >= max_iterations`.
    /// When `max_iterations = 0`, there is no limit (always returns `false`).
    ///
    /// # Example
    ///
    /// ```ignore
    /// // With max_iterations = 10
    /// // Returns true when iteration_count >= 10
    ///
    /// // With max_iterations = 0 (unlimited)
    /// // Always returns false
    /// ```
    #[must_use]
    pub fn iteration_limit_reached(&self) -> bool {
        let max = self.config.general.max_iterations;
        max > 0 && self.state.iteration_count >= max
    }

    /// Gets the effective timeout for a specific agent.
    ///
    /// Returns the agent-specific timeout if set, otherwise the global default.
    ///
    /// # Arguments
    ///
    /// * `agent` - The agent name ("dev", "review", "next_action", or "done")
    ///
    /// # Returns
    ///
    /// The timeout in seconds for the specified agent.
    #[must_use]
    pub fn agent_timeout(&self, agent: &str) -> u32 {
        let agent_config = match agent {
            "dev" => &self.config.agents.dev,
            "review" => &self.config.agents.review,
            "next_action" | "next-action" => &self.config.agents.next_action,
            "done" => &self.config.agents.done,
            _ => return self.config.general.timeout,
        };

        agent_config
            .timeout()
            .unwrap_or(self.config.general.timeout)
    }

    /// Creates an `AgentRunner` with the timeout for a specific agent.
    ///
    /// # Arguments
    ///
    /// * `agent` - The agent name ("dev", "review", or "next_action")
    ///
    /// # Returns
    ///
    /// An `AgentRunner` configured with the appropriate timeout.
    #[must_use]
    pub fn runner_for_agent(&self, agent: &str) -> AgentRunner {
        AgentRunner::new(self.agent_timeout(agent))
    }

    /// Gets the appropriate prompt template for the current iteration.
    ///
    /// Returns the `starting` template for iteration 1, or the `continuation`
    /// template for iteration 2+.
    #[must_use]
    pub fn get_dev_prompt_template(&self) -> &str {
        if self.is_first_iteration() {
            &self.config.prompts.starting
        } else {
            &self.config.prompts.continuation
        }
    }

    /// Gets the review prompt template.
    #[must_use]
    pub fn get_review_prompt_template(&self) -> &str {
        &self.config.prompts.review
    }

    /// Gets the next-action prompt template.
    #[must_use]
    pub fn get_next_action_prompt_template(&self) -> &str {
        &self.config.prompts.next_action
    }

    /// Gets the done prompt template.
    #[must_use]
    pub fn get_done_prompt_template(&self) -> &str {
        &self.config.prompts.done
    }

    /// Gets the command for a specific agent.
    ///
    /// # Arguments
    ///
    /// * `agent` - The agent name ("dev", "review", "next_action", or "done")
    #[must_use]
    pub fn get_agent_command(&self, agent: &str) -> &str {
        match agent {
            "dev" => self.config.agents.dev.command(),
            "review" => self.config.agents.review.command(),
            "next_action" | "next-action" => self.config.agents.next_action.command(),
            "done" => self.config.agents.done.command(),
            _ => "",
        }
    }

    /// Runs the dev agent with the current context.
    ///
    /// This method:
    /// 1. Builds a template context with current state variables
    /// 2. Renders the appropriate prompt template (starting or continuation)
    /// 3. Renders the agent command with the prompt
    /// 4. Executes the command via shell
    /// 5. Displays a summary (FR17)
    ///
    /// # Returns
    ///
    /// Returns the `AgentResult` on success, or an error message on failure.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Template rendering fails (undefined variable)
    /// - Agent execution fails (spawn error, timeout)
    pub async fn run_dev_agent(&self) -> Result<crate::agent::AgentResult, String> {
        // Build context with state variables
        let mut ctx = self.build_context();

        // Get and render the prompt template
        let prompt_template = self.get_dev_prompt_template();
        let rendered_prompt = ctx
            .render(prompt_template)
            .map_err(|e| format!("Failed to render dev prompt: {e}"))?;

        // Add rendered prompt to context for command template
        // Shell-escape single quotes to prevent command injection/breakage
        ctx.set("prompt", shell_escape_single_quotes(&rendered_prompt));

        // Get and render the command template
        let command_template = self.get_agent_command("dev");
        let command = ctx
            .render(command_template)
            .map_err(|e| format!("Failed to render dev command: {e}"))?;

        // Run the agent
        let runner = self.runner_for_agent("dev");
        let result = runner
            .run(&command)
            .await
            .map_err(|e| format!("Dev agent error: {e}"))?;

        // Display summary
        let summary = super::IterationSummary::new(
            self.iteration(),
            "dev",
            result.exit_code,
            result.stdout.len(),
            result.stderr.len(),
        );
        self.display_summary(&summary);

        Ok(result)
    }

    /// Runs the review agent with dev agent output.
    ///
    /// This method:
    /// 1. Builds a template context with state and dev output
    /// 2. Renders the review prompt template
    /// 3. Renders the agent command with the prompt
    /// 4. Executes the command via shell
    /// 5. Displays a summary (FR17)
    ///
    /// # Arguments
    ///
    /// * `dev_response` - The stdout from the dev agent
    /// * `dev_errors` - The stderr from the dev agent
    ///
    /// # Returns
    ///
    /// Returns the `AgentResult` on success, or an error message on failure.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Template rendering fails (undefined variable)
    /// - Agent execution fails (spawn error, timeout)
    /// - Review agent exits with non-zero code (FR23 - fatal error)
    pub async fn run_review_agent(
        &self,
        dev_response: &str,
        dev_errors: &str,
    ) -> Result<crate::agent::AgentResult, String> {
        // Build context with state variables
        let mut ctx = self.build_context();

        // Add dev agent output
        ctx.set("dev_response", dev_response.to_string());
        ctx.set("dev_errors", dev_errors.to_string());

        // Get and render the prompt template
        let prompt_template = self.get_review_prompt_template();
        let rendered_prompt = ctx
            .render(prompt_template)
            .map_err(|e| format!("Failed to render review prompt: {e}"))?;

        // Add rendered prompt to context for command template
        // Shell-escape single quotes to prevent command injection/breakage
        ctx.set("prompt", shell_escape_single_quotes(&rendered_prompt));

        // Get and render the command template
        let command_template = self.get_agent_command("review");
        let command = ctx
            .render(command_template)
            .map_err(|e| format!("Failed to render review command: {e}"))?;

        // Run the agent
        let runner = self.runner_for_agent("review");
        let result = runner
            .run(&command)
            .await
            .map_err(|e| format!("Review agent error: {e}"))?;

        // Display summary
        let summary = super::IterationSummary::new(
            self.iteration(),
            "review",
            result.exit_code,
            result.stdout.len(),
            result.stderr.len(),
        );
        self.display_summary(&summary);

        // FR23: Review agent failure is fatal
        if !result.success {
            return Err(format!(
                "Review agent failed with exit code {:?}",
                result.exit_code
            ));
        }

        Ok(result)
    }

    /// Runs the next-action agent with dev and review output.
    ///
    /// This method:
    /// 1. Builds a template context with state, dev, and review output
    /// 2. Renders the next-action prompt template
    /// 3. Renders the agent command with the prompt
    /// 4. Executes the command via shell
    /// 5. Displays a summary (FR17)
    ///
    /// # Arguments
    ///
    /// * `dev_response` - The stdout from the dev agent
    /// * `dev_errors` - The stderr from the dev agent
    /// * `review_response` - The stdout from the review agent
    /// * `review_errors` - The stderr from the review agent
    ///
    /// # Returns
    ///
    /// Returns the `AgentResult` on success, or an error message on failure.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Template rendering fails (undefined variable)
    /// - Agent execution fails (spawn error, timeout)
    /// - Next-action agent exits with non-zero code (FR41 - fatal error)
    pub async fn run_next_action_agent(
        &self,
        dev_response: &str,
        dev_errors: &str,
        review_response: &str,
        review_errors: &str,
    ) -> Result<crate::agent::AgentResult, String> {
        // Build context with state variables
        let mut ctx = self.build_context();

        // Add dev and review agent output
        ctx.set("dev_response", dev_response.to_string());
        ctx.set("dev_errors", dev_errors.to_string());
        ctx.set("review_response", review_response.to_string());
        ctx.set("review_errors", review_errors.to_string());

        // Get and render the prompt template
        let prompt_template = self.get_next_action_prompt_template();
        let rendered_prompt = ctx
            .render(prompt_template)
            .map_err(|e| format!("Failed to render next-action prompt: {e}"))?;

        // Add rendered prompt to context for command template
        // Shell-escape single quotes to prevent command injection/breakage
        ctx.set("prompt", shell_escape_single_quotes(&rendered_prompt));

        // Get and render the command template
        let command_template = self.get_agent_command("next_action");
        let command = ctx
            .render(command_template)
            .map_err(|e| format!("Failed to render next-action command: {e}"))?;

        // Run the agent
        let runner = self.runner_for_agent("next_action");
        let result = runner
            .run(&command)
            .await
            .map_err(|e| format!("Next-action agent error: {e}"))?;

        // Display summary
        let summary = super::IterationSummary::new(
            self.iteration(),
            "next-action",
            result.exit_code,
            result.stdout.len(),
            result.stderr.len(),
        );
        self.display_summary(&summary);

        // FR41: Next-action agent failure is fatal
        if !result.success {
            return Err(format!(
                "Next-action agent failed with exit code {:?}",
                result.exit_code
            ));
        }

        Ok(result)
    }

    /// Runs the done agent to confirm project completion.
    ///
    /// This method:
    /// 1. Builds a template context with ONLY state variables (no agent context)
    /// 2. Renders the done prompt template
    /// 3. Renders the agent command with the prompt
    /// 4. Executes the command via shell
    /// 5. Displays a summary (FR17)
    ///
    /// **IMPORTANT**: The done agent is intentionally isolated from other agents'
    /// context. It only receives `iteration_count` and `retry_count`, NOT
    /// `dev_response`, `dev_errors`, `review_response`, or `review_errors`.
    /// This ensures the done agent independently examines the filesystem.
    ///
    /// # Returns
    ///
    /// Returns the `AgentResult` on success, or an error message on failure.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Template rendering fails (undefined variable)
    /// - Agent execution fails (spawn error, timeout)
    /// - Done agent exits with non-zero code (fatal error)
    pub async fn run_done_agent(&self) -> Result<crate::agent::AgentResult, String> {
        // Build context with ONLY state variables - no agent context!
        // This ensures the done agent is isolated and must examine the filesystem
        let mut ctx = TemplateContext::new();
        ctx.set("iteration_count", self.state.iteration_count.to_string());
        ctx.set("retry_count", self.state.retry_count.to_string());

        // Get and render the prompt template
        let prompt_template = self.get_done_prompt_template();
        let rendered_prompt = ctx
            .render(prompt_template)
            .map_err(|e| format!("Failed to render done prompt: {e}"))?;

        // Add rendered prompt to context for command template
        // Shell-escape single quotes to prevent command injection/breakage
        ctx.set("prompt", shell_escape_single_quotes(&rendered_prompt));

        // Get and render the command template
        let command_template = self.get_agent_command("done");
        let command = ctx
            .render(command_template)
            .map_err(|e| format!("Failed to render done command: {e}"))?;

        // Run the agent
        let runner = self.runner_for_agent("done");
        let result = runner
            .run(&command)
            .await
            .map_err(|e| format!("Done agent error: {e}"))?;

        // Display summary
        let summary = super::IterationSummary::new(
            self.iteration(),
            "done",
            result.exit_code,
            result.stdout.len(),
            result.stderr.len(),
        );
        self.display_summary(&summary);

        // Done agent failure is fatal
        if !result.success {
            return Err(format!(
                "Done agent failed with exit code {:?}",
                result.exit_code
            ));
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates a minimal valid config for testing.
    fn test_config() -> Config {
        let toml_str = r#"
            [general]
            max_retries = 3
            max_iterations = 10
            timeout = 60

            [prompts]
            starting = "Start the task"
            continuation = "Continue with: {{next_prompt}}"
            review = "Review: {{dev_response}}"
            next_action = "What's next: {{dev_response}}"
            done = "Check if requirements are satisfied"

            [agents]
            dev = "echo '{{prompt}}'"
            review = "echo '{{prompt}}'"
            next_action = "echo '{{prompt}}'"
            done = "echo '{{prompt}}'"
        "#;
        toml::from_str(toml_str).unwrap()
    }

    /// Creates a config with custom agent timeouts.
    fn test_config_with_agent_timeouts() -> Config {
        let toml_str = r#"
            [general]
            max_retries = 3
            timeout = 60

            [prompts]
            starting = "Start"
            continuation = "Continue: {{next_prompt}}"
            review = "Review: {{dev_response}}"
            next_action = "Next: {{dev_response}}"
            done = "Check completion"

            [agents.dev]
            command = "echo '{{prompt}}'"
            timeout = 120

            [agents.review]
            command = "echo '{{prompt}}'"
            timeout = 30

            [agents]
            next_action = "echo '{{prompt}}'"
            done = "echo '{{prompt}}'"
        "#;
        toml::from_str(toml_str).unwrap()
    }

    // =========================================================================
    // Constructor and basic accessors
    // =========================================================================

    #[test]
    fn test_executor_new() {
        let config = test_config();
        let executor = LoopExecutor::new(config);

        assert_eq!(executor.iteration(), 1);
        assert_eq!(executor.retry_count(), 0);
        assert!(executor.is_first_iteration());
    }

    #[test]
    fn test_executor_state_accessors() {
        let config = test_config();
        let mut executor = LoopExecutor::new(config);

        // Read-only state access
        assert_eq!(executor.state().iteration_count, 1);
        assert_eq!(executor.state().retry_count, 0);

        // Mutable state access
        executor.state_mut().increment_retry();
        assert_eq!(executor.retry_count(), 1);
    }

    #[test]
    fn test_executor_config_accessor() {
        let config = test_config();
        let executor = LoopExecutor::new(config);

        assert_eq!(executor.config().general.max_retries, 3);
        assert_eq!(executor.config().general.max_iterations, 10);
        assert_eq!(executor.config().general.timeout, 60);
    }

    #[test]
    fn test_executor_max_retries() {
        let config = test_config();
        let executor = LoopExecutor::new(config);

        assert_eq!(executor.max_retries(), 3);
    }

    #[test]
    fn test_executor_max_iterations() {
        let config = test_config();
        let executor = LoopExecutor::new(config);

        assert_eq!(executor.max_iterations(), 10);
    }

    // =========================================================================
    // is_first_iteration tests
    // =========================================================================

    #[test]
    fn test_is_first_iteration_initially_true() {
        let config = test_config();
        let executor = LoopExecutor::new(config);

        assert!(executor.is_first_iteration());
    }

    #[test]
    fn test_is_first_iteration_after_next_iteration() {
        let config = test_config();
        let mut executor = LoopExecutor::new(config);

        executor.state_mut().next_iteration("next task".to_string());
        assert!(!executor.is_first_iteration());
    }

    #[test]
    fn test_is_first_iteration_after_retry() {
        let config = test_config();
        let mut executor = LoopExecutor::new(config);

        // Retries don't change iteration count
        executor.state_mut().increment_retry();
        assert!(executor.is_first_iteration());
    }

    // =========================================================================
    // build_context tests
    // =========================================================================

    #[test]
    fn test_build_context_initial() {
        let config = test_config();
        let executor = LoopExecutor::new(config);
        let ctx = executor.build_context();

        assert_eq!(ctx.get("iteration_count"), Some("1"));
        assert_eq!(ctx.get("retry_count"), Some("0"));
        assert_eq!(ctx.get("next_prompt"), Some(""));
    }

    #[test]
    fn test_build_context_after_retry() {
        let config = test_config();
        let mut executor = LoopExecutor::new(config);

        executor.state_mut().increment_retry();
        executor.state_mut().increment_retry();

        let ctx = executor.build_context();
        assert_eq!(ctx.get("iteration_count"), Some("1"));
        assert_eq!(ctx.get("retry_count"), Some("2"));
        assert_eq!(ctx.get("next_prompt"), Some(""));
    }

    #[test]
    fn test_build_context_after_next_iteration() {
        let config = test_config();
        let mut executor = LoopExecutor::new(config);

        executor
            .state_mut()
            .next_iteration("Fix the tests".to_string());

        let ctx = executor.build_context();
        assert_eq!(ctx.get("iteration_count"), Some("2"));
        assert_eq!(ctx.get("retry_count"), Some("0"));
        assert_eq!(ctx.get("next_prompt"), Some("Fix the tests"));
    }

    #[test]
    fn test_build_context_multiple_iterations() {
        let config = test_config();
        let mut executor = LoopExecutor::new(config);

        // Iteration 1 -> 2
        executor.state_mut().next_iteration("Task 2".to_string());

        // Some retries in iteration 2
        executor.state_mut().increment_retry();

        // Iteration 2 -> 3
        executor.state_mut().next_iteration("Task 3".to_string());

        let ctx = executor.build_context();
        assert_eq!(ctx.get("iteration_count"), Some("3"));
        assert_eq!(ctx.get("retry_count"), Some("0"));
        assert_eq!(ctx.get("next_prompt"), Some("Task 3"));
    }

    #[test]
    fn test_build_context_can_render_templates() {
        let config = test_config();
        let executor = LoopExecutor::new(config);
        let ctx = executor.build_context();

        // Should be able to render templates using the context
        let result = ctx
            .render("Iteration {{iteration_count}}, retry {{retry_count}}")
            .unwrap();
        assert_eq!(result, "Iteration 1, retry 0");
    }

    // =========================================================================
    // retry_limit_exceeded tests
    // =========================================================================

    #[test]
    fn test_retry_limit_not_exceeded_initially() {
        let config = test_config();
        let executor = LoopExecutor::new(config);

        assert!(!executor.retry_limit_exceeded());
    }

    #[test]
    fn test_retry_limit_not_exceeded_at_max() {
        let config = test_config();
        let mut executor = LoopExecutor::new(config);

        // max_retries = 3, so retry_count = 3 is allowed
        executor.state_mut().increment_retry();
        executor.state_mut().increment_retry();
        executor.state_mut().increment_retry();

        assert_eq!(executor.retry_count(), 3);
        assert!(!executor.retry_limit_exceeded());
    }

    #[test]
    fn test_retry_limit_exceeded_above_max() {
        let config = test_config();
        let mut executor = LoopExecutor::new(config);

        // max_retries = 3, so retry_count = 4 exceeds
        executor.state_mut().increment_retry();
        executor.state_mut().increment_retry();
        executor.state_mut().increment_retry();
        executor.state_mut().increment_retry();

        assert_eq!(executor.retry_count(), 4);
        assert!(executor.retry_limit_exceeded());
    }

    #[test]
    fn test_retry_limit_zero_max_retries() {
        // With max_retries = 0, first failure should exceed
        let toml_str = r#"
            [general]
            max_retries = 0

            [prompts]
            starting = "Start"
            continuation = "Continue: {{next_prompt}}"
            review = "Review: {{dev_response}}"
            next_action = "Next: {{dev_response}}"
            done = "Check completion"

            [agents]
            dev = "echo '{{prompt}}'"
            review = "echo '{{prompt}}'"
            next_action = "echo '{{prompt}}'"
            done = "echo '{{prompt}}'"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let mut executor = LoopExecutor::new(config);

        assert!(!executor.retry_limit_exceeded()); // 0 <= 0

        executor.state_mut().increment_retry();
        assert!(executor.retry_limit_exceeded()); // 1 > 0
    }

    // =========================================================================
    // iteration_limit_reached tests
    // =========================================================================

    #[test]
    fn test_iteration_limit_not_reached_initially() {
        let config = test_config();
        let executor = LoopExecutor::new(config);

        assert!(!executor.iteration_limit_reached());
    }

    #[test]
    fn test_iteration_limit_reached_at_max() {
        let config = test_config();
        let mut executor = LoopExecutor::new(config);

        // max_iterations = 10
        // Move to iteration 10
        for i in 1..10 {
            executor
                .state_mut()
                .next_iteration(format!("task {}", i + 1));
        }

        assert_eq!(executor.iteration(), 10);
        assert!(executor.iteration_limit_reached());
    }

    #[test]
    fn test_iteration_limit_not_reached_below_max() {
        let config = test_config();
        let mut executor = LoopExecutor::new(config);

        // max_iterations = 10, at iteration 9 should not be reached
        for i in 1..9 {
            executor
                .state_mut()
                .next_iteration(format!("task {}", i + 1));
        }

        assert_eq!(executor.iteration(), 9);
        assert!(!executor.iteration_limit_reached());
    }

    #[test]
    fn test_iteration_limit_unlimited() {
        // With max_iterations = 0, there is no limit
        let toml_str = r#"
            [general]
            max_retries = 3
            max_iterations = 0

            [prompts]
            starting = "Start"
            continuation = "Continue: {{next_prompt}}"
            review = "Review: {{dev_response}}"
            next_action = "Next: {{dev_response}}"
            done = "Check completion"

            [agents]
            dev = "echo '{{prompt}}'"
            review = "echo '{{prompt}}'"
            next_action = "echo '{{prompt}}'"
            done = "echo '{{prompt}}'"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let mut executor = LoopExecutor::new(config);

        // Even at high iteration counts, should not be reached
        for i in 1..100 {
            executor
                .state_mut()
                .next_iteration(format!("task {}", i + 1));
        }

        assert_eq!(executor.iteration(), 100);
        assert!(!executor.iteration_limit_reached());
    }

    // =========================================================================
    // agent_timeout tests
    // =========================================================================

    #[test]
    fn test_agent_timeout_default() {
        let config = test_config();
        let executor = LoopExecutor::new(config);

        // All agents use default timeout
        assert_eq!(executor.agent_timeout("dev"), 60);
        assert_eq!(executor.agent_timeout("review"), 60);
        assert_eq!(executor.agent_timeout("next_action"), 60);
        assert_eq!(executor.agent_timeout("next-action"), 60);
        assert_eq!(executor.agent_timeout("done"), 60);
    }

    #[test]
    fn test_agent_timeout_with_overrides() {
        let config = test_config_with_agent_timeouts();
        let executor = LoopExecutor::new(config);

        assert_eq!(executor.agent_timeout("dev"), 120);
        assert_eq!(executor.agent_timeout("review"), 30);
        assert_eq!(executor.agent_timeout("next_action"), 60); // Uses default
    }

    #[test]
    fn test_agent_timeout_unknown_agent() {
        let config = test_config();
        let executor = LoopExecutor::new(config);

        // Unknown agent returns default
        assert_eq!(executor.agent_timeout("unknown"), 60);
    }

    // =========================================================================
    // runner_for_agent tests
    // =========================================================================

    #[test]
    fn test_runner_for_agent() {
        let config = test_config_with_agent_timeouts();
        let executor = LoopExecutor::new(config);

        // Just verify we can create runners - actual timeout behavior
        // is tested in the agent module
        let _dev_runner = executor.runner_for_agent("dev");
        let _review_runner = executor.runner_for_agent("review");
        let _next_action_runner = executor.runner_for_agent("next_action");
    }

    // =========================================================================
    // display_summary tests (basic - actual logging tested via integration)
    // =========================================================================

    #[test]
    fn test_display_summary_does_not_panic() {
        let config = test_config();
        let executor = LoopExecutor::new(config);
        let summary = IterationSummary::new(1, "dev", Some(0), 1024, 0);

        // Just verify it doesn't panic - logging output tested via integration
        executor.display_summary(&summary);
    }
}
