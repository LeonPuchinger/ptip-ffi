use crate::config::LanguageConfig;

mod cpp;
mod python;
mod typescript;

pub fn register_languages() -> Vec<LanguageConfig> {
    vec![cpp::register(), python::register(), typescript::register()]
}
