use std::path::Path;

use crate::{codegen::persist_all, config::LanguageConfig, error::PTIPFFIError};

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
    registry: &[LanguageConfig],
    input: &str,
    input_language: &str,
    output_language: &str,
    callee_output_root: &Path,
    caller_output_root: &Path,
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
    persist_all(&callee, Some(callee_output_root))?;
    persist_all(&caller, Some(caller_output_root))?;
    Ok(())
}
