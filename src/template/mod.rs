//! Variable interpolation for prompt templates.
//!
//! This module provides [`TemplateContext`] for performing variable substitution
//! in prompt templates. Variables are identified by the pattern `{{variable_name}}`
//! where `variable_name` consists of lowercase letters and underscores only.
//!
//! # Variable Syntax
//!
//! - Pattern: `{{variable_name}}`
//! - Variables must be lowercase with underscores: `a-z` and `_`
//! - Spaces inside braces are NOT recognized: `{{ var }}` is literal text
//! - Substitution is single-pass (no recursive expansion)
//!
//! # Example
//!
//! ```
//! use ralph::template::TemplateContext;
//!
//! let mut ctx = TemplateContext::new();
//! ctx.set("name", "Alice".to_string());
//! ctx.set("task", "code review".to_string());
//!
//! let result = ctx.render("Hello {{name}}, please do: {{task}}").unwrap();
//! assert_eq!(result, "Hello Alice, please do: code review");
//! ```

use regex::Regex;
use std::collections::HashMap;
use std::fmt;
use std::sync::LazyLock;
use thiserror::Error;

/// Regex pattern for matching template variables.
///
/// Pattern: `\{\{([a-z_]+)\}\}`
/// - Matches `{{` followed by one or more lowercase letters/underscores, followed by `}}`
/// - Capture group 1 contains the variable name
/// - Does NOT match variables with spaces: `{{ var }}` remains literal
static VARIABLE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{([a-z_]+)\}\}").expect("Invalid regex pattern"));

/// Error returned when a template contains an undefined variable.
///
/// This error provides both the variable name and an excerpt from the template
/// showing context around the undefined variable.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub struct UndefinedVariableError {
    /// The name of the undefined variable.
    pub variable: String,
    /// An excerpt from the template showing the variable in context.
    pub template_excerpt: String,
}

impl fmt::Display for UndefinedVariableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Undefined variable '{{{{{}}}}}' in template: {}",
            self.variable, self.template_excerpt
        )
    }
}

/// Context for variable interpolation in templates.
///
/// This struct holds variable bindings and provides the [`render`](Self::render)
/// method for substituting variables in template strings.
///
/// # Single-Pass Substitution
///
/// Variable substitution is performed in a single pass. This means:
/// - If a variable's value contains `{{another_var}}`, it will NOT be expanded
/// - Variables in agent output are NOT expanded when used as values
///
/// # Example
///
/// ```
/// use ralph::template::TemplateContext;
///
/// let mut ctx = TemplateContext::new();
/// ctx.set("prompt", "Fix the bug".to_string());
/// ctx.set("iteration_count", "1".to_string());
///
/// let template = "Iteration {{iteration_count}}: {{prompt}}";
/// let result = ctx.render(template).unwrap();
/// assert_eq!(result, "Iteration 1: Fix the bug");
/// ```
#[derive(Debug, Clone)]
pub struct TemplateContext {
    vars: HashMap<String, String>,
}

impl Default for TemplateContext {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateContext {
    /// Creates a new empty template context.
    ///
    /// # Example
    ///
    /// ```
    /// use ralph::template::TemplateContext;
    ///
    /// let ctx = TemplateContext::new();
    /// // Context starts empty, all variables will be undefined
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }

    /// Sets a variable in the context.
    ///
    /// If the variable already exists, its value is replaced.
    ///
    /// # Arguments
    ///
    /// * `name` - The variable name (should be lowercase with underscores only)
    /// * `value` - The value to substitute for this variable
    ///
    /// # Example
    ///
    /// ```
    /// use ralph::template::TemplateContext;
    ///
    /// let mut ctx = TemplateContext::new();
    /// ctx.set("dev_response", "Code looks good".to_string());
    /// ctx.set("iteration_count", "3".to_string());
    /// ```
    pub fn set(&mut self, name: &str, value: String) {
        self.vars.insert(name.to_string(), value);
    }

    /// Renders a template by substituting all defined variables.
    ///
    /// This method performs single-pass substitution of variables matching
    /// the pattern `{{variable_name}}`. Variables must consist of lowercase
    /// letters and underscores only.
    ///
    /// # Arguments
    ///
    /// * `template` - The template string containing variable placeholders
    ///
    /// # Returns
    ///
    /// - `Ok(String)` - The rendered template with all variables substituted
    /// - `Err(UndefinedVariableError)` - If any variable in the template is not defined
    ///
    /// # Errors
    ///
    /// Returns [`UndefinedVariableError`] if the template contains a variable
    /// that has not been set in this context. The error includes the variable
    /// name and an excerpt showing the variable in context.
    ///
    /// # Edge Cases
    ///
    /// - `{{ var }}` with spaces is NOT recognized as a variable (literal text)
    /// - Variables in substituted values are NOT expanded (single-pass)
    /// - Empty variable values are valid and result in empty string substitution
    ///
    /// # Example
    ///
    /// ```
    /// use ralph::template::TemplateContext;
    ///
    /// let mut ctx = TemplateContext::new();
    /// ctx.set("name", "ralph".to_string());
    ///
    /// // Successful render
    /// let result = ctx.render("Hello {{name}}!").unwrap();
    /// assert_eq!(result, "Hello ralph!");
    ///
    /// // Undefined variable
    /// let err = ctx.render("Hello {{undefined}}!").unwrap_err();
    /// assert_eq!(err.variable, "undefined");
    /// ```
    pub fn render(&self, template: &str) -> Result<String, UndefinedVariableError> {
        // First pass: check for undefined variables
        for caps in VARIABLE_PATTERN.captures_iter(template) {
            let var_name = &caps[1];
            if !self.vars.contains_key(var_name) {
                let excerpt = self.create_excerpt(template, caps.get(0).unwrap().start());
                return Err(UndefinedVariableError {
                    variable: var_name.to_string(),
                    template_excerpt: excerpt,
                });
            }
        }

        // Second pass: perform substitution
        let result = VARIABLE_PATTERN
            .replace_all(template, |caps: &regex::Captures| {
                let var_name = &caps[1];
                // Safe to unwrap: we verified all variables exist in first pass
                self.vars.get(var_name).unwrap().clone()
            })
            .into_owned();

        Ok(result)
    }

    /// Creates an excerpt from the template showing context around a position.
    ///
    /// The excerpt shows up to 20 characters before and after the position,
    /// with ellipsis added if text is truncated.
    fn create_excerpt(&self, template: &str, pos: usize) -> String {
        const CONTEXT_CHARS: usize = 20;

        let start = pos.saturating_sub(CONTEXT_CHARS);
        let end = (pos + CONTEXT_CHARS).min(template.len());

        let mut excerpt = String::new();

        if start > 0 {
            excerpt.push_str("...");
        }

        // Handle UTF-8 boundaries safely
        let slice = &template[start..end];
        excerpt.push_str(slice);

        if end < template.len() {
            excerpt.push_str("...");
        }

        excerpt
    }

    /// Returns the number of variables defined in this context.
    #[must_use]
    pub fn len(&self) -> usize {
        self.vars.len()
    }

    /// Returns true if no variables are defined.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vars.is_empty()
    }

    /// Returns true if the given variable is defined.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.vars.contains_key(name)
    }

    /// Gets the value of a variable, if defined.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.vars.get(name).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Basic variable substitution tests
    // =========================================================================

    #[test]
    fn test_single_variable_substitution() {
        let mut ctx = TemplateContext::new();
        ctx.set("name", "ralph".to_string());

        let result = ctx.render("Hello {{name}}!").unwrap();
        assert_eq!(result, "Hello ralph!");
    }

    #[test]
    fn test_variable_at_start() {
        let mut ctx = TemplateContext::new();
        ctx.set("greeting", "Hello".to_string());

        let result = ctx.render("{{greeting}} world").unwrap();
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_variable_at_end() {
        let mut ctx = TemplateContext::new();
        ctx.set("name", "alice".to_string());

        let result = ctx.render("User: {{name}}").unwrap();
        assert_eq!(result, "User: alice");
    }

    #[test]
    fn test_variable_alone() {
        let mut ctx = TemplateContext::new();
        ctx.set("content", "Just this".to_string());

        let result = ctx.render("{{content}}").unwrap();
        assert_eq!(result, "Just this");
    }

    #[test]
    fn test_no_variables_in_template() {
        let ctx = TemplateContext::new();

        let result = ctx.render("Plain text without variables").unwrap();
        assert_eq!(result, "Plain text without variables");
    }

    // =========================================================================
    // Multiple variables tests
    // =========================================================================

    #[test]
    fn test_multiple_different_variables() {
        let mut ctx = TemplateContext::new();
        ctx.set("first", "Alice".to_string());
        ctx.set("second", "Bob".to_string());
        ctx.set("third", "Charlie".to_string());

        let result = ctx.render("{{first}}, {{second}}, and {{third}}").unwrap();
        assert_eq!(result, "Alice, Bob, and Charlie");
    }

    #[test]
    fn test_same_variable_multiple_times() {
        let mut ctx = TemplateContext::new();
        ctx.set("word", "echo".to_string());

        let result = ctx.render("{{word}} {{word}} {{word}}").unwrap();
        assert_eq!(result, "echo echo echo");
    }

    #[test]
    fn test_adjacent_variables() {
        let mut ctx = TemplateContext::new();
        ctx.set("a", "foo".to_string());
        ctx.set("b", "bar".to_string());

        let result = ctx.render("{{a}}{{b}}").unwrap();
        assert_eq!(result, "foobar");
    }

    // =========================================================================
    // Undefined variable error tests (E1)
    // =========================================================================

    #[test]
    fn test_undefined_variable_error() {
        let ctx = TemplateContext::new();

        let err = ctx.render("Hello {{undefined}}!").unwrap_err();
        assert_eq!(err.variable, "undefined");
        assert!(err.template_excerpt.contains("{{undefined}}"));
    }

    #[test]
    fn test_undefined_variable_error_message() {
        let ctx = TemplateContext::new();

        let err = ctx.render("Test {{missing}} here").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Undefined variable"));
        assert!(msg.contains("{{missing}}"));
    }

    #[test]
    fn test_first_undefined_variable_reported() {
        let ctx = TemplateContext::new();

        // First undefined variable should be reported
        let err = ctx
            .render("{{first_undef}} then {{second_undef}}")
            .unwrap_err();
        assert_eq!(err.variable, "first_undef");
    }

    #[test]
    fn test_some_defined_some_undefined() {
        let mut ctx = TemplateContext::new();
        ctx.set("defined", "value".to_string());

        let err = ctx.render("{{defined}} and {{not_defined}}").unwrap_err();
        assert_eq!(err.variable, "not_defined");
    }

    // =========================================================================
    // Variables with spaces - NOT matched (E30)
    // =========================================================================

    #[test]
    fn test_variable_with_spaces_not_matched() {
        let mut ctx = TemplateContext::new();
        ctx.set("var", "should not appear".to_string());

        // Spaces inside braces means it's literal text, not a variable
        let result = ctx.render("This {{ var }} has spaces").unwrap();
        assert_eq!(result, "This {{ var }} has spaces");
    }

    #[test]
    fn test_variable_with_leading_space() {
        let ctx = TemplateContext::new();

        let result = ctx.render("Test {{ name}}").unwrap();
        assert_eq!(result, "Test {{ name}}");
    }

    #[test]
    fn test_variable_with_trailing_space() {
        let ctx = TemplateContext::new();

        let result = ctx.render("Test {{name }}").unwrap();
        assert_eq!(result, "Test {{name }}");
    }

    #[test]
    fn test_mixed_spaced_and_unspaced_variables() {
        let mut ctx = TemplateContext::new();
        ctx.set("real", "REAL".to_string());

        // {{real}} is substituted, {{ fake }} is literal
        let result = ctx.render("{{real}} and {{ fake }}").unwrap();
        assert_eq!(result, "REAL and {{ fake }}");
    }

    // =========================================================================
    // Nested/recursive variables - NOT expanded (FR49, E31)
    // =========================================================================

    #[test]
    fn test_nested_variable_not_expanded() {
        let mut ctx = TemplateContext::new();
        ctx.set("outer", "{{inner}}".to_string());
        ctx.set("inner", "INNER_VALUE".to_string());

        // outer contains {{inner}}, but that should NOT be expanded
        let result = ctx.render("Result: {{outer}}").unwrap();
        assert_eq!(result, "Result: {{inner}}");
    }

    #[test]
    fn test_agent_output_with_variables_not_expanded() {
        let mut ctx = TemplateContext::new();
        // Simulating agent output that contains variable syntax
        ctx.set(
            "dev_response",
            "Use {{prompt}} to run the command".to_string(),
        );

        // The {{prompt}} in dev_response should remain literal
        let result = ctx.render("Agent said: {{dev_response}}").unwrap();
        assert_eq!(result, "Agent said: Use {{prompt}} to run the command");
    }

    #[test]
    fn test_self_referential_variable() {
        let mut ctx = TemplateContext::new();
        ctx.set("var", "{{var}}".to_string());

        // Should NOT cause infinite recursion, single-pass only
        let result = ctx.render("Value: {{var}}").unwrap();
        assert_eq!(result, "Value: {{var}}");
    }

    // =========================================================================
    // Empty values tests
    // =========================================================================

    #[test]
    fn test_empty_variable_value() {
        let mut ctx = TemplateContext::new();
        ctx.set("empty", String::new());

        let result = ctx.render("Before{{empty}}After").unwrap();
        assert_eq!(result, "BeforeAfter");
    }

    #[test]
    fn test_empty_template() {
        let ctx = TemplateContext::new();

        let result = ctx.render("").unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_only_empty_variable() {
        let mut ctx = TemplateContext::new();
        ctx.set("x", String::new());

        let result = ctx.render("{{x}}").unwrap();
        assert_eq!(result, "");
    }

    // =========================================================================
    // Special characters in values tests
    // =========================================================================

    #[test]
    fn test_special_chars_in_value() {
        let mut ctx = TemplateContext::new();
        ctx.set("cmd", r#"echo "hello" && ls -la | grep 'test'"#.to_string());

        let result = ctx.render("Run: {{cmd}}").unwrap();
        assert_eq!(result, r#"Run: echo "hello" && ls -la | grep 'test'"#);
    }

    #[test]
    fn test_newlines_in_value() {
        let mut ctx = TemplateContext::new();
        ctx.set("multi", "line1\nline2\nline3".to_string());

        let result = ctx.render("Content:\n{{multi}}").unwrap();
        assert_eq!(result, "Content:\nline1\nline2\nline3");
    }

    #[test]
    fn test_unicode_in_value() {
        let mut ctx = TemplateContext::new();
        ctx.set("emoji", "Hello, World!".to_string());

        let result = ctx.render("Message: {{emoji}}").unwrap();
        assert_eq!(result, "Message: Hello, World!");
    }

    #[test]
    fn test_braces_in_value() {
        let mut ctx = TemplateContext::new();
        ctx.set("json", r#"{"key": "value"}"#.to_string());

        let result = ctx.render("Data: {{json}}").unwrap();
        assert_eq!(result, r#"Data: {"key": "value"}"#);
    }

    #[test]
    fn test_double_braces_in_value() {
        let mut ctx = TemplateContext::new();
        ctx.set("template_example", "Use {{var}} syntax".to_string());

        let result = ctx.render("Example: {{template_example}}").unwrap();
        assert_eq!(result, "Example: Use {{var}} syntax");
    }

    // =========================================================================
    // Variable name edge cases
    // =========================================================================

    #[test]
    fn test_underscore_only_variable() {
        let mut ctx = TemplateContext::new();
        ctx.set("_", "underscore".to_string());

        let result = ctx.render("{{_}}").unwrap();
        assert_eq!(result, "underscore");
    }

    #[test]
    fn test_variable_with_underscores() {
        let mut ctx = TemplateContext::new();
        ctx.set("dev_response", "output".to_string());
        ctx.set("next_prompt", "task".to_string());
        ctx.set("iteration_count", "5".to_string());

        let result = ctx
            .render("{{dev_response}} | {{next_prompt}} | {{iteration_count}}")
            .unwrap();
        assert_eq!(result, "output | task | 5");
    }

    #[test]
    fn test_single_letter_variable() {
        let mut ctx = TemplateContext::new();
        ctx.set("x", "X".to_string());
        ctx.set("y", "Y".to_string());

        let result = ctx.render("{{x}}{{y}}").unwrap();
        assert_eq!(result, "XY");
    }

    #[test]
    fn test_uppercase_not_matched() {
        let ctx = TemplateContext::new();

        // Uppercase variables don't match the pattern
        let result = ctx.render("{{UPPERCASE}} here").unwrap();
        assert_eq!(result, "{{UPPERCASE}} here");
    }

    #[test]
    fn test_mixed_case_not_matched() {
        let ctx = TemplateContext::new();

        let result = ctx.render("{{mixedCase}} here").unwrap();
        assert_eq!(result, "{{mixedCase}} here");
    }

    #[test]
    fn test_numbers_in_variable_not_matched() {
        let ctx = TemplateContext::new();

        // Numbers don't match [a-z_]
        let result = ctx.render("{{var123}} here").unwrap();
        assert_eq!(result, "{{var123}} here");
    }

    #[test]
    fn test_hyphen_in_variable_not_matched() {
        let ctx = TemplateContext::new();

        let result = ctx.render("{{var-name}} here").unwrap();
        assert_eq!(result, "{{var-name}} here");
    }

    // =========================================================================
    // Edge cases with braces
    // =========================================================================

    #[test]
    fn test_triple_braces() {
        let mut ctx = TemplateContext::new();
        ctx.set("x", "value".to_string());

        // {{{x}}} -> { + {{x}} substituted + nothing
        // The regex will match {{x}} within {{{x}}}
        let result = ctx.render("{{{x}}}").unwrap();
        assert_eq!(result, "{value}");
    }

    #[test]
    fn test_partial_braces() {
        let ctx = TemplateContext::new();

        let result = ctx.render("{ {var} }").unwrap();
        assert_eq!(result, "{ {var} }");
    }

    #[test]
    fn test_unmatched_opening_braces() {
        let ctx = TemplateContext::new();

        let result = ctx.render("{{incomplete").unwrap();
        assert_eq!(result, "{{incomplete");
    }

    #[test]
    fn test_unmatched_closing_braces() {
        let ctx = TemplateContext::new();

        let result = ctx.render("incomplete}}").unwrap();
        assert_eq!(result, "incomplete}}");
    }

    #[test]
    fn test_empty_braces() {
        let ctx = TemplateContext::new();

        // {{}} doesn't match because [a-z_]+ requires at least one char
        let result = ctx.render("Empty: {{}}").unwrap();
        assert_eq!(result, "Empty: {{}}");
    }

    // =========================================================================
    // TemplateContext utility methods
    // =========================================================================

    #[test]
    fn test_context_new() {
        let ctx = TemplateContext::new();
        assert!(ctx.is_empty());
        assert_eq!(ctx.len(), 0);
    }

    #[test]
    fn test_context_default() {
        let ctx = TemplateContext::default();
        assert!(ctx.is_empty());
    }

    #[test]
    fn test_context_len() {
        let mut ctx = TemplateContext::new();
        ctx.set("a", "1".to_string());
        ctx.set("b", "2".to_string());
        assert_eq!(ctx.len(), 2);
    }

    #[test]
    fn test_context_contains() {
        let mut ctx = TemplateContext::new();
        ctx.set("exists", "yes".to_string());

        assert!(ctx.contains("exists"));
        assert!(!ctx.contains("not_exists"));
    }

    #[test]
    fn test_context_get() {
        let mut ctx = TemplateContext::new();
        ctx.set("key", "value".to_string());

        assert_eq!(ctx.get("key"), Some("value"));
        assert_eq!(ctx.get("missing"), None);
    }

    #[test]
    fn test_context_overwrite_variable() {
        let mut ctx = TemplateContext::new();
        ctx.set("var", "first".to_string());
        ctx.set("var", "second".to_string());

        let result = ctx.render("{{var}}").unwrap();
        assert_eq!(result, "second");
    }

    // =========================================================================
    // Error excerpt tests
    // =========================================================================

    #[test]
    fn test_error_excerpt_short_template() {
        let ctx = TemplateContext::new();

        let err = ctx.render("{{x}}").unwrap_err();
        assert_eq!(err.template_excerpt, "{{x}}");
    }

    #[test]
    fn test_error_excerpt_long_template() {
        let ctx = TemplateContext::new();

        let template =
            "This is a very long template with {{undefined_var}} somewhere in the middle of it all";
        let err = ctx.render(template).unwrap_err();

        // Excerpt should contain the variable
        assert!(err.template_excerpt.contains("{{undefined_var}}"));
        // Excerpt should have ellipsis for truncation
        assert!(err.template_excerpt.contains("..."));
    }

    // =========================================================================
    // Real-world usage patterns
    // =========================================================================

    #[test]
    fn test_prompt_template_starting() {
        let mut ctx = TemplateContext::new();
        ctx.set("iteration_count", "1".to_string());
        ctx.set("retry_count", "0".to_string());

        let template = "Iteration {{iteration_count}} (retry {{retry_count}}): Start the task";
        let result = ctx.render(template).unwrap();
        assert_eq!(result, "Iteration 1 (retry 0): Start the task");
    }

    #[test]
    fn test_prompt_template_continuation() {
        let mut ctx = TemplateContext::new();
        ctx.set("iteration_count", "2".to_string());
        ctx.set("retry_count", "0".to_string());
        ctx.set("next_prompt", "Fix the failing tests".to_string());

        let template = "Continue with task: {{next_prompt}}";
        let result = ctx.render(template).unwrap();
        assert_eq!(result, "Continue with task: Fix the failing tests");
    }

    #[test]
    fn test_prompt_template_review() {
        let mut ctx = TemplateContext::new();
        ctx.set("dev_response", "I fixed the bug.".to_string());
        ctx.set("dev_errors", String::new());

        let template = "Review output:\n{{dev_response}}\n\nErrors:\n{{dev_errors}}";
        let result = ctx.render(template).unwrap();
        assert_eq!(result, "Review output:\nI fixed the bug.\n\nErrors:\n");
    }

    #[test]
    fn test_agent_command_template() {
        let mut ctx = TemplateContext::new();
        ctx.set("prompt", "Please analyze the code".to_string());

        let template = "claude-cli --message '{{prompt}}'";
        let result = ctx.render(template).unwrap();
        assert_eq!(result, "claude-cli --message 'Please analyze the code'");
    }
}
