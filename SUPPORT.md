# Support

## Documentation

Before seeking support, please check the available documentation:

- [README](README.md) - Overview, installation, and quick start
- [Specification](specs/001-ralph-loop/spec.md) - Detailed feature specification
- [Data Model](specs/001-ralph-loop/data-model.md) - Configuration schema and state machine
- [CLI Contract](specs/001-ralph-loop/contracts/cli.md) - Command-line interface details

## Getting Help

### GitHub Issues

For bugs, feature requests, and questions:

- [Search existing issues](https://github.com/aaronbassett/unpossible-ralph/issues) first
- [Open a new issue](https://github.com/aaronbassett/unpossible-ralph/issues/new/choose) if your question isn't answered

## Common Issues

### Configuration Errors

**Problem**: "Config file not found"
```
Error: Failed to load configuration from './ralph.toml'
```
**Solution**: Ensure `ralph.toml` exists in the current directory or specify path with `--config`.

**Problem**: "Invalid TOML"
```
Error: Failed to parse configuration: expected value at line X
```
**Solution**: Check TOML syntax. Ensure strings are quoted and tables are properly formatted.

### Agent Execution

**Problem**: Agent command not found
```
Error: Failed to execute agent: No such file or directory
```
**Solution**: Verify the agent command is installed and in your PATH.

**Problem**: Agent timeout
```
Warning: Agent timed out after 1800 seconds
```
**Solution**: Increase timeout in config or optimize your agent's response time.

### RESULT Parsing

**Problem**: Missing RESULT signal (treated as REPEAT)
**Solution**: Ensure your review agent outputs `RESULT: CONTINUE`, `RESULT: REPEAT`, or `RESULT: DONE`.

## Reporting Issues

When opening an issue, please include:

1. **Environment**
   - Rust version: `rustc --version`
   - OS and version
   - ralph version: `ralph --version`

2. **Configuration** (sanitized)
   - Relevant parts of your `ralph.toml`
   - Remove any sensitive data

3. **Steps to Reproduce**
   - What commands you ran
   - What you expected
   - What actually happened

4. **Logs**
   - Set `RUST_LOG=debug` for verbose output
   - Include relevant log snippets

## Feature Requests

We welcome feature requests! Please:

1. Check if a similar request already exists
2. Describe the use case and problem you're solving
3. Propose a solution if you have one
4. Be open to discussion about alternatives

## Contributing

Interested in contributing? See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Security Issues

For security vulnerabilities, please see [SECURITY.md](SECURITY.md) for responsible disclosure instructions.
