use std::path::PathBuf;

pub mod template;

pub struct CodegenOutput {
    pub path: PathBuf,
    pub content: String,
}
