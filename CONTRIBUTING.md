# Contributing to unpossible-ralph

Thank you for your interest in contributing to unpossible-ralph! This document provides guidelines and instructions for contributing.

## How to Contribute

### Reporting Bugs

Before creating a bug report, please check existing issues to avoid duplicates. When creating a bug report, include:

- A clear, descriptive title
- Steps to reproduce the issue
- Expected behavior vs actual behavior
- Your environment (OS, Rust version, ralph version)
- Relevant configuration (with sensitive data removed)
- Error messages and logs

### Suggesting Features

Feature requests are welcome! Please include:

- A clear description of the feature
- The problem it solves or use case it enables
- Any alternative solutions you've considered

### Pull Requests

1. Fork the repository
2. Create a feature branch from `main`
3. Make your changes
4. Ensure all tests pass
5. Submit a pull request

## Development Setup

### Prerequisites

- Rust 1.75 or later
- Git

### Getting Started

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/unpossible-ralph.git
cd unpossible-ralph

# Add upstream remote
git remote add upstream https://github.com/aaronbassett/unpossible-ralph.git

# Build the project
cargo build

# Run tests
cargo test
```

### Development Commands

```bash
# Build
cargo build
cargo build --release

# Run tests
cargo test
cargo test -- --nocapture  # Show println! output

# Format code (required before committing)
cargo fmt

# Check formatting without modifying
cargo fmt --check

# Run linter (required before committing)
cargo clippy -- -D warnings

# Run the application
cargo run -- --help
cargo run -- --config ./ralph.toml
```

## Code Style

### Formatting

All code must be formatted with `rustfmt`. Run `cargo fmt` before committing.

### Linting

All code must pass `clippy` with warnings treated as errors:

```bash
cargo clippy -- -D warnings
```

### Guidelines

- Follow Rust naming conventions
- Write clear, descriptive variable and function names
- Add comments for complex logic
- Keep functions focused and small
- Prefer explicit error handling over panics

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run a specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture
```

### Writing Tests

- Write unit tests for parsing and validation logic
- Write integration tests for CLI behavior
- Use `assert_cmd` and `predicates` for CLI testing
- Use `tempfile` for tests that need temporary files

### Test Organization

```
src/
  config/
    mod.rs          # Unit tests for config parsing
  result/
    mod.rs          # Unit tests for RESULT parsing
tests/
  integration.rs    # CLI integration tests
```

## Project Structure

```
src/
  main.rs           # Entry point, CLI setup
  lib.rs            # Library root
  config/           # TOML parsing and validation
  loop/             # Core orchestration loop
  agent/            # Agent execution and capture
  template/         # Variable interpolation
  result/           # RESULT parsing and routing

specs/              # Feature specifications
discovery/          # Design decisions
```

## Commit Messages

Write clear, concise commit messages:

- Use the imperative mood ("Add feature" not "Added feature")
- Keep the first line under 72 characters
- Reference issues when applicable (`Fixes #123`)

Examples:
```
Add timeout configuration per agent

Implement FR22: per-agent timeout overrides global default.
Agents can now specify timeout in extended table format.

Fixes #45
```

## Pull Request Process

1. **Update documentation** if your changes affect user-facing behavior
2. **Add tests** for new functionality
3. **Run the full test suite** and ensure it passes
4. **Update the CHANGELOG** if applicable
5. **Request review** from maintainers

### PR Checklist

- [ ] Code follows project style guidelines
- [ ] `cargo fmt` has been run
- [ ] `cargo clippy -- -D warnings` passes
- [ ] All tests pass
- [ ] Documentation updated (if applicable)
- [ ] CHANGELOG updated (if applicable)

## Questions?

If you have questions, feel free to:

- Open an issue for discussion
- Check existing issues and documentation
- See [SUPPORT.md](SUPPORT.md) for additional resources

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
