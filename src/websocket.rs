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
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message as TungsteniteMessage};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentMessage {
    #[serde(rename = "presence")]
    Presence { agent_id: String },
    #[serde(rename = "chat")]
    Chat { agent_id: String, content: String },
}

pub struct WebSocketServer {
    agent_id: String,
    port: u16,
    tx: broadcast::Sender<String>,
    peers: Arc<RwLock<HashMap<String, broadcast::Sender<String>>>>,
}

impl WebSocketServer {
    pub fn new(agent_id: String, port: u16) -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            agent_id,
            port,
            tx,
            peers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start(&self) {
        let tx = self.tx.clone();
        let agent_id = self.agent_id.clone();
        let peers = self.peers.clone();

        let app_state = AppState {
            tx,
            agent_id: agent_id.clone(),
            peers,
        };

        let app = Router::new()
            .route("/ws", get(ws_handler))
            .with_state(app_state);

        let addr = format!("0.0.0.0:{}", self.port);
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        println!(
            "[{}] WebSocket server listening on :{}",
            agent_id, self.port
        );

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
    }

    pub async fn connect_to_peer(&self, peer_url: &str) {
        let agent_id = self.agent_id.clone();
        let peer_url = peer_url.to_string();
        let _peers = self.peers.clone();
        let mut rx = self.tx.subscribe();

        tokio::spawn(async move {
            match connect_async(&peer_url).await {
                Ok((ws_stream, _)) => {
                    println!("[{}] Connected to peer: {}", agent_id, peer_url);
                    let (mut write, mut read) = ws_stream.split();

                    // Send presence message
                    let presence = AgentMessage::Presence {
                        agent_id: agent_id.clone(),
                    };
                    let msg = serde_json::to_string(&presence).unwrap();
                    let _ = write.send(TungsteniteMessage::Text(msg.into())).await;

                    // Handle incoming messages from peer
                    let agent_id_clone = agent_id.clone();
                    tokio::spawn(async move {
                        while let Some(Ok(msg)) = read.next().await {
                            if let TungsteniteMessage::Text(text) = msg {
                                println!("[{}] Received: {}", agent_id_clone, text);
                            }
                        }
                    });

                    // Forward broadcast messages to peer
                    while let Ok(msg) = rx.recv().await {
                        if write
                            .send(TungsteniteMessage::Text(msg.into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                }
                Err(e) => {
                    println!("[{}] Failed to connect to {}: {}", agent_id, peer_url, e);
                }
            }
        });
    }

    pub async fn broadcast(&self, message: AgentMessage) {
        let msg = serde_json::to_string(&message).unwrap();
        let _ = self.tx.send(msg);
    }
}

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<String>,
    agent_id: String,
    peers: Arc<RwLock<HashMap<String, broadcast::Sender<String>>>>,
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (sender, receiver) = socket.split();
    let rx = state.tx.subscribe();
    let agent_id = state.agent_id.clone();
    let tx = state.tx.clone();

    // Spawn task to handle incoming messages
    let recv_agent_id = agent_id.clone();
    tokio::spawn(handle_incoming(receiver, tx, recv_agent_id));

    // Spawn task to handle outgoing messages
    tokio::spawn(handle_outgoing(sender, rx, agent_id));
}

async fn handle_incoming(
    mut receiver: SplitStream<WebSocket>,
    tx: broadcast::Sender<String>,
    agent_id: String,
) {
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            println!("[{}] Received: {}", agent_id, text);
            // Rebroadcast to other peers
            let _ = tx.send(text.to_string());
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
        if sender.send(Message::Text(msg.into())).await.is_err() {
            break;
        }
    }
}
