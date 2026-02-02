//! Result types for parsing review agent output.
//!
//! This module provides the [`ReviewResult`] enum and associated parsing logic
//! for interpreting the `RESULT: CONTINUE|REPEAT|DONE` signal from review agents.

use regex::Regex;
use std::sync::LazyLock;

/// Regex pattern for matching RESULT lines.
///
/// Pattern: `(?i)^RESULT:\s*(CONTINUE|REPEAT|DONE)\s*$`
/// - Case-insensitive match
/// - Line must start with "RESULT:" (any case)
/// - Followed by optional whitespace
/// - Then one of: CONTINUE, REPEAT, DONE (any case)
/// - Optional trailing whitespace, then end of line
static RESULT_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?im)^RESULT:\s*(CONTINUE|REPEAT|DONE)\s*$")
        .expect("RESULT_PATTERN regex should be valid")
});

/// Parsed RESULT from review agent output.
///
/// This enum represents the three possible routing decisions that a review agent
/// can signal to control the orchestration loop's behavior.
///
/// # Parsing
///
/// Use [`ReviewResult::parse`] to extract the result from review agent output.
/// The parser looks for lines matching the pattern `RESULT: <VALUE>` where
/// `<VALUE>` is one of `CONTINUE`, `REPEAT`, or `DONE` (case-insensitive).
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
/// let (result, _) = ReviewResult::parse("result: done");
/// assert_eq!(result, ReviewResult::Done);
///
/// // Missing result defaults to Repeat
/// let (result, explicit) = ReviewResult::parse("No result here");
/// assert_eq!(result, ReviewResult::Repeat);
/// assert!(!explicit);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReviewResult {
    /// Work complete, proceed to next task.
    ///
    /// When the review agent signals `CONTINUE`, the orchestrator will:
    /// 1. Reset the retry count to 0
    /// 2. Run the next-action agent to generate the next prompt
    /// 3. Increment the iteration count
    /// 4. Continue the loop with the continuation prompt
    Continue,

    /// Work incomplete, retry current task.
    ///
    /// When the review agent signals `REPEAT`, the orchestrator will:
    /// 1. Increment the retry count
    /// 2. Check if max retries has been exceeded (exit with code 2 if so)
    /// 3. Re-run the dev agent with the same prompt
    ///
    /// This is also the default result when no valid RESULT line is found.
    Repeat,

    /// All work complete, exit loop.
    ///
    /// When the review agent signals `DONE`, the orchestrator will
    /// exit the loop successfully with exit code 0.
    Done,
}

impl ReviewResult {
    /// Parse RESULT from review agent output.
    ///
    /// Searches for lines matching the pattern `RESULT: <VALUE>` where
    /// `<VALUE>` is one of `CONTINUE`, `REPEAT`, or `DONE` (case-insensitive).
    ///
    /// # Returns
    ///
    /// Returns a tuple of `(ReviewResult, was_explicit)`:
    /// - `was_explicit` is `true` if a valid RESULT line was found
    /// - `was_explicit` is `false` if defaulting to `Repeat` due to:
    ///   - Empty output
    ///   - No RESULT line found
    ///   - Invalid RESULT value (e.g., "MAYBE")
    ///   - RESULT with extra text on the line
    ///
    /// # Parsing Rules
    ///
    /// - Case-insensitive matching for both "RESULT:" and the value
    /// - If multiple valid RESULT lines exist, the last one wins (D22)
    /// - RESULT must be at the start of a line (not in the middle of a word)
    /// - Only the exact values CONTINUE, REPEAT, DONE are accepted
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
    /// let (result, explicit) = ReviewResult::parse("RESULT: DONE - all tasks complete");
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
            // Group 1 contains the result value (CONTINUE, REPEAT, or DONE)
            if let Some(value) = caps.get(1) {
                let result = match value.as_str().to_uppercase().as_str() {
                    "CONTINUE" => Self::Continue,
                    "REPEAT" => Self::Repeat,
                    "DONE" => Self::Done,
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
    /// assert_eq!(ReviewResult::Done.as_str(), "DONE");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Continue => "CONTINUE",
            Self::Repeat => "REPEAT",
            Self::Done => "DONE",
        }
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

    #[test]
    fn parse_result_done() {
        let (result, explicit) = ReviewResult::parse("RESULT: DONE");
        assert_eq!(result, ReviewResult::Done);
        assert!(explicit);
    }

    // ==========================================================================
    // Case insensitivity tests (D21)
    // ==========================================================================

    #[test]
    fn parse_case_insensitive_lowercase_result_keyword() {
        let (result, explicit) = ReviewResult::parse("result: DONE");
        assert_eq!(result, ReviewResult::Done);
        assert!(explicit);
    }

    #[test]
    fn parse_case_insensitive_lowercase_value() {
        let (result, explicit) = ReviewResult::parse("RESULT: done");
        assert_eq!(result, ReviewResult::Done);
        assert!(explicit);
    }

    #[test]
    fn parse_case_insensitive_all_lowercase() {
        let (result, explicit) = ReviewResult::parse("result: done");
        assert_eq!(result, ReviewResult::Done);
        assert!(explicit);
    }

    #[test]
    fn parse_case_insensitive_mixed_case() {
        let (result, explicit) = ReviewResult::parse("Result: Done");
        assert_eq!(result, ReviewResult::Done);
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
    fn parse_multiple_results_last_wins_continue_then_done() {
        let output = "RESULT: CONTINUE\nRESULT: DONE";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Done);
        assert!(explicit);
    }

    #[test]
    fn parse_multiple_results_last_wins_three_results() {
        let output = "RESULT: DONE\nRESULT: REPEAT\nRESULT: CONTINUE";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Continue);
        assert!(explicit);
    }

    #[test]
    fn parse_multiple_results_with_mixed_case() {
        let output = "result: repeat\nRESULT: DONE";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Done);
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
        let output = "RESULT: MAYBE\nRESULT: DONE";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Done);
        assert!(explicit);
    }

    // ==========================================================================
    // Extra whitespace handling
    // ==========================================================================

    #[test]
    fn parse_whitespace_after_colon() {
        let (result, explicit) = ReviewResult::parse("RESULT:   DONE");
        assert_eq!(result, ReviewResult::Done);
        assert!(explicit);
    }

    #[test]
    fn parse_whitespace_after_value() {
        let (result, explicit) = ReviewResult::parse("RESULT: DONE   ");
        assert_eq!(result, ReviewResult::Done);
        assert!(explicit);
    }

    #[test]
    fn parse_whitespace_both_sides() {
        let (result, explicit) = ReviewResult::parse("RESULT:   DONE   ");
        assert_eq!(result, ReviewResult::Done);
        assert!(explicit);
    }

    #[test]
    fn parse_no_whitespace_after_colon() {
        let (result, explicit) = ReviewResult::parse("RESULT:DONE");
        assert_eq!(result, ReviewResult::Done);
        assert!(explicit);
    }

    #[test]
    fn parse_tabs_as_whitespace() {
        let (result, explicit) = ReviewResult::parse("RESULT:\t\tDONE\t");
        assert_eq!(result, ReviewResult::Done);
        assert!(explicit);
    }

    // ==========================================================================
    // RESULT in middle of word - E17 (not matched)
    // ==========================================================================

    #[test]
    fn parse_result_not_at_line_start() {
        let output = "The RESULT: DONE is shown here";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Repeat);
        assert!(!explicit);
    }

    #[test]
    fn parse_preresult_not_matched() {
        let output = "PRERESULT: DONE";
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
        let output = "  RESULT: DONE";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Repeat);
        assert!(!explicit);
    }

    // ==========================================================================
    // RESULT with extra text on line - E18 (not matched)
    // ==========================================================================

    #[test]
    fn parse_extra_text_after_value() {
        let output = "RESULT: DONE - all tasks complete";
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
        let output = "RESULT: DONE now";
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
        let output = "Everything looks good!\nRESULT: DONE";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Done);
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
        let output = "Review complete\r\nRESULT: DONE\r\nEnd of review";
        let (result, explicit) = ReviewResult::parse(output);
        assert_eq!(result, ReviewResult::Done);
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
    fn as_str_done() {
        assert_eq!(ReviewResult::Done.as_str(), "DONE");
    }

    #[test]
    fn display_trait() {
        assert_eq!(format!("{}", ReviewResult::Continue), "CONTINUE");
        assert_eq!(format!("{}", ReviewResult::Repeat), "REPEAT");
        assert_eq!(format!("{}", ReviewResult::Done), "DONE");
    }

    // ==========================================================================
    // Hash and equality tests (derive verification)
    // ==========================================================================

    #[test]
    fn equality() {
        assert_eq!(ReviewResult::Continue, ReviewResult::Continue);
        assert_eq!(ReviewResult::Repeat, ReviewResult::Repeat);
        assert_eq!(ReviewResult::Done, ReviewResult::Done);
        assert_ne!(ReviewResult::Continue, ReviewResult::Repeat);
        assert_ne!(ReviewResult::Continue, ReviewResult::Done);
        assert_ne!(ReviewResult::Repeat, ReviewResult::Done);
    }

    #[test]
    fn clone() {
        let original = ReviewResult::Continue;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn copy() {
        let original = ReviewResult::Done;
        let copied = original; // Copy, not move
        assert_eq!(original, copied);
    }

    #[test]
    fn debug() {
        assert_eq!(format!("{:?}", ReviewResult::Continue), "Continue");
        assert_eq!(format!("{:?}", ReviewResult::Repeat), "Repeat");
        assert_eq!(format!("{:?}", ReviewResult::Done), "Done");
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
        set.insert(ReviewResult::Done);

        assert_eq!(set.len(), 3);
        assert!(set.contains(&ReviewResult::Continue));
        assert!(set.contains(&ReviewResult::Repeat));
        assert!(set.contains(&ReviewResult::Done));
    }
}
