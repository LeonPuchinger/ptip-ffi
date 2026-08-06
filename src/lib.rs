use std::path::Path;

use crate::{
    codegen::{copy_directory, persist_all},
    config::LanguageConfig,
    error::PTIPFFIError,
};

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
    library_root: &Path,
    library_entry_point: &Path,
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
    // The FFI currently only supports single module libraries
    let input = std::fs::read_to_string(
        library_entry_point
            .canonicalize()
            .unwrap_or_else(|_| library_entry_point.to_path_buf()),
    )?;
    // Parse the input module
    let features = (input_config.parse)(&input)?;
    // Perform code generation for both the callee and caller sides
    let callee = (input_config.generate_callee)(vec![&features]);
    let caller = (output_config.generate_caller)(vec![&features]);
    persist_all(&callee, Some(callee_output_root))?;
    persist_all(&caller, Some(caller_output_root))?;
    // Copy the library itself
    copy_directory(library_root, &callee_output_root.join("library"))?;
    Ok(())
}
