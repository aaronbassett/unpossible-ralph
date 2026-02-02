# Discovery State: ralph-loop

**Updated**: 2026-02-02 14:15 UTC
**Iteration**: 9
**Phase**: ✅ COMPLETE

---

## Problem Understanding

### Problem Statement
Developers using AI coding agents (like Claude Code) face a trust and automation gap: running agents in loops ("Ralph loops") works for iterative development, but requires manual intervention to verify work and decide whether to continue or retry. unpossible-ralph solves this by adding an intelligent orchestration layer: a Rust application that runs the development agent, uses a configurable secondary agent to verify work, and automatically routes to the next task or retry based on verification results—all with configurable prompts and retry limits.

### Personas
| Persona | Description | Primary Goals |
|---------|-------------|---------------|
| Solo Developer | Developer running automated coding workflows | Hands-off iteration on coding tasks without manual verification between steps |
| Team Lead | Coordinates agent-driven development | Reliable, auditable automated workflows with failure handling |

### Current State vs. Desired State
**Today (without feature)**: Developers run `while :; do cat PROMPT.md | claude-code ; done` (Ralph loops). They manually check if work is done correctly. When it fails, they manually edit PROMPT.md and restart. No automatic retry limits. No verification beyond the agent's self-assessment.

**Tomorrow (with feature)**: A Rust orchestrator runs `claude -p`, automatically verifies completion via a secondary agent, routes to next task or retry based on "RESULT: CONTINUE/REPEAT/DONE" signals, enforces configurable retry limits and max iterations, and uses configurable prompt templates for all interactions.

---

## Story Landscape

### Story Status Overview
| # | Story | Priority | Status | Confidence | Blocked By |
|---|-------|----------|--------|------------|------------|
| 6 | TOML Configuration | P1 | ✅ In Spec | 100% | - |
| 1 | Core Loop Execution | P2 | ✅ In Spec | 100% | - |
| 2 | Review Agent Integration | P3 | ✅ In Spec | 100% | - |
| 3 | Result Parsing & Routing | P4 | ✅ In Spec | 100% | - |
| 5 | Retry Management | P5 | ✅ In Spec | 100% | - |
| 4 | Next-Action Prompt Generation | P6 | ✅ In Spec | 100% | - |

### Story Dependencies
```
Story 6 (Config) ✅ ───► Story 1 (Core Loop) ✅ ───► Story 2 (Review) ✅ ───► Story 3 (Routing) ✅ ──┬──► Story 4 (Next-Action) ✅
                                                                                                      │
                                                                                                      └──► Story 5 (Retries) ✅
```

**ALL STORIES COMPLETE**

---

## Completed Stories Summary

| # | Story | Priority | Completed | Key Decisions | Revision Risk |
|---|-------|----------|-----------|---------------|---------------|
| 6 | TOML Configuration | P1 | 2026-02-02 | D5, D8-D13, D15, D17-D19, D25, D26, D28 | Low |
| 1 | Core Loop Execution | P2 | 2026-02-02 | D13-D17 | Low |
| 2 | Review Agent Integration | P3 | 2026-02-02 | D18-D20 | Low |
| 3 | Result Parsing & Routing | P4 | 2026-02-02 | D21-D23 | Low |
| 5 | Retry Management | P5 | 2026-02-02 | D4, D14, D24, D25 | Low |
| 4 | Next-Action Prompt Generation | P6 | 2026-02-02 | D26-D28 | Low |

*Full stories in SPEC.md*

---

## Specification Complete

The specification for **unpossible-ralph** is now complete with:

- **6 user stories** fully specified
- **28 key decisions** documented
- **41 functional requirements** defined
- **19 success criteria** measurable
- **24 edge cases** with expected behavior
- **50+ acceptance scenarios** testable

See `discovery/SPEC.md` for the complete specification.

---

## Glossary

- **Ralph Loop**: An infinite shell loop running an AI coding agent (`while :; do cat PROMPT.md | claude-code ; done`), relying on eventual consistency through iteration
- **Development Agent**: The primary agent doing the work (configurable command)
- **Review Agent**: The secondary agent that verifies completion (configurable command)
- **Next-Action Agent**: The agent that generates the next iteration's prompt (configurable command)
- **Orchestrator**: The Rust application managing the loop, verification, and routing
- **dev_response**: Captured stdout from development agent
- **dev_errors**: Captured stderr from development agent
- **review_response**: Captured stdout from review agent
- **review_errors**: Captured stderr from review agent
- **next_prompt**: Output from next-action agent, used in continuation template
- **RESULT signal**: The `RESULT: CONTINUE|REPEAT|DONE` line in review output that determines routing
- **retry_count**: Counter for consecutive failures, reset on CONTINUE
- **iteration_count**: Counter for total loop iterations

