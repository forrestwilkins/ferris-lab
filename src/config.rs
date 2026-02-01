use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub agent_id: String,
    pub agent_port: u16,
    pub ollama_enabled: bool,
    pub ollama_host: String,
    pub ollama_model: String,
    pub direction: String,
    pub peer_addresses: Vec<String>,
}

impl Config {
    pub fn from_env() -> Self {
        let peer_addresses = env::var("PEER_ADDRESSES")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Self {
            agent_id: env::var("AGENT_ID").unwrap_or_else(|_| "agent-1".to_string()),
            agent_port: env::var("AGENT_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            ollama_enabled: env::var("OLLAMA_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .to_lowercase()
                != "false",
            ollama_host: env::var("OLLAMA_HOST")
                .unwrap_or_else(|_| "http://localhost:11434".to_string()),
            ollama_model: env::var("OLLAMA_MODEL").unwrap_or_else(|_| "gpt-oss:20b".to_string()),
            direction: env::var("DIRECTION").unwrap_or_else(|_| "roam".to_string()),
            peer_addresses,
        }
    }
}
