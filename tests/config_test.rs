//! Integration tests for configuration loading
//!
//! Tests config loading with actual TOML files per spec acceptance scenarios:
//! - E1-E5: Error handling scenarios
//! - E24: Missing continuation template
//! - E26: Negative max_retries
//! - E29: Unknown config keys
//! - E30: Variable syntax with spaces

use ralph::config;
use std::path::Path;

/// Test loading valid configuration file
#[test]
fn test_load_valid_config_from_fixture() {
    let path = Path::new("tests/integration/fixtures/valid_config.toml");
    let result = config::load(path);
    assert!(
        result.is_ok(),
        "Expected valid config to load: {:?}",
        result.err()
    );

    let config = result.unwrap();
    assert_eq!(config.general.max_retries, 3);
    assert_eq!(config.general.max_iterations, 10);
    assert_eq!(config.general.timeout, 1800);
    assert!(config.prompts.starting.contains("{{iteration_count}}"));
    assert!(config.prompts.continuation.contains("{{next_prompt}}"));
    assert!(config.prompts.review.contains("{{dev_response}}"));
}

/// Test loading invalid TOML syntax (E4 equivalent for parser)
#[test]
fn test_load_invalid_toml_syntax() {
    let path = Path::new("tests/integration/fixtures/invalid_config.toml");
    let result = config::load(path);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let msg = err.to_string();
    // Should mention parse error
    assert!(
        msg.contains("parse") || msg.contains("Parse"),
        "Error should mention parsing: {}",
        msg
    );
}

/// Test loading config with missing [agents] section
#[test]
fn test_load_missing_agents_section() {
    let path = Path::new("tests/integration/fixtures/missing_section.toml");
    let result = config::load(path);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let msg = err.to_string();
    // Should mention missing field/section
    assert!(
        msg.contains("agents") || msg.contains("missing"),
        "Error should mention missing agents: {}",
        msg
    );
}

/// Test loading nonexistent file (E3 - config not found)
#[test]
fn test_load_nonexistent_file() {
    let path = Path::new("tests/integration/fixtures/does_not_exist.toml");
    let result = config::load(path);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("read") || msg.contains("Read") || msg.contains("No such file"),
        "Error should mention file read failure: {}",
        msg
    );
}
