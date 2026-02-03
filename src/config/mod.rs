//! Configuration loading and validation for ralph.
//!
//! This module provides the [`load`] function to read, parse, and validate
//! TOML configuration files. It exports all configuration types needed by
//! other modules.
//!
//! # Example
//!
//! ```no_run
//! use std::path::Path;
//! use ralph::config;
//!
//! let config = config::load(Path::new("ralph.toml"))?;
//! println!("Max retries: {}", config.general.max_retries);
//! # Ok::<(), config::ConfigError>(())
//! ```

mod types;
mod validate;

pub use types::{AgentConfig, AgentsConfig, Config, GeneralConfig, PromptsConfig};
pub use validate::ValidationError;

use std::path::Path;
use thiserror::Error;

/// Errors that can occur when loading configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Failed to read the configuration file.
    #[error("Failed to read config file '{path}': {source}")]
    ReadError {
        path: String,
        source: std::io::Error,
    },

    /// Failed to parse the TOML content.
    #[error("Failed to parse config file '{path}': {message}")]
    ParseError { path: String, message: String },

    /// Configuration validation failed.
    #[error("Config validation failed: {0}")]
    ValidationError(#[from] ValidationError),
}

/// Loads and validates a ralph configuration file.
///
/// This function reads the TOML file at the given path, deserializes it into
/// a [`Config`] struct, and validates it according to ralph's requirements.
///
/// # Arguments
///
/// * `path` - Path to the TOML configuration file
///
/// # Errors
///
/// Returns a [`ConfigError`] if:
/// - The file cannot be read ([`ConfigError::ReadError`])
/// - The TOML is malformed or has unknown fields ([`ConfigError::ParseError`])
/// - The configuration fails semantic validation ([`ConfigError::ValidationError`])
///
/// # Example
///
/// ```no_run
/// use std::path::Path;
/// use ralph::config;
///
/// match config::load(Path::new("ralph.toml")) {
///     Ok(config) => {
///         println!("Loaded config with {} max retries", config.general.max_retries);
///     }
///     Err(e) => {
///         eprintln!("Error: {e}");
///     }
/// }
/// ```
pub fn load(path: &Path) -> Result<Config, ConfigError> {
    // Read file contents
    let contents = std::fs::read_to_string(path).map_err(|e| ConfigError::ReadError {
        path: path.display().to_string(),
        source: e,
    })?;

    // Parse TOML
    let config: Config = toml::from_str(&contents).map_err(|e| ConfigError::ParseError {
        path: path.display().to_string(),
        message: e.message().to_string(),
    })?;

    // Validate config
    validate::validate(&config)?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn valid_toml() -> &'static str {
        r#"
[general]
max_retries = 3

[prompts]
starting = "Start the task"
continuation = "Continue with: {{next_prompt}}"
review = "Review this output: {{dev_response}}"
next_action = "What's next based on: {{dev_response}}"
done = "Check if requirements are satisfied"

[agents]
dev = "echo '{{prompt}}'"
review = "echo '{{prompt}}'"
next_action = "echo '{{prompt}}'"
done = "echo '{{prompt}}'"
"#
    }

    fn write_temp_config(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_load_valid_config() {
        let file = write_temp_config(valid_toml());
        let config = load(file.path()).unwrap();

        assert_eq!(config.general.max_retries, 3);
        assert_eq!(config.general.max_iterations, 0);
        assert_eq!(config.general.timeout, 1800);
        assert_eq!(config.prompts.starting, "Start the task");
        assert_eq!(config.agents.dev.command(), "echo '{{prompt}}'");
    }

    #[test]
    fn test_load_with_optional_fields() {
        let toml = r#"
[general]
max_retries = 5
max_iterations = 10
timeout = 3600

[prompts]
starting = "Start"
continuation = "Continue: {{next_prompt}}"
review = "Review: {{dev_response}}"
next_action = "Next: {{dev_errors}}"
done = "Check completion"

[agents]
dev = "cmd {{prompt}}"
review = "cmd {{prompt}}"
next_action = "cmd {{prompt}}"
done = "cmd {{prompt}}"
"#;
        let file = write_temp_config(toml);
        let config = load(file.path()).unwrap();

        assert_eq!(config.general.max_retries, 5);
        assert_eq!(config.general.max_iterations, 10);
        assert_eq!(config.general.timeout, 3600);
    }

    #[test]
    fn test_load_extended_agent_config() {
        let toml = r#"
[general]
max_retries = 3

[prompts]
starting = "Start"
continuation = "Continue: {{next_prompt}}"
review = "Review: {{dev_response}}"
next_action = "Next: {{dev_response}}"
done = "Check completion"

[agents.dev]
command = "custom-cmd {{prompt}}"
timeout = 7200

[agents.review]
command = "review-cmd {{prompt}}"

[agents]
next_action = "simple {{prompt}}"
done = "done-cmd {{prompt}}"
"#;
        let file = write_temp_config(toml);
        let config = load(file.path()).unwrap();

        assert_eq!(config.agents.dev.command(), "custom-cmd {{prompt}}");
        assert_eq!(config.agents.dev.timeout(), Some(7200));
        assert_eq!(config.agents.review.command(), "review-cmd {{prompt}}");
        assert_eq!(config.agents.review.timeout(), None);
        assert_eq!(config.agents.next_action.command(), "simple {{prompt}}");
        assert_eq!(config.agents.next_action.timeout(), None);
        assert_eq!(config.agents.done.command(), "done-cmd {{prompt}}");
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = load(Path::new("/nonexistent/path/config.toml"));
        assert!(matches!(result, Err(ConfigError::ReadError { .. })));
    }

    #[test]
    fn test_load_invalid_toml() {
        let file = write_temp_config("this is not valid toml [[[");
        let result = load(file.path());
        assert!(matches!(result, Err(ConfigError::ParseError { .. })));
    }

    #[test]
    fn test_load_unknown_field_rejected() {
        let toml = r#"
[general]
max_retries = 3
unknown_field = "bad"

[prompts]
starting = "Start"
continuation = "Continue: {{next_prompt}}"
review = "Review: {{dev_response}}"
next_action = "Next: {{dev_response}}"
done = "Check completion"

[agents]
dev = "cmd {{prompt}}"
review = "cmd {{prompt}}"
next_action = "cmd {{prompt}}"
done = "cmd {{prompt}}"
"#;
        let file = write_temp_config(toml);
        let result = load(file.path());
        assert!(matches!(result, Err(ConfigError::ParseError { .. })));
    }

    #[test]
    fn test_load_missing_required_field() {
        let toml = r#"
[general]
# missing max_retries

[prompts]
starting = "Start"
continuation = "Continue: {{next_prompt}}"
review = "Review: {{dev_response}}"
next_action = "Next: {{dev_response}}"
done = "Check completion"

[agents]
dev = "cmd {{prompt}}"
review = "cmd {{prompt}}"
next_action = "cmd {{prompt}}"
done = "cmd {{prompt}}"
"#;
        let file = write_temp_config(toml);
        let result = load(file.path());
        assert!(matches!(result, Err(ConfigError::ParseError { .. })));
    }

    #[test]
    fn test_load_validation_error() {
        let toml = r#"
[general]
max_retries = 3

[prompts]
starting = "Start"
continuation = "Continue without next_prompt"
review = "Review: {{dev_response}}"
next_action = "Next: {{dev_response}}"
done = "Check completion"

[agents]
dev = "cmd {{prompt}}"
review = "cmd {{prompt}}"
next_action = "cmd {{prompt}}"
done = "cmd {{prompt}}"
"#;
        let file = write_temp_config(toml);
        let result = load(file.path());
        assert!(matches!(result, Err(ConfigError::ValidationError(_))));
    }

    #[test]
    fn test_config_error_display() {
        let err = ConfigError::ReadError {
            path: "test.toml".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
        };
        let msg = err.to_string();
        assert!(msg.contains("test.toml"));
        assert!(msg.contains("file not found"));

        let err = ConfigError::ParseError {
            path: "bad.toml".to_string(),
            message: "unexpected character".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("bad.toml"));
        assert!(msg.contains("unexpected character"));
    }
}
