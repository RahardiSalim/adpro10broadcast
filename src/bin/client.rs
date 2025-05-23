use futures_util::{SinkExt, StreamExt};
use http::Uri;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::error::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_websockets::{ClientBuilder, Message};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatPacket {
    #[serde(rename = "messageType")]
    message_type: MessageType,
    data: Option<String>,
    #[serde(rename = "dataArray")]
    data_array: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum MessageType {
    Register,
    Users,
    Message,
}

#[derive(Deserialize)]
struct IncomingChat {
    from: String,
    message: String,
    time: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let username = std::env::args().nth(1).unwrap_or_else(|| {
        format!("guest-{}", rand::thread_rng().gen_range(1000..9999))
    });

    let uri = Uri::from_static("ws://127.0.0.1:8080");
    let (ws_stream, _) = ClientBuilder::from_uri(uri).connect().await?;
    let (mut sender, mut receiver) = ws_stream.split();

    // Send registration payload
    let registration = ChatPacket {
        message_type: MessageType::Register,
        data: Some(username.clone()),
        data_array: None,
    };
    let register_json = serde_json::to_string(&registration)?;
    sender.send(Message::text(register_json)).await?;

    println!("✅ You are connected as: {username}");

    // Create a channel for input messages
    let (input_tx, mut input_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    
    // Spawn input handler
    let input_handle = tokio::spawn(async move {
        let stdin = tokio::io::stdin();
        let mut lines = BufReader::new(stdin).lines();
        
        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            
            if input_tx.send(line).is_err() {
                break;
            }
        }
    });

    // Handle sending messages
    let send_handle = tokio::spawn(async move {
        while let Some(line) = input_rx.recv().await {
            let msg = ChatPacket {
                message_type: MessageType::Message,
                data: Some(line),
                data_array: None,
            };

            let msg_json = serde_json::to_string(&msg).unwrap();
            if sender.send(Message::text(msg_json)).await.is_err() {
                println!("❌ Failed to send message.");
                break;
            }
        }
    });

    // Spawn output handler
    let output_handle = tokio::spawn(async move {
        while let Some(incoming) = receiver.next().await {
            match incoming {
                Ok(msg) => {
                    if msg.is_close() {
                        println!("🔌 Server closed connection.");
                        break;
                    }
                    
                    if let Some(text) = msg.as_text() {
                        if let Ok(parsed) = serde_json::from_str::<ChatPacket>(text) {
                            match parsed.message_type {
                                MessageType::Users => {
                                    if let Some(users_list) = parsed.data_array {
                                        let users = users_list.join(", ");
                                        println!("👥 Online: {users}");
                                    }
                                }
                                MessageType::Message => {
                                    if let Some(raw) = parsed.data {
                                        if let Ok(chat) = serde_json::from_str::<IncomingChat>(&raw) {
                                            println!("💬 [{}] {}", chat.from, chat.message);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        } else {
                            println!("📩 {}", text);
                        }
                    }
                }
                Err(e) => {
                    println!("⚠️ Error receiving message: {e}");
                    break;
                }
            }
        }
    });

    // Wait for any task to complete
    tokio::select! {
        _ = input_handle => {},
        _ = send_handle => {},
        _ = output_handle => {},
    }
    
    Ok(())
}