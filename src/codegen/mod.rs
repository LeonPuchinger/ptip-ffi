use crate::features::ModulePath;

mod template;

pub struct CodegenOutput {
    pub path: ModulePath,
    pub content: String,
}
