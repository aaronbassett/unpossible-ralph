# Implementation Plan: ralph-loop

**Branch**: `001-ralph-loop` | **Date**: 2026-02-02 | **Spec**: [specs/001-ralph-loop/spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-ralph-loop/spec.md`
**Discovery**: Full context at [discovery/](../../discovery/) including 28 key decisions

## Summary

Build a Rust CLI orchestrator (`ralph`) that automates AI coding agent loops with intelligent verification. The orchestrator:
1. Loads TOML configuration with prompt templates and agent commands
2. Runs a development agent with configurable shell command
3. Passes output to a review agent that returns `RESULT: CONTINUE|REPEAT|DONE`
4. Routes based on result: continue to next task, retry current task, or exit
5. Enforces retry limits and optional iteration limits

**Technical Approach**: Simple, modular Rust CLI following Unix philosophy with tokio for async process management, TOML for config, and handlebars-style variable interpolation.

## Technical Context

**Language/Version**: Rust 1.75+ (latest stable), Edition 2021
**Primary Dependencies**:
- `clap` - CLI argument parsing
- `toml` + `serde` - Configuration parsing
- `tokio` - Async runtime for process management
- `anyhow` + `thiserror` - Error handling with context
- `tracing` - Structured logging
- `regex` - RESULT signal parsing

**Storage**: N/A (stateless CLI, config from TOML files)
**Testing**: `cargo test` with integration tests for CLI behavior, unit tests for parsing
**Target Platform**: Linux (primary), macOS, Windows (via cross-compilation)
**Project Type**: Single CLI application
**Performance Goals**: Fast enough for interactive use (<100ms startup)
**Constraints**: Agent execution is the bottleneck, not the orchestrator
**Scale/Scope**: Single-user CLI tool, process isolation per agent execution

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Principle I: Unix Philosophy ✅
- [x] Does one thing well: orchestrates agent loops with verification
  - *Justification*: Single purpose CLI, no GUI, no daemon mode, no plugin system
- [x] Accepts input from stdin (config path via --config) and arguments
  - *Justification*: Standard CLI flags, config file path, no interactive prompts during operation
- [x] Output to stdout (summaries), errors to stderr
  - *Justification*: Summaries to stdout enable piping; errors to stderr for filtering (FR51)
- [x] Exit codes: 0 (success/DONE), 1 (error), 2 (max retries exceeded)
  - *Justification*: Meaningful exit codes enable shell scripting and CI integration
- [x] Text-based I/O for config and output
  - *Justification*: TOML config is human-readable; output is plain text

### Principle II: Human-Readable Errors ✅
- [x] Config validation errors include field names and expected values
  - *Justification*: "Missing required field: agents.dev" vs generic "invalid config"
- [x] Process failures include command, exit code, and partial output
  - *Justification*: Enables debugging without re-running; shows what failed and why
- [x] Suggestions where applicable ("Check that command exists", etc.)
  - *Justification*: Actionable errors reduce user frustration; E6, E7 provide specific guidance

### Principle III: Keep It Simple (KISS) ✅
- [x] No abstractions until needed (direct process execution, no agent interface)
  - *Justification*: Agents are shell commands, not traits; no plugin architecture
- [x] Single config format (TOML only)
  - *Justification*: One format to document, one parser to maintain
- [x] Simple state machine: dev → review → route → repeat
  - *Justification*: Four states, three transitions; see data-model.md state diagram

### Principle IV: Modularity ✅
- [x] Clear module boundaries: config, loop, agent, template, result
  - *Justification*: Each module has single responsibility, can be tested independently
- [x] No circular dependencies in planned structure
  - *Justification*: config → template → agent → result → loop (one direction)
- [x] Single responsibility per module
  - *Justification*: config loads/validates, template interpolates, agent executes, result parses

### Principle V: Test What Matters ✅
- [x] Integration tests for CLI behavior (config loading, process execution)
  - *Justification*: Tests user-facing behavior, catches integration bugs
- [x] Unit tests for RESULT parsing and variable interpolation
  - *Justification*: Complex regex logic needs exhaustive unit testing (8 edge cases)
- [x] Focus on critical paths per constitution
  - *Justification*: No coverage targets; test what breaks, not what's easy to test

### Principle VI: Make It Work, Then Make It Fast ✅
- [x] Correctness first: proper error handling, clean state management
  - *Justification*: Error handling for all 31 edge cases before performance work
- [x] No premature optimization (agent execution is the bottleneck)
  - *Justification*: Agent commands take minutes; orchestrator overhead is negligible

**Gate Status**: ✅ PASSED - No violations identified

## Project Structure

### Documentation (this feature)

```text
specs/001-ralph-loop/
├── plan.md              # This file
├── research.md          # Phase 0: Technology decisions
├── data-model.md        # Phase 1: Config schema, state machine
├── quickstart.md        # Phase 1: Developer setup guide
├── contracts/           # Phase 1: CLI interface contract
└── tasks.md             # Phase 2 output (/sdd:tasks command)
```

### Source Code (repository root)

```text
src/
├── main.rs              # Entry point, CLI setup with clap
├── config/              # TOML parsing and validation
│   ├── mod.rs           # Public API for config module
│   ├── types.rs         # Config structs (General, Prompts, Agents)
│   └── validate.rs      # Validation rules (required fields, {{prompt}}, etc.)
├── loop/                # Core orchestration loop
│   ├── mod.rs           # Loop state machine
│   └── executor.rs      # Iteration execution logic
├── agent/               # Agent execution and capture
│   ├── mod.rs           # Agent runner with timeout
│   └── capture.rs       # Stdout/stderr capture
├── template/            # Variable interpolation
│   └── mod.rs           # {{variable}} substitution
└── result/              # RESULT parsing and routing
    ├── mod.rs           # Parser and routing logic
    └── types.rs         # Result enum (Continue, Repeat, Done)

tests/
├── integration/         # End-to-end CLI tests
│   ├── config_test.rs   # Config loading and validation
│   ├── loop_test.rs     # Full loop execution
│   └── fixtures/        # Test config files
└── unit/                # Unit tests (inline with source)
```

**Structure Decision**: Single Rust CLI project following standard `src/` layout. Modules organized by responsibility matching the spec's 6 stories. Tests split between integration (CLI behavior) and unit (parsing, interpolation) per constitution principle V.

## Complexity Tracking

> **No violations identified** - Constitution check passed with no exceptions needed.

## Key Decisions Summary

From discovery (28 decisions documented in `discovery/archive/DECISIONS.md`):

| ID | Decision | Impact |
|----|----------|--------|
| D1 | TOML for configuration | Rust ecosystem standard |
| D10 | Agents as shell commands with {{prompt}} | Maximum flexibility |
| D13 | Separate stdout/stderr as {{dev_response}}/{{dev_errors}} | Clean variable model |
| D15 | 30 minute default timeout, configurable | Safety with override |
| D21-23 | Case-insensitive RESULT parsing, last wins, missing=REPEAT | Robust parsing |
| D24 | Exit code 2 for max retries | Distinct from other errors |
| D28 | Two templates: starting (iter 1) and continuation (iter 2+) | Clean state transitions |

## Decision-to-Requirement Traceability

| Decision | Functional Requirements | Rationale |
|----------|------------------------|-----------|
| D1 | FR1, FR2-4 | TOML format enables section validation |
| D5 | FR1-4 | Config structure with [general], [prompts], [agents] |
| D10 | FR5, FR8, FR10 | Agents as shell commands with {{prompt}} |
| D13 | FR12-13, FR20-21 | Separate stdout/stderr capture |
| D15 | FR16, FR22 | 30-minute default timeout with per-agent override |
| D19 | FR11, FR43, FR44 | tokio process with shell execution and group kill |
| D21 | FR24 | Case-insensitive RESULT parsing |
| D22 | FR26 | Last RESULT wins when multiple found |
| D23 | FR27 | Missing RESULT treated as REPEAT |
| D24 | FR34, FR42 | Exit code 2 for max retries, 1 for other errors |
| D26 | FR7, FR49 | Single-pass variable interpolation |
| D28 | FR38, FR39, FR40 | Two-template system for starting vs continuation |

## Implementation Phases

### Phase 0: Research ✅
- Technology choices validated in STACK.md
- All 28 decisions documented
- No NEEDS CLARIFICATION items remaining
- Output: `research.md`

### Phase 1: Design ✅
- [x] Generate data-model.md with config schema and state machine
- [x] Define CLI contract in contracts/cli.md
- [x] Create quickstart.md for developer onboarding
- [x] Create CLAUDE.md with project context
- Output: `data-model.md`, `contracts/cli.md`, `quickstart.md`, `CLAUDE.md`

### Phase 2: Local Development Setup (Pending Rust Installation)
- [ ] Initialize Cargo project with dependencies
- [ ] Configure rustfmt and clippy
- [ ] Set up pre-commit hooks
- [ ] Configure CI pipeline

**Note**: Rust toolchain not available in current environment. Developer should run:
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Then follow quickstart.md for project initialization
```

### Phase 3: Implementation (via /sdd:tasks)
- Tasks generated from stories in priority order
- Each story independently testable
- Run `/sdd:tasks` after Phase 2 setup complete

## Generated Artifacts

| Artifact | Path | Status |
|----------|------|--------|
| Implementation Plan | `specs/001-ralph-loop/plan.md` | ✅ Complete |
| Research | `specs/001-ralph-loop/research.md` | ✅ Complete |
| Data Model | `specs/001-ralph-loop/data-model.md` | ✅ Complete |
| CLI Contract | `specs/001-ralph-loop/contracts/cli.md` | ✅ Complete |
| Developer Quickstart | `specs/001-ralph-loop/quickstart.md` | ✅ Complete |
| Agent Context | `CLAUDE.md` | ✅ Complete |
| Cargo Project | `Cargo.toml`, `src/` | ⏳ Pending Rust |
| Pre-commit Hooks | `.git/hooks/pre-commit` | ⏳ Pending Rust |
| CI Pipeline | `.github/workflows/ci.yml` | ⏳ Pending Rust |

## Next Steps

1. **Install Rust** (if not installed): See quickstart.md
2. **Initialize Cargo project**: `cargo init --name ralph`
3. **Add dependencies**: Per quickstart.md
4. **Run /sdd:tasks**: Generate implementation tasks
5. **Implement stories**: In priority order (S6 → S1 → S2 → S3 → S5 → S4)
