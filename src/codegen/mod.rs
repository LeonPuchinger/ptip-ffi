use std::path::PathBuf;

mod template;

pub struct CodegenOutput {
    pub path: PathBuf,
    pub content: String,
}
