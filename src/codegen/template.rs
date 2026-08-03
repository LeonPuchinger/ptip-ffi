use std::collections::HashMap;

use regex::Regex;

const NAME_PATTERN: &str = r"[A-Za-z_][A-Za-z0-9_]*";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateError {
    /// Returned when a template pattern does not contain exactly one named token.
    InvalidPattern,
}

/// Replaces named placeholders in input text based on a configurable pattern.
pub struct TemplateEngine {
    placeholder_regex: Regex,
}

impl TemplateEngine {
    /// Builds a template engine from a named placeholder pattern.
    ///
    /// The pattern must contain exactly one alphanumeric sequence of text referred to as the "name token".
    /// The name token is used to identify placeholders in the input text. The name token must match the regex `[A-Za-z_][A-Za-z0-9_]*`.
    /// The pattern can contain any other characters, which will be treated as literal text.
    /// For example, the pattern `${name}` has a name token of `name`, and the pattern `{{example}}` has a name token of `example`.
    /// The pattern `prefix_${some}_suffix` has a name token of `some`, with literal text before and after it.
    pub fn new(pattern: &str) -> Result<Self, TemplateError> {
        let token_regex = Regex::new(NAME_PATTERN).expect("failed to compile internal token regex");

        let mut matches = token_regex.find_iter(pattern);
        let token = matches.next().ok_or(TemplateError::InvalidPattern)?;
        if matches.next().is_some() {
            return Err(TemplateError::InvalidPattern);
        }

        let prefix = &pattern[..token.start()];
        let suffix = &pattern[token.end()..];
        let placeholder_pattern = format!(
            "{}(?P<key>{}){}",
            regex::escape(prefix),
            NAME_PATTERN,
            regex::escape(suffix)
        );

        let placeholder_regex =
            Regex::new(&placeholder_pattern).expect("failed to compile placeholder regex");

        Ok(Self { placeholder_regex })
    }

    /// Renders `input` by replacing placeholders whose keys exist in `values`.
    ///
    /// Placeholders with missing keys are left unchanged.
    /// The `indent_multiline` parameter controls whether multiline replacements
    /// are indented to match the indentation of the placeholder in the input text.
    pub fn render(
        &self,
        input: &str,
        values: &HashMap<&str, &str>,
        indent_multiline: bool,
    ) -> String {
        self.placeholder_regex
            .replace_all(input, |caps: &regex::Captures<'_>| {
                let Some(whole_match) = caps.get(0) else {
                    return String::new();
                };

                let Some(key_match) = caps.name("key") else {
                    return whole_match.as_str().to_string();
                };

                let replacement = values
                    .get(key_match.as_str())
                    .copied()
                    .unwrap_or(whole_match.as_str());

                if !indent_multiline || !replacement.contains('\n') {
                    return replacement.to_string();
                }

                let line_start = input[..whole_match.start()]
                    .rfind('\n')
                    .map_or(0, |index| index + 1);
                let line_prefix = &input[line_start..whole_match.start()];

                if !line_prefix
                    .chars()
                    .all(|character| character == ' ' || character == '\t')
                {
                    return replacement.to_string();
                }

                let mut rendered = String::with_capacity(replacement.len() + line_prefix.len());
                for (index, segment) in replacement.split_inclusive('\n').enumerate() {
                    if index > 0 {
                        rendered.push_str(line_prefix);
                    }

                    rendered.push_str(segment);
                }

                rendered
            })
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{TemplateEngine, TemplateError};

    #[test]
    fn replaces_single_placeholder() {
        let engine = TemplateEngine::new("${name}").expect("pattern should be valid");
        let values = HashMap::from([("name", "Steve")]);
        let rendered = engine.render("Hello ${name}!", &values, false);

        assert_eq!(rendered, "Hello Steve!");
    }

    #[test]
    fn replaces_multiple_placeholders() {
        let engine = TemplateEngine::new("${name}").expect("pattern should be valid");
        let values = HashMap::from([("greeting", "Hi"), ("name", "Steve")]);
        let rendered = engine.render("${greeting}, ${name}. ${greeting} again!", &values, false);

        assert_eq!(rendered, "Hi, Steve. Hi again!");
    }

    #[test]
    fn keeps_unknown_placeholders_unchanged() {
        let engine = TemplateEngine::new("${name}").expect("pattern should be valid");
        let values = HashMap::from([("known", "Alice")]);
        let rendered = engine.render("Hello ${known} and ${unknown}!", &values, false);

        assert_eq!(rendered, "Hello Alice and ${unknown}!");
    }

    #[test]
    fn supports_custom_pattern() {
        let engine = TemplateEngine::new("{{name}}").expect("pattern should be valid");
        let values = HashMap::from([("first", "Ada"), ("last", "Lovelace")]);
        let rendered = engine.render("{{first}} {{last}}", &values, false);

        assert_eq!(rendered, "Ada Lovelace");
    }

    #[test]
    fn indents_multiline_replacements_when_enabled() {
        let engine = TemplateEngine::new("${name}").expect("pattern should be valid");
        let values = HashMap::from([("name", "first\nsecond\nthird")]);

        let rendered = engine.render("    ${name}", &values, true);

        assert_eq!(rendered, "    first\n    second\n    third");
    }

    #[test]
    fn leaves_multiline_replacements_unindented_when_disabled() {
        let engine = TemplateEngine::new("${name}").expect("pattern should be valid");
        let values = HashMap::from([("name", "first\nsecond")]);

        let rendered = engine.render("    ${name}", &values, false);

        assert_eq!(rendered, "    first\nsecond");
    }

    #[test]
    fn rejects_pattern_without_name_token() {
        let result = TemplateEngine::new("${}");

        assert!(matches!(result, Err(TemplateError::InvalidPattern)));
    }

    #[test]
    fn rejects_pattern_with_multiple_name_tokens() {
        let result = TemplateEngine::new("${name}_${other}");

        assert!(matches!(result, Err(TemplateError::InvalidPattern)));
    }
}
