# Tech Stack: unpossible-ralph

**Status**: Planned (greenfield project)
**Last Updated**: 2026-02-02

## Language & Runtime

| Component | Technology | Version | Rationale |
|-----------|------------|---------|-----------|
| Primary Language | Rust | Latest stable (1.75+) | Per constitution - CLI tool with process management |
| Edition | 2021 | - | Standard modern Rust |

## Core Dependencies (Planned)

| Category | Crate | Purpose |
|----------|-------|---------|
| CLI | `clap` | Command-line argument parsing |
| Config | `toml` | TOML configuration parsing |
| Config | `serde` | Serialization/deserialization |
| Async | `tokio` | Async runtime for process management |
| Process | `tokio::process` | Subprocess execution with timeout |
| Template | Custom (regex-based) | Variable interpolation (`{{var}}`) - single-pass regex, see research.md |
| Error | `anyhow` | Error handling with context |
| Error | `thiserror` | Custom error types |
| Logging | `tracing` | Structured logging |
| Regex | `regex` | RESULT signal parsing |

## Build & Tooling

| Tool | Purpose |
|------|---------|
| `cargo` | Build system and package manager |
| `rustfmt` | Code formatting (per constitution) |
| `clippy` | Linting (per constitution) |
| `cargo test` | Test runner |

## Testing Strategy

Per constitution principle V (Test What Matters):
- **Integration tests**: CLI behavior, config loading, process execution
- **Unit tests**: RESULT parsing, variable interpolation
- Focus on critical paths, not coverage metrics

## Project Structure (Planned)

```
src/
├── main.rs           # Entry point, CLI setup
├── config/           # TOML parsing and validation
│   ├── mod.rs
│   └── types.rs
├── loop/             # Core orchestration loop
│   ├── mod.rs
│   └── executor.rs
├── agent/            # Agent execution and capture
│   └── mod.rs
├── template/         # Variable interpolation
│   └── mod.rs
└── result/           # RESULT parsing and routing
    └── mod.rs

tests/
├── integration/      # End-to-end CLI tests
└── fixtures/         # Test config files
```

## Environment

| Variable | Purpose | Required |
|----------|---------|----------|
| (Agent-specific) | API credentials for LLM agents | Per agent config |

Note: Per decision D9, API credentials are environment variables only - not stored in config files.
