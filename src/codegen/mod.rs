use std::path::{Path, PathBuf};

pub mod template;

pub struct CodegenOutput {
    pub path: PathBuf,
    pub content: String,
}

impl CodegenOutput {
    /// Persist the codegen output to disk relative to the supplied `output_root`.
    /// If no `output_root` is provided, it is assumed that the `path` inside the
    /// `CodegenOutput` is already absolute and will be used as-is.
    pub fn persist(&self, output_root: Option<&Path>) -> std::io::Result<()> {
        let path = match output_root {
            Some(root) => root.join(&self.path),
            None => self.path.clone(),
        };
        // Resolve as an absolute path
        let path = path.canonicalize().unwrap_or(path);
        // Make sure the path exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, &self.content)
    }
}

/// Persist all codegen outputs to disk relative to the supplied `output_root`.
/// If no `output_root` is provided, it is assumed that the `path` inside
/// the instances of `CodegenOutput` is already absolute and will be used as-is.
pub fn persist_all(outputs: &[CodegenOutput], output_root: Option<&Path>) -> std::io::Result<()> {
    for output in outputs {
        output.persist(output_root)?;
    }
    Ok(())
}
