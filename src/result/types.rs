//! Result types for parsing agent output.
//!
//! This module provides:
//! - [`ReviewResult`] enum for interpreting `RESULT: CONTINUE|REPEAT` from review agents
//! - [`DoneResult`] enum for interpreting `DONE: YES|NO` from done agents
//! - [`parse_dev_done`] function for detecting `DEV_DONE: YES` in dev agent output

use regex::Regex;
use std::sync::LazyLock;

/// Regex pattern for matching RESULT lines (review agent).
///
/// Pattern: `(?im)^RESULT:\s*(CONTINUE|REPEAT)\s*$`
/// - Case-insensitive match
/// - Line must start with "RESULT:" (any case)
/// - Followed by optional whitespace
/// - Then one of: CONTINUE, REPEAT (any case)
/// - Optional trailing whitespace, then end of line
static RESULT_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?im)^RESULT:\s*(CONTINUE|REPEAT)\s*$")
        .expect("RESULT_PATTERN regex should be valid")
});

/// Regex pattern for matching DEV_DONE lines (dev agent).
///
/// Pattern: `(?im)^DEV_DONE:\s*YES\s*$`
/// - Case-insensitive match
/// - Line must start with "DEV_DONE:" (any case)
/// - Followed by optional whitespace
/// - Then "YES" (any case)
/// - Optional trailing whitespace, then end of line
static DEV_DONE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?im)^DEV_DONE:\s*YES\s*$").expect("DEV_DONE_PATTERN regex should be valid")
});

/// Regex pattern for matching DONE lines (done agent).
///
/// Pattern: `(?im)^DONE:\s*(YES|NO)\s*$`
/// - Case-insensitive match
/// - Line must start with "DONE:" (any case)
/// - Followed by optional whitespace
/// - Then "YES" or "NO" (any case)
/// - Optional trailing whitespace, then end of line
static DONE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?im)^DONE:\s*(YES|NO)\s*$").expect("DONE_PATTERN regex should be valid")
});

/// Parsed RESULT from review agent output.
///
/// This enum represents the two possible quality assessment decisions that a review agent
/// can signal. The review agent only assesses quality (CONTINUE/REPEAT), while completion
/// confirmation is handled by the separate done agent.
///
/// # Parsing
///
/// Use [`ReviewResult::parse`] to extract the result from review agent output.
/// The parser looks for lines matching the pattern `RESULT: <VALUE>` where
/// `<VALUE>` is one of `CONTINUE` or `REPEAT` (case-insensitive).
///
/// # Examples
///
/// ```
/// use ralph::result::ReviewResult;
///
/// // Basic parsing
/// let (result, explicit) = ReviewResult::parse("RESULT: CONTINUE");
/// assert_eq!(result, ReviewResult::Continue);
/// assert!(explicit);
///
/// // Case insensitive
/// let (result, _) = ReviewResult::parse("result: repeat");
/// assert_eq!(result, ReviewResult::Repeat);
///
/// // Missing result defaults to Repeat
/// let (result, explicit) = ReviewResult::parse("No result here");
/// assert_eq!(result, ReviewResult::Repeat);
/// assert!(!explicit);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReviewResult {
    /// Work quality is acceptable, proceed.
    ///
    /// When the review agent signals `CONTINUE`, the orchestrator will:
    /// 1. Check if the dev agent claimed completion (DEV_DONE: YES)
    /// 2. If dev claimed done, run the done agent to confirm
    /// 3. If done agent confirms, exit successfully
    /// 4. Otherwise, run next-action agent and continue to next iteration
    Continue,

    /// Work quality is not acceptable, retry current task.
    ///
    /// When the review agent signals `REPEAT`, the orchestrator will:
    /// 1. Increment the retry count
    /// 2. Check if max retries has been exceeded (exit with code 2 if so)
    /// 3. Re-run the dev agent with the same prompt
    ///
    /// This is also the default result when no valid RESULT line is found.
    Repeat,
}

impl ReviewResult {
    /// Parse RESULT from review agent output.
    ///
    /// Searches for lines matching the pattern `RESULT: <VALUE>` where
    /// `<VALUE>` is one of `CONTINUE` or `REPEAT` (case-insensitive).
    ///
    /// # Returns
    ///
    /// Returns a tuple of `(ReviewResult, was_explicit)`:
    /// - `was_explicit` is `true` if a valid RESULT line was found
    /// - `was_explicit` is `false` if defaulting to `Repeat` due to:
    ///   - Empty output
    ///   - No RESULT line found
    ///   - Invalid RESULT value (e.g., "MAYBE", "DONE")
    ///   - RESULT with extra text on the line
    ///
    /// # Parsing Rules
    ///
    /// - Case-insensitive matching for both "RESULT:" and the value
    /// - If multiple valid RESULT lines exist, the last one wins (D22)
    /// - RESULT must be at the start of a line (not in the middle of a word)
    /// - Only the exact values CONTINUE, REPEAT are accepted
    /// - Extra whitespace around the value is trimmed
    ///
    /// # Examples
    ///
    /// ```
    /// use ralph::result::ReviewResult;
    ///
    /// // Multiple results - last wins
    /// let output = "RESULT: REPEAT\nMore text\nRESULT: CONTINUE";
    /// let (result, explicit) = ReviewResult::parse(output);
    /// assert_eq!(result, ReviewResult::Continue);
    /// assert!(explicit);
    ///
    /// // Invalid value - defaults to Repeat
    /// let (result, explicit) = ReviewResult::parse("RESULT: MAYBE");
    /// assert_eq!(result, ReviewResult::Repeat);
    /// assert!(!explicit);
    ///
    /// // Extra text on line - not matched
    /// let (result, explicit) = ReviewResult::parse("RESULT: CONTINUE - proceed");
    /// assert_eq!(result, ReviewResult::Repeat);
    /// assert!(!explicit);
    /// ```
    pub fn parse(output: &str) -> (Self, bool) {
        // Handle empty output (E19)
        if output.is_empty() {
            return (Self::Repeat, false);
        }

        // Find all matches and use the last one (D22)
        let mut last_match: Option<Self> = None;

        for caps in RESULT_PATTERN.captures_iter(output) {
            // Group 1 contains the result value (CONTINUE or REPEAT)
            if let Some(value) = caps.get(1) {
                let result = match value.as_str().to_uppercase().as_str() {
                    "CONTINUE" => Self::Continue,
                    "REPEAT" => Self::Repeat,
                    // This shouldn't happen due to regex, but handle defensively
                    _ => continue,
                };
                last_match = Some(result);
            }
        }

        match last_match {
            Some(result) => (result, true),
            None => (Self::Repeat, false), // D23: Missing RESULT defaults to Repeat
        }
    }

    /// Returns the string representation of this result.
    ///
    /// # Examples
    ///
    /// ```
    /// use ralph::result::ReviewResult;
    ///
    /// assert_eq!(ReviewResult::Continue.as_str(), "CONTINUE");
    /// assert_eq!(ReviewResult::Repeat.as_str(), "REPEAT");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Continue => "CONTINUE",
            Self::Repeat => "REPEAT",
        }
    }
}

/// Parse DEV_DONE signal from dev agent output.
///
/// Searches for lines matching the pattern `DEV_DONE: YES` (case-insensitive).
/// The dev agent uses this signal to indicate it believes all work is complete.
///
/// # Returns
///
/// Returns `true` if `DEV_DONE: YES` was found, `false` otherwise.
///
/// # Examples
///
/// ```
/// use ralph::result::parse_dev_done;
///
/// assert!(parse_dev_done("Work complete!\nDEV_DONE: YES"));
/// assert!(parse_dev_done("dev_done: yes"));
/// assert!(!parse_dev_done("DEV_DONE: NO"));
/// assert!(!parse_dev_done("No signal here"));
/// ```
pub fn parse_dev_done(output: &str) -> bool {
    DEV_DONE_PATTERN.is_match(output)
}

/// Parsed result from done agent output.
///
/// The done agent independently confirms whether the project is complete by
/// examining the filesystem. It has no access to other agents' context.
///
/// # Parsing
///
/// Use [`DoneResult::parse`] to extract the result from done agent output.
/// The parser looks for lines matching `DONE: YES` or `DONE: NO` (case-insensitive).
///
/// # Examples
///
/// ```
/// use ralph::result::DoneResult;
///
/// let (result, explicit) = DoneResult::parse("DONE: YES");
/// assert_eq!(result, DoneResult::Done);
/// assert!(explicit);
///
/// let (result, explicit) = DoneResult::parse("DONE: NO");
/// assert_eq!(result, DoneResult::NotDone);
/// assert!(explicit);
///
/// // Missing signal defaults to NotDone
/// let (result, explicit) = DoneResult::parse("No signal");
/// assert_eq!(result, DoneResult::NotDone);
/// assert!(!explicit);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DoneResult {
    /// Project is confirmed complete.
    ///
    /// When the done agent signals `DONE: YES`, the orchestrator will
    /// exit the loop successfully with exit code 0.
    Done,

    /// Project is not yet complete.
    ///
    /// When the done agent signals `DONE: NO` (or no signal is found),
    /// the orchestrator will continue with the next-action agent flow.
    NotDone,
}

impl DoneResult {
    /// Parse DONE signal from done agent output.
    ///
    /// Searches for lines matching the pattern `DONE: YES` or `DONE: NO`
    /// (case-insensitive).
    ///
    /// # Returns
    ///
    /// Returns a tuple of `(DoneResult, was_explicit)`:
    /// - `was_explicit` is `true` if a valid DONE line was found
    /// - `was_explicit` is `false` if defaulting to `NotDone` due to:
    ///   - Empty output
    ///   - No DONE line found
    ///   - Invalid DONE value
    ///   - DONE with extra text on the line
    ///
    /// # Parsing Rules
    ///
    /// - Case-insensitive matching for both "DONE:" and the value
    /// - If multiple valid DONE lines exist, the last one wins
    /// - DONE must be at the start of a line
    /// - Only the exact values YES, NO are accepted
    /// - Extra whitespace around the value is trimmed
    ///
    /// # Examples
    ///
    /// ```
    /// use ralph::result::DoneResult;
    ///
    /// // Basic parsing
    /// let (result, explicit) = DoneResult::parse("DONE: YES");
    /// assert_eq!(result, DoneResult::Done);
    /// assert!(explicit);
    ///
    /// // Case insensitive
    /// let (result, _) = DoneResult::parse("done: no");
    /// assert_eq!(result, DoneResult::NotDone);
    ///
    /// // Missing signal defaults to NotDone
    /// let (result, explicit) = DoneResult::parse("No signal here");
    /// assert_eq!(result, DoneResult::NotDone);
    /// assert!(!explicit);
    /// ```
    pub fn parse(output: &str) -> (Self, bool) {
        // Handle empty output
        if output.is_empty() {
            return (Self::NotDone, false);
        }

        // Find all matches and use the last one
        let mut last_match: Option<Self> = None;

        for caps in DONE_PATTERN.captures_iter(output) {
            if let Some(value) = caps.get(1) {
                let result = match value.as_str().to_uppercase().as_str() {
                    "YES" => Self::Done,
                    "NO" => Self::NotDone,
                    _ => continue,
                };
                last_match = Some(result);
            }
        }

        match last_match {
            Some(result) => (result, true),
            None => (Self::NotDone, false),
        }
    }

    /// Returns the string representation of this result.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Done => "YES",
            Self::NotDone => "NO",
        }
    }
}

impl std::fmt::Display for DoneResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DONE: {}", self.as_str())
    }
}

impl std::fmt::Display for ReviewResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // Basic RESULT parsing tests
    // ==========================================================================

    #[test]
    fn parse_result_continue() {
        let (result, explicit) = ReviewResult::parse("RESULT: CONTINUE");
        assert_eq!(result, ReviewResult::Continue);
        assert!(explicit);
    }

    #[test]
    fn parse_result_repeat() {
        let (result, explicit) = ReviewResult::parse("RESULT: REPEAT");
        assert_eq!(result, ReviewResult::Repeat);
        assert!(explicit);
    }

    // Note: RESULT: DONE is no longer valid - done agent uses separate DONE: YES signal

    // ==========================================================================
    // Case insensitivity tests (D21)
    // ==========================================================================

    #[test]
    fn parse_case_insensitive_lowercase_result_keyword() {
        let (result, explicit) = ReviewResult::parse("result: CONTINUE");
        assert_eq!(result, ReviewResult::Continue);
        assert!(explicit);
    }

    #[test]
    fn parse_case_insensitive_lowercase_value() {
        let (result, explicit) = ReviewResult::parse("RESULT: repeat");
        assert_eq!(result, ReviewResult::Repeat);
        assert!(explicit);
    }

    #[test]
    fn parse_case_insensitive_all_lowercase() {
        let (result, explicit) = ReviewResult::parse("result: continue");
        assert_eq!(result, ReviewResult::Continue);
        assert!(explicit);
    }

    #[test]
    fn parse_case_insensitive_mixed_case() {
        let (result, explicit) = ReviewResult::parse("Result: Continue");
        assert_eq!(result, ReviewResult::Continue);
        assert!(explicit);
    }

    #[test]
    fn parse_case_insensitive_continue_variations() {
        let test_cases = [
            "RESULT: CONTINUE",
            "result: continue",
            "Result: Continue",
            "RESULT: continue",
            "result: CONTINUE",
        ];

        for input in test_cases {
            let (result, explicit) = ReviewResult::parse(input);
            assert_eq!(
                result,
                ReviewResult::Continue,
                "Failed for input: {}",
                input
            );
            assert!(explicit, "Should be explicit for input: {}", input);
        }
    }

    #[test]
    fn parse_case_insensitive_repeat_variations() {
        let test_cases = [
            "RESULT: REPEAT",
            "result: repeat",
            "Result: Repeat",
            "RESULT: repeat",
            "result: REPEAT",
        ];

        for input in test_cases {
            let (result, explicit) = ReviewResult::parse(input);
            assert_eq!(result, ReviewResult::Repeat, "Failed for input: {}", input);
            assert!(explicit, "Should be explicit for input: {}", input);
        }
    }

    // ==========================================================================
    // Multiple RESULT lines - last wins (D22)
    // ==========================================================================

    #[test]
    fn parse_multiple_results_last_wins_repeat_then_continue() {
        let output = "RESULT: REPEAT\nSome explanation\nRESULT: CONTINUE";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Continue);
        assert!(explicit);
    }

    #[test]
    fn parse_multiple_results_last_wins_continue_then_repeat() {
        let output = "RESULT: CONTINUE\nRESULT: REPEAT";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Repeat);
        assert!(explicit);
    }

    #[test]
    fn parse_multiple_results_last_wins_three_results() {
        let output = "RESULT: REPEAT\nRESULT: CONTINUE\nRESULT: REPEAT";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Repeat);
        assert!(explicit);
    }

    #[test]
    fn parse_multiple_results_with_mixed_case() {
        let output = "result: repeat\nRESULT: CONTINUE";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Continue);
        assert!(explicit);
    }

    // ==========================================================================
    // Missing RESULT - defaults to Repeat (D23)
    // ==========================================================================

    #[test]
    fn parse_no_result_line() {
        let output = "The code looks good but needs more tests.\nPlease add unit tests.";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Repeat);
        assert!(!explicit);
    }

    #[test]
    fn parse_empty_output_defaults_to_repeat() {
        let (result, explicit) = ReviewResult::parse("");
        assert_eq!(result, ReviewResult::Repeat);
        assert!(!explicit);
    }

    #[test]
    fn parse_whitespace_only_defaults_to_repeat() {
        let (result, explicit) = ReviewResult::parse("   \n\n   \t   ");
        assert_eq!(result, ReviewResult::Repeat);
        assert!(!explicit);
    }

    // ==========================================================================
    // Invalid RESULT values - E16 (e.g., MAYBE)
    // ==========================================================================

    #[test]
    fn parse_invalid_value_maybe_defaults_to_repeat() {
        let (result, explicit) = ReviewResult::parse("RESULT: MAYBE");
        assert_eq!(result, ReviewResult::Repeat);
        assert!(!explicit);
    }

    #[test]
    fn parse_invalid_value_success_defaults_to_repeat() {
        let (result, explicit) = ReviewResult::parse("RESULT: SUCCESS");
        assert_eq!(result, ReviewResult::Repeat);
        assert!(!explicit);
    }

    #[test]
    fn parse_invalid_value_fail_defaults_to_repeat() {
        let (result, explicit) = ReviewResult::parse("RESULT: FAIL");
        assert_eq!(result, ReviewResult::Repeat);
        assert!(!explicit);
    }

    #[test]
    fn parse_invalid_value_with_valid_later() {
        // Invalid followed by valid - valid is used
        let output = "RESULT: MAYBE\nRESULT: CONTINUE";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Continue);
        assert!(explicit);
    }

    #[test]
    fn parse_done_is_now_invalid_for_review() {
        // DONE is no longer valid for ReviewResult
        let (result, explicit) = ReviewResult::parse("RESULT: DONE");
        assert_eq!(result, ReviewResult::Repeat);
        assert!(!explicit);
    }

    // ==========================================================================
    // Extra whitespace handling
    // ==========================================================================

    #[test]
    fn parse_whitespace_after_colon() {
        let (result, explicit) = ReviewResult::parse("RESULT:   CONTINUE");
        assert_eq!(result, ReviewResult::Continue);
        assert!(explicit);
    }

    #[test]
    fn parse_whitespace_after_value() {
        let (result, explicit) = ReviewResult::parse("RESULT: CONTINUE   ");
        assert_eq!(result, ReviewResult::Continue);
        assert!(explicit);
    }

    #[test]
    fn parse_whitespace_both_sides() {
        let (result, explicit) = ReviewResult::parse("RESULT:   CONTINUE   ");
        assert_eq!(result, ReviewResult::Continue);
        assert!(explicit);
    }

    #[test]
    fn parse_no_whitespace_after_colon() {
        let (result, explicit) = ReviewResult::parse("RESULT:CONTINUE");
        assert_eq!(result, ReviewResult::Continue);
        assert!(explicit);
    }

    #[test]
    fn parse_tabs_as_whitespace() {
        let (result, explicit) = ReviewResult::parse("RESULT:\t\tCONTINUE\t");
        assert_eq!(result, ReviewResult::Continue);
        assert!(explicit);
    }

    // ==========================================================================
    // RESULT in middle of word - E17 (not matched)
    // ==========================================================================

    #[test]
    fn parse_result_not_at_line_start() {
        let output = "The RESULT: CONTINUE is shown here";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Repeat);
        assert!(!explicit);
    }

    #[test]
    fn parse_preresult_not_matched() {
        let output = "PRERESULT: CONTINUE";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Repeat);
        assert!(!explicit);
    }

    #[test]
    fn parse_myresult_not_matched() {
        let output = "MYRESULT: CONTINUE";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Repeat);
        assert!(!explicit);
    }

    #[test]
    fn parse_indented_result_not_matched() {
        // Leading whitespace means not at line start
        let output = "  RESULT: CONTINUE";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Repeat);
        assert!(!explicit);
    }

    // ==========================================================================
    // RESULT with extra text on line - E18 (not matched)
    // ==========================================================================

    #[test]
    fn parse_extra_text_after_value() {
        let output = "RESULT: CONTINUE - all tasks complete";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Repeat);
        assert!(!explicit);
    }

    #[test]
    fn parse_extra_text_comment_style() {
        let output = "RESULT: CONTINUE # move to next task";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Repeat);
        assert!(!explicit);
    }

    #[test]
    fn parse_extra_word_after_value() {
        let output = "RESULT: CONTINUE now";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Repeat);
        assert!(!explicit);
    }

    #[test]
    fn parse_parenthetical_after_value() {
        let output = "RESULT: REPEAT (needs more work)";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Repeat);
        assert!(!explicit);
    }

    // ==========================================================================
    // RESULT embedded in larger output
    // ==========================================================================

    #[test]
    fn parse_result_in_multiline_output() {
        let output = r#"
## Review Summary

The implementation looks correct. All tests pass and the code follows
the established patterns.

RESULT: CONTINUE

## Next Steps

Please proceed with the next task.
"#;
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Continue);
        assert!(explicit);
    }

    #[test]
    fn parse_result_at_end_of_output() {
        let output = "Everything looks good!\nRESULT: CONTINUE";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Continue);
        assert!(explicit);
    }

    #[test]
    fn parse_result_at_start_of_output() {
        let output = "RESULT: REPEAT\nThe tests are failing.";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Repeat);
        assert!(explicit);
    }

    #[test]
    fn parse_result_only_line() {
        let output = "RESULT: CONTINUE";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Continue);
        assert!(explicit);
    }

    // ==========================================================================
    // Edge cases with line endings
    // ==========================================================================

    #[test]
    fn parse_windows_line_endings() {
        let output = "Review complete\r\nRESULT: CONTINUE\r\nEnd of review";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Continue);
        assert!(explicit);
    }

    #[test]
    fn parse_mixed_line_endings() {
        let output = "Line 1\nRESULT: CONTINUE\r\nLine 3";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Continue);
        assert!(explicit);
    }

    // ==========================================================================
    // as_str and Display tests
    // ==========================================================================

    #[test]
    fn as_str_continue() {
        assert_eq!(ReviewResult::Continue.as_str(), "CONTINUE");
    }

    #[test]
    fn as_str_repeat() {
        assert_eq!(ReviewResult::Repeat.as_str(), "REPEAT");
    }

    #[test]
    fn display_trait() {
        assert_eq!(format!("{}", ReviewResult::Continue), "CONTINUE");
        assert_eq!(format!("{}", ReviewResult::Repeat), "REPEAT");
    }

    // ==========================================================================
    // Hash and equality tests (derive verification)
    // ==========================================================================

    #[test]
    fn equality() {
        assert_eq!(ReviewResult::Continue, ReviewResult::Continue);
        assert_eq!(ReviewResult::Repeat, ReviewResult::Repeat);
        assert_ne!(ReviewResult::Continue, ReviewResult::Repeat);
    }

    #[test]
    fn clone() {
        let original = ReviewResult::Continue;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn copy() {
        let original = ReviewResult::Continue;
        let copied = original; // Copy, not move
        assert_eq!(original, copied);
    }

    #[test]
    fn debug() {
        assert_eq!(format!("{:?}", ReviewResult::Continue), "Continue");
        assert_eq!(format!("{:?}", ReviewResult::Repeat), "Repeat");
    }

    // ==========================================================================
    // Hash trait tests
    // ==========================================================================

    #[test]
    fn hash_consistent() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(ReviewResult::Continue);
        set.insert(ReviewResult::Repeat);

        assert_eq!(set.len(), 2);
        assert!(set.contains(&ReviewResult::Continue));
        assert!(set.contains(&ReviewResult::Repeat));
    }

    // ==========================================================================
    // DEV_DONE parsing tests
    // ==========================================================================

    #[test]
    fn parse_dev_done_present() {
        assert!(parse_dev_done("Work complete!\nDEV_DONE: YES\nEnd"));
    }

    #[test]
    fn parse_dev_done_case_insensitive() {
        assert!(parse_dev_done("dev_done: yes"));
        assert!(parse_dev_done("Dev_Done: Yes"));
        assert!(parse_dev_done("DEV_DONE: YES"));
    }

    #[test]
    fn parse_dev_done_not_present() {
        assert!(!parse_dev_done("No signal here"));
        assert!(!parse_dev_done("DEV_DONE: NO"));
        assert!(!parse_dev_done(""));
    }

    #[test]
    fn parse_dev_done_with_whitespace() {
        assert!(parse_dev_done("DEV_DONE:   YES   "));
        assert!(parse_dev_done("DEV_DONE:\tYES\t"));
    }

    #[test]
    fn parse_dev_done_extra_text_not_matched() {
        assert!(!parse_dev_done("DEV_DONE: YES - all done"));
        assert!(!parse_dev_done("  DEV_DONE: YES")); // indented
    }

    // ==========================================================================
    // DoneResult parsing tests
    // ==========================================================================

    #[test]
    fn parse_done_result_yes() {
        let (result, explicit) = DoneResult::parse("DONE: YES");
        assert_eq!(result, DoneResult::Done);
        assert!(explicit);
    }

    #[test]
    fn parse_done_result_no() {
        let (result, explicit) = DoneResult::parse("DONE: NO");
        assert_eq!(result, DoneResult::NotDone);
        assert!(explicit);
    }

    #[test]
    fn parse_done_result_case_insensitive() {
        let (result, _) = DoneResult::parse("done: yes");
        assert_eq!(result, DoneResult::Done);

        let (result, _) = DoneResult::parse("Done: No");
        assert_eq!(result, DoneResult::NotDone);
    }

    #[test]
    fn parse_done_result_missing_defaults_to_not_done() {
        let (result, explicit) = DoneResult::parse("No signal here");
        assert_eq!(result, DoneResult::NotDone);
        assert!(!explicit);
    }

    #[test]
    fn parse_done_result_empty_defaults_to_not_done() {
        let (result, explicit) = DoneResult::parse("");
        assert_eq!(result, DoneResult::NotDone);
        assert!(!explicit);
    }

    #[test]
    fn parse_done_result_last_wins() {
        let output = "DONE: NO\nDONE: YES";
        let (result, explicit) = DoneResult::parse(output);
        assert_eq!(result, DoneResult::Done);
        assert!(explicit);
    }

    #[test]
    fn parse_done_result_with_whitespace() {
        let (result, _) = DoneResult::parse("DONE:   YES   ");
        assert_eq!(result, DoneResult::Done);
    }

    #[test]
    fn parse_done_result_extra_text_not_matched() {
        let (result, explicit) = DoneResult::parse("DONE: YES - all complete");
        assert_eq!(result, DoneResult::NotDone);
        assert!(!explicit);
    }

    #[test]
    fn done_result_as_str() {
        assert_eq!(DoneResult::Done.as_str(), "YES");
        assert_eq!(DoneResult::NotDone.as_str(), "NO");
    }

    #[test]
    fn done_result_display() {
        assert_eq!(format!("{}", DoneResult::Done), "DONE: YES");
        assert_eq!(format!("{}", DoneResult::NotDone), "DONE: NO");
    }
}
