use std::path::Path;
use std::process::Output;
use thiserror::Error;
use tokio::process::Command;

#[derive(Error, Debug)]
pub enum ExecutorError {
    #[error("Command failed to execute: {0}")]
    Io(#[from] std::io::Error),
    #[error("Command failed with status {status}: {stderr}")]
    CommandFailed { status: i32, stderr: String },
}

pub struct Executor {
    workspace: String,
}

impl Executor {
    pub fn new(workspace: String) -> Self {
        Self { workspace }
    }

    async fn run_cargo(&self, args: &[&str], project_dir: &str) -> Result<Output, ExecutorError> {
        let path = Path::new(&self.workspace).join(project_dir);
        let output = Command::new("cargo")
            .args(args)
            .current_dir(&path)
            .output()
            .await?;

        Ok(output)
    }

    pub async fn cargo_new(&self, name: &str) -> Result<String, ExecutorError> {
        let output = Command::new("cargo")
            .args(["new", name])
            .current_dir(&self.workspace)
            .output()
            .await?;

        if output.status.success() {
            Ok(format!("Created project: {}", name))
        } else {
            Err(ExecutorError::CommandFailed {
                status: output.status.code().unwrap_or(-1),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            })
        }
    }

    pub async fn cargo_build(&self, project_dir: &str) -> Result<String, ExecutorError> {
        let output = self.run_cargo(&["build"], project_dir).await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(ExecutorError::CommandFailed {
                status: output.status.code().unwrap_or(-1),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            })
        }
    }

    pub async fn cargo_run(&self, project_dir: &str) -> Result<String, ExecutorError> {
        let output = self.run_cargo(&["run"], project_dir).await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(ExecutorError::CommandFailed {
                status: output.status.code().unwrap_or(-1),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            })
        }
    }

    pub async fn cargo_test(&self, project_dir: &str) -> Result<String, ExecutorError> {
        let output = self.run_cargo(&["test"], project_dir).await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(ExecutorError::CommandFailed {
                status: output.status.code().unwrap_or(-1),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            })
        }
    }
}
