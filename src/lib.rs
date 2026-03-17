use crate::config::LanguageConfig;

mod config;
mod features;
mod languages;
mod parser;
mod util;

pub fn initialize() -> Vec<LanguageConfig> {
    languages::register_languages()
}

pub fn generate(
    input: &str,
    input_language: &str,
    output_language: &str,
    registry: &[LanguageConfig],
) -> Result<(), String> {
    let input_config = registry
        .iter()
        .find(|config| config.name == input_language)
        .ok_or(format!(
            "Input language '{}' not found in registry",
            input_language
        ))?;
    let output_config = registry
        .iter()
        .find(|config| config.name == output_language)
        .ok_or(format!(
            "Output language '{}' not found in registry",
            output_language
        ))?;
    let mut lexer = (input_config.build_lexer)(input);
    let _features =
        (input_config.parser)(&mut *lexer).map_err(|e| format!("Error parsing input: {:?}", e))?;
    Ok(())
}
