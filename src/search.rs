use reqwest::Client;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),
}

pub struct WebSearch {
    client: Client,
}

impl WebSearch {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn fetch_url(&self, url: &str) -> Result<String, SearchError> {
        let response = self.client.get(url).send().await?.text().await?;
        Ok(response)
    }
}

impl Default for WebSearch {
    fn default() -> Self {
        Self::new()
    }
}
