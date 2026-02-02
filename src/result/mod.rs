//! Result parsing for review agent output.
//!
//! This module provides functionality for parsing the `RESULT: CONTINUE|REPEAT|DONE`
//! signal from review agent output and routing the orchestration loop accordingly.
//!
//! # Overview
//!
//! The review agent communicates its decision via a structured signal embedded in its
//! output. This module provides the [`ReviewResult`] enum and parsing logic to extract
//! and interpret these signals.
//!
//! # Parsing Rules
//!
//! The parser follows these rules (from the specification):
//!
//! - Pattern: `(?i)^RESULT:\s*(CONTINUE|REPEAT|DONE)\s*$`
//! - Case-insensitive matching (D21)
//! - If multiple valid RESULT lines exist, the last one wins (D22)
//! - If no valid RESULT is found, defaults to `Repeat` with a warning (D23)
//! - RESULT must be at the start of a line (E17)
//! - Only the exact values CONTINUE, REPEAT, DONE are accepted (E16)
//! - Extra text after the value is not matched (E18)
//! - Empty output is treated as missing RESULT (E19)
//!
//! # Examples
//!
//! ```
//! use ralph::result::ReviewResult;
//!
//! // Parse from review output
//! let output = r#"
//! ## Review Summary
//! The code looks correct.
//!
//! RESULT: CONTINUE
//! "#;
//!
//! let (result, was_explicit) = ReviewResult::parse(output);
//! assert_eq!(result, ReviewResult::Continue);
//! assert!(was_explicit);
//!
//! // Handle missing result
//! let (result, was_explicit) = ReviewResult::parse("No signal here");
//! assert_eq!(result, ReviewResult::Repeat);
//! assert!(!was_explicit); // Indicates this was a default, not explicit
//! ```

mod types;

pub use types::ReviewResult;
