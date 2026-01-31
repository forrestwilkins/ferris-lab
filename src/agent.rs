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

        // Test Ollama connection
        if self.ollama.is_available().await {
            println!("[{}] Ollama connection OK", self.config.agent_id);

            match self.ollama.generate("Say hello in exactly 5 words.").await {
                Ok(response) => println!("[{}] Ollama test: {}", self.config.agent_id, response.trim()),
                Err(e) => println!("[{}] Ollama generate failed: {}", self.config.agent_id, e),
            }
        } else {
            println!("[{}] Warning: Ollama not available", self.config.agent_id);
        }

        // Test web fetch with weather
        match self.search.fetch_url("https://wttr.in/North+Carolina?format=%l:+%c+%t&u").await {
            Ok(body) => println!("[{}] Web fetch OK ({})", self.config.agent_id, body.trim()),
            Err(e) => println!("[{}] Web fetch failed: {}", self.config.agent_id, e),
        }

        println!("[{}] Agent ready", self.config.agent_id);
    }
}
