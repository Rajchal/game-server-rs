use pixel_game_server::network::messages::{ClientMessage, ServerMessage, Direction};

#[test]
fn test_client_message_serialization() {
    let msg = ClientMessage::Join {
        player_name: "Alice".to_string(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"type\":\"Join\""));
    assert!(json.contains("\"player_name\":\"Alice\""));
}

#[test]
fn test_client_message_deserialization() {
    let json = r#"{"type":"Move","direction":"Up"}"#;
    let msg: ClientMessage = serde_json::from_str(json).unwrap();
    match msg {
        ClientMessage::Move { direction } => {
            let (dx, dy) = direction.to_delta();
            assert_eq!(dx, 0);
            assert_eq!(dy, -1);
        }
        _ => panic!("Expected Move message"),
    }
}

#[test]
fn test_attack_message_deserialization() {
    let json = r#"{"type":"Attack"}"#;
    let msg: ClientMessage = serde_json::from_str(json).unwrap();
    assert!(matches!(msg, ClientMessage::Attack));
}

#[test]
fn test_server_message_serialization() {
    let msg = ServerMessage::Welcome {
        player_id: "player_1".to_string(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"type\":\"Welcome\""));
    assert!(json.contains("\"player_id\":\"player_1\""));
}

#[test]
fn test_direction_deltas() {
    assert_eq!(Direction::Up.to_delta(), (0, -1));
    assert_eq!(Direction::Down.to_delta(), (0, 1));
    assert_eq!(Direction::Left.to_delta(), (-1, 0));
    assert_eq!(Direction::Right.to_delta(), (1, 0));
}
