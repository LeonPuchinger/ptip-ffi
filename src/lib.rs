use crate::{config::LanguageConfig, error::PTIPFFIError};

mod config;
mod error;
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
) -> Result<(), PTIPFFIError> {
    let input_config = registry
        .iter()
        .find(|config| config.name == input_language)
        .ok_or(PTIPFFIError::LanguageNotFound(input_language.into()))?;
    let _output_config = registry
        .iter()
        .find(|config| config.name == output_language)
        .ok_or(PTIPFFIError::LanguageNotFound(output_language.into()))?;
    let _features = (input_config.parse)(input)?;
    Ok(())
}
