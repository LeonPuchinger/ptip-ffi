use std::collections::HashMap;

use regex::Regex;

const NAME_TOKEN_PATTERN: &str = r"[A-Za-z_][A-Za-z0-9_]*";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateError {
    InvalidPattern,
}

pub struct TemplateEngine {
    placeholder_regex: Regex,
}

impl TemplateEngine {
    pub fn new(pattern: &str) -> Result<Self, TemplateError> {
        let token_regex = Regex::new(NAME_TOKEN_PATTERN)
            .expect("failed to compile internal token regex");

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
            NAME_TOKEN_PATTERN,
            regex::escape(suffix)
        );

        let placeholder_regex =
            Regex::new(&placeholder_pattern).expect("failed to compile placeholder regex");

        Ok(Self { placeholder_regex })
    }

    pub fn render(&self, input: &str, values: &[(&str, &str)]) -> String {
        let value_map: HashMap<&str, &str> = values.iter().copied().collect();

        self.placeholder_regex
            .replace_all(input, |caps: &regex::Captures<'_>| {
                let Some(key_match) = caps.name("key") else {
                    return caps[0].to_string();
                };

                value_map
                    .get(key_match.as_str())
                    .copied()
                    .unwrap_or(caps.get(0).map_or("", |m| m.as_str()))
                    .to_string()
            })
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{TemplateEngine, TemplateError};

    #[test]
    fn replaces_single_placeholder() {
        let engine = TemplateEngine::new("${name}").expect("pattern should be valid");
        let rendered = engine.render("Hello ${name}!", &[("name", "Leon")]);

        assert_eq!(rendered, "Hello Leon!");
    }

    #[test]
    fn replaces_multiple_placeholders() {
        let engine = TemplateEngine::new("${name}").expect("pattern should be valid");
        let rendered = engine.render(
            "${greeting}, ${name}. ${greeting} again!",
            &[("greeting", "Hi"), ("name", "Leon")],
        );

        assert_eq!(rendered, "Hi, Leon. Hi again!");
    }

    #[test]
    fn keeps_unknown_placeholders_unchanged() {
        let engine = TemplateEngine::new("${name}").expect("pattern should be valid");
        let rendered = engine.render("Hello ${known} and ${unknown}!", &[("known", "Alice")]);

        assert_eq!(rendered, "Hello Alice and ${unknown}!");
    }

    #[test]
    fn supports_custom_pattern() {
        let engine = TemplateEngine::new("{{name}}").expect("pattern should be valid");
        let rendered = engine.render("{{first}} {{last}}", &[("first", "Ada"), ("last", "Lovelace")]);

        assert_eq!(rendered, "Ada Lovelace");
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