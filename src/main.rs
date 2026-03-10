use crate::parser::{ParserError, atoms::token_kind, lexer::Lexer};

mod config;
mod features;
mod languages;
mod parser;

fn main() {
    println!("Hello, world!");

    let languages = languages::register_languages();

    for lang in languages {
        println!("Language found: {}", lang.name);
    }
}
