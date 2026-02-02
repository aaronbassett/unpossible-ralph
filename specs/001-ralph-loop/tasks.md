# Implementation Tasks: ralph-loop

**Branch**: `001-ralph-loop`
**Generated**: 2026-02-02
**Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

## Overview

| Phase | Name | Implementation | Git/Workflow | Total | Stories |
|-------|------|----------------|--------------|-------|---------|
| 1 | Setup | 7 | 6 | 13 | - |
| 2 | Foundational | 8 | 13 | 21 | - |
| 3 | TOML Configuration | 10 | 15 | 25 | US1 (P1) |
| 4 | Core Loop Execution | 10 | 16 | 26 | US2 (P2) |
| 5 | Review Agent Integration | 7 | 13 | 20 | US3 (P3) |
| 6 | Result Parsing & Routing | 7 | 13 | 20 | US4 (P4) |
| 7 | Next-Action Prompt Generation | 7 | 13 | 20 | US5 (P5) |
| 8 | Retry Management | 5 | 13 | 18 | US6 (P6) |
| 9 | Polish & Integration | 10 | 13 | 23 | - |

**Total Tasks**: 186 (71 implementation + 115 git/workflow/retro/mapping)

---

## Phase 1: Setup

**Goal**: Initialize development environment and project structure

### Rust Installation

- [ ] T001 Install Rust toolchain via rustup (curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh)
- [ ] T002 Verify Rust installation (rustc --version should show 1.75+)
- [ ] T003 [GIT] Verify on main branch and working tree is clean

### Project Initialization

- [ ] T004 [GIT] Pull latest changes from origin/main
- [ ] T005 [GIT] Create feature branch: 001-ralph-loop
- [ ] T006 Initialize Cargo project with `cargo init --name ralph`
- [ ] T007 [GIT] Commit: initialize Cargo project
- [ ] T008 Add dependencies to Cargo.toml per STACK.md (use devs:rust-dev agent)
- [ ] T009 [GIT] Commit: add project dependencies

### Directory Structure

- [ ] T010 Create source directory structure per plan.md (src/config/, src/loop/, src/agent/, src/template/, src/result/)
- [ ] T011 [GIT] Commit: create source directory structure
- [ ] T012 Create test directory structure (tests/integration/, tests/integration/fixtures/)
- [ ] T013 [GIT] Commit: create test directory structure

---

## Phase 2: Foundational

**Goal**: Set up development tooling and shared infrastructure

### Development Tooling

- [ ] T014 Create retro/P2.md for this phase
- [ ] T015 [GIT] Commit: initialize phase 2 retro
- [ ] T016 Configure rustfmt.toml with project formatting rules (use init-local-tooling skill)
- [ ] T017 Configure clippy.toml with linting rules per constitution
- [ ] T018 [GIT] Commit: add rustfmt and clippy configuration
- [ ] T019 Create .git/hooks/pre-commit with cargo fmt --check and cargo clippy -- -D warnings
- [ ] T020 [GIT] Commit: add pre-commit hooks

### CI Pipeline

- [ ] T021 [P] Create .github/workflows/ci.yml with build, test, clippy, and rustfmt checks
- [ ] T022 [GIT] Commit: add GitHub Actions CI pipeline

### Test Infrastructure

- [ ] T023 [P] Create tests/integration/fixtures/valid_config.toml with complete example
- [ ] T024 [P] Create tests/integration/fixtures/invalid_config.toml with syntax error
- [ ] T025 [P] Create tests/integration/fixtures/missing_section.toml without [agents]
- [ ] T026 [GIT] Commit: add test fixtures

### Phase Completion

- [ ] T027 Run /sdd:map incremental for Phase 2 changes
- [ ] T028 [GIT] Commit: update codebase documents for phase 2
- [ ] T029 Review retro/P2.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T030 [GIT] Commit: finalize phase 2 retro
- [ ] T031 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T032 [GIT] Create/update PR to main with phase summary
- [ ] T033 [GIT] Verify all CI checks pass
- [ ] T034 [GIT] Report PR ready status

---

## Phase 3: TOML Configuration [US1]

**Goal**: Implement config loading, parsing, and validation (Priority P1)

**Independent Test Criteria**: Config loads from valid file; validation errors returned for invalid files

### Setup

- [ ] T035 [GIT] Verify working tree is clean before starting Phase 3
- [ ] T036 [GIT] Pull and rebase on origin/main if needed
- [ ] T037 [US1] Create retro/P3.md for this phase
- [ ] T038 [GIT] Commit: initialize phase 3 retro

### Config Types

- [ ] T039 [P] [US1] [FR2-4] Implement Config, GeneralConfig, PromptsConfig structs in src/config/types.rs (use devs:rust-dev agent)
- [ ] T040 [P] [US1] [FR10] Implement AgentsConfig and AgentConfig enum in src/config/types.rs (use devs:rust-dev agent)
- [ ] T041 [GIT] Commit: add config type definitions

### Config Validation

- [ ] T042 [US1] [FR5,FR6,FR35,FR50] Implement validation rules in src/config/validate.rs (use devs:rust-dev agent)
- [ ] T043 [GIT] Commit: add config validation logic
- [ ] T044 [US1] [E1-E5,E26,E29] Add unit tests for validation in src/config/validate.rs (use devs:rust-dev agent)
- [ ] T045 [GIT] Commit: add validation unit tests

### Config Loading

- [ ] T046 [US1] [FR1,FR9] Implement config loading in src/config/mod.rs with file path resolution (use devs:rust-dev agent)
- [ ] T047 [GIT] Commit: add config loading logic
- [ ] T048 [US1] [FR9] Add error types with human-readable messages per constitution principle II (use devs:rust-dev agent)
- [ ] T049 [GIT] Commit: add config error types

### Integration Tests

- [ ] T050 [US1] [E1-E5,E24,E26,E29,E30] Create config integration tests in tests/integration/config_test.rs (use devs:rust-dev agent)
- [ ] T051 [GIT] Commit: add config integration tests

### Phase Completion

- [ ] T052 [US1] Run /sdd:map incremental for Phase 3 changes
- [ ] T053 [GIT] Commit: update codebase documents for phase 3
- [ ] T054 [US1] Review retro/P3.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T055 [GIT] Commit: finalize phase 3 retro
- [ ] T056 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T057 [GIT] Create/update PR to main with phase summary
- [ ] T058 [GIT] Verify all CI checks pass
- [ ] T059 [GIT] Report PR ready status

---

## Phase 4: Core Loop Execution [US2]

**Goal**: Implement dev agent execution with output capture (Priority P2)

**Independent Test Criteria**: Echo command runs and captures stdout/stderr separately

### Setup

- [ ] T060 [GIT] Verify working tree is clean before starting Phase 4
- [ ] T061 [GIT] Pull and rebase on origin/main if needed
- [ ] T062 [US2] Create retro/P4.md for this phase
- [ ] T063 [GIT] Commit: initialize phase 4 retro

### Template Engine

- [ ] T064 [P] [US2] [FR7,FR8,FR49] Implement TemplateContext with variable substitution in src/template/mod.rs (use devs:rust-dev agent)
- [ ] T065 [GIT] Commit: add template engine
- [ ] T066 [US2] [E1,E30,E31] Add unit tests for template interpolation including edge cases E30, E31 (use devs:rust-dev agent)
- [ ] T067 [GIT] Commit: add template unit tests

### Agent Execution

- [ ] T068 [US2] [FR11,FR44] Implement AgentRunner with tokio process execution in src/agent/mod.rs (use devs:rust-dev agent)
- [ ] T069 [GIT] Commit: add agent runner
- [ ] T070 [US2] [FR12,FR13,FR48] Implement stdout/stderr capture in src/agent/capture.rs (use devs:rust-dev agent)
- [ ] T071 [GIT] Commit: add output capture logic
- [ ] T072 [US2] [FR16,FR43] Implement timeout and process group kill in src/agent/mod.rs (use devs:rust-dev agent)
- [ ] T073 [GIT] Commit: add timeout and process group handling

### Loop State

- [ ] T074 [US2] [FR14,FR15] Implement LoopState struct in src/loop/mod.rs per data-model.md (use devs:rust-dev agent)
- [ ] T075 [GIT] Commit: add loop state management
- [ ] T076 [US2] [FR17,FR18] Implement iteration summary display in src/loop/executor.rs (use devs:rust-dev agent)
- [ ] T077 [GIT] Commit: add iteration summary display

### Phase Completion

- [ ] T078 [US2] Run /sdd:map incremental for Phase 4 changes
- [ ] T079 [GIT] Commit: update codebase documents for phase 4
- [ ] T080 [US2] Review retro/P4.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T081 [GIT] Commit: finalize phase 4 retro
- [ ] T082 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T083 [GIT] Create/update PR to main with phase summary
- [ ] T084 [GIT] Verify all CI checks pass
- [ ] T085 [GIT] Report PR ready status

---

## Phase 5: Review Agent Integration [US3]

**Goal**: Implement review agent execution after dev agent (Priority P3)

**Independent Test Criteria**: Review agent receives dev output and returns response

### Setup

- [ ] T086 [GIT] Verify working tree is clean before starting Phase 5
- [ ] T087 [GIT] Pull and rebase on origin/main if needed
- [ ] T088 [US3] Create retro/P5.md for this phase
- [ ] T089 [GIT] Commit: initialize phase 5 retro

### Review Agent

- [ ] T090 [US3] [FR19] Extend AgentRunner to support review agent execution in src/agent/mod.rs (use devs:rust-dev agent)
- [ ] T091 [GIT] Commit: add review agent execution
- [ ] T092 [US3] [FR22] Implement per-agent timeout override in src/agent/mod.rs (use devs:rust-dev agent)
- [ ] T093 [GIT] Commit: add per-agent timeout support
- [ ] T094 [US3] [FR23] Implement fatal error handling for review failures in src/loop/executor.rs (use devs:rust-dev agent)
- [ ] T095 [GIT] Commit: add review failure handling

### Integration

- [ ] T096 [US3] [E13-E15] Add review agent integration tests in tests/integration/loop_test.rs (use devs:rust-dev agent)
- [ ] T097 [GIT] Commit: add review integration tests

### Phase Completion

- [ ] T098 [US3] Run /sdd:map incremental for Phase 5 changes
- [ ] T099 [GIT] Commit: update codebase documents for phase 5
- [ ] T100 [US3] Review retro/P5.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T101 [GIT] Commit: finalize phase 5 retro
- [ ] T102 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T103 [GIT] Create/update PR to main with phase summary
- [ ] T104 [GIT] Verify all CI checks pass
- [ ] T105 [GIT] Report PR ready status

---

## Phase 6: Result Parsing & Routing [US4]

**Goal**: Implement RESULT signal parsing and routing logic (Priority P4)

**Independent Test Criteria**: Various RESULT strings correctly parsed and routed

### Setup

- [ ] T106 [GIT] Verify working tree is clean before starting Phase 6
- [ ] T107 [GIT] Pull and rebase on origin/main if needed
- [ ] T108 [US4] Create retro/P6.md for this phase
- [ ] T109 [GIT] Commit: initialize phase 6 retro

### Result Parsing

- [ ] T110 [US4] [FR25] Implement ReviewResult enum and parse() in src/result/types.rs (use devs:rust-dev agent)
- [ ] T111 [GIT] Commit: add result types
- [ ] T112 [US4] [FR24,FR26,FR27] Implement regex-based RESULT parsing with case-insensitivity (D21) in src/result/mod.rs (use devs:rust-dev agent)
- [ ] T113 [GIT] Commit: add result parsing logic
- [ ] T114 [US4] [E16-E19] Add comprehensive unit tests for parsing edge cases in src/result/mod.rs (use devs:rust-dev agent)
- [ ] T115 [GIT] Commit: add result parsing unit tests

### Routing Logic

- [ ] T116 [US4] [FR28-FR30] Implement routing logic (CONTINUE/REPEAT/DONE) in src/loop/executor.rs (use devs:rust-dev agent)
- [ ] T117 [GIT] Commit: add routing logic

### Phase Completion

- [ ] T118 [US4] Run /sdd:map incremental for Phase 6 changes
- [ ] T119 [GIT] Commit: update codebase documents for phase 6
- [ ] T120 [US4] Review retro/P6.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T121 [GIT] Commit: finalize phase 6 retro
- [ ] T122 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T123 [GIT] Create/update PR to main with phase summary
- [ ] T124 [GIT] Verify all CI checks pass
- [ ] T125 [GIT] Report PR ready status

---

## Phase 7: Next-Action Prompt Generation [US5]

**Goal**: Implement next-action agent and continuation flow (Priority P5)

**Independent Test Criteria**: Next-action output stored and used in continuation template

### Setup

- [ ] T126 [GIT] Verify working tree is clean before starting Phase 7
- [ ] T127 [GIT] Pull and rebase on origin/main if needed
- [ ] T128 [US5] Create retro/P7.md for this phase
- [ ] T129 [GIT] Commit: initialize phase 7 retro

### Next-Action Agent

- [ ] T130 [US5] [FR36,FR37] Implement next-action agent execution after CONTINUE in src/loop/executor.rs (use devs:rust-dev agent)
- [ ] T131 [GIT] Commit: add next-action execution
- [ ] T132 [US5] [FR38,FR39,FR40] Implement starting vs continuation template selection (D28) in src/loop/mod.rs (use devs:rust-dev agent)
- [ ] T133 [GIT] Commit: add template selection logic
- [ ] T134 [US5] [FR41] Implement fatal error handling for next-action failures in src/loop/executor.rs (use devs:rust-dev agent)
- [ ] T135 [GIT] Commit: add next-action failure handling

### Integration

- [ ] T136 [US5] [E20-E24] Add next-action integration tests in tests/integration/loop_test.rs (use devs:rust-dev agent)
- [ ] T137 [GIT] Commit: add next-action integration tests

### Phase Completion

- [ ] T138 [US5] Run /sdd:map incremental for Phase 7 changes
- [ ] T139 [GIT] Commit: update codebase documents for phase 7
- [ ] T140 [US5] Review retro/P7.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T141 [GIT] Commit: finalize phase 7 retro
- [ ] T142 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T143 [GIT] Create/update PR to main with phase summary
- [ ] T144 [GIT] Verify all CI checks pass
- [ ] T145 [GIT] Report PR ready status

---

## Phase 8: Retry Management [US6]

**Goal**: Implement retry counting and max_retries enforcement (Priority P6)

**Independent Test Criteria**: Retry count increments on failure, resets on success, exits at limit

### Setup

- [ ] T146 [GIT] Verify working tree is clean before starting Phase 8
- [ ] T147 [GIT] Pull and rebase on origin/main if needed
- [ ] T148 [US6] Create retro/P8.md for this phase
- [ ] T149 [GIT] Commit: initialize phase 8 retro

### Retry Logic

- [ ] T150 [US6] [FR31,FR32,FR33] Implement retry_count tracking in LoopState per data-model.md (use devs:rust-dev agent)
- [ ] T151 [GIT] Commit: add retry count tracking
- [ ] T152 [US6] [FR34] Implement max_retries check with exit code 2 (D24) in src/loop/executor.rs (use devs:rust-dev agent)
- [ ] T153 [GIT] Commit: add max retries enforcement

### Integration

- [ ] T154 [US6] [E25-E27] Add retry management integration tests in tests/integration/loop_test.rs (use devs:rust-dev agent)
- [ ] T155 [GIT] Commit: add retry integration tests

### Phase Completion

- [ ] T156 [US6] Run /sdd:map incremental for Phase 8 changes
- [ ] T157 [GIT] Commit: update codebase documents for phase 8
- [ ] T158 [US6] Review retro/P8.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T159 [GIT] Commit: finalize phase 8 retro
- [ ] T160 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T161 [GIT] Create/update PR to main with phase summary
- [ ] T162 [GIT] Verify all CI checks pass
- [ ] T163 [GIT] Report PR ready status

---

## Phase 9: Polish & Integration

**Goal**: Complete CLI, signal handling, and final integration

### Setup

- [ ] T164 [GIT] Verify working tree is clean before starting Phase 9
- [ ] T165 [GIT] Pull and rebase on origin/main if needed
- [ ] T166 Create retro/P9.md for this phase
- [ ] T167 [GIT] Commit: initialize phase 9 retro

### CLI Implementation

- [ ] T168 [FR46,FR47] Implement CLI with clap in src/main.rs (--config, --help, --version) (use devs:rust-dev agent)
- [ ] T169 [GIT] Commit: add CLI implementation
- [ ] T170 [FR45] Implement signal handling (SIGINT/SIGTERM) in src/main.rs (use devs:rust-dev agent)
- [ ] T171 [GIT] Commit: add signal handling

### Final Integration

- [ ] T172 Wire all components together in src/main.rs (use devs:rust-dev agent)
- [ ] T173 [GIT] Commit: complete main integration
- [ ] T174 Add end-to-end integration test with real agent execution in tests/integration/e2e_test.rs (use devs:rust-dev agent)
- [ ] T175 [GIT] Commit: add end-to-end tests

### Documentation

- [ ] T176 [P] Create ralph.toml.example with complete documented example
- [ ] T177 [P] Update README.md with installation and usage
- [ ] T178 [GIT] Commit: add documentation

### Phase Completion

- [ ] T179 Run /sdd:map incremental for Phase 9 changes
- [ ] T180 [GIT] Commit: update codebase documents for phase 9
- [ ] T181 Review retro/P9.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T182 [GIT] Commit: finalize phase 9 retro
- [ ] T183 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T184 [GIT] Create/update PR to main with phase summary
- [ ] T185 [GIT] Verify all CI checks pass
- [ ] T186 [GIT] Report PR ready status

---

## Dependencies

### Story Completion Order

```
Phase 1 (Setup) ──► Phase 2 (Foundational) ──► Phase 3 [US1: Config]
                                                    │
                                                    ▼
                                              Phase 4 [US2: Core Loop]
                                                    │
                                                    ▼
                                              Phase 5 [US3: Review]
                                                    │
                                                    ▼
                                              Phase 6 [US4: Parsing]
                                                    │
                                    ┌───────────────┴───────────────┐
                                    ▼                               ▼
                              Phase 7 [US5: Next-Action]     Phase 8 [US6: Retry]
                                    │                               │
                                    └───────────────┬───────────────┘
                                                    ▼
                                              Phase 9 (Polish)
```

### Parallel Execution Opportunities

**Within Phase 2 (Foundational)**:
- T021 (CI pipeline) can run parallel with T023-T025 (fixtures)

**Within Phase 3 (Config)**:
- T039-T040 (config types) can run in parallel

**Within Phase 4 (Core Loop)**:
- T064 (template engine) can start while others proceed

**Within Phase 9 (Polish)**:
- T176-T177 (documentation) can run in parallel

---

## Implementation Strategy

### MVP Scope (Phase 1-4)

First deliverable: Config loading + single dev agent execution with output capture

**Validates**:
- Rust toolchain setup
- Config parsing and validation
- Basic agent execution
- Template interpolation

### Incremental Delivery

| Milestone | Phases | Capability |
|-----------|--------|------------|
| MVP | 1-4 | Config + dev agent execution |
| Review | 5-6 | Dev → review → routing |
| Multi-iteration | 7-8 | Full loop with next-action and retry |
| Release | 9 | CLI complete, documented |

---

## Summary

**Total Tasks**: 186
**Parallelizable**: 9 (tasks marked with [P] that can run concurrently)

Task Breakdown:
- Implementation tasks: Code, config, and test creation
- Git workflow tasks: Commits, pushes, PRs, branch verification
- Infrastructure tasks: Retro creation, codebase mapping, CI verification

### Tasks Per Story

| Story | Priority | Tasks | Description |
|-------|----------|-------|-------------|
| Setup | - | 13 | Rust installation and project init |
| Foundational | - | 21 | Tooling and infrastructure |
| US1 | P1 | 25 | TOML Configuration |
| US2 | P2 | 26 | Core Loop Execution |
| US3 | P3 | 20 | Review Agent Integration |
| US4 | P4 | 20 | Result Parsing & Routing |
| US5 | P5 | 20 | Next-Action Prompt Generation |
| US6 | P6 | 18 | Retry Management |
| Polish | - | 23 | CLI and final integration |
