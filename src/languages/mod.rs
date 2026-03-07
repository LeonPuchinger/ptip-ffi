use crate::config::LanguageConfig;

mod typescript;

pub fn register_languages<'a>() -> Vec<LanguageConfig<'a>> {
    vec![typescript::register()]
}
