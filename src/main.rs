//! ralph CLI entry point.
//!
//! This is the main entry point for the ralph AI coding agent loop orchestrator.
//! It handles CLI argument parsing, signal handling, configuration loading, and
//! orchestrates the main execution loop.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use clap::Parser;
use ralph::config::{self, Config, ConfigError};
use ralph::r#loop::{LoopExecutor, LoopOutcome};
use ralph::result::{parse_dev_done, DoneResult, ReviewResult};
use tokio::signal;
use tokio::sync::watch;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

/// Exit codes as defined in the CLI contract.
#[allow(dead_code)]
mod exit_code {
    /// Success: DONE or max_iterations reached.
    pub const SUCCESS: u8 = 0;
    /// Error: configuration error, command not found, critical agent failure.
    pub const ERROR: u8 = 1;
    /// Max retries exceeded.
    pub const MAX_RETRIES: u8 = 2;
    /// Interrupted by SIGINT (Ctrl+C).
    pub const INTERRUPTED: u8 = 130;
    /// Terminated by SIGTERM.
    pub const TERMINATED: u8 = 143;
}

/// AI coding agent loop orchestrator with intelligent verification.
///
/// Runs a development agent, verifies work via a review agent, and routes
/// to the next task or retry based on `RESULT: CONTINUE|REPEAT|DONE` signals.
#[derive(Parser, Debug)]
#[command(name = "ralph")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to configuration file.
    #[arg(short, long, default_value = "./ralph.toml")]
    config: PathBuf,
}

/// Print a message to stdout with the [ralph] prefix.
macro_rules! ralph_println {
    ($($arg:tt)*) => {
        println!("[ralph] {}", format_args!($($arg)*))
    };
}

/// Print an error message to stderr with the [ralph] prefix.
macro_rules! ralph_eprintln {
    ($($arg:tt)*) => {
        eprintln!("[ralph] {}", format_args!($($arg)*))
    };
}

fn main() -> ExitCode {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Initialize tracing subscriber with environment filter
    // Defaults to "info" level, can be overridden with RUST_LOG
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    // Build and run the async runtime
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");

    runtime.block_on(async_main(cli))
}

async fn async_main(cli: Cli) -> ExitCode {
    // Set up signal handling
    let (shutdown_tx, shutdown_rx) = watch::channel(ShutdownSignal::None);
    let shutdown_flag = Arc::new(AtomicBool::new(false));

    // Spawn signal handlers
    spawn_signal_handlers(shutdown_tx, Arc::clone(&shutdown_flag));

    // Load and validate configuration
    let config = match load_config(&cli.config) {
        Ok(config) => config,
        Err(code) => return code,
    };

    // Run the orchestration loop
    match run_loop(config, shutdown_rx, shutdown_flag).await {
        Ok(outcome) => {
            let exit_code = outcome.exit_code();
            match &outcome {
                LoopOutcome::Done => {
                    ralph_println!("Completed successfully");
                }
                LoopOutcome::MaxIterations => {
                    ralph_println!("Completed: max_iterations limit reached");
                }
                LoopOutcome::MaxRetries { count } => {
                    ralph_eprintln!("Error: Max retries ({}) exceeded", count);
                }
                LoopOutcome::Error(msg) => {
                    ralph_eprintln!("Error: {}", msg);
                }
            }
            ExitCode::from(exit_code as u8)
        }
        Err(signal) => {
            match signal {
                ShutdownSignal::Interrupt => {
                    ralph_println!("Interrupted");
                    ExitCode::from(exit_code::INTERRUPTED)
                }
                ShutdownSignal::Terminate => {
                    ralph_println!("Terminated");
                    ExitCode::from(exit_code::TERMINATED)
                }
                ShutdownSignal::None => {
                    // Shouldn't happen, but handle gracefully
                    ralph_eprintln!("Error: Unexpected shutdown");
                    ExitCode::from(exit_code::ERROR)
                }
            }
        }
    }
}

/// Signal that triggered shutdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShutdownSignal {
    None,
    Interrupt,
    Terminate,
}

/// Spawn signal handlers for SIGINT and SIGTERM.
fn spawn_signal_handlers(
    shutdown_tx: watch::Sender<ShutdownSignal>,
    shutdown_flag: Arc<AtomicBool>,
) {
    // SIGINT handler (Ctrl+C)
    let sigint_tx = shutdown_tx.clone();
    let sigint_flag = Arc::clone(&shutdown_flag);
    tokio::spawn(async move {
        if signal::ctrl_c().await.is_ok() {
            info!("Received SIGINT");
            sigint_flag.store(true, Ordering::SeqCst);
            let _ = sigint_tx.send(ShutdownSignal::Interrupt);
        }
    });

    // SIGTERM handler (Unix only)
    #[cfg(unix)]
    {
        let sigterm_tx = shutdown_tx;
        let sigterm_flag = shutdown_flag;
        tokio::spawn(async move {
            let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();
            if sigterm.recv().await.is_some() {
                info!("Received SIGTERM");
                sigterm_flag.store(true, Ordering::SeqCst);
                let _ = sigterm_tx.send(ShutdownSignal::Terminate);
            }
        });
    }
}

/// Load and validate configuration from the given path.
fn load_config(path: &Path) -> Result<Config, ExitCode> {
    info!(path = %path.display(), "Loading configuration");

    match config::load(path) {
        Ok(config) => {
            info!(
                max_retries = config.general.max_retries,
                max_iterations = config.general.max_iterations,
                timeout = config.general.timeout,
                "Configuration loaded"
            );
            Ok(config)
        }
        Err(e) => {
            let error_msg = match &e {
                ConfigError::ReadError { path, .. } => {
                    format!("Config file not found: {}", path)
                }
                ConfigError::ParseError { path, message } => {
                    format!("Failed to parse config file '{}': {}", path, message)
                }
                ConfigError::ValidationError(ve) => {
                    format!("Config validation failed: {}", ve)
                }
            };
            ralph_eprintln!("Error: {}", error_msg);
            error!(error = %e, "Configuration error");
            Err(ExitCode::from(exit_code::ERROR))
        }
    }
}

/// Run the main orchestration loop.
///
/// Returns `Ok(LoopOutcome)` on normal termination or `Err(ShutdownSignal)` if
/// interrupted by a signal.
async fn run_loop(
    config: Config,
    mut shutdown_rx: watch::Receiver<ShutdownSignal>,
    shutdown_flag: Arc<AtomicBool>,
) -> Result<LoopOutcome, ShutdownSignal> {
    let mut executor = LoopExecutor::new(config);

    loop {
        // Check for shutdown signal
        if shutdown_flag.load(Ordering::SeqCst) {
            return Err(*shutdown_rx.borrow());
        }

        // Check iteration limit at the start of each iteration
        if executor.iteration_limit_reached() {
            info!(
                iteration = executor.iteration(),
                max = executor.max_iterations(),
                "Max iterations reached"
            );
            return Ok(LoopOutcome::MaxIterations);
        }

        ralph_println!("Iteration {} started", executor.iteration());

        // Run dev agent
        let dev_result = tokio::select! {
            result = executor.run_dev_agent() => result,
            _ = shutdown_rx.changed() => {
                return Err(*shutdown_rx.borrow());
            }
        };

        let dev_result = match dev_result {
            Ok(result) => {
                ralph_println!(
                    "dev agent completed: exit={}, stdout={} bytes, stderr={} bytes",
                    result
                        .exit_code
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "signal".to_string()),
                    result.stdout.len(),
                    result.stderr.len()
                );
                result
            }
            Err(e) => {
                error!(error = %e, "Dev agent failed");
                return Ok(LoopOutcome::Error(e));
            }
        };

        // Check if dev agent claims work is complete
        let dev_claims_done = parse_dev_done(&dev_result.stdout);
        if dev_claims_done {
            ralph_println!("Dev agent claims completion (DEV_DONE: YES)");
            info!("Dev agent signaled DEV_DONE: YES");
        }

        // Handle dev agent failure (non-zero exit)
        if !dev_result.success {
            warn!(
                exit_code = ?dev_result.exit_code,
                "Dev agent exited with non-zero code"
            );
            executor.state_mut().increment_retry();

            // Check retry limit immediately after incrementing
            if executor.retry_limit_exceeded() {
                info!(
                    retry_count = executor.retry_count(),
                    max_retries = executor.max_retries(),
                    "Max retries exceeded after dev agent failure"
                );
                return Ok(LoopOutcome::MaxRetries {
                    count: executor.retry_count(),
                });
            }

            // Continue to review agent even on dev failure (it can provide feedback)
        }

        // Run review agent
        let review_result = tokio::select! {
            result = executor.run_review_agent(&dev_result.stdout, &dev_result.stderr) => result,
            _ = shutdown_rx.changed() => {
                return Err(*shutdown_rx.borrow());
            }
        };

        let review_result = match review_result {
            Ok(result) => {
                ralph_println!(
                    "review agent completed: exit={}, stdout={} bytes",
                    result
                        .exit_code
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "signal".to_string()),
                    result.stdout.len()
                );
                result
            }
            Err(e) => {
                // Review agent failure is fatal (FR23)
                error!(error = %e, "Review agent failed (fatal)");
                return Ok(LoopOutcome::Error(e));
            }
        };

        // Parse RESULT from review output
        let (result, was_explicit) = ReviewResult::parse(&review_result.stdout);

        if !was_explicit {
            ralph_eprintln!("Warning: No RESULT found in review output, treating as REPEAT");
            warn!("No RESULT found in review output, defaulting to REPEAT");
        }

        ralph_println!("RESULT: {}", result);

        // Route based on result
        match result {
            ReviewResult::Repeat => {
                info!("Received RESULT: REPEAT");
                executor.state_mut().increment_retry();

                // Check retry limit immediately after incrementing
                if executor.retry_limit_exceeded() {
                    info!(
                        retry_count = executor.retry_count(),
                        max_retries = executor.max_retries(),
                        "Max retries exceeded after RESULT: REPEAT"
                    );
                    return Ok(LoopOutcome::MaxRetries {
                        count: executor.retry_count(),
                    });
                }

                // Continue loop for retry (same iteration)
            }

            ReviewResult::Continue => {
                info!("Received RESULT: CONTINUE");

                // If dev claimed done, verify with done agent
                if dev_claims_done {
                    ralph_println!("Verifying completion with done agent...");

                    let done_result = tokio::select! {
                        result = executor.run_done_agent() => result,
                        _ = shutdown_rx.changed() => {
                            return Err(*shutdown_rx.borrow());
                        }
                    };

                    let done_result = match done_result {
                        Ok(result) => {
                            ralph_println!(
                                "done agent completed: exit={}, stdout={} bytes",
                                result
                                    .exit_code
                                    .map(|c| c.to_string())
                                    .unwrap_or_else(|| "signal".to_string()),
                                result.stdout.len()
                            );
                            result
                        }
                        Err(e) => {
                            // Done agent failure is fatal
                            error!(error = %e, "Done agent failed (fatal)");
                            return Ok(LoopOutcome::Error(e));
                        }
                    };

                    // Parse done agent's decision
                    let (done_decision, was_explicit) = DoneResult::parse(&done_result.stdout);

                    if !was_explicit {
                        ralph_eprintln!(
                            "Warning: No DONE signal found in done agent output, treating as NOT_DONE"
                        );
                        warn!("No DONE signal found in done agent output, defaulting to NOT_DONE");
                    }

                    ralph_println!("{}", done_decision);

                    match done_decision {
                        DoneResult::Done => {
                            info!("Done agent confirmed completion (DONE: YES)");
                            return Ok(LoopOutcome::Done);
                        }
                        DoneResult::NotDone => {
                            info!("Done agent rejected completion (DONE: NO), continuing with next-action");
                            ralph_println!(
                                "Done agent says work is not complete, continuing to next iteration"
                            );
                            // Fall through to run next-action agent
                        }
                    }
                }

                // Run next-action agent
                let next_action_result = tokio::select! {
                    result = executor.run_next_action_agent(
                        &dev_result.stdout,
                        &dev_result.stderr,
                        &review_result.stdout,
                        &review_result.stderr,
                    ) => result,
                    _ = shutdown_rx.changed() => {
                        return Err(*shutdown_rx.borrow());
                    }
                };

                let next_action_result = match next_action_result {
                    Ok(result) => {
                        ralph_println!(
                            "next-action agent completed: exit={}, stdout={} bytes",
                            result
                                .exit_code
                                .map(|c| c.to_string())
                                .unwrap_or_else(|| "signal".to_string()),
                            result.stdout.len()
                        );
                        result
                    }
                    Err(e) => {
                        // Next-action agent failure is fatal (FR41)
                        error!(error = %e, "Next-action agent failed (fatal)");
                        return Ok(LoopOutcome::Error(e));
                    }
                };

                // Transition to next iteration
                let next_prompt = next_action_result.stdout.clone();
                executor.state_mut().next_iteration(next_prompt);

                info!(
                    new_iteration = executor.iteration(),
                    "Proceeding to next iteration"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_default_config() {
        let cli = Cli::try_parse_from(["ralph"]).unwrap();
        assert_eq!(cli.config, PathBuf::from("./ralph.toml"));
    }

    #[test]
    fn test_cli_custom_config_short() {
        let cli = Cli::try_parse_from(["ralph", "-c", "/path/to/config.toml"]).unwrap();
        assert_eq!(cli.config, PathBuf::from("/path/to/config.toml"));
    }

    #[test]
    fn test_cli_custom_config_long() {
        let cli = Cli::try_parse_from(["ralph", "--config", "/path/to/config.toml"]).unwrap();
        assert_eq!(cli.config, PathBuf::from("/path/to/config.toml"));
    }

    #[test]
    fn test_shutdown_signal_enum() {
        assert_eq!(ShutdownSignal::None, ShutdownSignal::None);
        assert_eq!(ShutdownSignal::Interrupt, ShutdownSignal::Interrupt);
        assert_eq!(ShutdownSignal::Terminate, ShutdownSignal::Terminate);
        assert_ne!(ShutdownSignal::None, ShutdownSignal::Interrupt);
    }

    #[test]
    fn test_exit_codes() {
        assert_eq!(exit_code::SUCCESS, 0);
        assert_eq!(exit_code::ERROR, 1);
        assert_eq!(exit_code::MAX_RETRIES, 2);
        assert_eq!(exit_code::INTERRUPTED, 130);
        assert_eq!(exit_code::TERMINATED, 143);
    }
}
