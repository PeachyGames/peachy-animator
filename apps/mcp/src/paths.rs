use peachy_document::Document;
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Debug)]
pub struct WorkspaceRoot(PathBuf);

impl WorkspaceRoot {
    pub fn new(path: &Path) -> Result<Self, String> {
        let root = path
            .canonicalize()
            .map_err(|e| format!("Workspace root: {e}"))?;
        if !root.is_dir() {
            return Err("Workspace root must be an existing directory".into());
        }
        Ok(Self(root))
    }

    fn joined(&self, value: &str) -> Result<PathBuf, String> {
        let path = Path::new(value);
        if value.is_empty() || path.components().any(|c| matches!(c, Component::ParentDir)) {
            return Err("Paths must be nonempty and cannot contain '..'".into());
        }
        Ok(if path.is_absolute() {
            path.to_owned()
        } else {
            self.0.join(path)
        })
    }

    pub fn existing(&self, value: &str) -> Result<PathBuf, String> {
        self.checked(self.joined(value)?)
    }

    fn checked(&self, path: PathBuf) -> Result<PathBuf, String> {
        let path = path.canonicalize().map_err(|e| e.to_string())?;
        if !path.starts_with(&self.0) {
            return Err("Path resolves outside the configured workspace root".into());
        }
        Ok(path)
    }

    pub fn fresh(&self, value: &str) -> Result<PathBuf, String> {
        let path = self.joined(value)?;
        let name = path
            .file_name()
            .ok_or("Output needs a final name")?
            .to_owned();
        let parent = self.checked(path.parent().ok_or("Output needs a parent")?.to_path_buf())?;
        let output = parent.join(name);
        // symlink_metadata also detects a dangling symlink, unlike exists().
        match fs::symlink_metadata(&output) {
            Ok(_) => Err("Output already exists; choose a new name".into()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(output),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn check_assets(&self, document: &Document, asset_root: &Path) -> Result<(), String> {
        for image in document.image_paths() {
            let path = asset_root.join("images").join(format!("{image}.png"));
            let path = self
                .checked(path)
                .map_err(|e| format!("Image {image}: {e}"))?;
            if !path.is_file() {
                return Err(format!("Image {image} is not a regular file"));
            }
        }
        Ok(())
    }

    pub fn display(&self, path: &Path) -> String {
        path.strip_prefix(&self.0)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned()
    }
}
