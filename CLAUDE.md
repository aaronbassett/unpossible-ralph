# CLAUDE.md - Project Context for AI Assistants

## Project Overview

**unpossible-ralph** is a Rust CLI tool that orchestrates AI coding agent loops with intelligent verification. It runs a development agent, verifies work via a review agent, and routes to the next task or retry based on `RESULT: CONTINUE|REPEAT|DONE` signals.

## Active Technologies

| Category | Technology | Version |
|----------|------------|---------|
| Language | Rust | 1.75+ (Edition 2021) |
| CLI | clap | 4.x |
| Config | toml + serde | Latest |
| Async | tokio | 1.x |
| Error Handling | anyhow + thiserror | Latest |
| Logging | tracing | 0.1 |
| Regex | regex | 1.x |

## Project Structure

```
src/
├── main.rs           # Entry point, CLI setup
├── config/           # TOML parsing and validation
├── loop/             # Core orchestration loop
├── agent/            # Agent execution and capture
├── template/         # Variable interpolation
└── result/           # RESULT parsing and routing

specs/001-ralph-loop/
├── spec.md           # Feature specification
├── plan.md           # Implementation plan
├── research.md       # Technology decisions
├── data-model.md     # Config schema, state machine
├── quickstart.md     # Developer setup
└── contracts/        # CLI interface contract

discovery/            # Discovery documentation (28 decisions)
.sdd/                 # SDD workflow artifacts
```

## Common Commands

```bash
# Build
cargo build
cargo build --release

# Test
cargo test
cargo test -- --nocapture

# Lint & Format
cargo fmt --check
cargo clippy -- -D warnings

# Run
cargo run -- --help
cargo run -- --config ./ralph.toml
```

## Constitution Principles

This project follows 6 core principles (see `.sdd/memory/constitution.md`):

1. **Unix Philosophy**: Do one thing well, predictable exit codes
2. **Human-Readable Errors**: Clear context and suggestions
3. **Keep It Simple (KISS)**: Avoid unnecessary abstractions
4. **Modularity**: Clear boundaries between modules
5. **Test What Matters**: Integration tests for CLI, unit tests for parsing
6. **Correctness First**: Get it working before optimizing

## Key Design Decisions

- **Config**: TOML format with `{{variable}}` interpolation
- **Agents**: Shell commands with `{{prompt}}` placeholder
- **RESULT Parsing**: Case-insensitive, last match wins, missing = REPEAT
- **Exit Codes**: 0 (success), 1 (error), 2 (max retries exceeded)
- **Process Execution**: tokio with process group kills

## Current Focus

**Feature**: 001-ralph-loop (Core orchestrator implementation)

Stories (in priority order):
1. S6: TOML Configuration (P1)
2. S1: Core Loop Execution (P2)
3. S2: Review Agent Integration (P3)
4. S3: Result Parsing & Routing (P4)
5. S5: Retry Management (P5)
6. S4: Next-Action Prompt Generation (P6)

## Recent Changes

- 2026-02-02: Initial project setup and specification complete
- 2026-02-02: Implementation plan created with research, data model, contracts

<!-- MANUAL ADDITIONS START -->
<!-- Add project-specific notes here that should persist across updates -->
<!-- MANUAL ADDITIONS END -->
