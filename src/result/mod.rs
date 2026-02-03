//! Result parsing for agent output.
//!
//! This module provides functionality for parsing signals from agent output
//! and routing the orchestration loop accordingly.
//!
//! # Signal Types
//!
//! - **Review Result**: `RESULT: CONTINUE|REPEAT` - Quality assessment from review agent
//! - **Dev Done Signal**: `DEV_DONE: YES` - Completion claim from dev agent
//! - **Done Result**: `DONE: YES|NO` - Completion confirmation from done agent
//!
//! # Parsing Rules
//!
//! The parser follows these rules (from the specification):
//!
//! - Case-insensitive matching (D21)
//! - If multiple valid signal lines exist, the last one wins (D22)
//! - If no valid signal is found, defaults to safe fallback (D23)
//! - Signal must be at the start of a line (E17)
//! - Extra text after the value is not matched (E18)
//! - Empty output is treated as missing signal (E19)
//!
//! # Examples
//!
//! ```
//! use ralph::result::{ReviewResult, DoneResult, parse_dev_done};
//!
//! // Parse review result
//! let (result, was_explicit) = ReviewResult::parse("RESULT: CONTINUE");
//! assert_eq!(result, ReviewResult::Continue);
//! assert!(was_explicit);
//!
//! // Check if dev claims done
//! assert!(parse_dev_done("Work complete!\nDEV_DONE: YES"));
//!
//! // Parse done agent confirmation
//! let (result, was_explicit) = DoneResult::parse("DONE: YES");
//! assert_eq!(result, DoneResult::Done);
//! assert!(was_explicit);
//! ```

mod types;

pub use types::{parse_dev_done, DoneResult, ReviewResult};
