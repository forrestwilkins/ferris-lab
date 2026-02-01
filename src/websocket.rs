use crate::output;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
    routing::get,
    Router,
};
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::{timeout, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message as TungsteniteMessage};

/// Protocol message types for agent-to-agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentMessage {
    /// Announce presence when connecting
    #[serde(rename = "presence")]
    Presence { agent_id: String },

    /// Acknowledge a presence message
    #[serde(rename = "presence_ack")]
    PresenceAck { agent_id: String },

    /// Ping to check if peer is alive
    #[serde(rename = "ping")]
    Ping { agent_id: String, seq: u64 },

    /// Pong response to ping
    #[serde(rename = "pong")]
    Pong { agent_id: String, seq: u64 },

    /// Text message (for LLM-generated content)
    #[serde(rename = "text")]
    Text { agent_id: String, content: String },

    /// Number message (for simple testing without LLM)
    #[serde(rename = "number")]
    Number { agent_id: String, value: u64 },
}

impl AgentMessage {
    pub fn sender_id(&self) -> &str {
        match self {
            AgentMessage::Presence { agent_id } => agent_id,
            AgentMessage::PresenceAck { agent_id } => agent_id,
            AgentMessage::Ping { agent_id, .. } => agent_id,
            AgentMessage::Pong { agent_id, .. } => agent_id,
            AgentMessage::Text { agent_id, .. } => agent_id,
            AgentMessage::Number { agent_id, .. } => agent_id,
        }
    }
}

/// Result of attempting to connect to a peer
#[derive(Debug)]
pub enum PeerConnectionResult {
    Connected(String),
    Failed(String, String),
}

#[derive(Clone)]
pub struct WebSocketServer {
    agent_id: String,
    port: u16,
    tx: broadcast::Sender<String>,
    connected_peers: Arc<RwLock<HashSet<String>>>,
    connected_urls: Arc<RwLock<HashSet<String>>>,
    incoming_rx: Arc<RwLock<Option<mpsc::Receiver<AgentMessage>>>>,
    incoming_tx: mpsc::Sender<AgentMessage>,
}

impl WebSocketServer {
    pub fn new(agent_id: String, port: u16) -> Self {
        let (tx, _) = broadcast::channel(100);
        let (incoming_tx, incoming_rx) = mpsc::channel(100);
        Self {
            agent_id,
            port,
            tx,
            connected_peers: Arc::new(RwLock::new(HashSet::new())),
            connected_urls: Arc::new(RwLock::new(HashSet::new())),
            incoming_rx: Arc::new(RwLock::new(Some(incoming_rx))),
            incoming_tx,
        }
    }

    /// Check if we're already connected to a peer URL
    pub async fn is_connected_to_url(&self, url: &str) -> bool {
        self.connected_urls.read().await.contains(url)
    }

    pub async fn start(&self) {
        let tx = self.tx.clone();
        let agent_id = self.agent_id.clone();
        let connected_peers = self.connected_peers.clone();
        let incoming_tx = self.incoming_tx.clone();

        let app_state = AppState {
            tx,
            agent_id: agent_id.clone(),
            connected_peers,
            incoming_tx,
        };

        let app = Router::new()
            .route("/ws", get(ws_handler))
            .with_state(app_state);

        let addr = format!("0.0.0.0:{}", self.port);
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        output::agent_success(
            &agent_id,
            &format!("WebSocket server listening on :{}", self.port),
        );

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
    }

    /// Connect to a peer and return the result
    pub async fn connect_to_peer(&self, peer_url: &str) -> PeerConnectionResult {
        // Skip if already connected to this URL
        if self.is_connected_to_url(peer_url).await {
            return PeerConnectionResult::Connected(peer_url.to_string());
        }

        let agent_id = self.agent_id.clone();
        let peer_url_owned = peer_url.to_string();
        let connected_peers = self.connected_peers.clone();
        let connected_urls = self.connected_urls.clone();
        let mut rx = self.tx.subscribe();
        let incoming_tx = self.incoming_tx.clone();

        // Try to connect with a timeout
        let connect_result = timeout(Duration::from_secs(5), connect_async(&peer_url_owned)).await;

        match connect_result {
            Ok(Ok((ws_stream, _))) => {
                output::peer_event(&agent_id, &format!("Connected to peer: {}", peer_url_owned));
                let (mut write, mut read) = ws_stream.split();

                // Send presence message
                let presence = AgentMessage::Presence {
                    agent_id: agent_id.clone(),
                };
                let msg = serde_json::to_string(&presence).unwrap();
                let _ = write.send(TungsteniteMessage::Text(msg.into())).await;

                // Wait for presence_ack with timeout
                let ack_result = timeout(Duration::from_secs(3), async {
                    while let Some(Ok(msg)) = read.next().await {
                        if let TungsteniteMessage::Text(text) = msg {
                            if let Ok(parsed) = serde_json::from_str::<AgentMessage>(&text) {
                                if let AgentMessage::PresenceAck { agent_id: peer_id } = &parsed {
                                    return Some(peer_id.clone());
                                }
                                // Handle other messages
                                log_peer_message_received(&parsed);
                                let _ = incoming_tx.send(parsed).await;
                            }
                        }
                    }
                    None
                })
                .await;

                match ack_result {
                    Ok(Some(peer_id)) => {
                        output::agent_success(
                            &agent_id,
                            &format!("🤝 Handshake complete with peer: {}", peer_id),
                        );
                        connected_peers.write().await.insert(peer_id.clone());
                        connected_urls.write().await.insert(peer_url_owned.clone());

                        // Spawn tasks to handle ongoing communication
                        let agent_id_recv = agent_id.clone();
                        let incoming_tx_clone = incoming_tx.clone();
                        let connected_peers_clone = connected_peers.clone();
                        let connected_urls_clone = connected_urls.clone();
                        let peer_url_for_cleanup = peer_url_owned.clone();
                        tokio::spawn(async move {
                            while let Some(Ok(msg)) = read.next().await {
                                if let TungsteniteMessage::Text(text) = msg {
                                    if let Ok(parsed) = serde_json::from_str::<AgentMessage>(&text)
                                    {
                                        log_peer_message_received(&parsed);
                                        let _ = incoming_tx_clone.send(parsed).await;
                                    }
                                }
                            }
                            // Peer disconnected - only log if we actually removed them
                            let was_connected = connected_peers_clone.write().await.remove(&peer_id);
                            connected_urls_clone.write().await.remove(&peer_url_for_cleanup);
                            if was_connected {
                                output::agent_warn(&agent_id_recv, &format!("Peer disconnected: {}", peer_id));
                            }
                        });

                        let agent_id_send = agent_id.clone();
                        tokio::spawn(async move {
                            while let Ok(msg) = rx.recv().await {
                                if write
                                    .send(TungsteniteMessage::Text(msg.clone().into()))
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                                log_peer_message_sent(&agent_id_send, &msg);
                            }
                        });

                        PeerConnectionResult::Connected(peer_url_owned)
                    }
                    Ok(None) => {
                        let err = "Connection closed before handshake".to_string();
                        output::agent_error(
                            &agent_id,
                            &format!("Failed to connect to {}: {}", peer_url_owned, err),
                        );
                        PeerConnectionResult::Failed(peer_url_owned, err)
                    }
                    Err(_) => {
                        let err = "Handshake timeout".to_string();
                        output::agent_error(
                            &agent_id,
                            &format!("Failed to connect to {}: {}", peer_url_owned, err),
                        );
                        PeerConnectionResult::Failed(peer_url_owned, err)
                    }
                }
            }
            Ok(Err(e)) => {
                let err = e.to_string();
                output::agent_error(
                    &agent_id,
                    &format!("Failed to connect to {}: {}", peer_url_owned, err),
                );
                PeerConnectionResult::Failed(peer_url_owned, err)
            }
            Err(_) => {
                let err = "Connection timeout".to_string();
                output::agent_error(
                    &agent_id,
                    &format!("Failed to connect to {}: {}", peer_url_owned, err),
                );
                PeerConnectionResult::Failed(peer_url_owned, err)
            }
        }
    }

    /// Broadcast a message to all connected peers
    pub async fn broadcast(&self, message: AgentMessage) {
        let msg = serde_json::to_string(&message).unwrap();
        let _ = self.tx.send(msg);
    }

    /// Get the number of connected peers
    pub async fn peer_count(&self) -> usize {
        self.connected_peers.read().await.len()
    }

    /// Take the incoming message receiver (can only be called once)
    pub async fn take_incoming_receiver(&self) -> Option<mpsc::Receiver<AgentMessage>> {
        self.incoming_rx.write().await.take()
    }

    /// Check if we have any connected peers
    pub async fn has_peers(&self) -> bool {
        !self.connected_peers.read().await.is_empty()
    }

    /// Get all connected peer IDs
    pub async fn get_peer_ids(&self) -> Vec<String> {
        self.connected_peers.read().await.iter().cloned().collect()
    }
}

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<String>,
    agent_id: String,
    connected_peers: Arc<RwLock<HashSet<String>>>,
    incoming_tx: mpsc::Sender<AgentMessage>,
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (sender, receiver) = socket.split();
    let rx = state.tx.subscribe();
    let agent_id = state.agent_id.clone();
    let tx = state.tx.clone();
    let connected_peers = state.connected_peers.clone();
    let incoming_tx = state.incoming_tx.clone();

    // Spawn task to handle incoming messages
    let recv_agent_id = agent_id.clone();
    tokio::spawn(handle_incoming(
        receiver,
        tx,
        recv_agent_id,
        connected_peers,
        incoming_tx,
    ));

    // Spawn task to handle outgoing messages
    tokio::spawn(handle_outgoing(sender, rx, agent_id));
}

async fn handle_incoming(
    mut receiver: SplitStream<WebSocket>,
    tx: broadcast::Sender<String>,
    agent_id: String,
    connected_peers: Arc<RwLock<HashSet<String>>>,
    incoming_tx: mpsc::Sender<AgentMessage>,
) {
    let mut peer_agent_id: Option<String> = None;

    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            if let Ok(parsed) = serde_json::from_str::<AgentMessage>(&text) {
                match &parsed {
                    AgentMessage::Presence { agent_id: peer_id } => {
                        // New peer connected, send ack and track them
                        peer_agent_id = Some(peer_id.clone());
                        connected_peers.write().await.insert(peer_id.clone());
                        output::peer_event(&agent_id, &format!("Peer joined: {}", peer_id));

                        // Send presence ack
                        let ack = AgentMessage::PresenceAck {
                            agent_id: agent_id.clone(),
                        };
                        let ack_msg = serde_json::to_string(&ack).unwrap();
                        let _ = tx.send(ack_msg);
                    }
                    AgentMessage::Ping {
                        agent_id: _peer_id,
                        seq,
                    } => {
                        // Respond with pong
                        let pong = AgentMessage::Pong {
                            agent_id: agent_id.clone(),
                            seq: *seq,
                        };
                        let pong_msg = serde_json::to_string(&pong).unwrap();
                        let _ = tx.send(pong_msg);
                    }
                    _ => {
                        // Forward to incoming channel for agent to process
                        let _ = incoming_tx.send(parsed).await;
                    }
                }
            }
        }
    }

    // Peer disconnected - only log if we actually removed them
    if let Some(peer_id) = peer_agent_id {
        let was_connected = connected_peers.write().await.remove(&peer_id);
        if was_connected {
            output::agent_warn(&agent_id, &format!("Peer disconnected: {}", peer_id));
        }
    }
}

async fn handle_outgoing(
    mut sender: SplitSink<WebSocket, Message>,
    mut rx: broadcast::Receiver<String>,
    agent_id: String,
) {
    // Send our presence first
    let presence = AgentMessage::Presence {
        agent_id: agent_id.clone(),
    };
    let msg = serde_json::to_string(&presence).unwrap();
    let _ = sender.send(Message::Text(msg.into())).await;

    // Forward broadcast messages
    while let Ok(msg) = rx.recv().await {
        if sender.send(Message::Text(msg.clone().into())).await.is_err() {
            break;
        }
        log_peer_message_sent(&agent_id, &msg);
    }
}

fn log_peer_message_sent(agent_id: &str, raw: &str) {
    if let Ok(parsed) = serde_json::from_str::<AgentMessage>(raw) {
        match parsed {
            AgentMessage::Text { content, .. } => {
                output::peer_send_text(agent_id, content.trim_matches('"'))
            }
            AgentMessage::Number { value, .. } => output::peer_send_number(agent_id, value),
            _ => {}
        }
    }
}

fn log_peer_message_received(message: &AgentMessage) {
    match message {
        AgentMessage::Text { agent_id, content } => {
            output::peer_recv_text(agent_id, content.trim_matches('"'))
        }
        AgentMessage::Number { agent_id, value } => output::peer_recv_number(agent_id, *value),
        _ => {}
    }
}
