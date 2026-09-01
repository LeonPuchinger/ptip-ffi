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

pub fn clear_directory(path: &Path) -> std::io::Result<()> {
    if path.exists() {
        std::fs::remove_dir_all(path)?;
    }
    std::fs::create_dir_all(path)?;
    Ok(())
}

/// Recursively copy the contents of the source directory to the destination directory.
pub fn copy_directory(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !src.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Source path is not a directory: {}", src.display()),
        ));
    }
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_directory(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
