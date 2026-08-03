use crate::{config::LanguageConfig, error::PTIPFFIError};

mod codegen;
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
    let output_config = registry
        .iter()
        .find(|config| config.name == output_language)
        .ok_or(PTIPFFIError::LanguageNotFound(output_language.into()))?;
    let features = (input_config.parse)(input)?;
    let callee = (input_config.generate_callee)(vec![&features]);
    let caller = (output_config.generate_caller)(vec![&features]);
    Ok(())
}
