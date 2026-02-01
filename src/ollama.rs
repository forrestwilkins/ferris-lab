use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OllamaError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Failed to parse response: {0}")]
    Parse(#[from] serde_json::Error),
}

#[derive(Debug, Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
}

#[derive(Debug, Deserialize)]
pub struct GenerateResponse {
    pub response: String,
    pub done: bool,
}

#[derive(Clone)]
pub struct OllamaClient {
    client: Client,
    host: String,
    model: String,
}

impl OllamaClient {
    pub fn new(host: String, model: String) -> Self {
        Self {
            client: Client::new(),
            host,
            model,
        }
    }

    pub async fn generate(&self, prompt: &str) -> Result<String, OllamaError> {
        let url = format!("{}/api/generate", self.host);
        let request = GenerateRequest {
            model: &self.model,
            prompt,
            stream: false,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?
            .json::<GenerateResponse>()
            .await?;

        Ok(response.response)
    }

    pub async fn is_available(&self) -> bool {
        let url = format!("{}/api/tags", self.host);
        self.client.get(&url).send().await.is_ok()
    }
}
