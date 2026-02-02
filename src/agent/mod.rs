//! Agent execution module for running shell commands with timeout handling.
//!
//! This module provides the [`AgentRunner`] for executing agent commands via `/bin/sh`
//! with proper timeout handling, process group management, and output capture.
//!
//! # Features
//!
//! - Execute commands via `/bin/sh -c` (FR44)
//! - Capture stdout and stderr separately (FR12, FR13)
//! - Kill entire process group on timeout (FR43)
//! - Replace invalid UTF-8 with U+FFFD (FR48)
//! - Handle signals gracefully (exit_code = None if killed by signal)
//!
//! # Example
//!
//! ```no_run
//! use ralph::agent::{AgentRunner, AgentResult};
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let runner = AgentRunner::new(30); // 30 second timeout
//! let result = runner.run("echo 'Hello, World!'").await?;
//!
//! assert!(result.success);
//! assert_eq!(result.stdout.trim(), "Hello, World!");
//! # Ok(())
//! # }
//! ```

mod capture;

pub use capture::capture_output;

use std::time::Duration;
use thiserror::Error;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, warn};

/// Errors that can occur during agent execution.
#[derive(Debug, Error)]
pub enum AgentError {
    /// Failed to spawn the agent command.
    #[error("failed to spawn command '{command}': {source}")]
    SpawnError {
        /// The command that failed to spawn.
        command: String,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The agent command timed out.
    #[error("command timed out after {timeout_secs} seconds: '{command}'")]
    Timeout {
        /// The command that timed out.
        command: String,
        /// The timeout duration in seconds.
        timeout_secs: u32,
    },
}

/// Result of running an agent command.
///
/// Contains the captured output streams and exit status information.
#[derive(Debug, Clone)]
pub struct AgentResult {
    /// Captured stdout output (UTF-8, invalid bytes replaced with U+FFFD).
    pub stdout: String,

    /// Captured stderr output (UTF-8, invalid bytes replaced with U+FFFD).
    pub stderr: String,

    /// The exit code of the process, or `None` if killed by a signal.
    pub exit_code: Option<i32>,

    /// Whether the command completed successfully (exit_code == Some(0)).
    pub success: bool,
}

impl AgentResult {
    /// Create a new `AgentResult` from raw output and exit code.
    fn new(stdout: Vec<u8>, stderr: Vec<u8>, exit_code: Option<i32>) -> Self {
        let stdout = capture::lossy_utf8(&stdout);
        let stderr = capture::lossy_utf8(&stderr);
        let success = exit_code == Some(0);

        Self {
            stdout,
            stderr,
            exit_code,
            success,
        }
    }
}

/// Agent runner for executing shell commands with timeout handling.
///
/// The runner executes commands via `/bin/sh -c` on Unix systems and manages
/// timeouts by killing the entire process group to prevent orphan processes.
#[derive(Debug, Clone)]
pub struct AgentRunner {
    /// Timeout duration for command execution.
    timeout: Option<Duration>,
    /// Timeout in seconds (stored for error messages).
    timeout_secs: u32,
}

impl AgentRunner {
    /// Create a new `AgentRunner` with the specified timeout.
    ///
    /// # Arguments
    ///
    /// * `timeout_secs` - Timeout in seconds. Use 0 for no timeout (E12).
    ///
    /// # Example
    ///
    /// ```
    /// use ralph::agent::AgentRunner;
    ///
    /// // 30 second timeout
    /// let runner = AgentRunner::new(30);
    ///
    /// // No timeout (runs until completion)
    /// let unlimited_runner = AgentRunner::new(0);
    /// ```
    pub fn new(timeout_secs: u32) -> Self {
        let timeout = if timeout_secs == 0 {
            None // E12: timeout = 0 means no timeout limit
        } else {
            Some(Duration::from_secs(timeout_secs as u64))
        };

        Self {
            timeout,
            timeout_secs,
        }
    }

    /// Execute a command via shell with the given rendered command string.
    ///
    /// This method:
    /// - Executes the command via `/bin/sh -c` on Unix (FR44)
    /// - Captures stdout and stderr separately (FR12, FR13)
    /// - Kills the entire process group on timeout (FR43)
    /// - Replaces invalid UTF-8 bytes with U+FFFD (FR48)
    ///
    /// # Arguments
    ///
    /// * `command` - The shell command to execute (already interpolated with `{{prompt}}`).
    ///
    /// # Returns
    ///
    /// - `Ok(AgentResult)` on successful execution (even if exit code is non-zero).
    /// - `Err(AgentError::SpawnError)` if the command failed to start.
    /// - `Err(AgentError::Timeout)` if the command exceeded the timeout.
    ///
    /// # Edge Cases
    ///
    /// - E6: Command not found results in non-zero exit code from shell.
    /// - E7: Permission denied results in non-zero exit code from shell.
    /// - E10: Binary/non-UTF8 output is replaced with U+FFFD.
    /// - E11: Agent killed by signal has `exit_code = None`.
    /// - E12: timeout = 0 means no timeout limit.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ralph::agent::AgentRunner;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let runner = AgentRunner::new(30);
    /// let result = runner.run("ls -la").await?;
    ///
    /// println!("Exit code: {:?}", result.exit_code);
    /// println!("Output: {}", result.stdout);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run(&self, command: &str) -> Result<AgentResult, AgentError> {
        debug!(command = %command, timeout_secs = self.timeout_secs, "executing agent command");

        let mut cmd = self.build_command(command);

        // Spawn the process
        let child = cmd.spawn().map_err(|source| AgentError::SpawnError {
            command: command.to_string(),
            source,
        })?;

        // Capture the PID before moving the child (needed for timeout kill)
        let pid = child.id();

        // Wait for completion with optional timeout
        let output_result = if let Some(timeout_duration) = self.timeout {
            match timeout(timeout_duration, child.wait_with_output()).await {
                Ok(result) => result,
                Err(_) => {
                    // Timeout elapsed - kill the process group
                    self.kill_process_group(pid);
                    return Err(AgentError::Timeout {
                        command: command.to_string(),
                        timeout_secs: self.timeout_secs,
                    });
                }
            }
        } else {
            // No timeout - wait indefinitely
            child.wait_with_output().await
        };

        // Process the output
        let output = output_result.map_err(|source| AgentError::SpawnError {
            command: command.to_string(),
            source,
        })?;

        // Extract exit code (None if killed by signal)
        let exit_code = output.status.code();

        if exit_code.is_none() {
            warn!(command = %command, "agent killed by signal");
        }

        debug!(
            command = %command,
            exit_code = ?exit_code,
            stdout_len = output.stdout.len(),
            stderr_len = output.stderr.len(),
            "agent command completed"
        );

        Ok(AgentResult::new(output.stdout, output.stderr, exit_code))
    }

    /// Build a command for shell execution.
    #[cfg(unix)]
    fn build_command(&self, command: &str) -> Command {
        let mut cmd = Command::new("/bin/sh");
        cmd.arg("-c");
        cmd.arg(command);

        // Set up process group for clean timeout kills (FR43)
        // SAFETY: setsid() is async-signal-safe and creates a new session,
        // making this process the leader of a new process group.
        unsafe {
            cmd.pre_exec(|| {
                // Create new session (and process group)
                // This makes the child the leader of its own process group
                libc::setsid();
                Ok(())
            });
        }

        // Capture stdout and stderr
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        cmd
    }

    /// Build a command for shell execution (non-Unix fallback).
    #[cfg(not(unix))]
    fn build_command(&self, command: &str) -> Command {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C");
        cmd.arg(command);

        // Capture stdout and stderr
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        cmd
    }

    /// Kill the entire process group (FR43).
    #[cfg(unix)]
    fn kill_process_group(&self, pid: Option<u32>) {
        if let Some(pid) = pid {
            warn!(pid = pid, "killing process group due to timeout");

            // Kill the entire process group with SIGKILL
            // The child is the process group leader due to setsid() in pre_exec
            let pgid = nix::unistd::Pid::from_raw(pid as i32);
            if let Err(e) = nix::sys::signal::killpg(pgid, nix::sys::signal::Signal::SIGKILL) {
                // If killpg fails, try to kill just the process
                warn!(pid = pid, error = %e, "killpg failed, attempting kill");
                let _ = nix::sys::signal::kill(pgid, nix::sys::signal::Signal::SIGKILL);
            }
        }
    }

    /// Kill the process (non-Unix fallback).
    #[cfg(not(unix))]
    fn kill_process_group(&self, pid: Option<u32>) {
        // On non-Unix, we can only try to kill the main process
        // This may leave orphan children
        if let Some(pid) = pid {
            warn!(
                pid = pid,
                "killing process due to timeout (process group kill not supported)"
            );
        }
        // Note: On non-Unix, we rely on tokio dropping the child handle to clean up
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test successful command execution.
    #[tokio::test]
    async fn test_successful_execution() {
        let runner = AgentRunner::new(10);
        let result = runner.run("echo 'hello world'").await.unwrap();

        assert!(result.success);
        assert_eq!(result.exit_code, Some(0));
        assert_eq!(result.stdout.trim(), "hello world");
        assert!(result.stderr.is_empty());
    }

    /// Test command with non-zero exit code.
    #[tokio::test]
    async fn test_nonzero_exit_code() {
        let runner = AgentRunner::new(10);
        let result = runner.run("exit 42").await.unwrap();

        assert!(!result.success);
        assert_eq!(result.exit_code, Some(42));
    }

    /// Test command timeout.
    #[tokio::test]
    async fn test_command_timeout() {
        let runner = AgentRunner::new(1); // 1 second timeout
        let result = runner.run("sleep 10").await;

        assert!(matches!(result, Err(AgentError::Timeout { .. })));
        if let Err(AgentError::Timeout { timeout_secs, .. }) = result {
            assert_eq!(timeout_secs, 1);
        }
    }

    /// Test stdout capture.
    #[tokio::test]
    async fn test_stdout_capture() {
        let runner = AgentRunner::new(10);
        let result = runner.run("echo 'line1'; echo 'line2'").await.unwrap();

        assert!(result.success);
        assert!(result.stdout.contains("line1"));
        assert!(result.stdout.contains("line2"));
    }

    /// Test stderr capture.
    #[tokio::test]
    async fn test_stderr_capture() {
        let runner = AgentRunner::new(10);
        let result = runner.run("echo 'error message' >&2").await.unwrap();

        assert!(result.success);
        assert_eq!(result.stderr.trim(), "error message");
        assert!(result.stdout.is_empty());
    }

    /// Test invalid UTF-8 handling - replaced with U+FFFD.
    #[tokio::test]
    async fn test_invalid_utf8_handling() {
        let runner = AgentRunner::new(10);
        // Use /usr/bin/printf which reliably handles \xff escapes
        // On some systems, shell built-in printf may behave differently
        let result = runner
            .run("/usr/bin/printf 'hello\\xffworld'")
            .await
            .unwrap();

        assert!(result.success);
        // Invalid byte should be replaced with U+FFFD (replacement character)
        assert!(
            result.stdout.contains('\u{FFFD}'),
            "Expected U+FFFD replacement character in output, got: {:?}",
            result.stdout
        );
        assert!(result.stdout.contains("hello"));
        assert!(result.stdout.contains("world"));
    }

    /// Test zero timeout (no timeout limit).
    #[tokio::test]
    async fn test_zero_timeout_no_limit() {
        let runner = AgentRunner::new(0);
        // Should complete without timeout (runs quickly)
        let result = runner.run("echo 'quick'").await.unwrap();

        assert!(result.success);
        assert_eq!(result.stdout.trim(), "quick");
    }

    /// Test command not found (E6).
    #[tokio::test]
    async fn test_command_not_found() {
        let runner = AgentRunner::new(10);
        let result = runner.run("nonexistent_command_12345").await.unwrap();

        // Command not found results in shell error, not spawn error
        assert!(!result.success);
        assert!(result.exit_code.is_some());
        assert!(result.exit_code.unwrap() != 0);
        // Shell writes error to stderr
        assert!(!result.stderr.is_empty() || !result.stdout.is_empty());
    }

    /// Test mixed stdout and stderr output.
    #[tokio::test]
    async fn test_mixed_output() {
        let runner = AgentRunner::new(10);
        let result = runner
            .run("echo 'stdout'; echo 'stderr' >&2")
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.stdout.trim(), "stdout");
        assert_eq!(result.stderr.trim(), "stderr");
    }

    /// Test empty output.
    #[tokio::test]
    async fn test_empty_output() {
        let runner = AgentRunner::new(10);
        let result = runner.run("true").await.unwrap();

        assert!(result.success);
        assert!(result.stdout.is_empty());
        assert!(result.stderr.is_empty());
    }

    /// Test multiline output preservation.
    #[tokio::test]
    async fn test_multiline_output() {
        let runner = AgentRunner::new(10);
        let result = runner.run("echo 'line1\nline2\nline3'").await.unwrap();

        assert!(result.success);
        let lines: Vec<&str> = result.stdout.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "line1");
        assert_eq!(lines[1], "line2");
        assert_eq!(lines[2], "line3");
    }

    /// Test spawn error for invalid shell path (only possible if /bin/sh doesn't exist).
    #[tokio::test]
    async fn test_spawn_error_message() {
        // We can't easily trigger a spawn error with /bin/sh existing,
        // so we test the error type structure instead
        let err = AgentError::SpawnError {
            command: "test cmd".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
        };

        let msg = err.to_string();
        assert!(msg.contains("test cmd"));
        assert!(msg.contains("spawn"));
    }

    /// Test timeout error message.
    #[tokio::test]
    async fn test_timeout_error_message() {
        let err = AgentError::Timeout {
            command: "sleep 100".to_string(),
            timeout_secs: 5,
        };

        let msg = err.to_string();
        assert!(msg.contains("sleep 100"));
        assert!(msg.contains("5 seconds"));
    }
}
