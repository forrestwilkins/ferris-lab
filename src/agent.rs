use crate::config::Config;
use crate::executor::Executor;
use crate::ollama::OllamaClient;
use crate::output;
use crate::search::WebSearch;
use crate::websocket::{AgentMessage, PeerConnectionResult, WebSocketServer};
use crate::writer::FileWriter;
use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Maximum messages per conversation (total across both agents)
const MAX_CONVERSATION_MESSAGES: usize = 4;

pub struct Agent {
    pub config: Config,
    pub ollama: OllamaClient,
    pub executor: Executor,
    pub search: WebSearch,
    pub writer: FileWriter,
    pub websocket: WebSocketServer,
    /// Track message counts per peer conversation: peer_id -> messages sent by us
    conversation_counts: Arc<RwLock<HashMap<String, usize>>>,
    /// Track which peers we've already logged as complete
    conversation_completed: Arc<RwLock<HashSet<String>>>,
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
            conversation_counts: Arc::new(RwLock::new(HashMap::new())),
            conversation_completed: Arc::new(RwLock::new(HashSet::new())),
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

        // Initialize Ollama before starting the WebSocket server
        let ollama_ready = if self.config.ollama_enabled {
            output::section("Ollama LLM");
            output::config_item(&self.config.agent_id, "Host", &self.config.ollama_host);
            output::config_item(&self.config.agent_id, "Model", &self.config.ollama_model);
            if self.ollama.is_available().await {
                output::agent_success(&self.config.agent_id, "Ollama connection OK ✓");
                true
            } else {
                output::agent_warn(&self.config.agent_id, "Ollama not available");
                false
            }
        } else {
            output::agent_info(&self.config.agent_id, "Ollama disabled");
            false
        };

        // Fetch weather before starting the WebSocket server
        output::section("Weather");
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
                output::agent_error(&self.config.agent_id, &format!("Web fetch failed: {}", e));
                None
            }
        };

        if let Some(weather) = weather {
            if self.config.ollama_enabled && ollama_ready {
                let prompt = format!(
                    "Based on this weather data: {}\n\nDescribe the weather in one short sentence - accurately based on the data.",
                    weather
                );
                match self.ollama.generate(&prompt).await {
                    Ok(response) => output::agent_info(
                        &self.config.agent_id,
                        &format!("🌤️  Weather: {}", response.trim()),
                    ),
                    Err(e) => {
                        output::agent_error(
                            &self.config.agent_id,
                            &format!("Ollama generate failed: {}", e),
                        );
                        output::agent_info(
                            &self.config.agent_id,
                            &format!("🌤️  Weather: {}", weather),
                        );
                    }
                }
            } else {
                output::agent_info(
                    &self.config.agent_id,
                    &format!("🌤️  Weather: {}", weather),
                );
            }
        }

        // Generate code before starting the WebSocket server
        if self.config.ollama_enabled && ollama_ready {
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
        } else if self.config.ollama_enabled {
            output::agent_warn(
                &self.config.agent_id,
                "Ollama not available, skipping code generation",
            );
        } else {
            output::agent_info(
                &self.config.agent_id,
                "Ollama disabled, skipping code generation",
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

        // Start conversation handler
        output::section("Agent Communication");

        // Take the incoming message receiver and spawn handler
        if let Some(mut incoming_rx) = self.websocket.take_incoming_receiver().await {
            let agent_id = self.config.agent_id.clone();
            let ollama = self.ollama.clone();
            let ollama_enabled = self.config.ollama_enabled;
            let websocket = self.websocket.clone();
            let conversation_counts = self.conversation_counts.clone();
            let conversation_completed = self.conversation_completed.clone();

            tokio::spawn(async move {
                while let Some(msg) = incoming_rx.recv().await {
                    if let AgentMessage::Text {
                        agent_id: peer_id,
                        content,
                    } = msg
                    {
                        // Check if we should respond (limit conversation length)
                        let our_count = {
                            let counts = conversation_counts.read().await;
                            counts.get(&peer_id).copied().unwrap_or(0)
                        };

                        // Each agent sends at most 2 messages (4 total in conversation)
                        if our_count >= MAX_CONVERSATION_MESSAGES / 2 {
                            let should_log = {
                                let mut completed = conversation_completed.write().await;
                                if completed.contains(&peer_id) {
                                    false
                                } else {
                                    completed.insert(peer_id.clone());
                                    true
                                }
                            };

                            if should_log {
                                output::agent_info(
                                    &agent_id,
                                    &format!(
                                        "Conversation with {} complete ({} messages sent)",
                                        peer_id, our_count
                                    ),
                                );
                            }
                            continue;
                        }

                        // Generate a response if Ollama is available
                        if ollama_enabled && ollama.is_available().await {
                            let prompt = format!(
                                "You are an AI agent named {}. Another AI agent named {} just said: \"{}\"\n\n\
                                 Generate a brief, friendly response (under 25 words). Be conversational but concise. \
                                 If this feels like a closing message, say a brief goodbye.",
                                agent_id, peer_id, content
                            );

                            match ollama.generate(&prompt).await {
                                Ok(response) => {
                                    let response = response.trim().to_string();

                                    // Update our message count
                                    let new_count = {
                                        let mut counts = conversation_counts.write().await;
                                        let entry = counts.entry(peer_id.clone()).or_insert(0);
                                        *entry += 1;
                                        *entry
                                    };

                                    websocket
                                        .broadcast(AgentMessage::Text {
                                            agent_id: agent_id.clone(),
                                            content: response,
                                        })
                                        .await;

                                    if new_count >= MAX_CONVERSATION_MESSAGES / 2 {
                                        let should_log = {
                                            let mut completed =
                                                conversation_completed.write().await;
                                            if completed.contains(&peer_id) {
                                                false
                                            } else {
                                                completed.insert(peer_id.clone());
                                                true
                                            }
                                        };

                                        if should_log {
                                            output::agent_info(
                                                &agent_id,
                                                &format!(
                                                    "Conversation with {} complete ({} messages sent)",
                                                    peer_id, new_count
                                                ),
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    output::agent_error(
                                        &agent_id,
                                        &format!("Failed to generate response: {}", e),
                                    );
                                }
                            }
                        }
                    }
                }
            });
        }

        // Only initiate conversation if we have the "lower" agent ID (to avoid both starting)
        // This ensures exactly one agent starts the conversation
        let has_peers = self.websocket.has_peers().await;
        let peers = self.websocket.get_peer_ids().await;

        if has_peers {
            let should_initiate = peers.iter().all(|peer| self.config.agent_id < *peer);

            if should_initiate && self.config.ollama_enabled && self.ollama.is_available().await {
                output::agent_status(&self.config.agent_id, "Initiating conversation with peers...");

                let prompt = "Generate a brief, friendly greeting to start a conversation with other AI agents. Keep it under 20 words. Be warm and inviting.";
                match self.ollama.generate(prompt).await {
                    Ok(greeting) => {
                        let greeting = greeting.trim().to_string();

                        // Count this as our first message to all peers
                        {
                            let mut counts = self.conversation_counts.write().await;
                            for peer in &peers {
                                *counts.entry(peer.clone()).or_insert(0) += 1;
                            }
                        }

                        output::agent_success(
                            &self.config.agent_id,
                            &format!("Starting conversation: \"{}\"", greeting),
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
                            &format!("Failed to generate greeting: {}", e),
                        );
                    }
                }
            } else if !should_initiate {
                output::agent_info(
                    &self.config.agent_id,
                    "Waiting for peer to initiate conversation...",
                );
            } else if !self.config.ollama_enabled {
                self.send_random_number().await;
            }
        } else {
            output::agent_info(
                &self.config.agent_id,
                "No peers connected, waiting for connections...",
            );
        }

        output::agent_ready(&self.config.agent_id, self.websocket.peer_count().await);

        // Quick poll for peers if we don't have any yet (check every 500ms for 5 seconds)
        let mut initiated_conversation = has_peers;
        if !initiated_conversation {
            for _ in 0..10 {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                if self.websocket.has_peers().await {
                    let peers = self.websocket.get_peer_ids().await;
                    let should_initiate = peers.iter().all(|peer| self.config.agent_id < *peer);

                    if should_initiate
                        && self.config.ollama_enabled
                        && self.ollama.is_available().await
                    {
                        initiated_conversation = true;
                        output::peer_event(
                            &self.config.agent_id,
                            "Peers connected! Starting conversation...",
                        );

                        let prompt = "Generate a brief, friendly greeting to start a conversation with other AI agents. Keep it under 20 words. Be warm and inviting.";
                        if let Ok(greeting) = self.ollama.generate(prompt).await {
                            let greeting = greeting.trim().to_string();

                            {
                                let mut counts = self.conversation_counts.write().await;
                                for peer in &peers {
                                    *counts.entry(peer.clone()).or_insert(0) += 1;
                                }
                            }

                            output::agent_success(
                                &self.config.agent_id,
                                &format!("Starting conversation: \"{}\"", greeting),
                            );
                            self.websocket
                                .broadcast(AgentMessage::Text {
                                    agent_id: self.config.agent_id.clone(),
                                    content: greeting,
                                })
                                .await;
                        }
                        break;
                    } else if !should_initiate {
                        // The other agent will initiate
                        initiated_conversation = true;
                        break;
                    }
                }
            }
        }

        // Keep the agent running and periodically retry peer connections
        let retry_interval = tokio::time::Duration::from_secs(10);

        loop {
            tokio::time::sleep(retry_interval).await;

            // Check if we should initiate now (if we have new peers and haven't initiated yet)
            if !initiated_conversation && self.websocket.has_peers().await {
                let peers = self.websocket.get_peer_ids().await;
                let should_initiate = peers.iter().all(|peer| self.config.agent_id < *peer);

                if should_initiate && self.config.ollama_enabled && self.ollama.is_available().await
                {
                    initiated_conversation = true;
                    output::peer_event(
                        &self.config.agent_id,
                        "Peers connected! Starting conversation...",
                    );

                    let prompt = "Generate a brief, friendly greeting to start a conversation with other AI agents. Keep it under 20 words. Be warm and inviting.";
                    if let Ok(greeting) = self.ollama.generate(prompt).await {
                        let greeting = greeting.trim().to_string();

                        {
                            let mut counts = self.conversation_counts.write().await;
                            for peer in &peers {
                                *counts.entry(peer.clone()).or_insert(0) += 1;
                            }
                        }

                        output::agent_success(
                            &self.config.agent_id,
                            &format!("Starting conversation: \"{}\"", greeting),
                        );
                        self.websocket
                            .broadcast(AgentMessage::Text {
                                agent_id: self.config.agent_id.clone(),
                                content: greeting,
                            })
                            .await;
                    }
                }
            }

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
