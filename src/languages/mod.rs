use crate::config::LanguageConfig;

mod typescript;

pub fn register_languages() -> Vec<LanguageConfig<'static>> {
    vec![typescript::register()]
}
