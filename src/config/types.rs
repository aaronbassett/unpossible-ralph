//! Configuration types for ralph.
//!
//! This module defines the TOML configuration schema using serde for deserialization.
//! All types use `deny_unknown_fields` to reject invalid configuration keys.

use serde::Deserialize;

/// Default timeout in seconds (30 minutes).
fn default_timeout() -> u32 {
    1800
}

/// Root configuration structure.
///
/// # Example TOML
///
/// ```toml
/// [general]
/// max_retries = 3
///
/// [prompts]
/// starting = "Initial task: {{task}}"
/// continuation = "Continue with: {{next_prompt}}"
/// review = "Review this: {{dev_response}}"
/// next_action = "What's next? {{dev_response}}"
///
/// [agents]
/// dev = "claude --prompt '{{prompt}}'"
/// review = "claude --prompt '{{prompt}}'"
/// next_action = "claude --prompt '{{prompt}}'"
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// General loop settings.
    pub general: GeneralConfig,
    /// Prompt templates for different stages.
    pub prompts: PromptsConfig,
    /// Agent command configurations.
    pub agents: AgentsConfig,
}

/// General loop settings.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneralConfig {
    /// Maximum consecutive retries before failing with exit code 2.
    pub max_retries: u32,
    /// Maximum iterations before stopping (0 = unlimited).
    #[serde(default)]
    pub max_iterations: u32,
    /// Default timeout for agent commands in seconds.
    #[serde(default = "default_timeout")]
    pub timeout: u32,
}

/// Prompt templates for different stages of the loop.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptsConfig {
    /// Prompt used for the first iteration.
    pub starting: String,
    /// Prompt used for subsequent iterations (must contain `{{next_prompt}}`).
    pub continuation: String,
    /// Prompt for the review agent (must contain `{{dev_response}}` or `{{dev_errors}}`).
    pub review: String,
    /// Prompt for the next-action agent (must contain `{{dev_response}}` or `{{dev_errors}}`).
    pub next_action: String,
}

/// Agent command configurations.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentsConfig {
    /// Development agent command.
    pub dev: AgentConfig,
    /// Review agent command.
    pub review: AgentConfig,
    /// Next-action agent command.
    pub next_action: AgentConfig,
}

/// Agent configuration supporting both simple and extended formats.
///
/// # Simple Format
///
/// ```toml
/// dev = "claude --prompt '{{prompt}}'"
/// ```
///
/// # Extended Format
///
/// ```toml
/// [agents.dev]
/// command = "claude --prompt '{{prompt}}'"
/// timeout = 3600  # Override default timeout
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum AgentConfig {
    /// Simple string command.
    Simple(String),
    /// Extended configuration with optional per-agent timeout.
    Extended {
        /// The shell command to execute (must contain `{{prompt}}`).
        command: String,
        /// Optional timeout override in seconds.
        #[serde(default)]
        timeout: Option<u32>,
    },
}

impl AgentConfig {
    /// Returns the command string for this agent.
    pub fn command(&self) -> &str {
        match self {
            AgentConfig::Simple(cmd) => cmd,
            AgentConfig::Extended { command, .. } => command,
        }
    }

    /// Returns the per-agent timeout override, if any.
    pub fn timeout(&self) -> Option<u32> {
        match self {
            AgentConfig::Simple(_) => None,
            AgentConfig::Extended { timeout, .. } => *timeout,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_simple() {
        // TOML requires a key-value pair at the root, so we use a wrapper
        #[derive(Deserialize)]
        struct Wrapper {
            agent: AgentConfig,
        }
        let config: Wrapper = toml::from_str(r#"agent = "echo {{prompt}}""#).unwrap();
        assert_eq!(config.agent.command(), "echo {{prompt}}");
        assert_eq!(config.agent.timeout(), None);
    }

    #[test]
    fn test_agent_config_extended() {
        let toml_str = r#"
            command = "echo {{prompt}}"
            timeout = 3600
        "#;
        let config: AgentConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.command(), "echo {{prompt}}");
        assert_eq!(config.timeout(), Some(3600));
    }

    #[test]
    fn test_agent_config_extended_no_timeout() {
        let toml_str = r#"
            command = "echo {{prompt}}"
        "#;
        let config: AgentConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.command(), "echo {{prompt}}");
        assert_eq!(config.timeout(), None);
    }

    #[test]
    fn test_general_config_defaults() {
        let toml_str = r#"
            max_retries = 3
        "#;
        let config: GeneralConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.max_iterations, 0);
        assert_eq!(config.timeout, 1800);
    }

    #[test]
    fn test_config_rejects_unknown_fields() {
        let toml_str = r#"
            max_retries = 3
            unknown_field = "bad"
        "#;
        let result: Result<GeneralConfig, _> = toml::from_str(toml_str);
        assert!(result.is_err());
    }
}
