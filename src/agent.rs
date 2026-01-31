use crate::config::Config;
use crate::executor::Executor;
use crate::ollama::OllamaClient;
use crate::search::WebSearch;
use crate::websocket::{AgentMessage, PeerConnectionResult, WebSocketServer};
use crate::writer::FileWriter;
use rand::Rng;

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
        println!(
            "[{}] Ollama enabled: {}",
            self.config.agent_id, self.config.ollama_enabled
        );
        if !self.config.peer_addresses.is_empty() {
            println!(
                "[{}] Peer addresses: {:?}",
                self.config.agent_id, self.config.peer_addresses
            );
        }

        // Start WebSocket server
        self.websocket.start().await;

        // Give the server a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Connect to peer agents
        let mut connected_count = 0;
        let mut failed_count = 0;

        if self.config.peer_addresses.is_empty() {
            println!(
                "[{}] No peer addresses configured (PEER_ADDRESSES not set)",
                self.config.agent_id
            );
        } else {
            println!(
                "[{}] Attempting to connect to {} peer(s)...",
                self.config.agent_id,
                self.config.peer_addresses.len()
            );

            for peer in &self.config.peer_addresses {
                match self.websocket.connect_to_peer(peer).await {
                    PeerConnectionResult::Connected(_) => {
                        connected_count += 1;
                    }
                    PeerConnectionResult::Failed(url, reason) => {
                        failed_count += 1;
                        println!(
                            "[{}] Could not establish connection to {}: {}",
                            self.config.agent_id, url, reason
                        );
                    }
                }
            }

            println!(
                "[{}] Peer connection summary: {} connected, {} failed",
                self.config.agent_id, connected_count, failed_count
            );
        }

        // Give time for any incoming connections to complete handshake
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Test peer communication
        if self.websocket.has_peers().await {
            println!("[{}] Testing peer communication...", self.config.agent_id);

            if self.config.ollama_enabled {
                // Send LLM-generated greeting
                if self.ollama.is_available().await {
                    let prompt = "Generate a brief, friendly greeting message to say hello to other AI agents you're collaborating with. Keep it under 20 words.";
                    match self.ollama.generate(prompt).await {
                        Ok(greeting) => {
                            let greeting = greeting.trim().to_string();
                            println!(
                                "[{}] Sending LLM-generated greeting to peers: {}",
                                self.config.agent_id, greeting
                            );
                            self.websocket
                                .broadcast(AgentMessage::Text {
                                    agent_id: self.config.agent_id.clone(),
                                    content: greeting,
                                })
                                .await;
                        }
                        Err(e) => {
                            println!(
                                "[{}] Failed to generate greeting: {}, sending random number instead",
                                self.config.agent_id, e
                            );
                            self.send_random_number().await;
                        }
                    }
                } else {
                    println!(
                        "[{}] Ollama not available, sending random number instead",
                        self.config.agent_id
                    );
                    self.send_random_number().await;
                }
            } else {
                // Ollama disabled, send random number
                self.send_random_number().await;
            }
        } else {
            println!(
                "[{}] No peers connected, skipping communication test",
                self.config.agent_id
            );
        }

        // Run Ollama functionality tests only if enabled
        if self.config.ollama_enabled {
            println!(
                "[{}] Ollama: {}",
                self.config.agent_id, self.config.ollama_host
            );
            println!(
                "[{}] Model: {}",
                self.config.agent_id, self.config.ollama_model
            );

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

            // Use Ollama to describe the weather
            if self.ollama.is_available().await {
                println!("[{}] Ollama connection OK", self.config.agent_id);

                if let Some(weather) = weather {
                    let prompt = format!(
                        "Based on this weather data: {}\n\nDescribe the weather in one short sentence - accurately based on the data.",
                        weather
                    );
                    match self.ollama.generate(&prompt).await {
                        Ok(response) => {
                            println!("[{}] Weather: {}", self.config.agent_id, response.trim())
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
        println!(
            "[{}] Connected peers: {}",
            self.config.agent_id,
            self.websocket.peer_count().await
        );

        // Keep the agent running and periodically retry peer connections
        let retry_interval = tokio::time::Duration::from_secs(10);
        let mut sent_first_message = self.websocket.has_peers().await;

        loop {
            tokio::time::sleep(retry_interval).await;

            // Retry connecting to any peers we're not connected to
            if !self.config.peer_addresses.is_empty() {
                for peer in &self.config.peer_addresses {
                    if !self.websocket.is_connected_to_url(peer).await {
                        println!(
                            "[{}] Retrying connection to {}...",
                            self.config.agent_id, peer
                        );
                        match self.websocket.connect_to_peer(peer).await {
                            PeerConnectionResult::Connected(_) => {
                                println!(
                                    "[{}] Successfully connected to {}",
                                    self.config.agent_id, peer
                                );
                            }
                            PeerConnectionResult::Failed(_, _) => {
                                // Already logged in connect_to_peer
                            }
                        }
                    }
                }
            }

            // Send test message when we first get peers (if we didn't have any at startup)
            if !sent_first_message && self.websocket.has_peers().await {
                sent_first_message = true;
                println!(
                    "[{}] First peer connected! Sending test message...",
                    self.config.agent_id
                );
                if self.config.ollama_enabled {
                    if self.ollama.is_available().await {
                        let prompt = "Generate a brief, friendly greeting message to say hello to other AI agents. Keep it under 20 words.";
                        if let Ok(greeting) = self.ollama.generate(prompt).await {
                            self.websocket
                                .broadcast(AgentMessage::Text {
                                    agent_id: self.config.agent_id.clone(),
                                    content: greeting.trim().to_string(),
                                })
                                .await;
                        } else {
                            self.send_random_number().await;
                        }
                    } else {
                        self.send_random_number().await;
                    }
                } else {
                    self.send_random_number().await;
                }
            }
        }
    }

    async fn send_random_number(&self) {
        let value: u64 = rand::rng().random_range(1..=1000000);
        println!(
            "[{}] Sending random number to peers: {}",
            self.config.agent_id, value
        );
        self.websocket
            .broadcast(AgentMessage::Number {
                agent_id: self.config.agent_id.clone(),
                value,
            })
            .await;
    }
}
