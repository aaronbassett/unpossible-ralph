# Developer Quickstart: ralph-loop

**Date**: 2026-02-02

## Prerequisites

- Rust 1.75+ (latest stable)
- Cargo (comes with Rust)
- Git

### Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustup update stable
```

## Project Setup

### Clone and Initialize

```bash
cd ~/Projects/unpossible-ralph

# Initialize Cargo project (if not done)
cargo init --name ralph

# Add dependencies
cargo add clap --features derive
cargo add toml
cargo add serde --features derive
cargo add tokio --features "rt-multi-thread process time signal macros"
cargo add anyhow
cargo add thiserror
cargo add tracing
cargo add tracing-subscriber --features "fmt env-filter"
cargo add regex
```

### Directory Structure

```
unpossible-ralph/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── config/
│   │   ├── mod.rs
│   │   ├── types.rs
│   │   └── validate.rs
│   ├── loop/
│   │   ├── mod.rs
│   │   └── executor.rs
│   ├── agent/
│   │   ├── mod.rs
│   │   └── capture.rs
│   ├── template/
│   │   └── mod.rs
│   └── result/
│       ├── mod.rs
│       └── types.rs
├── tests/
│   ├── integration/
│   │   ├── config_test.rs
│   │   ├── loop_test.rs
│   │   └── fixtures/
│   └── unit/
├── ralph.toml.example
└── README.md
```

## Development Commands

### Build

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run from source
cargo run -- --help
cargo run -- --config ./ralph.toml.example
```

### Test

```bash
# Run all tests
cargo test

# Run specific test
cargo test config_loading

# Run with output
cargo test -- --nocapture

# Run integration tests only
cargo test --test '*'
```

### Lint & Format

```bash
# Check formatting
cargo fmt --check

# Apply formatting
cargo fmt

# Run clippy
cargo clippy -- -D warnings

# All checks (for CI)
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

### Documentation

```bash
# Generate docs
cargo doc --open

# Check doc comments
cargo doc --no-deps
```

## Example Configuration

Create `ralph.toml.example`:

```toml
[general]
max_retries = 3
max_iterations = 10
timeout = 1800  # 30 minutes

[prompts]
starting = """
You are a coding assistant. Complete the following task:

Task: Write a simple hello world function.

Iteration: {{iteration_count}}, Retries: {{retry_count}}
"""

continuation = """
Continue working on the task.

## Previous work feedback
{{next_prompt}}

Iteration: {{iteration_count}}, Retries: {{retry_count}}
"""

review = """
Review the following development work.

## Development Output
{{dev_response}}

## Development Errors (if any)
{{dev_errors}}

Evaluate the work and respond with exactly one of:
- RESULT: CONTINUE - if work is complete and correct
- RESULT: REPEAT - if work needs improvement
- RESULT: DONE - if all work is finished
"""

next_action = """
Based on the completed work, generate the next task prompt.

## Development Output
{{dev_response}}

## Review Feedback
{{review_response}}

Write a clear prompt for the next development task.
"""

[agents]
# Simple format
dev = "echo '{{prompt}}' | head -c 100"

# Extended format with timeout
review = { command = "echo 'RESULT: CONTINUE'", timeout = 60 }

# Simple format
next_action = "echo 'Continue with the next task'"
```

## Testing Setup

### Unit Test Example

```rust
// src/result/mod.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_result_continue() {
        let (result, explicit) = ReviewResult::parse("RESULT: CONTINUE");
        assert_eq!(result, ReviewResult::Continue);
        assert!(explicit);
    }

    #[test]
    fn parse_result_case_insensitive() {
        let (result, _) = ReviewResult::parse("result: done");
        assert_eq!(result, ReviewResult::Done);
    }

    #[test]
    fn parse_result_missing_defaults_to_repeat() {
        let (result, explicit) = ReviewResult::parse("No result here");
        assert_eq!(result, ReviewResult::Repeat);
        assert!(!explicit);
    }
}
```

### Integration Test Example

```rust
// tests/integration/config_test.rs
use std::fs;
use std::process::Command;

#[test]
fn test_missing_config_error() {
    let output = Command::new("cargo")
        .args(["run", "--", "--config", "/nonexistent/path.toml"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Config file not found") || stderr.contains("No such file"));
}

#[test]
fn test_valid_config_loads() {
    // Create temp config
    let config = r#"
[general]
max_retries = 3

[prompts]
starting = "test"
continuation = "{{next_prompt}}"
review = "{{dev_response}}"
next_action = "{{dev_response}}"

[agents]
dev = "echo '{{prompt}}'"
review = "echo 'RESULT: DONE'"
next_action = "echo 'next'"
"#;

    let temp_path = "/tmp/ralph_test_config.toml";
    fs::write(temp_path, config).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--", "--config", temp_path])
        .output()
        .expect("Failed to execute command");

    // Should succeed with RESULT: DONE
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    fs::remove_file(temp_path).ok();
}
```

## Git Hooks (Optional)

### Pre-commit Hook

Create `.git/hooks/pre-commit`:

```bash
#!/bin/bash
set -e

echo "Running pre-commit checks..."

# Format check
cargo fmt --check || {
    echo "Format check failed. Run 'cargo fmt' to fix."
    exit 1
}

# Clippy
cargo clippy -- -D warnings || {
    echo "Clippy found issues."
    exit 1
}

# Tests
cargo test --quiet || {
    echo "Tests failed."
    exit 1
}

echo "Pre-commit checks passed!"
```

Make executable:
```bash
chmod +x .git/hooks/pre-commit
```

## Troubleshooting

### Common Issues

**Rust version too old**
```bash
rustup update stable
rustc --version  # Should be 1.75+
```

**Cargo add not found**
```bash
# cargo-edit provides 'cargo add'
cargo install cargo-edit
```

**Tokio runtime issues**
```bash
# Ensure tokio has required features
cargo add tokio --features "rt-multi-thread process time signal macros"
```

## Next Steps

1. Create module structure with `mod.rs` files
2. Implement config loading (Story 6)
3. Implement core loop (Story 1)
4. Add review integration (Story 2)
5. Add result parsing (Story 3)
6. Add retry management (Story 5)
7. Add next-action generation (Story 4)

See `plan.md` for implementation phases and `spec.md` for detailed requirements.
