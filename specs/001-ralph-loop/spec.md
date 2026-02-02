# Feature Specification: ralph-loop

**Feature Branch**: `001-ralph-loop`
**Created**: 2026-02-02
**Status**: Complete
**Codebase Documentation**: See [.sdd/codebase/](.sdd/codebase/) for technical details
**Discovery Documentation**: See [discovery/](../../discovery/) for full context, decisions, and iteration history

---

## Problem Statement

Developers using AI coding agents (like Claude Code) face a trust and automation gap: running agents in loops ("Ralph loops") works for iterative development, but requires manual intervention to verify work and decide whether to continue or retry. **unpossible-ralph** solves this by adding an intelligent orchestration layer: a Rust application that runs the development agent, uses a configurable secondary agent to verify work, and automatically routes to the next task or retry based on verification results—all with configurable prompts and retry limits.

## Personas

| Persona | Description | Primary Goals |
|---------|-------------|---------------|
| Solo Developer | Developer running automated coding workflows | Hands-off iteration on coding tasks without manual verification between steps |
| Team Lead | Coordinates agent-driven development | Reliable, auditable automated workflows with failure handling |

---

## User Scenarios & Testing

### Story 1: TOML Configuration (Priority: P1)

**User Story**: As a developer, I want to configure prompt templates, agent settings, and retry limits via a TOML file, so that I can customize the workflow for my needs.

**Why this priority**: Configuration is the foundation - nothing else works without it.

**Independent Test**: Can be fully tested by loading a valid/invalid config file and verifying parsing behavior.

**Key Decisions**: D5, D8-D13, D15, D17-D19, D25, D26, D28

#### Configuration Structure

```toml
[general]
max_retries = 3        # consecutive failures before exit error
max_iterations = 0     # total iterations limit (0 = unlimited)
timeout = 1800         # default timeout for all agents (30 min)

[prompts]
# Used for iteration 1 only
starting = """
Your starting prompt template here.
Available: {{iteration_count}}, {{retry_count}}
"""

# Used for iteration 2+ (after RESULT: CONTINUE)
continuation = """
Continue working on the task.

## Instructions from previous iteration
{{next_prompt}}

Iteration: {{iteration_count}}, Retries: {{retry_count}}
"""

review = """
Review the following work output.

## Development Agent Output
{{dev_response}}

## Development Agent Errors
{{dev_errors}}

Respond with:
- RESULT: CONTINUE if work is complete and ready for next task
- RESULT: REPEAT if work needs to be redone
- RESULT: DONE if all work is complete and loop should exit
"""

next_action = """
Generate the next prompt based on:

## Development Agent Output
{{dev_response}}

## Development Agent Errors
{{dev_errors}}

## Review Agent Output
{{review_response}}

## Review Agent Errors
{{review_errors}}

Iteration: {{iteration_count}}, Retries: {{retry_count}}
"""

[agents]
# Simple format (uses global timeout):
dev = "claude -p '{{prompt}}'"

# Extended format (with per-agent timeout):
review = { command = "claude -p '{{prompt}}'", timeout = 600 }

# Simple format for next_action:
next_action = "openai chat -m gpt-4o-mini '{{prompt}}'"
```

#### Template Variables

| Variable | Available In | Description |
|----------|--------------|-------------|
| `{{prompt}}` | Agent commands | The fully-rendered prompt to send |
| `{{dev_response}}` | review, next_action prompts | Stdout from development agent |
| `{{dev_errors}}` | review, next_action prompts | Stderr from development agent |
| `{{review_response}}` | next_action prompt | Stdout from review agent |
| `{{review_errors}}` | next_action prompt | Stderr from review agent |
| `{{next_prompt}}` | continuation prompt | Output from next-action agent |
| `{{iteration_count}}` | All prompts | Current loop iteration (1-indexed) |
| `{{retry_count}}` | All prompts | Consecutive failures on current task |

**Acceptance Scenarios**:

1. **Given** `ralph.toml` exists in cwd, **When** run `ralph`, **Then** loads config from `./ralph.toml`
2. **Given** config at `/path/to/config.toml`, **When** run `ralph --config /path/to/config.toml`, **Then** loads config from specified path
3. **Given** no `ralph.toml` in cwd and no `--config`, **When** run `ralph`, **Then** exit with error: "No config file found"
4. **Given** `ralph.toml` has syntax error, **When** run `ralph`, **Then** exit with error showing parse error location
5. **Given** config missing `[agents]` section, **When** run `ralph`, **Then** exit with error listing missing fields
6. **Given** prompt contains `{{dev_response}}`, **When** review prompt is rendered, **Then** variable replaced with actual dev output
7. **Given** command is `claude -p '{{prompt}}'`, **When** agent is invoked, **Then** `{{prompt}}` replaced with rendered prompt
8. **Given** config omits `timeout`, **When** config loaded, **Then** timeout defaults to 1800 seconds
9. **Given** config omits `max_iterations`, **When** config loaded, **Then** max_iterations defaults to 0 (unlimited)
10. **Given** agent has `timeout = 600`, **When** agent runs, **Then** uses 600s timeout, not global
11. **Given** agent is string or table, **When** config parsed, **Then** both formats supported
12. **Given** config has `[prompts]`, **When** validation, **Then** `continuation` prompt required
13. **Given** iteration 1 vs 2+, **When** dev prompt rendered, **Then** correct template selected

---

### Story 2: Core Loop Execution (Priority: P2)

**User Story**: As a developer, I want the orchestrator to run my dev agent command and capture its output, so that the output can be passed to the review agent.

**Why this priority**: The core loop is the main functionality after configuration.

**Independent Test**: Can be tested by running a simple echo command and verifying output capture.

**Key Decisions**: D13-D17

#### Loop Structure

```
┌─────────────────────────────────────────────────────────────────────┐
│  1. Load config                                                     │
│  2. Render starting prompt (iteration_count=1, retry_count=0)       │
│  3. Run dev agent → capture dev_response, dev_errors                │
│     - If non-zero exit: increment retry_count                       │
│     - If timeout: kill process, treat as failure                    │
│  4. Display summary (iteration, exit code, output lengths)          │
│  5. [Story 3] Run review agent                                      │
│  6. [Story 4] Parse RESULT and route                                │
│  7. [Story 6] Check retry limits                                    │
│  8. [Story 5] Generate next prompt if continuing                    │
│  9. Repeat from step 3                                              │
└─────────────────────────────────────────────────────────────────────┘
```

**Acceptance Scenarios**:

1. **Given** valid config with dev command, **When** orchestrator starts iteration, **Then** dev command executed with rendered prompt
2. **Given** dev agent writes to stdout, **When** command completes, **Then** output available as `{{dev_response}}`
3. **Given** dev agent writes to stderr, **When** command completes, **Then** output available as `{{dev_errors}}`
4. **Given** loop running, **When** each iteration starts, **Then** `{{iteration_count}}` increments (1-indexed)
5. **Given** dev agent exits with code 1, **When** command completes, **Then** retry count incremented, proceed to review
6. **Given** dev agent runs > configured timeout, **When** timeout reached, **Then** process killed, treated as failure, proceed to review
7. **Given** agent completes, **When** output captured, **Then** summary displayed: iteration #, exit code, output lengths
8. **Given** `max_iterations = 10` and iteration 10 completes, **When** after routing, **Then** exit with success: "Max iterations reached"

---

### Story 3: Review Agent Integration (Priority: P3)

**User Story**: As a developer, I want the orchestrator to run a review agent that evaluates the dev agent's output, so that work completion can be verified automatically.

**Why this priority**: Review is the verification step that enables automation.

**Independent Test**: Can be tested by providing mock dev output and verifying review agent receives it.

**Key Decisions**: D18-D20

**Acceptance Scenarios**:

1. **Given** dev agent completed, **When** after dev summary, **Then** review command executed with rendered review prompt
2. **Given** review prompt contains `{{dev_response}}`, **When** prompt rendered, **Then** variable replaced with dev stdout
3. **Given** review agent writes to stdout, **When** command completes, **Then** output available as `{{review_response}}`
4. **Given** review agent writes to stderr, **When** command completes, **Then** output available as `{{review_errors}}`
5. **Given** review agent completes, **When** output captured, **Then** summary displayed: exit code, output length
6. **Given** review config has `timeout = 600`, **When** review runs > 600s, **Then** process killed at 600s, not global timeout
7. **Given** review has no timeout override, **When** review runs, **Then** uses global timeout from `[general]`

---

### Story 4: Result Parsing & Routing (Priority: P4)

**User Story**: As a developer, I want the orchestrator to parse the review agent's RESULT signal and route accordingly, so that the loop automatically continues, repeats, or exits based on the review.

**Why this priority**: Routing logic determines loop behavior.

**Independent Test**: Can be tested by providing various RESULT strings and verifying routing decisions.

**Key Decisions**: D21-D23

#### Parsing Strategy

```
Pattern: (?i)^RESULT:\s*(CONTINUE|REPEAT|DONE)\s*$

- Case-insensitive match
- Line starts with "RESULT:" (any case)
- Followed by optional whitespace
- Then one of: CONTINUE, REPEAT, DONE (any case)
- Optional trailing whitespace, end of line
- If multiple matches: use last
- If no match: treat as REPEAT, log warning
```

#### Routing Behavior

| Result | Action |
|--------|--------|
| `CONTINUE` | Reset retry_count to 0, run next-action agent (Story 5), continue loop |
| `REPEAT` | Increment retry_count, check max_retries (Story 6), re-run dev agent |
| `DONE` | Exit loop with success code 0 |
| (missing) | Log warning, treat as REPEAT |

**Acceptance Scenarios**:

1. **Given** review contains `RESULT: CONTINUE`, **When** parsing, **Then** route to next-action, reset retry_count
2. **Given** review contains `RESULT: REPEAT`, **When** parsing, **Then** increment retry_count, re-run dev
3. **Given** review contains `RESULT: DONE`, **When** parsing, **Then** exit with success (code 0)
4. **Given** review contains `result: done`, **When** parsing, **Then** recognized as DONE
5. **Given** `RESULT: DONE` after explanation, **When** parsing, **Then** still detected
6. **Given** `RESULT: REPEAT` then `RESULT: CONTINUE`, **When** parsing, **Then** uses CONTINUE (last)
7. **Given** no RESULT line in output, **When** parsing, **Then** log warning, treat as REPEAT
8. **Given** `RESULT:   DONE  ` (extra spaces), **When** parsing, **Then** still recognized

---

### Story 5: Next-Action Prompt Generation (Priority: P5)

**User Story**: As a developer, I want the orchestrator to generate the next prompt using a configurable agent, so that each iteration has context-aware instructions based on previous results.

**Why this priority**: Next-action enables intelligent multi-step workflows.

**Independent Test**: Can be tested by providing mock outputs and verifying next prompt generation.

**Key Decisions**: D26-D28

#### Flow

```
Iteration 1:
  1. Render `starting` prompt
  2. Run dev agent
  3. Run review agent
  4. Parse RESULT

If RESULT: CONTINUE:
  5. Run next_action agent → output stored as {{next_prompt}}
  6. Reset retry_count to 0
  7. Increment iteration_count

Iteration 2+:
  8. Render `continuation` prompt (with {{next_prompt}})
  9. Run dev agent
  10. Continue loop...
```

**Acceptance Scenarios**:

1. **Given** RESULT: CONTINUE, **When** after routing, **Then** next-action command executed
2. **Given** next-action template, **When** rendering, **Then** all variables substituted
3. **Given** next-action outputs text, **When** agent completes, **Then** output stored as `{{next_prompt}}`
4. **Given** iteration 2+, **When** dev prompt rendering, **Then** `continuation` template used
5. **Given** continuation has `{{next_prompt}}`, **When** rendering, **Then** replaced with next-action output
6. **Given** after next-action completes, **When** next iteration, **Then** `iteration_count` incremented
7. **Given** next-action has timeout override, **When** agent runs too long, **Then** killed at agent timeout

---

### Story 6: Retry Management (Priority: P6)

**User Story**: As a developer, I want the orchestrator to track consecutive failures and exit when max retries is exceeded, so that I don't get stuck in infinite retry loops.

**Why this priority**: Retry management is the safety valve for the loop.

**Independent Test**: Can be tested by simulating failures and verifying retry counting and exit behavior.

**Key Decisions**: D4, D14, D24, D25

#### Behavior

```
retry_count starts at 0

On dev agent non-zero exit:
  retry_count++

On RESULT: REPEAT:
  retry_count++

On RESULT: CONTINUE:
  retry_count = 0

After incrementing retry_count:
  if retry_count > max_retries:
    exit with code 2, message: "Max retries (N) exceeded"
```

**Acceptance Scenarios**:

1. **Given** dev agent fails (non-zero), **When** after failure, **Then** `retry_count` incremented
2. **Given** RESULT: REPEAT parsed, **When** after routing, **Then** `retry_count` incremented
3. **Given** RESULT: CONTINUE parsed, **When** after routing, **Then** `retry_count` reset to 0
4. **Given** `retry_count = 2`, `max_retries = 3`, **When** after increment, **Then** loop continues
5. **Given** `retry_count = 3`, `max_retries = 3`, **When** after increment to 4, **Then** exit with code 2
6. **Given** max retries exceeded, **When** exit, **Then** message: "Max retries (3) exceeded"
7. **Given** `max_retries = 0`, **When** first failure, **Then** exit immediately with code 2

---

## Edge Cases

| ID | Scenario | Expected Behavior | Story |
|----|----------|-------------------|-------|
| E1 | Undefined variable in prompt | Exit with error listing undefined variable | S1 |
| E2 | Agent command missing `{{prompt}}` | Exit with error | S1 |
| E3 | Empty prompt template | Exit with error | S1 |
| E4 | Very long agent response | Pass through (no truncation) | S1 |
| E5 | Review/next_action missing dev output vars | Exit with error | S1 |
| E6 | Dev command not found | Exit with error: "Command not found: [command]" | S2 |
| E7 | Dev command permission denied | Exit with error showing permission error | S2 |
| E8 | Empty stdout | Proceed normally | S2 |
| E9 | Empty stderr | Proceed normally | S2 |
| E10 | Binary/non-UTF8 output | Replace invalid bytes | S2 |
| E11 | Agent killed by signal | Treat as failure | S2 |
| E12 | Timeout = 0 | No timeout limit; process waits until completion or OS/system limit. Validation accepts timeout >= 0. | S2 |
| E13 | Review command not found | Exit with error | S3 |
| E14 | Review agent non-zero exit | Exit with error (critical) | S3 |
| E15 | Review agent timeout | Exit with error (critical) | S3 |
| E16 | Invalid result value (e.g., MAYBE) | Log warning, treat as REPEAT | S4 |
| E17 | RESULT in middle of word | Not matched, treat as REPEAT | S4 |
| E18 | RESULT with extra text | Not matched, treat as REPEAT | S4 |
| E19 | Empty review_response | Treat as REPEAT with warning | S4 |
| E20 | Next-action command not found | Exit with error | S5 |
| E21 | Next-action non-zero exit | Exit with error (critical) | S5 |
| E22 | Next-action timeout | Exit with error (critical) | S5 |
| E23 | Empty next-action output | Proceed with empty string | S5 |
| E24 | Missing continuation template | Config validation error at config load time (before loop starts), not at first CONTINUE | S5, S1 |
| E25 | max_retries = 0 | First failure exits with code 2 | S6 |
| E26 | Negative max_retries | Config validation error | S6 |
| E27 | SIGINT during agent execution | Kill agent, exit with message "Interrupted" | All |
| E28 | Agent spawns child processes | Kill process group, not just parent | S2 |
| E29 | Config has unknown keys | Exit with error listing unrecognized key | S1 |
| E30 | Variable syntax `{{ var }}` with spaces | Not recognized, treated as literal text | S1 |
| E31 | dev_response contains `{{retry_count}}` | Not expanded (single-pass substitution) | S1 |

---

## CLI Interface

### Command Line Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--config <PATH>` | `-c` | Path to TOML configuration file | `./ralph.toml` |
| `--help` | `-h` | Display help information and exit | - |
| `--version` | `-V` | Display version number and exit | - |

### Exit Codes

| Code | Name | Description | Trigger |
|------|------|-------------|---------|
| 0 | Success | Loop completed successfully | `RESULT: DONE` or `max_iterations` reached |
| 1 | Error | Fatal error occurred | Config error, command not found, review/next-action failure |
| 2 | Max Retries | Retry limit exceeded | `retry_count > max_retries` after increment |
| 130 | Interrupted | Process interrupted | SIGINT (Ctrl+C) received |
| 143 | Terminated | Process terminated | SIGTERM received |

**Acceptance Scenarios**:

1. **Given** `ralph --help`, **When** executed, **Then** displays usage information with all options and exits with code 0
2. **Given** `ralph -h`, **When** executed, **Then** behaves same as `--help`
3. **Given** `ralph --version`, **When** executed, **Then** displays version number (e.g., "ralph 0.1.0") and exits with code 0
4. **Given** `ralph -V`, **When** executed, **Then** behaves same as `--version`
5. **Given** `ralph --config /path/to/config.toml`, **When** executed, **Then** uses specified config file
6. **Given** `ralph` with no arguments, **When** `./ralph.toml` exists, **Then** loads default config
7. **Given** `ralph --unknown-flag`, **When** executed, **Then** exits with code 1 and error message

---

## Requirements

### Functional Requirements

| ID | Requirement | Story |
|----|-------------|-------|
| FR1 | System MUST parse TOML config from `./ralph.toml` or `--config` path | S1 |
| FR2 | System MUST validate required sections: `[general]`, `[prompts]`, `[agents]` | S1 |
| FR3 | System MUST validate required prompts: `starting`, `continuation`, `review`, `next_action` | S1 |
| FR4 | System MUST validate required agents: `dev`, `review`, `next_action` | S1 |
| FR5 | System MUST validate all agent commands contain `{{prompt}}` | S1 |
| FR6 | System MUST validate review/next_action prompts contain `{{dev_response}}` or `{{dev_errors}}` | S1 |
| FR7 | System MUST interpolate variables into prompt templates at runtime | S1 |
| FR8 | System MUST interpolate rendered prompt into agent commands | S1 |
| FR9 | System MUST exit with descriptive error on validation failure | S1 |
| FR10 | System MUST support both simple string and extended table format for agents | S1 |
| FR11 | System MUST execute dev agent command via shell | S2 |
| FR12 | System MUST capture stdout separately as `dev_response` | S2 |
| FR13 | System MUST capture stderr separately as `dev_errors` | S2 |
| FR14 | System MUST track iteration count (1-indexed) | S2 |
| FR15 | System MUST increment retry count on non-zero exit | S2 |
| FR16 | System MUST kill process and treat as failure on timeout | S2 |
| FR17 | System MUST display summary after each agent completes | S2 |
| FR18 | System MUST exit successfully when max_iterations reached | S2 |
| FR19 | System MUST execute review agent after dev agent completes | S3 |
| FR20 | System MUST capture review stdout as `review_response` | S3 |
| FR21 | System MUST capture review stderr as `review_errors` | S3 |
| FR22 | System MUST use per-agent timeout if specified, else global default | S3 |
| FR23 | System MUST exit immediately with error if review agent fails | S3 |
| FR24 | System MUST parse RESULT signal from review_response (case-insensitive) | S4 |
| FR25 | System MUST support RESULT values: CONTINUE, REPEAT, DONE | S4 |
| FR26 | System MUST use last RESULT if multiple found | S4 |
| FR27 | System MUST treat missing/invalid RESULT as REPEAT with warning | S4 |
| FR28 | System MUST route CONTINUE to next-action, reset retry_count | S4 |
| FR29 | System MUST route REPEAT to dev agent, increment retry_count | S4 |
| FR30 | System MUST route DONE to successful exit (code 0) | S4 |
| FR31 | System MUST track retry_count (consecutive failures) | S6 |
| FR32 | System MUST increment retry_count on dev failure or REPEAT | S6 |
| FR33 | System MUST reset retry_count to 0 on CONTINUE | S6 |
| FR34 | System MUST exit with code 2 when retry_count > max_retries | S6 |
| FR35 | System MUST validate max_retries >= 0 at config load | S6 |
| FR36 | System MUST execute next-action agent after RESULT: CONTINUE | S5 |
| FR37 | System MUST capture next-action stdout as `{{next_prompt}}` | S5 |
| FR38 | System MUST use `starting` template for iteration 1 | S5 |
| FR39 | System MUST use `continuation` template for iteration 2+ | S5 |
| FR40 | System MUST validate `continuation` template exists | S5 |
| FR41 | System MUST exit immediately with error if next-action agent fails | S5 |
| FR42 | System MUST exit with code 1 for configuration errors and other failures | All |
| FR43 | System MUST kill entire process group on timeout (prevent orphan processes) | S2 |
| FR44 | System MUST execute commands via `/bin/sh` on Unix | S2 |
| FR45 | System MUST exit cleanly on SIGINT/SIGTERM with message indicating interruption | All |
| FR46 | System MUST support `--version` flag showing version number | CLI |
| FR47 | System MUST support `--help` flag showing usage information | CLI |
| FR48 | System MUST replace invalid UTF-8 sequences with U+FFFD (replacement character) | S2 |
| FR49 | System MUST substitute variables exactly once (no recursive expansion) | S1 |
| FR50 | System MUST reject config files containing unknown keys with error | S1 |
| FR51 | System MUST write all errors and warnings to stderr, summaries to stdout | All |

### Key Entities

| Entity | Description |
|--------|-------------|
| Config | TOML configuration containing prompts, agents, and settings |
| Prompt Template | String template with `{{variable}}` placeholders |
| Agent Command | Shell command with `{{prompt}}` placeholder (string or table format) |
| Iteration | One complete cycle: dev → review → route |
| dev_response | Captured stdout from development agent |
| dev_errors | Captured stderr from development agent |
| review_response | Captured stdout from review agent |
| review_errors | Captured stderr from review agent |
| next_prompt | Output from next-action agent, used in continuation template |

---

## Success Criteria

### Measurable Outcomes

| ID | Criterion | Measurement |
|----|-----------|-------------|
| SC1 | Config loads successfully | `ralph` starts without error when valid config exists |
| SC2 | Config validation catches errors | All edge cases produce specific error messages |
| SC3 | Variables interpolate correctly | All 8 template variables (`{{prompt}}`, `{{dev_response}}`, `{{dev_errors}}`, `{{review_response}}`, `{{review_errors}}`, `{{next_prompt}}`, `{{iteration_count}}`, `{{retry_count}}`) substitute to expected values in unit tests |
| SC4 | Dev agent executes | Command runs and captures output |
| SC5 | Stdout/stderr separated | `{{dev_response}}` and `{{dev_errors}}` contain correct streams |
| SC6 | Failures increment retry | Non-zero exit increments `{{retry_count}}` |
| SC7 | Timeout enforced | Long-running commands killed at timeout |
| SC8 | Review agent executes | Review command runs after dev completes |
| SC9 | Review output captured | `{{review_response}}` and `{{review_errors}}` available |
| SC10 | Review failure is fatal | Non-zero exit or timeout exits immediately |
| SC11 | RESULT parsed correctly | All 3 result types route to correct action |
| SC12 | Case insensitivity works | `result: done` recognized same as `RESULT: DONE` |
| SC13 | Missing RESULT handled | Warning logged, treated as REPEAT |
| SC14 | Retry count tracked | Consecutive failures increment `retry_count` |
| SC15 | Max retries enforced | Exit code 2 when `retry_count > max_retries` |
| SC16 | CONTINUE resets retries | `retry_count` returns to 0 on success |
| SC17 | Next-action generates prompt | Output captured as `{{next_prompt}}` |
| SC18 | Correct template selected | `starting` for iter 1, `continuation` for 2+ |
| SC19 | Next-action failure is fatal | Non-zero exit or timeout exits immediately |

---

## Development Standards

### Code Quality

| Requirement | Tool/Method | Enforcement |
|-------------|-------------|-------------|
| Code formatting | rustfmt | Pre-commit hook, CI check |
| Linting | clippy | Pre-commit hook, CI check |
| Type safety | Rust compiler | Build failure on error |

### Pre-commit Hooks

System MUST enforce quality gates before commits:
- Run `cargo fmt --check` - fail if formatting differs
- Run `cargo clippy -- -D warnings` - fail on any warning
- Hooks configured via git hooks or cargo-husky

### Continuous Integration

System MUST have automated CI via GitHub Actions:
- Run on push to main and all pull requests
- Build and test on Linux (primary target)
- Run clippy and rustfmt checks
- Cache cargo dependencies for performance

### Release Management

System MUST support automated releases via cargo-release:
- Semantic versioning (per constitution)
- Automated CHANGELOG updates
- Git tag creation on release
- Crates.io publishing (when ready)

---

## Appendix

### Discovery Documentation

Full discovery context available at:
- `discovery/STATE.md` - Discovery process state and iteration history
- `discovery/OPEN_QUESTIONS.md` - All questions raised and resolved
- `discovery/archive/DECISIONS.md` - 28 key decisions with rationale
- `discovery/archive/ITERATIONS.md` - Discovery iteration log
- `discovery/archive/REVISIONS.md` - Story revision history

### Glossary

- **Ralph Loop**: An infinite shell loop running an AI coding agent, relying on eventual consistency through iteration
- **Development Agent**: The primary agent doing the work (configurable command)
- **Review Agent**: The secondary agent that verifies completion (configurable command)
- **Next-Action Agent**: The agent that generates the next iteration's prompt (configurable command)
- **Orchestrator**: The Rust application managing the loop, verification, and routing
- **RESULT signal**: The `RESULT: CONTINUE|REPEAT|DONE` line in review output that determines routing
