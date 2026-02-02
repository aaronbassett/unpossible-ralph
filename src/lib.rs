//! ralph - AI coding agent loop orchestrator with intelligent verification.
//!
//! This crate provides the core functionality for running AI coding agent loops
//! with intelligent verification. It orchestrates development agents, reviews
//! their work via review agents, and routes to the next task based on
//! `RESULT: CONTINUE|REPEAT|DONE` signals.
//!
//! # Modules
//!
//! - [`config`] - Configuration loading and validation

pub mod config;
