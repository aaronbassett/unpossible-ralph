//! Output capture utilities for agent execution.
//!
//! This module provides utilities for capturing and processing output from
//! agent commands, including UTF-8 handling for potentially invalid byte sequences.

use std::borrow::Cow;

/// Convert raw bytes to a UTF-8 string, replacing invalid sequences with U+FFFD.
///
/// This implements FR48: "System MUST replace invalid UTF-8 sequences with U+FFFD
/// (replacement character)".
///
/// # Arguments
///
/// * `bytes` - The raw byte output to convert.
///
/// # Returns
///
/// A valid UTF-8 string where any invalid byte sequences have been replaced
/// with the Unicode replacement character (U+FFFD).
///
/// # Example
///
/// ```
/// use ralph::agent::capture::lossy_utf8;
///
/// // Valid UTF-8 passes through unchanged
/// let valid = b"hello world";
/// assert_eq!(lossy_utf8(valid), "hello world");
///
/// // Invalid bytes are replaced with U+FFFD
/// let invalid = b"hello\xffworld";
/// assert_eq!(lossy_utf8(invalid), "hello\u{FFFD}world");
/// ```
pub fn lossy_utf8(bytes: &[u8]) -> String {
    match String::from_utf8_lossy(bytes) {
        Cow::Borrowed(s) => s.to_string(),
        Cow::Owned(s) => s,
    }
}

/// Capture output from a command execution.
///
/// This is a convenience struct for holding captured stdout and stderr
/// that has been converted to valid UTF-8.
#[derive(Debug, Clone, Default)]
struct CapturedOutput {
    /// Captured stdout as valid UTF-8.
    pub stdout: String,
    /// Captured stderr as valid UTF-8.
    pub stderr: String,
}

impl CapturedOutput {
    /// Create a new `CapturedOutput` from raw byte streams.
    ///
    /// Invalid UTF-8 sequences in either stream are replaced with U+FFFD.
    ///
    /// # Arguments
    ///
    /// * `stdout` - Raw stdout bytes.
    /// * `stderr` - Raw stderr bytes.
    ///
    /// # Example
    ///
    /// ```
    /// use ralph::agent::capture::CapturedOutput;
    ///
    /// let output = CapturedOutput::from_bytes(
    ///     b"stdout content",
    ///     b"stderr content"
    /// );
    ///
    /// assert_eq!(output.stdout, "stdout content");
    /// assert_eq!(output.stderr, "stderr content");
    /// ```
    pub fn from_bytes(stdout: &[u8], stderr: &[u8]) -> Self {
        Self {
            stdout: lossy_utf8(stdout),
            stderr: lossy_utf8(stderr),
        }
    }

    /// Check if both stdout and stderr are empty.
    #[allow(dead_code)]
    fn is_empty(&self) -> bool {
        self.stdout.is_empty() && self.stderr.is_empty()
    }

    /// Get the total length of captured output.
    #[allow(dead_code)]
    fn total_len(&self) -> usize {
        self.stdout.len() + self.stderr.len()
    }
}

/// Capture output from raw byte vectors, converting to valid UTF-8.
///
/// This is a convenience function for capturing stdout and stderr output,
/// converting invalid UTF-8 sequences to U+FFFD replacement characters.
///
/// # Arguments
///
/// * `stdout` - Raw stdout bytes.
/// * `stderr` - Raw stderr bytes.
///
/// # Returns
///
/// A tuple of `(stdout, stderr)` as valid UTF-8 strings.
pub fn capture_output(stdout: Vec<u8>, stderr: Vec<u8>) -> (String, String) {
    let output = CapturedOutput::from_bytes(&stdout, &stderr);
    (output.stdout, output.stderr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lossy_utf8_valid() {
        let input = b"hello world";
        assert_eq!(lossy_utf8(input), "hello world");
    }

    #[test]
    fn test_lossy_utf8_empty() {
        let input = b"";
        assert_eq!(lossy_utf8(input), "");
    }

    #[test]
    fn test_lossy_utf8_unicode() {
        let input = "こんにちは".as_bytes();
        assert_eq!(lossy_utf8(input), "こんにちは");
    }

    #[test]
    fn test_lossy_utf8_invalid_single_byte() {
        let input = b"\xff";
        assert_eq!(lossy_utf8(input), "\u{FFFD}");
    }

    #[test]
    fn test_lossy_utf8_invalid_surrounded() {
        let input = b"hello\xffworld";
        assert_eq!(lossy_utf8(input), "hello\u{FFFD}world");
    }

    #[test]
    fn test_lossy_utf8_multiple_invalid() {
        let input = b"\xff\xfe\xfd";
        let result = lossy_utf8(input);
        // Each invalid byte becomes a replacement character
        assert_eq!(result.chars().filter(|&c| c == '\u{FFFD}').count(), 3);
    }

    #[test]
    fn test_lossy_utf8_invalid_continuation() {
        // Invalid continuation byte after valid ASCII
        let input = b"a\x80b";
        let result = lossy_utf8(input);
        assert!(result.contains('\u{FFFD}'));
        assert!(result.starts_with('a'));
        assert!(result.ends_with('b'));
    }

    #[test]
    fn test_lossy_utf8_truncated_multibyte() {
        // Truncated 2-byte sequence (missing continuation)
        let input = b"\xc3"; // Start of 2-byte sequence
        let result = lossy_utf8(input);
        assert_eq!(result, "\u{FFFD}");
    }

    #[test]
    fn test_captured_output_from_bytes() {
        let output = CapturedOutput::from_bytes(b"stdout", b"stderr");
        assert_eq!(output.stdout, "stdout");
        assert_eq!(output.stderr, "stderr");
    }

    #[test]
    fn test_captured_output_empty() {
        let output = CapturedOutput::from_bytes(b"", b"");
        assert!(output.is_empty());
        assert_eq!(output.total_len(), 0);
    }

    #[test]
    fn test_captured_output_not_empty() {
        let output = CapturedOutput::from_bytes(b"hello", b"");
        assert!(!output.is_empty());
        assert_eq!(output.total_len(), 5);
    }

    #[test]
    fn test_captured_output_total_len() {
        let output = CapturedOutput::from_bytes(b"hello", b"world");
        assert_eq!(output.total_len(), 10);
    }

    #[test]
    fn test_captured_output_with_invalid_utf8() {
        let output = CapturedOutput::from_bytes(b"valid\xff", b"also\xfevalid");
        assert!(output.stdout.contains('\u{FFFD}'));
        assert!(output.stderr.contains('\u{FFFD}'));
    }

    #[test]
    fn test_capture_output_function() {
        let (stdout, stderr) = capture_output(b"stdout".to_vec(), b"stderr".to_vec());
        assert_eq!(stdout, "stdout");
        assert_eq!(stderr, "stderr");
    }

    #[test]
    fn test_captured_output_default() {
        let output = CapturedOutput::default();
        assert!(output.is_empty());
        assert_eq!(output.stdout, "");
        assert_eq!(output.stderr, "");
    }

    #[test]
    fn test_lossy_utf8_newlines_preserved() {
        let input = b"line1\nline2\r\nline3";
        assert_eq!(lossy_utf8(input), "line1\nline2\r\nline3");
    }

    #[test]
    fn test_lossy_utf8_null_bytes() {
        let input = b"hello\0world";
        assert_eq!(lossy_utf8(input), "hello\0world");
    }

    #[test]
    fn test_captured_output_clone() {
        let output = CapturedOutput::from_bytes(b"test", b"err");
        let cloned = output.clone();
        assert_eq!(cloned.stdout, output.stdout);
        assert_eq!(cloned.stderr, output.stderr);
    }
}
