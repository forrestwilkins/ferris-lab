use ferris_lab::websocket::{AgentMessage, PeerConnectionResult, WebSocketServer};
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

/// Helper to receive the next non-presence message
async fn recv_non_presence(rx: &mut mpsc::Receiver<AgentMessage>) -> Option<AgentMessage> {
    loop {
        match rx.recv().await {
            Some(AgentMessage::Presence { .. }) => continue,
            Some(AgentMessage::PresenceAck { .. }) => continue,
            other => return other,
        }
    }
}

/// Test that two agents can connect and exchange messages
#[tokio::test]
async fn test_two_agents_can_communicate() {
    // Create two WebSocket servers on different ports
    let server1 = WebSocketServer::new("agent-1".to_string(), 19001);
    let server2 = WebSocketServer::new("agent-2".to_string(), 19002);

    // Start both servers
    server1.start().await;
    server2.start().await;

    // Give servers time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Take the incoming message receivers before connecting
    let mut rx1 = server1
        .take_incoming_receiver()
        .await
        .expect("Should get receiver for agent-1");
    let mut rx2 = server2
        .take_incoming_receiver()
        .await
        .expect("Should get receiver for agent-2");

    // Agent 1 connects to Agent 2
    let result = server1.connect_to_peer("ws://localhost:19002/ws").await;
    assert!(
        matches!(result, PeerConnectionResult::Connected(_)),
        "Agent-1 should connect to Agent-2"
    );

    // Give time for handshake to complete
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify both see each other as peers
    assert!(server1.has_peers().await, "Agent-1 should have peers");
    assert!(server2.has_peers().await, "Agent-2 should have peers");

    // Agent 1 sends a number message
    let test_value = 42u64;
    server1
        .broadcast(AgentMessage::Number {
            agent_id: "agent-1".to_string(),
            value: test_value,
        })
        .await;

    // Agent 2 should receive the message (skip any presence messages)
    let received = timeout(Duration::from_secs(2), recv_non_presence(&mut rx2))
        .await
        .expect("Should receive within timeout")
        .expect("Should have a message");

    match received {
        AgentMessage::Number { agent_id, value } => {
            assert_eq!(agent_id, "agent-1");
            assert_eq!(value, test_value);
        }
        other => panic!("Expected Number message, got {:?}", other),
    }

    // Agent 2 sends a text message back
    let test_content = "Hello from agent-2!".to_string();
    server2
        .broadcast(AgentMessage::Text {
            agent_id: "agent-2".to_string(),
            content: test_content.clone(),
        })
        .await;

    // Agent 1 should receive the message (skip any presence messages)
    let received = timeout(Duration::from_secs(2), recv_non_presence(&mut rx1))
        .await
        .expect("Should receive within timeout")
        .expect("Should have a message");

    match received {
        AgentMessage::Text { agent_id, content } => {
            assert_eq!(agent_id, "agent-2");
            assert_eq!(content, test_content);
        }
        other => panic!("Expected Text message, got {:?}", other),
    }

    println!("Two-agent communication test passed!");
}

/// Test that connection fails gracefully when peer is not available
#[tokio::test]
async fn test_connection_fails_gracefully() {
    let server = WebSocketServer::new("agent-solo".to_string(), 19003);
    server.start().await;

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Try to connect to non-existent peer
    let result = server.connect_to_peer("ws://localhost:19999/ws").await;

    assert!(
        matches!(result, PeerConnectionResult::Failed(_, _)),
        "Should fail to connect to non-existent peer"
    );

    assert!(
        !server.has_peers().await,
        "Should have no peers after failed connection"
    );

    println!("Graceful failure test passed!");
}

/// Test that multiple agents can form a network
#[tokio::test]
async fn test_three_agent_network() {
    let server1 = WebSocketServer::new("agent-a".to_string(), 19004);
    let server2 = WebSocketServer::new("agent-b".to_string(), 19005);
    let server3 = WebSocketServer::new("agent-c".to_string(), 19006);

    // Start all servers
    server1.start().await;
    server2.start().await;
    server3.start().await;

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create a chain: 1 -> 2 -> 3
    let result1 = server1.connect_to_peer("ws://localhost:19005/ws").await;
    assert!(matches!(result1, PeerConnectionResult::Connected(_)));

    let result2 = server2.connect_to_peer("ws://localhost:19006/ws").await;
    assert!(matches!(result2, PeerConnectionResult::Connected(_)));

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Agent 1 should have 1 peer (agent 2)
    assert_eq!(server1.peer_count().await, 1, "Agent-A should have 1 peer");

    // Agent 2 should have 2 peers (agent 1 connected to it, and it connected to agent 3)
    assert_eq!(server2.peer_count().await, 2, "Agent-B should have 2 peers");

    // Agent 3 should have 1 peer (agent 2)
    assert_eq!(server3.peer_count().await, 1, "Agent-C should have 1 peer");

    println!("Three-agent network test passed!");
}

/// Test that retry mechanism works when peer starts late
#[tokio::test]
async fn test_retry_connects_to_late_peer() {
    let server1 = WebSocketServer::new("agent-early".to_string(), 19007);
    server1.start().await;

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Try to connect - should fail since server2 isn't running yet
    let result = server1.connect_to_peer("ws://localhost:19008/ws").await;
    assert!(matches!(result, PeerConnectionResult::Failed(_, _)));
    assert!(!server1.has_peers().await);

    // Now start server2
    let server2 = WebSocketServer::new("agent-late".to_string(), 19008);
    server2.start().await;

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Retry connection - should succeed now
    let result = server1.connect_to_peer("ws://localhost:19008/ws").await;
    assert!(
        matches!(result, PeerConnectionResult::Connected(_)),
        "Should connect on retry"
    );

    tokio::time::sleep(Duration::from_millis(200)).await;

    assert!(server1.has_peers().await, "Agent-early should have peers");
    assert!(server2.has_peers().await, "Agent-late should have peers");

    println!("Retry mechanism test passed!");
}
