use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub agent_id: String,
    pub ollama_host: String,
    pub ollama_model: String,
    pub direction: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            agent_id: env::var("AGENT_ID").unwrap_or_else(|_| "agent-1".to_string()),
            ollama_host: env::var("OLLAMA_HOST")
                .unwrap_or_else(|_| "http://localhost:11434".to_string()),
            ollama_model: env::var("OLLAMA_MODEL").unwrap_or_else(|_| "gpt-oss:20b".to_string()),
            direction: env::var("DIRECTION").unwrap_or_else(|_| "roam".to_string()),
        }
    }
}
