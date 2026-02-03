# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take security vulnerabilities seriously. If you discover a security issue, please report it responsibly.

### How to Report

**Please do NOT report security vulnerabilities through public GitHub issues.**

Instead, please send an email to: **aaronbassett@gmail.com**

Include the following information in your report:

- Type of vulnerability (e.g., command injection, path traversal)
- Full paths of source files related to the vulnerability
- Location of the affected source code (tag/branch/commit or direct URL)
- Step-by-step instructions to reproduce the issue
- Proof-of-concept or exploit code (if possible)
- Impact of the vulnerability

### What to Expect

- **Acknowledgment**: We will acknowledge your report within 48 hours
- **Assessment**: We will investigate and assess the vulnerability
- **Updates**: We will keep you informed of our progress
- **Resolution**: We aim to resolve critical vulnerabilities within 30 days
- **Credit**: We will credit you in the security advisory (unless you prefer to remain anonymous)

### Safe Harbor

We consider security research conducted in accordance with this policy to be:

- Authorized concerning any applicable anti-hacking laws
- Authorized concerning any relevant anti-circumvention laws
- Exempt from restrictions in our Terms of Service that would interfere with security research

We will not pursue civil action or initiate a complaint to law enforcement for accidental, good-faith violations of this policy.

## Security Considerations

### Command Execution

unpossible-ralph executes shell commands specified in configuration files. Users should:

- Only run configurations from trusted sources
- Review configuration files before execution
- Be aware that agent commands have full shell access
- Not expose the tool to untrusted input

### Configuration Files

- Store sensitive configuration securely
- Do not commit credentials to version control
- Use environment variables for secrets when possible

### Process Management

- The tool kills entire process groups on timeout
- Child processes spawned by agents are also terminated
- Orphan process prevention is implemented via process group management

## Scope

The following are considered in-scope for security reports:

- Command injection vulnerabilities
- Path traversal issues
- Denial of service vulnerabilities
- Process isolation bypasses
- Configuration parsing vulnerabilities

The following are generally out-of-scope:

- Issues in third-party dependencies (report to upstream)
- Social engineering attacks
- Physical attacks
- Issues requiring local access that the user already has

## Updates

Security fixes will be released as patch versions and announced in the GitHub releases.
