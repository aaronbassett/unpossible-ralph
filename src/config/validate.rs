//! Validation rules for ralph configuration.
//!
//! This module implements semantic validation that cannot be expressed through
//! serde's deserialization alone.

use super::types::{AgentConfig, Config};
use thiserror::Error;

/// Errors that can occur during configuration validation.
#[derive(Debug, Error)]
pub enum ValidationError {
    /// An agent command is missing the required `{{prompt}}` placeholder.
    #[error("Agent '{agent}' command must include {{{{prompt}}}} placeholder")]
    MissingPromptPlaceholder { agent: String },

    /// A prompt template is empty.
    #[error("Prompt '{prompt}' cannot be empty")]
    EmptyPrompt { prompt: String },

    /// The review prompt is missing required placeholders.
    #[error("Prompt 'review' must include {{{{dev_response}}}} or {{{{dev_errors}}}} placeholder")]
    ReviewPromptMissingPlaceholder,

    /// The next_action prompt is missing required placeholders.
    #[error(
        "Prompt 'next_action' must include {{{{dev_response}}}} or {{{{dev_errors}}}} placeholder"
    )]
    NextActionPromptMissingPlaceholder,

    /// The continuation prompt is missing the required `{{next_prompt}}` placeholder.
    #[error("Prompt 'continuation' must include {{{{next_prompt}}}} placeholder")]
    ContinuationPromptMissingPlaceholder,

    /// The done prompt contains forbidden context placeholders.
    /// The done agent must be isolated from other agents' context.
    #[error("Prompt 'done' must NOT include {{{{dev_response}}}}, {{{{dev_errors}}}}, {{{{review_response}}}}, or {{{{review_errors}}}} (done agent must be context-isolated)")]
    DonePromptContainsForbiddenPlaceholder,
}

/// Validates the configuration according to ralph's requirements.
///
/// # Validation Rules
///
/// 1. All agent commands must contain `{{prompt}}`
/// 2. Review prompt must contain `{{dev_response}}` or `{{dev_errors}}`
/// 3. Next-action prompt must contain `{{dev_response}}` or `{{dev_errors}}`
/// 4. Continuation prompt must contain `{{next_prompt}}`
/// 5. No prompt templates can be empty
/// 6. Done prompt must NOT contain `{{dev_response}}`, `{{dev_errors}}`, `{{review_response}}`, or `{{review_errors}}`
///
/// # Errors
///
/// Returns the first validation error encountered.
pub fn validate(config: &Config) -> Result<(), ValidationError> {
    // Validate prompts are not empty
    validate_prompt_not_empty(&config.prompts.starting, "starting")?;
    validate_prompt_not_empty(&config.prompts.continuation, "continuation")?;
    validate_prompt_not_empty(&config.prompts.review, "review")?;
    validate_prompt_not_empty(&config.prompts.next_action, "next_action")?;
    validate_prompt_not_empty(&config.prompts.done, "done")?;

    // Validate agent commands contain {{prompt}}
    validate_agent_has_prompt(&config.agents.dev, "dev")?;
    validate_agent_has_prompt(&config.agents.review, "review")?;
    validate_agent_has_prompt(&config.agents.next_action, "next_action")?;
    validate_agent_has_prompt(&config.agents.done, "done")?;

    // Validate review prompt contains {{dev_response}} or {{dev_errors}}
    if !contains_placeholder(&config.prompts.review, "dev_response")
        && !contains_placeholder(&config.prompts.review, "dev_errors")
    {
        return Err(ValidationError::ReviewPromptMissingPlaceholder);
    }

    // Validate next_action prompt contains {{dev_response}} or {{dev_errors}}
    if !contains_placeholder(&config.prompts.next_action, "dev_response")
        && !contains_placeholder(&config.prompts.next_action, "dev_errors")
    {
        return Err(ValidationError::NextActionPromptMissingPlaceholder);
    }

    // Validate continuation prompt contains {{next_prompt}}
    if !contains_placeholder(&config.prompts.continuation, "next_prompt") {
        return Err(ValidationError::ContinuationPromptMissingPlaceholder);
    }

    // Validate done prompt does NOT contain forbidden context placeholders
    // The done agent must be isolated from other agents' context
    if contains_placeholder(&config.prompts.done, "dev_response")
        || contains_placeholder(&config.prompts.done, "dev_errors")
        || contains_placeholder(&config.prompts.done, "review_response")
        || contains_placeholder(&config.prompts.done, "review_errors")
    {
        return Err(ValidationError::DonePromptContainsForbiddenPlaceholder);
    }

    Ok(())
}

/// Validates that a prompt template is not empty.
fn validate_prompt_not_empty(prompt: &str, name: &str) -> Result<(), ValidationError> {
    if prompt.trim().is_empty() {
        return Err(ValidationError::EmptyPrompt {
            prompt: name.to_string(),
        });
    }
    Ok(())
}

/// Validates that an agent command contains the `{{prompt}}` placeholder.
fn validate_agent_has_prompt(agent: &AgentConfig, name: &str) -> Result<(), ValidationError> {
    if !contains_placeholder(agent.command(), "prompt") {
        return Err(ValidationError::MissingPromptPlaceholder {
            agent: name.to_string(),
        });
    }
    Ok(())
}

/// Checks if a string contains a placeholder in the form `{{name}}`.
fn contains_placeholder(s: &str, name: &str) -> bool {
    let placeholder = format!("{{{{{}}}}}", name);
    s.contains(&placeholder)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::{AgentsConfig, GeneralConfig, PromptsConfig};

    fn valid_config() -> Config {
        Config {
            general: GeneralConfig {
                max_retries: 3,
                max_iterations: 0,
                timeout: 1800,
            },
            prompts: PromptsConfig {
                starting: "Start task".to_string(),
                continuation: "Continue: {{next_prompt}}".to_string(),
                review: "Review: {{dev_response}}".to_string(),
                next_action: "Next: {{dev_response}}".to_string(),
                done: "Check if requirements are satisfied".to_string(),
            },
            agents: AgentsConfig {
                dev: AgentConfig::Simple("echo {{prompt}}".to_string()),
                review: AgentConfig::Simple("echo {{prompt}}".to_string()),
                next_action: AgentConfig::Simple("echo {{prompt}}".to_string()),
                done: AgentConfig::Simple("echo {{prompt}}".to_string()),
            },
        }
    }

    #[test]
    fn test_valid_config() {
        let config = valid_config();
        assert!(validate(&config).is_ok());
    }

    #[test]
    fn test_empty_starting_prompt() {
        let mut config = valid_config();
        config.prompts.starting = "".to_string();
        let err = validate(&config).unwrap_err();
        assert!(matches!(err, ValidationError::EmptyPrompt { prompt } if prompt == "starting"));
    }

    #[test]
    fn test_whitespace_only_prompt() {
        let mut config = valid_config();
        config.prompts.starting = "   ".to_string();
        let err = validate(&config).unwrap_err();
        assert!(matches!(err, ValidationError::EmptyPrompt { prompt } if prompt == "starting"));
    }

    #[test]
    fn test_missing_prompt_in_dev_agent() {
        let mut config = valid_config();
        config.agents.dev = AgentConfig::Simple("echo hello".to_string());
        let err = validate(&config).unwrap_err();
        assert!(
            matches!(err, ValidationError::MissingPromptPlaceholder { agent } if agent == "dev")
        );
    }

    #[test]
    fn test_missing_prompt_in_review_agent() {
        let mut config = valid_config();
        config.agents.review = AgentConfig::Simple("echo hello".to_string());
        let err = validate(&config).unwrap_err();
        assert!(
            matches!(err, ValidationError::MissingPromptPlaceholder { agent } if agent == "review")
        );
    }

    #[test]
    fn test_missing_prompt_in_next_action_agent() {
        let mut config = valid_config();
        config.agents.next_action = AgentConfig::Simple("echo hello".to_string());
        let err = validate(&config).unwrap_err();
        assert!(
            matches!(err, ValidationError::MissingPromptPlaceholder { agent } if agent == "next_action")
        );
    }

    #[test]
    fn test_review_prompt_missing_dev_response_and_dev_errors() {
        let mut config = valid_config();
        config.prompts.review = "No placeholders here".to_string();
        let err = validate(&config).unwrap_err();
        assert!(matches!(
            err,
            ValidationError::ReviewPromptMissingPlaceholder
        ));
    }

    #[test]
    fn test_review_prompt_with_dev_errors() {
        let mut config = valid_config();
        config.prompts.review = "Errors: {{dev_errors}}".to_string();
        assert!(validate(&config).is_ok());
    }

    #[test]
    fn test_next_action_prompt_missing_dev_response_and_dev_errors() {
        let mut config = valid_config();
        config.prompts.next_action = "No placeholders here".to_string();
        let err = validate(&config).unwrap_err();
        assert!(matches!(
            err,
            ValidationError::NextActionPromptMissingPlaceholder
        ));
    }

    #[test]
    fn test_next_action_prompt_with_dev_errors() {
        let mut config = valid_config();
        config.prompts.next_action = "Errors: {{dev_errors}}".to_string();
        assert!(validate(&config).is_ok());
    }

    #[test]
    fn test_continuation_prompt_missing_next_prompt() {
        let mut config = valid_config();
        config.prompts.continuation = "Continue without placeholder".to_string();
        let err = validate(&config).unwrap_err();
        assert!(matches!(
            err,
            ValidationError::ContinuationPromptMissingPlaceholder
        ));
    }

    #[test]
    fn test_extended_agent_config_validation() {
        let mut config = valid_config();
        config.agents.dev = AgentConfig::Extended {
            command: "echo {{prompt}}".to_string(),
            timeout: Some(3600),
        };
        assert!(validate(&config).is_ok());
    }

    #[test]
    fn test_extended_agent_config_missing_prompt() {
        let mut config = valid_config();
        config.agents.dev = AgentConfig::Extended {
            command: "echo hello".to_string(),
            timeout: Some(3600),
        };
        let err = validate(&config).unwrap_err();
        assert!(
            matches!(err, ValidationError::MissingPromptPlaceholder { agent } if agent == "dev")
        );
    }

    #[test]
    fn test_contains_placeholder() {
        assert!(contains_placeholder("hello {{name}} world", "name"));
        assert!(!contains_placeholder("hello {name} world", "name"));
        assert!(!contains_placeholder("hello {{ name }} world", "name"));
        assert!(contains_placeholder("{{name}}", "name"));
        assert!(!contains_placeholder("", "name"));
    }

    #[test]
    fn test_empty_done_prompt() {
        let mut config = valid_config();
        config.prompts.done = "".to_string();
        let err = validate(&config).unwrap_err();
        assert!(matches!(err, ValidationError::EmptyPrompt { prompt } if prompt == "done"));
    }

    #[test]
    fn test_missing_prompt_in_done_agent() {
        let mut config = valid_config();
        config.agents.done = AgentConfig::Simple("echo hello".to_string());
        let err = validate(&config).unwrap_err();
        assert!(
            matches!(err, ValidationError::MissingPromptPlaceholder { agent } if agent == "done")
        );
    }

    #[test]
    fn test_done_prompt_with_dev_response_forbidden() {
        let mut config = valid_config();
        config.prompts.done = "Check: {{dev_response}}".to_string();
        let err = validate(&config).unwrap_err();
        assert!(matches!(
            err,
            ValidationError::DonePromptContainsForbiddenPlaceholder
        ));
    }

    #[test]
    fn test_done_prompt_with_dev_errors_forbidden() {
        let mut config = valid_config();
        config.prompts.done = "Check errors: {{dev_errors}}".to_string();
        let err = validate(&config).unwrap_err();
        assert!(matches!(
            err,
            ValidationError::DonePromptContainsForbiddenPlaceholder
        ));
    }

    #[test]
    fn test_done_prompt_with_review_response_forbidden() {
        let mut config = valid_config();
        config.prompts.done = "Review said: {{review_response}}".to_string();
        let err = validate(&config).unwrap_err();
        assert!(matches!(
            err,
            ValidationError::DonePromptContainsForbiddenPlaceholder
        ));
    }

    #[test]
    fn test_done_prompt_with_review_errors_forbidden() {
        let mut config = valid_config();
        config.prompts.done = "Review errors: {{review_errors}}".to_string();
        let err = validate(&config).unwrap_err();
        assert!(matches!(
            err,
            ValidationError::DonePromptContainsForbiddenPlaceholder
        ));
    }

    #[test]
    fn test_done_prompt_with_allowed_placeholders() {
        let mut config = valid_config();
        // iteration_count and retry_count are allowed
        config.prompts.done =
            "Iter {{iteration_count}}, retry {{retry_count}}: Check requirements".to_string();
        assert!(validate(&config).is_ok());
    }
}
