use futures_util::StreamExt;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use pixel_game_server::config::GameConfig;
use pixel_game_server::network::messages::{ClientMessage, ServerMessage};
use pixel_game_server::server::session::{self, send_msg, WaitingPlayer};

#[tokio::main]
async fn main() {
    let config = GameConfig::default();
    let lobby: Arc<Mutex<Option<WaitingPlayer>>> = Arc::new(Mutex::new(None));

    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.port))
        .await
        .expect("Failed to bind to address");
    println!(
        "Dragon Speedrun server listening on port {}",
        config.port
    );

    let mut player_counter: u64 = 0;

    while let Ok((stream, addr)) = listener.accept().await {
        let ws_stream = match accept_async(stream).await {
            Ok(ws) => ws,
            Err(e) => {
                eprintln!("WebSocket handshake failed for {}: {}", addr, e);
                continue;
            }
        };

        let (mut sender, mut receiver) = ws_stream.split();

        // Wait for the client to send a Join message
        let player_name = {
            let mut found_name = None;
            while let Some(msg) = receiver.next().await {
                if let Ok(WsMessage::Text(text)) = msg {
                    if let Ok(ClientMessage::Join { player_name }) =
                        serde_json::from_str(&text)
                    {
                        found_name = Some(player_name);
                        break;
                    }
                }
            }
            match found_name {
                Some(name) => name,
                None => {
                    eprintln!("Client {} disconnected before joining", addr);
                    continue;
                }
            }
        };

        player_counter += 1;
        let player_id = format!("player_{}", player_counter);

        // Acknowledge the join
        send_msg(
            &mut sender,
            &ServerMessage::Welcome {
                player_id: player_id.clone(),
            },
        )
        .await;

        println!("'{}' ({}) connected from {}", player_name, player_id, addr);

        // Match with a waiting player, or wait in the lobby
        let mut lobby_guard = lobby.lock().await;
        if let Some(waiting) = lobby_guard.take() {
            println!(
                "Match starting: '{}' vs '{}'",
                waiting.name, player_name
            );
            let config_clone = config.clone();
            tokio::spawn(async move {
                session::run_match(
                    waiting,
                    WaitingPlayer {
                        id: player_id,
                        name: player_name,
                        sender,
                        receiver,
                    },
                    config_clone,
                )
                .await;
            });
        } else {
            send_msg(&mut sender, &ServerMessage::WaitingForOpponent).await;
            println!("'{}' waiting for an opponent...", player_name);
            *lobby_guard = Some(WaitingPlayer {
                id: player_id,
                name: player_name,
                sender,
                receiver,
            });
        }
    }
}
