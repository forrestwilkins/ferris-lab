use crate::config::Config;
use crate::executor::Executor;
use crate::ollama::OllamaClient;
use crate::search::WebSearch;
use crate::websocket::{AgentMessage, WebSocketServer};
use crate::writer::FileWriter;

pub struct Agent {
    pub config: Config,
    pub ollama: OllamaClient,
    pub executor: Executor,
    pub search: WebSearch,
    pub writer: FileWriter,
    pub websocket: WebSocketServer,
}

impl Agent {
    pub fn new(config: Config) -> Self {
        let ollama = OllamaClient::new(config.ollama_host.clone(), config.ollama_model.clone());
        let executor = Executor::new("./workspace".to_string());
        let search = WebSearch::new();
        let writer = FileWriter::new("./workspace".to_string());
        let websocket = WebSocketServer::new(config.agent_id.clone(), config.agent_port);

        Self {
            config,
            ollama,
            executor,
            search,
            writer,
            websocket,
        }
    }

    pub async fn run(&self) {
        println!("[{}] Agent starting...", self.config.agent_id);
        println!(
            "[{}] Direction: {}",
            self.config.agent_id, self.config.direction
        );

        // Start WebSocket server
        self.websocket.start().await;

        // Connect to peer agents
        for peer in &self.config.peer_addresses {
            self.websocket.connect_to_peer(peer).await;
        }

        // Broadcast presence
        self.websocket
            .broadcast(AgentMessage::Presence {
                agent_id: self.config.agent_id.clone(),
            })
            .await;

        // Fetch weather from the web
        let weather = match self
            .search
            .fetch_url("https://wttr.in/North+Carolina?format=%l:+%c+%t&u")
            .await
        {
            Ok(body) => {
                let weather = body.trim().to_string();
                println!("[{}] Weather fetched: {}", self.config.agent_id, weather);
                Some(weather)
            }
            Err(e) => {
                println!("[{}] Web fetch failed: {}", self.config.agent_id, e);
                None
            }
        };

        // Run Ollama tests only if enabled
        if self.config.ollama_enabled {
            println!(
                "[{}] Ollama: {}",
                self.config.agent_id, self.config.ollama_host
            );
            println!(
                "[{}] Model: {}",
                self.config.agent_id, self.config.ollama_model
            );

            // Use Ollama to describe the weather
            if self.ollama.is_available().await {
                println!("[{}] Ollama connection OK", self.config.agent_id);

                if let Some(weather) = weather {
                    let prompt = format!(
                        "Based on this weather data: {}\n\n Describe the weather
                        in one short sentence - accurately based on the data.",
                        weather
                    );
                    match self.ollama.generate(&prompt).await {
                        Ok(response) => {
                            println!("[{}] 🤖: {}", self.config.agent_id, response.trim())
                        }
                        Err(e) => {
                            println!("[{}] Ollama generate failed: {}", self.config.agent_id, e)
                        }
                    }
                }

                // Test code generation and file writing
                let code_prompt = "Write a simple Rust function called `add` that takes two i32 parameters and returns their sum. Only output the code, no explanation.";
                match self.ollama.generate(code_prompt).await {
                    Ok(code) => {
                        let code = code.trim();
                        match self.writer.write_file("test/add.rs", code).await {
                            Ok(path) => {
                                println!("[{}] Code written to: {}", self.config.agent_id, path);
                                println!("[{}] Generated:\n{}", self.config.agent_id, code);
                            }
                            Err(e) => {
                                println!("[{}] File write failed: {}", self.config.agent_id, e)
                            }
                        }
                    }
                    Err(e) => println!("[{}] Code generation failed: {}", self.config.agent_id, e),
                }
            } else {
                println!("[{}] Warning: Ollama not available", self.config.agent_id);
            }
        } else {
            println!(
                "[{}] Ollama disabled, skipping LLM tests",
                self.config.agent_id
            );
        }

        println!("[{}] Agent ready", self.config.agent_id);

        // Keep the agent running to maintain WebSocket connections
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    }
}
