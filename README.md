# unpossible-ralph

[![CI](https://github.com/aaronbassett/unpossible-ralph/actions/workflows/ci.yml/badge.svg)](https://github.com/aaronbassett/unpossible-ralph/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

AI coding agent loop orchestrator with intelligent verification.

## Overview

**unpossible-ralph** (or just `ralph`) solves the trust and automation gap when running AI coding agents in loops. It orchestrates a development agent, uses a review agent to verify work quality, and an independent done agent to confirm project completion.

### Key Features

- **Configurable Agents**: Use any CLI-based AI agent (Claude, GPT, local models)
- **Quality Verification**: Review agent evaluates work quality (`RESULT: CONTINUE|REPEAT`)
- **Independent Completion Check**: Done agent confirms completion by examining the filesystem, with no access to other agents' context
- **Automatic Routing**: CONTINUE to next task, REPEAT on failure, DONE when verified complete
- **Retry Management**: Configurable retry limits prevent infinite loops
- **Template Variables**: Dynamic prompt generation with context from previous iterations

## Installation

### From Source

Requires Rust 1.75 or later.

```bash
git clone https://github.com/aaronbassett/unpossible-ralph.git
cd unpossible-ralph
cargo build --release
```

The binary will be at `target/release/ralph`.

### Cargo Install

```bash
cargo install --git https://github.com/aaronbassett/unpossible-ralph.git
```

## Quick Start

1. Create a `ralph.toml` configuration file:

```toml
[general]
max_retries = 3
max_iterations = 10
timeout = 1800

[prompts]
starting = """
Implement the feature described in TASK.md.
Iteration: {{iteration_count}}
"""

continuation = """
Continue working on the task.

## Instructions from previous iteration
{{next_prompt}}

Iteration: {{iteration_count}}, Retries: {{retry_count}}
"""

review = """
Review the following work output for quality.

## Development Agent Output
{{dev_response}}

## Development Agent Errors
{{dev_errors}}

Respond with:
- RESULT: CONTINUE if work quality is acceptable
- RESULT: REPEAT if work needs to be redone
"""

next_action = """
Generate the next prompt based on the completed work.

## Development Agent Output
{{dev_response}}

## Review Agent Output
{{review_response}}
"""

done = """
Check if all requirements in TASK.md are satisfied by examining the source code.

Respond with:
- DONE: YES if all requirements are satisfied
- DONE: NO if more work is needed
"""

[agents]
dev = "claude -p '{{prompt}}'"
review = { command = "claude -p '{{prompt}}'", timeout = 600 }
next_action = "claude -p '{{prompt}}'"
done = "claude -p '{{prompt}}'"
```

2. Run ralph:

```bash
ralph
# or with a custom config path
ralph --config /path/to/config.toml
```

## Configuration Reference

### General Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `max_retries` | integer | 3 | Consecutive failures before exit (code 2) |
| `max_iterations` | integer | 0 | Total iteration limit (0 = unlimited) |
| `timeout` | integer | 1800 | Default agent timeout in seconds |

### Template Variables

| Variable | Available In | Description |
|----------|--------------|-------------|
| `{{prompt}}` | Agent commands | The fully-rendered prompt |
| `{{dev_response}}` | review, next_action | Stdout from dev agent |
| `{{dev_errors}}` | review, next_action | Stderr from dev agent |
| `{{review_response}}` | next_action | Stdout from review agent |
| `{{review_errors}}` | next_action | Stderr from review agent |
| `{{next_prompt}}` | continuation | Output from next-action agent |
| `{{iteration_count}}` | All prompts | Current iteration (1-indexed) |
| `{{retry_count}}` | All prompts | Consecutive failures |

**Note:** The done agent prompt intentionally cannot use `{{dev_response}}`, `{{dev_errors}}`, `{{review_response}}`, or `{{review_errors}}`. This ensures the done agent independently verifies completion by examining the filesystem rather than relying on other agents' output.

### Agent Configuration

Agents can be configured as simple strings or extended tables:

```toml
# Simple format (uses global timeout)
dev = "claude -p '{{prompt}}'"

# Extended format (with per-agent timeout)
review = { command = "claude -p '{{prompt}}'", timeout = 600 }
```

**Required agents:** `dev`, `review`, `next_action`, `done`

### Agent Signals

| Agent | Signal | Values | Purpose |
|-------|--------|--------|---------|
| Dev | `DEV_DONE: YES` | YES only | Claims work is complete |
| Review | `RESULT:` | CONTINUE, REPEAT | Quality assessment |
| Done | `DONE:` | YES, NO | Completion verification |

## Exit Codes

| Code | Name | Description |
|------|------|-------------|
| 0 | Success | Done agent confirms `DONE: YES` or max_iterations reached |
| 1 | Error | Configuration error, command not found, agent failure |
| 2 | Max Retries | Retry limit exceeded |
| 130 | Interrupted | SIGINT (Ctrl+C) |
| 143 | Terminated | SIGTERM |

## Architecture

```
                    +------------------+
                    |   Load Config    |
                    +--------+---------+
                             |
                    +--------v---------+
           +------->|   Dev Agent      |<----+
           |        | (may output      |     |
           |        |  DEV_DONE: YES)  |     |
           |        +--------+---------+     |
           |                 |               |
           |        +--------v---------+     |
           |        |  Review Agent    |     |
           |        | RESULT: CONTINUE |     |
           |        |    or REPEAT     |     |
           |        +--------+---------+     |
           |                 |               |
           |        +--------v---------+     |
           |        |  Parse RESULT    |     |
           |        +--------+---------+     |
           |                 |               |
           |     +-----------+-----------+   |
           |     |                       |   |
      +----v-----v----+            +-----v---+
      |    REPEAT     |            | CONTINUE|
      |    retry++    |            +---------+
      +---------------+                  |
                               +---------v---------+
                               | Dev claimed done? |
                               +---------+---------+
                                    |         |
                                   YES        NO
                                    |         |
                          +---------v----+    |
                          | Done Agent   |    |
                          | DONE: YES/NO |    |
                          +------+-------+    |
                                 |            |
                          +------v------+     |
                          | Parse DONE  |     |
                          +------+------+     |
                                 |            |
                     +-----------+----+       |
                     |                |       |
                +----v----+     +-----v-------v----+
                |  DONE   |     | Next-Action Agent|
                | exit(0) |     +--------+---------+
                +---------+              |
                                         v
                                   (next iteration)
```

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.

## Documentation

- [Specification](specs/001-ralph-loop/spec.md) - Detailed feature specification
- [Data Model](specs/001-ralph-loop/data-model.md) - Configuration schema and state machine
- [CLI Contract](specs/001-ralph-loop/contracts/cli.md) - Command-line interface details
- [Discovery](discovery/) - Design decisions and research

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Security

For security concerns, please see [SECURITY.md](SECURITY.md).
