use std::path::Path;
use thiserror::Error;
use tokio::fs;

#[derive(Error, Debug)]
pub enum WriterError {
    #[error("Failed to write file: {0}")]
    Io(#[from] std::io::Error),
}

pub struct FileWriter {
    workspace: String,
}

impl FileWriter {
    pub fn new(workspace: String) -> Self {
        Self { workspace }
    }

    pub async fn write_file(&self, path: &str, content: &str) -> Result<String, WriterError> {
        let full_path = Path::new(&self.workspace).join(path);

        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&full_path, content).await?;
        Ok(full_path.to_string_lossy().to_string())
    }

    pub async fn read_file(&self, path: &str) -> Result<String, WriterError> {
        let full_path = Path::new(&self.workspace).join(path);
        let content = fs::read_to_string(&full_path).await?;
        Ok(content)
    }
}
