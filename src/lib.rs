//! ralph - AI coding agent loop orchestrator with intelligent verification.
//!
//! This crate provides the core functionality for running AI coding agent loops
//! with intelligent verification. It orchestrates development agents, reviews
//! their work via review agents, and routes to the next task based on
//! `RESULT: CONTINUE|REPEAT|DONE` signals.
//!
//! # Modules
//!
//! - [`agent`] - Agent execution with timeout handling and output capture
//! - [`config`] - Configuration loading and validation
//! - [`r#loop`] - Core orchestration loop state machine and executor
//! - [`result`] - RESULT signal parsing from review agent output
//! - [`template`] - Variable interpolation for prompt templates

pub mod agent;
pub mod config;
pub mod r#loop;
pub mod result;
pub mod template;
