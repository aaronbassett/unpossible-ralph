# Research: ralph-loop

**Date**: 2026-02-02
**Status**: Complete
**Source**: Discovery documentation at `discovery/`

## Technology Decisions

### 1. Configuration Format: TOML

**Decision**: Use TOML for all configuration (D1, D5)

**Rationale**:
- Standard in Rust ecosystem (Cargo.toml precedent)
- Multi-line string support for prompt templates
- Native table syntax for agent configuration
- Excellent `toml` crate with serde integration

**Alternatives Considered**:
- YAML: More complex, prone to subtle bugs, less Rust-idiomatic
- JSON: No comments, poor multi-line string support
- Custom format: Unnecessary complexity

### 2. Variable Interpolation: Handlebars-style

**Decision**: Use `{{variable}}` syntax with single-pass substitution (D26, spec FR49)

**Rationale**:
- Familiar syntax from many template systems
- Simple regex-based implementation sufficient
- Single-pass prevents recursive expansion attacks

**Implementation Notes**:
- Pattern: `\{\{([a-z_]+)\}\}`
- Reject spaces: `{{ var }}` is literal text (edge case E30)
- No recursive expansion: `{{dev_response}}` containing `{{retry_count}}` stays literal

**Variables**:
| Variable | Context | Source |
|----------|---------|--------|
| `{{prompt}}` | Agent commands | Rendered prompt template |
| `{{dev_response}}` | review, next_action prompts | Dev agent stdout |
| `{{dev_errors}}` | review, next_action prompts | Dev agent stderr |
| `{{review_response}}` | next_action prompt | Review agent stdout |
| `{{review_errors}}` | next_action prompt | Review agent stderr |
| `{{next_prompt}}` | continuation prompt | Next-action agent stdout |
| `{{iteration_count}}` | All prompts | 1-indexed loop counter |
| `{{retry_count}}` | All prompts | Consecutive failure counter |

### 3. Process Execution: tokio::process

**Decision**: Use tokio for async process management with timeout (D15, D19)

**Rationale**:
- Built-in timeout support via `tokio::time::timeout`
- Process group management for clean kills (spec FR43)
- Separate stdout/stderr capture (spec FR12-13, FR20-21)
- Signal handling for graceful shutdown

**Cost-Benefit Analysis**:
| Factor | tokio | std::process |
|--------|-------|--------------|
| Binary size | +~1MB | Baseline |
| Timeout support | Built-in | Manual threading |
| Signal handling | Native async | Complex |
| Process groups | Supported | Manual |
| Learning curve | Moderate | Low |

**Verdict**: tokio's overhead is acceptable given the orchestrator needs timeout, signal handling, and process group management. Alternative would require manual threading and complex coordination.

**Implementation Notes**:
- Execute via `/bin/sh -c` on Unix (spec FR44)
- Kill process group, not just parent (edge case E28)
- Replace invalid UTF-8 with U+FFFD (spec FR48)
- Timeout=0 means no timeout (edge case E12)

### 4. RESULT Parsing: Regex with Last-Wins

**Decision**: Case-insensitive regex, use last match if multiple (D21-23)

**Rationale**:
- Forgiving of LLM output variations
- Last occurrence represents final answer
- Missing RESULT treated as REPEAT (conservative)

**Pattern**: `(?i)^RESULT:\s*(CONTINUE|REPEAT|DONE)\s*$` (multiline mode)

**Edge Cases**:
- `RESULT: MAYBE` → treat as REPEAT with warning
- `THERESULT: DONE` → not matched (must start line)
- `RESULT: DONE now` → not matched (extra text)
- Empty response → treat as REPEAT with warning

### 5. Error Handling: anyhow + thiserror

**Decision**: Use anyhow for application errors, thiserror for library errors

**Rationale**:
- anyhow provides context chaining for human-readable errors
- thiserror enables structured error types for matching
- Aligns with constitution principle II (Human-Readable Errors)

**Exit Codes** (spec):
- 0: Success (RESULT: DONE or max_iterations reached)
- 1: Configuration error, command not found, critical agent failure
- 2: Max retries exceeded

### 6. CLI Framework: clap

**Decision**: Use clap for argument parsing

**Rationale**:
- Industry standard for Rust CLIs
- Derive macros for clean definition
- Built-in --help and --version (spec FR46-47)
- Subcommand support for future extension

**Interface**:
```
ralph [OPTIONS]

Options:
  -c, --config <PATH>  Path to config file [default: ./ralph.toml]
  -h, --help           Print help
  -V, --version        Print version
```

### 7. Logging: tracing

**Decision**: Use tracing for structured logging

**Rationale**:
- Async-compatible (works with tokio)
- Structured output for debugging
- Level filtering for verbosity control
- Per-constitution: log operations at appropriate levels

## Dependency Analysis

### Core Dependencies

| Crate | Version | Purpose | Size Impact |
|-------|---------|---------|-------------|
| clap | 4.5.x | CLI parsing | ~400KB |
| toml | 0.8.x | Config parsing | ~150KB |
| serde | 1.0.x | Serialization | ~200KB |
| tokio | 1.40.x | Async runtime | ~1MB (features: rt, process, time) |
| anyhow | 1.0.x | Error handling | ~30KB |
| thiserror | 1.0.x | Error types | ~15KB |
| tracing | 0.1.x | Logging | ~100KB |
| regex | 1.10.x | RESULT parsing | ~300KB |

**Total estimate**: ~2.5MB binary (release, stripped)

### Version Pinning Strategy

**Approach**: Use tilde requirements (`~x.y`) for patch-level flexibility while maintaining stability.

```toml
[dependencies]
clap = { version = "~4.5", features = ["derive"] }
toml = "~0.8"
serde = { version = "~1.0", features = ["derive"] }
tokio = { version = "~1.40", features = ["rt", "process", "time", "signal"] }
anyhow = "~1.0"
thiserror = "~1.0"
tracing = "~0.1"
tracing-subscriber = "~0.3"
regex = "~1.10"
```

**Rationale**: Pin to known-working minor versions but allow patch updates for security fixes. Run `cargo update` periodically and test before releases.

### Optional Dependencies

- `tracing-subscriber`: Formatted log output
- `once_cell` / `lazy_static`: Global state if needed

## Open Questions Resolved

All questions from `discovery/OPEN_QUESTIONS.md` have been resolved through 28 decisions. No blocking questions remain.

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Agent command injection | Medium | High | Document shell escaping in config, warn about untrusted input |
| Orphan processes | Medium | Medium | Kill process group, not just parent (FR43) |
| Memory from large output | Low | Medium | No truncation per D12, but could add optional limit later |
| Signal handling edge cases | Low | Low | Use tokio signal handling, test SIGINT/SIGTERM |

## References

- [Discovery SPEC.md](../../discovery/SPEC.md) - Complete feature specification
- [Discovery STATE.md](../../discovery/STATE.md) - Discovery process summary
- [Discovery DECISIONS.md](../../discovery/archive/DECISIONS.md) - All 28 decisions
- [Constitution](../../.sdd/memory/constitution.md) - Project principles
- [STACK.md](../../.sdd/codebase/STACK.md) - Technology choices
