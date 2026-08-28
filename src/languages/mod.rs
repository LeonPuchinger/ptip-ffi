use crate::config::LanguageConfig;

mod python;
mod typescript;

pub fn register_languages() -> Vec<LanguageConfig> {
    vec![python::register(), typescript::register()]
}
