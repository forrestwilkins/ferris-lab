use crate::config::Config;
use crate::executor::Executor;
use crate::ollama::OllamaClient;
use crate::search::WebSearch;

pub struct Agent {
    pub config: Config,
    pub ollama: OllamaClient,
    pub executor: Executor,
    pub search: WebSearch,
}

impl Agent {
    pub fn new(config: Config) -> Self {
        let ollama = OllamaClient::new(config.ollama_host.clone(), config.ollama_model.clone());
        let executor = Executor::new("/workspace".to_string());
        let search = WebSearch::new();

        Self {
            config,
            ollama,
            executor,
            search,
        }
    }

    pub async fn run(&self) {
        println!("[{}] Agent starting...", self.config.agent_id);
        println!("[{}] Direction: {}", self.config.agent_id, self.config.direction);
        println!("[{}] Ollama: {}", self.config.agent_id, self.config.ollama_host);
        println!("[{}] Model: {}", self.config.agent_id, self.config.ollama_model);

        if self.ollama.is_available().await {
            println!("[{}] Ollama connection OK", self.config.agent_id);
        } else {
            println!("[{}] Warning: Ollama not available", self.config.agent_id);
        }

        // Placeholder for main agent loop (will be expanded in later steps)
        println!("[{}] Agent ready", self.config.agent_id);
    }
}
