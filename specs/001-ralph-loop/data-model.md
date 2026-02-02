# Data Model: ralph-loop

**Date**: 2026-02-02
**Status**: Complete

## Configuration Schema

### TOML Structure

```toml
[general]
max_retries = 3        # u32, required, >= 0
max_iterations = 0     # u32, optional, default 0 (unlimited)
timeout = 1800         # u32, optional, default 1800 seconds

[prompts]
starting = "..."       # String, required, non-empty
continuation = "..."   # String, required, non-empty, must contain {{next_prompt}}
review = "..."         # String, required, must contain {{dev_response}} or {{dev_errors}}
next_action = "..."    # String, required, must contain {{dev_response}} or {{dev_errors}}

[agents]
dev = "..."            # String or AgentConfig, required, must contain {{prompt}}
review = "..."         # String or AgentConfig, required, must contain {{prompt}}
next_action = "..."    # String or AgentConfig, required, must contain {{prompt}}
```

### Rust Types

```rust
use serde::Deserialize;
use std::path::PathBuf;

/// Root configuration structure
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]  // Reject unknown keys (edge case E29)
pub struct Config {
    pub general: GeneralConfig,
    pub prompts: PromptsConfig,
    pub agents: AgentsConfig,
}

/// General settings
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneralConfig {
    pub max_retries: u32,
    #[serde(default)]
    pub max_iterations: u32,  // 0 = unlimited
    #[serde(default = "default_timeout")]
    pub timeout: u32,  // seconds
}

fn default_timeout() -> u32 { 1800 }

/// Prompt templates
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptsConfig {
    pub starting: String,
    pub continuation: String,
    pub review: String,
    pub next_action: String,
}

/// Agent configurations
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentsConfig {
    pub dev: AgentConfig,
    pub review: AgentConfig,
    pub next_action: AgentConfig,
}

/// Agent configuration (simple string or extended table)
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum AgentConfig {
    Simple(String),
    Extended {
        command: String,
        #[serde(default)]
        timeout: Option<u32>,  // Per-agent override
    },
}

impl AgentConfig {
    pub fn command(&self) -> &str {
        match self {
            AgentConfig::Simple(cmd) => cmd,
            AgentConfig::Extended { command, .. } => command,
        }
    }

    pub fn timeout(&self) -> Option<u32> {
        match self {
            AgentConfig::Simple(_) => None,
            AgentConfig::Extended { timeout, .. } => *timeout,
        }
    }
}
```

### Validation Rules

| Rule | Error Message | Spec Reference |
|------|---------------|----------------|
| All agent commands must contain `{{prompt}}` | "Agent command must include {{prompt}}" | FR5, E2 |
| review prompt must contain `{{dev_response}}` or `{{dev_errors}}` | "Review prompt must include {{dev_response}} or {{dev_errors}}" | FR6, E5 |
| next_action prompt must contain `{{dev_response}}` or `{{dev_errors}}` | "next_action prompt must include {{dev_response}} or {{dev_errors}}" | FR6, E5 |
| continuation prompt must contain `{{next_prompt}}` | "Continuation prompt must include {{next_prompt}}" | Implied by D28 |
| Prompt templates cannot be empty | "Prompt template cannot be empty" | E3 |
| max_retries must be >= 0 | "max_retries cannot be negative" | FR35, E26 |
| Unknown keys rejected | "Unknown configuration key: {key}" | FR50, E29 |

## State Machine

### Loop State

```rust
/// Current state of the orchestration loop
pub struct LoopState {
    pub iteration_count: u32,   // 1-indexed, incremented after each dev run
    pub retry_count: u32,       // Consecutive failures, reset on CONTINUE
    pub next_prompt: String,    // Output from next-action agent, empty for iter 1
}

impl LoopState {
    pub fn new() -> Self {
        Self {
            iteration_count: 1,
            retry_count: 0,
            next_prompt: String::new(),
        }
    }

    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }

    pub fn reset_retry(&mut self) {
        self.retry_count = 0;
    }

    pub fn next_iteration(&mut self, next_prompt: String) {
        self.iteration_count += 1;
        self.retry_count = 0;
        self.next_prompt = next_prompt;
    }
}
```

### Agent Execution Result

```rust
/// Result of running an agent command
pub struct AgentResult {
    pub stdout: String,         // Captured stdout (UTF-8, invalid bytes replaced)
    pub stderr: String,         // Captured stderr (UTF-8, invalid bytes replaced)
    pub exit_code: Option<i32>, // None if killed by signal
    pub success: bool,          // exit_code == Some(0)
}
```

### Review Result

```rust
/// Parsed RESULT from review agent
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewResult {
    Continue,  // Work complete, proceed to next task
    Repeat,    // Work incomplete, retry current task
    Done,      // All work complete, exit loop
}

impl ReviewResult {
    /// Parse RESULT from review output
    /// Returns (result, was_explicit) - was_explicit is false if defaulted to Repeat
    pub fn parse(output: &str) -> (Self, bool) {
        // Pattern: (?i)^RESULT:\s*(CONTINUE|REPEAT|DONE)\s*$
        // Use last match if multiple
        // ...
    }
}
```

### Retry Loop Boundary

The retry loop operates within a single iteration. When `retry_count > max_retries`:

1. The loop exits immediately with code 2
2. No more agents are executed
3. The current iteration is not completed

**Boundary Conditions**:
- `max_retries = 0`: First failure (non-zero exit OR RESULT: REPEAT) triggers exit code 2
- `max_retries = 3`: Up to 3 retries allowed (total 4 attempts), 4th failure triggers exit
- RESULT: CONTINUE resets retry_count to 0, starting fresh for the next iteration
- Both dev agent failure (non-zero exit) and RESULT: REPEAT increment retry_count

**Check Timing**: Retry limit is checked immediately after incrementing `retry_count`, before any further processing.

### State Transitions

```
                                    ┌─────────────────────────┐
                                    │       START             │
                                    │ iteration=1, retry=0    │
                                    └───────────┬─────────────┘
                                                │
                    ┌───────────────────────────▼───────────────────────────┐
                    │                    RUN DEV AGENT                       │
                    │   Use: starting (iter 1) or continuation (iter 2+)    │
                    └───────────────────────────┬───────────────────────────┘
                                                │
                         ┌──────────────────────┼──────────────────────┐
                         │                      │                      │
                    ┌────▼────┐           ┌─────▼─────┐          ┌─────▼─────┐
                    │ Success │           │ Non-Zero  │          │  Timeout  │
                    │ exit=0  │           │ exit!=0   │          │  killed   │
                    └────┬────┘           └─────┬─────┘          └─────┬─────┘
                         │                      │                      │
                         │                      └───────┬──────────────┘
                         │                              │
                         │                        retry_count++
                         │                              │
                         └──────────────┬───────────────┘
                                        │
                    ┌───────────────────▼───────────────────┐
                    │           RUN REVIEW AGENT             │
                    │   Input: {{dev_response}}, {{dev_errors}}  │
                    └───────────────────┬───────────────────┘
                                        │
                    ┌───────────────────┼───────────────────┐
                    │                   │                   │
               ┌────▼────┐         ┌────▼────┐         ┌────▼────┐
               │ CONTINUE│         │ REPEAT  │         │  DONE   │
               └────┬────┘         └────┬────┘         └────┬────┘
                    │                   │                   │
                    │                   │              ┌────▼────┐
                    │                   │              │  EXIT   │
                    │                   │              │ code=0  │
                    │                   │              └─────────┘
                    │                   │
                    ▼                   ▼
          ┌─────────────────┐   ┌─────────────────┐
          │ RUN NEXT-ACTION │   │ retry_count++   │
          │  → next_prompt  │   └────────┬────────┘
          └────────┬────────┘            │
                   │                     │
           retry_count=0                 │
           iteration_count++             │
                   │                     │
                   ▼                     ▼
          ┌─────────────────────────────────────────┐
          │         CHECK LIMITS                     │
          │ retry > max_retries? → EXIT code=2       │
          │ iter > max_iterations? → EXIT code=0     │
          └──────────────────┬──────────────────────┘
                             │
                             │ Continue loop
                             │
                             └──────────► RUN DEV AGENT
```

## Variable Context

### Available Variables by Template

| Template | Available Variables |
|----------|---------------------|
| starting | `{{iteration_count}}`, `{{retry_count}}` |
| continuation | `{{iteration_count}}`, `{{retry_count}}`, `{{next_prompt}}` |
| review | `{{iteration_count}}`, `{{retry_count}}`, `{{dev_response}}`, `{{dev_errors}}` |
| next_action | `{{iteration_count}}`, `{{retry_count}}`, `{{dev_response}}`, `{{dev_errors}}`, `{{review_response}}`, `{{review_errors}}` |
| agent commands | `{{prompt}}` (the fully-rendered prompt) |

### Variable Interpolation Context

```rust
use std::collections::HashMap;

/// Context for variable interpolation
pub struct TemplateContext {
    vars: HashMap<String, String>,
}

impl TemplateContext {
    pub fn new() -> Self {
        Self { vars: HashMap::new() }
    }

    pub fn set(&mut self, name: &str, value: String) {
        self.vars.insert(name.to_string(), value);
    }

    /// Render template with variable substitution
    /// Returns Err if undefined variable found
    pub fn render(&self, template: &str) -> Result<String, UndefinedVariableError> {
        // Single-pass substitution
        // Pattern: \{\{([a-z_]+)\}\}
        // ...
    }
}

pub struct UndefinedVariableError {
    pub variable: String,
    pub template_excerpt: String,
}
```

## Exit Codes

| Code | Meaning | Trigger |
|------|---------|---------|
| 0 | Success | `RESULT: DONE` or `max_iterations` reached |
| 1 | Error | Config error, command not found, agent failure (review/next-action) |
| 2 | Max retries | `retry_count > max_retries` |
| 130 | Interrupted | SIGINT (Ctrl+C) |
| 143 | Terminated | SIGTERM |
