# Feature Specification: ralph-loop

**Feature Branch**: `feature/ralph-loop`
**Created**: 2026-02-02
**Last Updated**: 2026-02-02
**Status**: Complete
**Discovery**: See `discovery/` folder for full context

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

<!--
  IMPORTANT: Story Numbering Note
  ================================
  This discovery spec uses ORIGINAL story numbers (Story 6, 1, 2, 3, 5, 4).
  The main implementation spec (specs/001-ralph-loop/spec.md) uses REORDERED
  numbers aligned with priority (Story 1=P1, Story 2=P2, etc.).

  Mapping by content (note: priorities P5/P6 were swapped in main spec):
    Discovery Story 6 [P1] (Config)      → Main Spec Story 1 (P1) - US1 in tasks.md
    Discovery Story 1 [P2] (Core Loop)   → Main Spec Story 2 (P2) - US2 in tasks.md
    Discovery Story 2 [P3] (Review)      → Main Spec Story 3 (P3) - US3 in tasks.md
    Discovery Story 3 [P4] (Parsing)     → Main Spec Story 4 (P4) - US4 in tasks.md
    Discovery Story 4 [P6] (Next-Action) → Main Spec Story 5 (P5) - US5 in tasks.md
    Discovery Story 5 [P5] (Retry)       → Main Spec Story 6 (P6) - US6 in tasks.md

  For implementation, use the main spec's Story 1-6 numbering (matches US1-US6 in tasks.md).
  The main spec is authoritative for implementation order.
-->

<!--
  Stories are ordered by priority (P1 first).
  Each story is independently testable and delivers standalone value.
  Stories may be revised if later discovery reveals gaps - see REVISIONS.md
-->

### Story 6: TOML Configuration [P1] ✅

**User Story**: As a developer, I want to configure prompt templates, agent settings, and retry limits via a TOML file, so that I can customize the workflow for my needs.

**Key Decisions**:
- D5: TOML format (consistent with Rust ecosystem)
- D8: Config from `./ralph.toml` or `--config` CLI flag
- D9: API credentials via environment variables only
- D10: Agents configured as shell commands with variable substitution
- D11: Development agent is configurable (not hardcoded)
- D12: No automatic response truncation
- D13: Separate stdout/stderr as `{{dev_response}}` and `{{dev_errors}}`
- D15: Configurable timeout (default 1800 seconds)
- D17: Configurable max_iterations (default 0 = unlimited)
- D18: Capture `{{review_errors}}` for consistency
- D19: Per-agent timeout configuration
- D26: Next-action output replaces `{{next_prompt}}` variable
- D28: Two dev prompt templates: `starting` (iteration 1) and `continuation` (iteration 2+)

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

#### Acceptance Scenarios

| ID | Scenario | Given | When | Then |
|----|----------|-------|------|------|
| 6.1 | Load from cwd | `ralph.toml` exists in cwd | Run `ralph` | Loads config from `./ralph.toml` |
| 6.2 | Load from CLI flag | Config at `/path/to/config.toml` | Run `ralph --config /path/to/config.toml` | Loads config from specified path |
| 6.3 | Missing config | No `ralph.toml` in cwd, no `--config` | Run `ralph` | Exit with error: "No config file found" |
| 6.4 | Invalid TOML | `ralph.toml` has syntax error | Run `ralph` | Exit with error showing parse error location |
| 6.5 | Missing required fields | Config missing `[agents]` section | Run `ralph` | Exit with error listing missing fields |
| 6.6 | Prompt variable interpolation | Prompt contains `{{dev_response}}` | Review prompt is rendered | Variable replaced with actual dev output |
| 6.7 | Command variable interpolation | Command is `claude -p '{{prompt}}'` | Agent is invoked | `{{prompt}}` replaced with rendered prompt |
| 6.8 | Default timeout | Config omits `timeout` | Config loaded | Timeout defaults to 1800 seconds |
| 6.9 | Default max_iterations | Config omits `max_iterations` | Config loaded | max_iterations defaults to 0 (unlimited) |
| 6.10 | Per-agent timeout | Agent has `timeout = 600` | Agent runs | Uses 600s timeout, not global |
| 6.11 | Simple vs extended agent format | Agent is string or table | Config parsed | Both formats supported |
| 6.12 | Continuation template required | Config has `[prompts]` | Validation | `continuation` prompt required |
| 6.13 | Starting vs continuation | Iteration 1 vs 2+ | Dev prompt rendered | Correct template selected |

#### Edge Cases

| ID | Scenario | Expected Behavior |
|----|----------|-------------------|
| 6.E1 | Undefined variable in prompt | Exit with error listing undefined variable |
| 6.E2 | Agent command missing `{{prompt}}` | Exit with error: "Agent command must include {{prompt}}" |
| 6.E3 | Empty prompt template | Exit with error: "Prompt template cannot be empty" |
| 6.E4 | Very long agent response | Pass through full response (no truncation) |
| 6.E5 | Review/next_action prompt missing both `{{dev_response}}` and `{{dev_errors}}` | Exit with error: "Prompt must include {{dev_response}} or {{dev_errors}}" |

---

### Story 1: Core Loop Execution [P2] ✅

**User Story**: As a developer, I want the orchestrator to run my dev agent command and capture its output, so that the output can be passed to the review agent.

**Key Decisions**:
- D13: Separate stdout (`{{dev_response}}`) and stderr (`{{dev_errors}}`)
- D14: Non-zero exit code = automatic failure, increment retry, still proceed to review
- D15: Configurable timeout, 30 minute default
- D16: Silent capture, show summary after completion
- D17: Loop terminates on `RESULT: DONE` or max_iterations reached

#### Loop Structure

```
┌─────────────────────────────────────────────────────────────────────┐
│  1. Load config                                                     │
│  2. Render starting prompt (iteration_count=1, retry_count=0)       │
│  3. Run dev agent → capture dev_response, dev_errors                │
│     - If non-zero exit: increment retry_count                       │
│     - If timeout: kill process, treat as failure                    │
│  4. Display summary (iteration, exit code, output lengths)          │
│  5. [Story 2] Run review agent                                      │
│  6. [Story 3] Parse RESULT and route                                │
│  7. [Story 5] Check retry limits                                    │
│  8. [Story 4] Generate next prompt if continuing                    │
│  9. Repeat from step 3                                              │
└─────────────────────────────────────────────────────────────────────┘
```

#### Acceptance Scenarios

| ID | Scenario | Given | When | Then |
|----|----------|-------|------|------|
| 1.1 | Run dev agent | Valid config with dev command | Orchestrator starts iteration | Dev command executed with rendered prompt |
| 1.2 | Capture stdout | Dev agent writes to stdout | Command completes | Output available as `{{dev_response}}` |
| 1.3 | Capture stderr | Dev agent writes to stderr | Command completes | Output available as `{{dev_errors}}` |
| 1.4 | Track iteration count | Loop running | Each iteration starts | `{{iteration_count}}` increments (1-indexed) |
| 1.5 | Non-zero exit | Dev agent exits with code 1 | Command completes | Retry count incremented, proceed to review |
| 1.6 | Timeout triggers | Dev agent runs > configured timeout | Timeout reached | Process killed, treated as failure, proceed to review |
| 1.7 | Show summary | Agent completes | Output captured | Summary displayed: iteration #, exit code, output lengths |
| 1.8 | Max iterations reached | `max_iterations = 10`, iteration 10 completes | After routing | Exit with success: "Max iterations reached" |

#### Edge Cases

| ID | Scenario | Expected Behavior |
|----|----------|-------------------|
| 1.E1 | Dev command not found | Exit with error: "Command not found: [command]" |
| 1.E2 | Dev command permission denied | Exit with error showing permission error |
| 1.E3 | Empty stdout (valid) | `{{dev_response}}` = empty string, proceed normally |
| 1.E4 | Empty stderr (valid) | `{{dev_errors}}` = empty string, proceed normally |
| 1.E5 | Binary/non-UTF8 output | UTF-8 decode, replace invalid bytes with replacement char |
| 1.E6 | Agent killed by signal | Treat as failure, capture partial output, proceed to review |
| 1.E7 | Timeout = 0 in config | No timeout (wait indefinitely) |

---

### Story 2: Review Agent Integration [P3] ✅

**User Story**: As a developer, I want the orchestrator to run a review agent that evaluates the dev agent's output, so that work completion can be verified automatically.

**Key Decisions**:
- D18: Capture `{{review_errors}}` (stderr) for consistency with dev agent
- D19: Per-agent timeout configuration
- D20: Review agent failure exits immediately (critical infrastructure)

#### Acceptance Scenarios

| ID | Scenario | Given | When | Then |
|----|----------|-------|------|------|
| 2.1 | Run review agent | Dev agent completed | After dev summary | Review command executed with rendered review prompt |
| 2.2 | Prompt has dev output | Review prompt contains `{{dev_response}}` | Prompt rendered | Variable replaced with dev stdout |
| 2.3 | Capture review stdout | Review agent writes to stdout | Command completes | Output available as `{{review_response}}` |
| 2.4 | Capture review stderr | Review agent writes to stderr | Command completes | Output available as `{{review_errors}}` |
| 2.5 | Show summary | Review agent completes | Output captured | Summary displayed: exit code, output length |
| 2.6 | Per-agent timeout | Review config has `timeout = 600` | Review runs > 600s | Process killed at 600s, not global timeout |
| 2.7 | Default timeout fallback | Review has no timeout override | Review runs | Uses global timeout from `[general]` |

#### Edge Cases

| ID | Scenario | Expected Behavior |
|----|----------|-------------------|
| 2.E1 | Review command not found | Exit with error: "Review command not found: [command]" |
| 2.E2 | Review agent non-zero exit | Exit with error: "Review agent failed with exit code [N]" |
| 2.E3 | Review agent timeout | Exit with error: "Review agent timed out after [N] seconds" |
| 2.E4 | Review output missing RESULT | Handled by Story 3 (parsing) |
| 2.E5 | Empty review output | Proceed to Story 3 parsing (will fail there) |

---

### Story 3: Result Parsing & Routing [P4] ✅

**User Story**: As a developer, I want the orchestrator to parse the review agent's RESULT signal and route accordingly, so that the loop automatically continues, repeats, or exits based on the review.

**Key Decisions**:
- D21: Case-insensitive parsing (`RESULT: DONE`, `result: done` both valid)
- D22: Missing RESULT treated as REPEAT (conservative, with warning)
- D23: Multiple RESULT lines → use last occurrence

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
| `CONTINUE` | Reset retry_count to 0, run next-action agent (Story 4), continue loop |
| `REPEAT` | Increment retry_count, check max_retries (Story 5), re-run dev agent |
| `DONE` | Exit loop with success code 0 |
| (missing) | Log warning, treat as REPEAT |

#### Acceptance Scenarios

| ID | Scenario | Given | When | Then |
|----|----------|-------|------|------|
| 3.1 | Parse CONTINUE | Review contains `RESULT: CONTINUE` | Parsing | Route to next-action, reset retry_count |
| 3.2 | Parse REPEAT | Review contains `RESULT: REPEAT` | Parsing | Increment retry_count, re-run dev |
| 3.3 | Parse DONE | Review contains `RESULT: DONE` | Parsing | Exit with success (code 0) |
| 3.4 | Case insensitive | Review contains `result: done` | Parsing | Recognized as DONE |
| 3.5 | RESULT mid-output | `RESULT: DONE` after explanation | Parsing | Still detected |
| 3.6 | Multiple RESULT | `RESULT: REPEAT` then `RESULT: CONTINUE` | Parsing | Uses CONTINUE (last) |
| 3.7 | Missing RESULT | No RESULT line in output | Parsing | Log warning, treat as REPEAT |
| 3.8 | Whitespace tolerance | `RESULT:   DONE  ` (extra spaces) | Parsing | Still recognized |

#### Edge Cases

| ID | Scenario | Expected Behavior |
|----|----------|-------------------|
| 3.E1 | Invalid result value | `RESULT: MAYBE` — log warning, treat as REPEAT |
| 3.E2 | RESULT in middle of word | `THERESULT: DONE` — not matched, treat as REPEAT |
| 3.E3 | RESULT with extra text | `RESULT: DONE now` — not matched, treat as REPEAT |
| 3.E4 | Empty review_response | Treat as REPEAT with warning |

---

### Story 5: Retry Management [P5] ✅

**User Story**: As a developer, I want the orchestrator to track consecutive failures and exit when max retries is exceeded, so that I don't get stuck in infinite retry loops.

**Key Decisions**:
- D4: Exit with error when max retries exceeded
- D14: Dev agent non-zero exit increments retry_count
- D24: Exit code 2 specifically for max retries exceeded
- D25: Negative max_retries is config validation error

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

#### Acceptance Scenarios

| ID | Scenario | Given | When | Then |
|----|----------|-------|------|------|
| 5.1 | Track retry count | Dev agent fails (non-zero) | After failure | `retry_count` incremented |
| 5.2 | REPEAT increments | RESULT: REPEAT parsed | After routing | `retry_count` incremented |
| 5.3 | CONTINUE resets | RESULT: CONTINUE parsed | After routing | `retry_count` reset to 0 |
| 5.4 | Max not exceeded | `retry_count = 2`, `max_retries = 3` | After increment | Loop continues |
| 5.5 | Max exceeded | `retry_count = 3`, `max_retries = 3` | After increment to 4 | Exit with code 2 |
| 5.6 | Error message | Max retries exceeded | Exit | Message: "Max retries (3) exceeded" |
| 5.7 | Zero max_retries | `max_retries = 0` | First failure | Exit immediately with code 2 |

#### Edge Cases

| ID | Scenario | Expected Behavior |
|----|----------|-------------------|
| 5.E1 | max_retries = 0 | First failure exits immediately with code 2 |
| 5.E2 | Negative max_retries | Config validation error at startup |

---

### Story 4: Next-Action Prompt Generation [P6] ✅

**User Story**: As a developer, I want the orchestrator to generate the next prompt using a configurable agent, so that each iteration has context-aware instructions based on previous results.

**Key Decisions**:
- D26: Next-action output replaces `{{next_prompt}}` variable in continuation template
- D27: Next-action agent failure exits immediately (critical infrastructure)
- D28: Two dev prompt templates: `starting` (iteration 1) and `continuation` (iteration 2+)

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

#### Acceptance Scenarios

| ID | Scenario | Given | When | Then |
|----|----------|-------|------|------|
| 4.1 | Run next-action agent | RESULT: CONTINUE | After routing | Next-action command executed |
| 4.2 | Next-action prompt rendered | Next-action template | Rendering | All variables substituted |
| 4.3 | Output captured | Next-action outputs text | Agent completes | Output stored as `{{next_prompt}}` |
| 4.4 | Continuation template used | Iteration 2+ | Dev prompt rendering | `continuation` template used |
| 4.5 | next_prompt substituted | Continuation has `{{next_prompt}}` | Rendering | Replaced with next-action output |
| 4.6 | Iteration increments | After next-action completes | Next iteration | `iteration_count` incremented |
| 4.7 | Per-agent timeout | Next-action has timeout override | Agent runs too long | Killed at agent timeout |

#### Edge Cases

| ID | Scenario | Expected Behavior |
|----|----------|-------------------|
| 4.E1 | Next-action command not found | Exit with error |
| 4.E2 | Next-action non-zero exit | Exit with error (critical) |
| 4.E3 | Next-action timeout | Exit with error (critical) |
| 4.E4 | Empty next-action output | `{{next_prompt}}` is empty string, proceed |
| 4.E5 | Missing continuation template | Config validation error |

---

## Edge Cases

| ID | Scenario | Handling | Stories Affected |
|----|----------|----------|------------------|
| 6.E1 | Undefined variable in prompt | Exit with error listing undefined variable | Story 6 |
| 6.E2 | Agent command missing `{{prompt}}` | Exit with error | Story 6 |
| 6.E3 | Empty prompt template | Exit with error | Story 6 |
| 6.E4 | Very long agent response | Pass through (no truncation) | Story 6 |
| 6.E5 | Review/next_action missing dev output vars | Exit with error | Story 6 |
| 1.E1 | Dev command not found | Exit with error | Story 1 |
| 1.E2 | Dev command permission denied | Exit with error | Story 1 |
| 1.E3 | Empty stdout | Proceed normally | Story 1 |
| 1.E4 | Empty stderr | Proceed normally | Story 1 |
| 1.E5 | Binary/non-UTF8 output | Replace invalid bytes | Story 1 |
| 1.E6 | Agent killed by signal | Treat as failure | Story 1 |
| 1.E7 | Timeout = 0 | No timeout | Story 1 |
| 2.E1 | Review command not found | Exit with error | Story 2 |
| 2.E2 | Review agent non-zero exit | Exit with error (critical) | Story 2 |
| 2.E3 | Review agent timeout | Exit with error (critical) | Story 2 |
| 2.E4 | Review output missing RESULT | Handled by Story 3 | Story 2, 3 |
| 2.E5 | Empty review output | Proceed to parsing | Story 2, 3 |
| 3.E1 | Invalid result value | Log warning, treat as REPEAT | Story 3 |
| 3.E2 | RESULT in middle of word | Not matched, treat as REPEAT | Story 3 |
| 3.E3 | RESULT with extra text | Not matched, treat as REPEAT | Story 3 |
| 3.E4 | Empty review_response | Treat as REPEAT with warning | Story 3 |
| 5.E1 | max_retries = 0 | First failure exits with code 2 | Story 5 |
| 5.E2 | Negative max_retries | Config validation error | Story 5, 6 |
| 4.E1 | Next-action command not found | Exit with error | Story 4 |
| 4.E2 | Next-action non-zero exit | Exit with error (critical) | Story 4 |
| 4.E3 | Next-action timeout | Exit with error (critical) | Story 4 |
| 4.E4 | Empty next-action output | Proceed with empty string | Story 4 |
| 4.E5 | Missing continuation template | Config validation error | Story 4, 6 |

---

## Requirements

### Functional Requirements

| ID | Requirement | Stories | Confidence |
|----|-------------|---------|------------|
| FR1 | Parse TOML config from `./ralph.toml` or `--config` path | Story 6 | 100% |
| FR2 | Validate required sections: `[general]`, `[prompts]`, `[agents]` | Story 6 | 100% |
| FR3 | Validate required prompts: `starting`, `review`, `next_action` | Story 6 | 100% |
| FR4 | Validate required agents: `dev`, `review`, `next_action` | Story 6 | 100% |
| FR5 | Validate all agent commands contain `{{prompt}}` | Story 6 | 100% |
| FR6 | Validate review/next_action prompts contain `{{dev_response}}` or `{{dev_errors}}` | Story 6 | 100% |
| FR7 | Interpolate variables into prompt templates at runtime | Story 6 | 100% |
| FR8 | Interpolate rendered prompt into agent commands | Story 6 | 100% |
| FR9 | Exit with descriptive error on validation failure | Story 6 | 100% |
| FR10 | Support both simple string and extended table format for agents | Story 6 | 100% |
| FR11 | Execute dev agent command via shell | Story 1 | 100% |
| FR12 | Capture stdout separately as `dev_response` | Story 1 | 100% |
| FR13 | Capture stderr separately as `dev_errors` | Story 1 | 100% |
| FR14 | Track iteration count (1-indexed) | Story 1 | 100% |
| FR15 | Increment retry count on non-zero exit | Story 1 | 100% |
| FR16 | Kill process and treat as failure on timeout | Story 1 | 100% |
| FR17 | Display summary after each agent completes | Story 1 | 100% |
| FR18 | Exit successfully when max_iterations reached | Story 1 | 100% |
| FR19 | Execute review agent after dev agent completes | Story 2 | 100% |
| FR20 | Capture review stdout as `review_response` | Story 2 | 100% |
| FR21 | Capture review stderr as `review_errors` | Story 2 | 100% |
| FR22 | Use per-agent timeout if specified, else global default | Story 2 | 100% |
| FR23 | Exit immediately with error if review agent fails | Story 2 | 100% |
| FR24 | Parse RESULT signal from review_response (case-insensitive) | Story 3 | 100% |
| FR25 | Support RESULT values: CONTINUE, REPEAT, DONE | Story 3 | 100% |
| FR26 | Use last RESULT if multiple found | Story 3 | 100% |
| FR27 | Treat missing/invalid RESULT as REPEAT with warning | Story 3 | 100% |
| FR28 | Route CONTINUE to next-action, reset retry_count | Story 3 | 100% |
| FR29 | Route REPEAT to dev agent, increment retry_count | Story 3 | 100% |
| FR30 | Route DONE to successful exit (code 0) | Story 3 | 100% |
| FR31 | Track retry_count (consecutive failures) | Story 5 | 100% |
| FR32 | Increment retry_count on dev failure or REPEAT | Story 5 | 100% |
| FR33 | Reset retry_count to 0 on CONTINUE | Story 5 | 100% |
| FR34 | Exit with code 2 when retry_count > max_retries | Story 5 | 100% |
| FR35 | Validate max_retries >= 0 at config load | Story 5, 6 | 100% |
| FR36 | Execute next-action agent after RESULT: CONTINUE | Story 4 | 100% |
| FR37 | Capture next-action stdout as `{{next_prompt}}` | Story 4 | 100% |
| FR38 | Use `starting` template for iteration 1 | Story 4 | 100% |
| FR39 | Use `continuation` template for iteration 2+ | Story 4 | 100% |
| FR40 | Validate `continuation` template exists | Story 4, 6 | 100% |
| FR41 | Exit immediately with error if next-action agent fails | Story 4 | 100% |

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

| ID | Criterion | Measurement | Stories |
|----|-----------|-------------|---------|
| SC1 | Config loads successfully | `ralph` starts without error when valid config exists | Story 6 |
| SC2 | Config validation catches errors | All 5 edge cases produce specific error messages | Story 6 |
| SC3 | Variables interpolate correctly | All 8 variables substitute correctly in templates | Story 6 |
| SC4 | Dev agent executes | Command runs and captures output | Story 1 |
| SC5 | Stdout/stderr separated | `{{dev_response}}` and `{{dev_errors}}` contain correct streams | Story 1 |
| SC6 | Failures increment retry | Non-zero exit increments `{{retry_count}}` | Story 1 |
| SC7 | Timeout enforced | Long-running commands killed at timeout | Story 1 |
| SC8 | Review agent executes | Review command runs after dev completes | Story 2 |
| SC9 | Review output captured | `{{review_response}}` and `{{review_errors}}` available | Story 2 |
| SC10 | Review failure is fatal | Non-zero exit or timeout exits immediately | Story 2 |
| SC11 | RESULT parsed correctly | All 3 result types route to correct action | Story 3 |
| SC12 | Case insensitivity works | `result: done` recognized same as `RESULT: DONE` | Story 3 |
| SC13 | Missing RESULT handled | Warning logged, treated as REPEAT | Story 3 |
| SC14 | Retry count tracked | Consecutive failures increment `retry_count` | Story 5 |
| SC15 | Max retries enforced | Exit code 2 when `retry_count > max_retries` | Story 5 |
| SC16 | CONTINUE resets retries | `retry_count` returns to 0 on success | Story 5 |
| SC17 | Next-action generates prompt | Output captured as `{{next_prompt}}` | Story 4 |
| SC18 | Correct template selected | `starting` for iter 1, `continuation` for 2+ | Story 4 |
| SC19 | Next-action failure is fatal | Non-zero exit or timeout exits immediately | Story 4 |

---

## Appendix: Story Revision History

*Major revisions to graduated stories. Full details in `archive/REVISIONS.md`*

| Date | Story | Change | Reason |
|------|-------|--------|--------|
| 2026-02-02 | Story 6 | Added `{{dev_errors}}` variable | D13: Separate stdout/stderr capture |
| 2026-02-02 | Story 6 | Added `timeout` config (default 1800s) | D15: Configurable agent timeout |
| 2026-02-02 | Story 6 | Added `max_iterations` config (default 0) | D17: Loop termination safety limit |
| 2026-02-02 | Story 6 | Added validation for dev output vars in prompts | D13: Must include dev_response or dev_errors |
| 2026-02-02 | Story 6 | Added `{{review_errors}}` variable | D18: Capture review stderr for consistency |
| 2026-02-02 | Story 6 | Added per-agent timeout support | D19: Agents can override global timeout |
| 2026-02-02 | Story 6 | Added `continuation` prompt template | D28: Two templates for dev prompt |
| 2026-02-02 | Story 6 | Added `{{next_prompt}}` variable | D26: Next-action output variable |
