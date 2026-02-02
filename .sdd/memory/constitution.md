<!--
==============================================================================
SYNC IMPACT REPORT
==============================================================================
Version Change: 1.0.0 → 1.0.0 (no change - initial SDD integration)
Location Change: CONSTITUTION.md → .sdd/memory/constitution.md

Modified Principles: None
Added Sections: None
Removed Sections: None

Templates Requiring Updates:
  - plan-template.md: ✅ Compatible (Constitution Check section can reference I-VI)
  - spec-template.md: ✅ Compatible (Success criteria align with principles)
  - tasks-template.md: ✅ Compatible (Testing philosophy matches Principle V)

Follow-up TODOs: None
==============================================================================
-->

# unpossible-ralph Constitution

## Core Principles

### I. Unix Philosophy

This is a CLI tool. Follow Unix conventions:
- Do one thing well
- Accept input from stdin and arguments
- Output results to stdout, errors to stderr
- Exit with predictable codes: 0 for success, non-zero for errors
- Support piping and composition with other tools
- Prefer text-based I/O that humans can read and scripts can parse

**Rationale**: As a CLI tool, users expect predictable behavior that integrates with their existing workflows and other command-line tools.

### II. Human-Readable Errors

When something goes wrong, tell the user what happened and what they can do:
- Include context: what was attempted, what failed, why
- Suggest fixes when possible
- Use plain language, not just error codes
- Write errors to stderr, not stdout

**Rationale**: Solo-maintained open source means users need to self-diagnose. Clear errors reduce support burden and improve user experience.

### III. Keep It Simple (KISS)

Do the simplest thing that works:
- If you can't explain a design decision in one sentence, reconsider
- Avoid abstractions until the third repetition (Rule of Three)
- Don't build features until they're needed (YAGNI)
- Boring and working beats clever and fragile

**Rationale**: Long-lived solo projects succeed through maintainability. Complexity accumulates over time; fight it actively.

### IV. Modularity

Organize code with clear boundaries:
- Each module has a single, well-defined purpose
- Dependencies between modules are explicit
- No circular dependencies
- New functionality goes in the right place, not just the convenient place

**Rationale**: As the codebase grows over years, modularity makes it possible to understand and change one part without breaking others.

### V. Test What Matters

Focus testing effort on critical paths:
- Test the happy path and important edge cases
- Prioritize integration tests over unit tests for CLI behavior
- Don't chase coverage metrics; chase confidence
- If a bug would be embarrassing or hard to diagnose, test for it

**Rationale**: Critical-paths-only testing balances confidence with development velocity. Tests should catch real bugs, not satisfy metrics.

### VI. Make It Work, Then Make It Fast

Correctness before performance:
- Get it working first
- Measure before optimizing
- "Fast enough" is good enough for most cases
- Document performance-critical sections

**Rationale**: Premature optimization wastes time and adds complexity. Measure first to ensure effort goes where it matters.

## Development Workflow

### Commit Standards

Use conventional commits for clear, searchable history:
- Format: `type(scope): subject`
- Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`
- Keep commits small and focused
- Write commit messages that explain *why*, not just *what*

### Release Strategy

Ship small, ship often:
- Continuous releases as features are ready
- Use semantic versioning (MAJOR.MINOR.PATCH)
- Breaking changes require MAJOR version bump
- Maintain a CHANGELOG

### Documentation

- README covers installation, basic usage, and examples
- Document the *why* in comments, not the *what*
- Keep documentation next to the code it describes
- Update docs when behavior changes

## Quality Standards

### Error Handling

- Fail fast with clear context
- Use Rust's `Result` type properly—no silent failures
- Propagate errors with context using `anyhow` or similar
- Log operations at appropriate levels for debugging

### Code Style

- Follow Rust idioms and `clippy` recommendations
- Use `rustfmt` for consistent formatting
- Prefer explicit over implicit
- Name things clearly—good names reduce need for comments

## Governance

This constitution guides development decisions for unpossible-ralph:
- Principles here supersede ad-hoc practices
- When in doubt, favor simplicity and user experience
- Amendments should be documented with rationale
- Review code changes against these principles

**Version**: 1.0.0 | **Ratified**: 2026-02-02 | **Last Amended**: 2026-02-02
