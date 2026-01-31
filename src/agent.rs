use crate::config::Config;
use crate::executor::Executor;
use crate::ollama::OllamaClient;
use crate::output;
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
        output::startup_banner(&self.config.agent_id);

        output::section("Configuration");
        output::config_item(&self.config.agent_id, "Direction", &self.config.direction);
        output::config_item(
            &self.config.agent_id,
            "Ollama",
            if self.config.ollama_enabled {
                "enabled ✓"
            } else {
                "disabled"
            },
        );
        if !self.config.peer_addresses.is_empty() {
            output::config_item(
                &self.config.agent_id,
                "Peers",
                &format!("{:?}", self.config.peer_addresses),
            );
        }

        // Start WebSocket server
        self.websocket.start().await;

        // Give the server a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Connect to peer agents
        let mut connected_count = 0;
        let mut failed_count = 0;

        output::section("Peer Connections");

        if self.config.peer_addresses.is_empty() {
            output::agent_info(
                &self.config.agent_id,
                "No peer addresses configured (PEER_ADDRESSES not set)",
            );
        } else {
            output::agent_status(
                &self.config.agent_id,
                &format!(
                    "Attempting to connect to {} peer(s)...",
                    self.config.peer_addresses.len()
                ),
            );

            for peer in &self.config.peer_addresses {
                match self.websocket.connect_to_peer(peer).await {
                    PeerConnectionResult::Connected(_) => {
                        connected_count += 1;
                    }
                    PeerConnectionResult::Failed(url, reason) => {
                        failed_count += 1;
                        output::agent_warn(
                            &self.config.agent_id,
                            &format!("Could not connect to {}: {}", url, reason),
                        );
                    }
                }
            }

            if connected_count > 0 {
                output::agent_success(
                    &self.config.agent_id,
                    &format!(
                        "Peer connection summary: {} connected, {} failed",
                        connected_count, failed_count
                    ),
                );
            } else {
                output::agent_warn(
                    &self.config.agent_id,
                    &format!(
                        "Peer connection summary: {} connected, {} failed",
                        connected_count, failed_count
                    ),
                );
            }
        }

        // Give time for any incoming connections to complete handshake
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Test peer communication
        output::section("Communication Test");

        if self.websocket.has_peers().await {
            output::agent_status(&self.config.agent_id, "Testing peer communication...");

            if self.config.ollama_enabled {
                // Send LLM-generated greeting
                if self.ollama.is_available().await {
                    let prompt = "Generate a brief, friendly greeting message to say hello to other AI agents you're collaborating with. Keep it under 20 words.";
                    match self.ollama.generate(prompt).await {
                        Ok(greeting) => {
                            let greeting = greeting.trim().to_string();
                            output::agent_success(
                                &self.config.agent_id,
                                &format!("Sending greeting to peers: \"{}\"", greeting),
                            );
                            self.websocket
                                .broadcast(AgentMessage::Text {
                                    agent_id: self.config.agent_id.clone(),
                                    content: greeting,
                                })
                                .await;
                        }
                        Err(e) => {
                            output::agent_warn(
                                &self.config.agent_id,
                                &format!(
                                    "Failed to generate greeting: {}, sending random number",
                                    e
                                ),
                            );
                            self.send_random_number().await;
                        }
                    }
                } else {
                    output::agent_warn(
                        &self.config.agent_id,
                        "Ollama not available, sending random number instead",
                    );
                    self.send_random_number().await;
                }
            } else {
                // Ollama disabled, send random number
                self.send_random_number().await;
            }
        } else {
            output::agent_info(
                &self.config.agent_id,
                "No peers connected, skipping communication test",
            );
        }

        // Run Ollama functionality tests only if enabled
        if self.config.ollama_enabled {
            output::section("Ollama LLM");
            output::config_item(&self.config.agent_id, "Host", &self.config.ollama_host);
            output::config_item(&self.config.agent_id, "Model", &self.config.ollama_model);

            // Fetch weather from the web
            let weather = match self
                .search
                .fetch_url("https://wttr.in/North+Carolina?format=%l:+%c+%t&u")
                .await
            {
                Ok(body) => {
                    let weather = body.trim().to_string();
                    output::agent_success(
                        &self.config.agent_id,
                        &format!("Weather fetched: {}", weather),
                    );
                    Some(weather)
                }
                Err(e) => {
                    output::agent_error(
                        &self.config.agent_id,
                        &format!("Web fetch failed: {}", e),
                    );
                    None
                }
            };

            // Use Ollama to describe the weather
            if self.ollama.is_available().await {
                output::agent_success(&self.config.agent_id, "Ollama connection OK ✓");

                if let Some(weather) = weather {
                    let prompt = format!(
                        "Based on this weather data: {}\n\nDescribe the weather in one short sentence - accurately based on the data.",
                        weather
                    );
                    match self.ollama.generate(&prompt).await {
                        Ok(response) => output::agent_info(
                            &self.config.agent_id,
                            &format!("🌤️  Weather: {}", response.trim()),
                        ),
                        Err(e) => output::agent_error(
                            &self.config.agent_id,
                            &format!("Ollama generate failed: {}", e),
                        ),
                    }
                }

                // Test code generation and file writing
                output::section("Code Generation");
                let code_prompt = "Write a simple Rust function called `add` that takes two i32 parameters and returns their sum. Only output the code, no explanation.";
                match self.ollama.generate(code_prompt).await {
                    Ok(code) => {
                        let code = code.trim();
                        match self.writer.write_file("test/add.rs", code).await {
                            Ok(path) => {
                                output::agent_success(
                                    &self.config.agent_id,
                                    &format!("Code written to: {}", path),
                                );
                                output::code_block(&self.config.agent_id, code);
                            }
                            Err(e) => output::agent_error(
                                &self.config.agent_id,
                                &format!("File write failed: {}", e),
                            ),
                        }
                    }
                    Err(e) => output::agent_error(
                        &self.config.agent_id,
                        &format!("Code generation failed: {}", e),
                    ),
                }
            } else {
                output::agent_warn(&self.config.agent_id, "Ollama not available");
            }
        } else {
            output::agent_info(&self.config.agent_id, "Ollama disabled, skipping LLM tests");
        }

        output::agent_ready(&self.config.agent_id, self.websocket.peer_count().await);

        // Keep the agent running and periodically retry peer connections
        let retry_interval = tokio::time::Duration::from_secs(10);
        let mut sent_first_message = self.websocket.has_peers().await;

        loop {
            tokio::time::sleep(retry_interval).await;

            // Retry connecting to any peers we're not connected to
            if !self.config.peer_addresses.is_empty() {
                for peer in &self.config.peer_addresses {
                    if !self.websocket.is_connected_to_url(peer).await {
                        output::agent_status(
                            &self.config.agent_id,
                            &format!("Retrying connection to {}...", peer),
                        );
                        match self.websocket.connect_to_peer(peer).await {
                            PeerConnectionResult::Connected(_) => {
                                output::agent_success(
                                    &self.config.agent_id,
                                    &format!("Successfully connected to {}", peer),
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
                output::peer_event(
                    &self.config.agent_id,
                    "First peer connected! Sending test message...",
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
        output::agent_info(
            &self.config.agent_id,
            &format!("Sending random number to peers: {}", value),
        );
        self.websocket
            .broadcast(AgentMessage::Number {
                agent_id: self.config.agent_id.clone(),
                value,
            })
            .await;
    }
}
