# CLI Contract: ralph

**Version**: 1.0.0
**Date**: 2026-02-02

## Synopsis

```
ralph [OPTIONS]
```

## Description

Orchestrates AI coding agent loops with intelligent verification. Runs a development agent, verifies work via a review agent, and routes to next task or retry based on `RESULT: CONTINUE|REPEAT|DONE` signals.

## Options

| Flag | Short | Description | Default |
|------|-------|-------------|---------|
| `--config <PATH>` | `-c` | Path to configuration file | `./ralph.toml` |
| `--help` | `-h` | Print help information | - |
| `--version` | `-V` | Print version information | - |

## Exit Codes

| Code | Name | Description |
|------|------|-------------|
| 0 | Success | Loop completed: `RESULT: DONE` or `max_iterations` reached |
| 1 | Error | Configuration error, missing command, critical agent failure |
| 2 | MaxRetries | Consecutive failures exceeded `max_retries` |
| 130 | Interrupted | Received SIGINT (Ctrl+C) |
| 143 | Terminated | Received SIGTERM |

## Standard Streams

### stdout

- Iteration summaries after each agent completes
- Final status message on exit

**Format**:
```
[ralph] Iteration 1 started
[ralph] dev agent completed: exit=0, stdout=1234 bytes, stderr=0 bytes
[ralph] review agent completed: exit=0, stdout=567 bytes
[ralph] RESULT: CONTINUE
[ralph] next-action agent completed: exit=0, stdout=890 bytes
[ralph] Iteration 2 started
...
[ralph] RESULT: DONE
[ralph] Completed successfully after 3 iterations
```

### stderr

- Error messages with context
- Warnings (e.g., missing RESULT treated as REPEAT)
- Debug/trace output when enabled

**Format**:
```
[ralph] Error: Config file not found: ./ralph.toml
[ralph] Warning: No RESULT found in review output, treating as REPEAT
[ralph] Error: Max retries (3) exceeded
```

## Configuration File

See `data-model.md` for full schema. Required structure:

```toml
[general]
max_retries = 3

[prompts]
starting = "..."
continuation = "..."
review = "..."
next_action = "..."

[agents]
dev = "..."
review = "..."
next_action = "..."
```

## Environment Variables

The orchestrator itself requires no environment variables. Agent commands may require their own (e.g., `ANTHROPIC_API_KEY` for Claude).

## Signals

| Signal | Behavior |
|--------|----------|
| SIGINT | Kill running agent, exit with message "Interrupted" |
| SIGTERM | Kill running agent, exit with message "Terminated" |

When killing agents, the entire process group is terminated to prevent orphan child processes.

## Examples

### Basic usage
```bash
# Run with config in current directory
ralph

# Run with specific config
ralph --config /path/to/ralph.toml

# Check version
ralph --version
```

### Exit code handling
```bash
ralph
case $? in
  0) echo "Success!" ;;
  1) echo "Error - check stderr" ;;
  2) echo "Max retries exceeded" ;;
  130) echo "Interrupted by user" ;;
  *) echo "Unknown exit code" ;;
esac
```

### Integration with shell scripts
```bash
#!/bin/bash
set -e

# Run orchestrator, fail script on error
ralph --config ./my-workflow.toml

# Or capture exit code
if ralph --config ./my-workflow.toml; then
  echo "Workflow completed"
else
  exit_code=$?
  if [ $exit_code -eq 2 ]; then
    echo "Workflow failed after max retries"
  else
    echo "Workflow error: $exit_code"
  fi
fi
```

## Versioning

This CLI follows semantic versioning:
- MAJOR: Breaking changes to CLI interface or config format
- MINOR: New features, backward-compatible
- PATCH: Bug fixes

## See Also

- `data-model.md` - Configuration schema and state machine
- `research.md` - Technology decisions and rationale
- `../spec.md` - Feature specification with acceptance criteria
