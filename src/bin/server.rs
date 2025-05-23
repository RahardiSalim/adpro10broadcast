use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::SystemTime};
use tokio::{net::TcpListener, sync::{broadcast, Mutex}};
use tokio_websockets::{Message, ServerBuilder, WebSocketStream};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct WsEvent {
    #[serde(rename = "messageType")]
    msg_type: MsgType,
    data: Option<String>,
    #[serde(rename = "dataArray")]
    data_array: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
enum MsgType {
    Register,
    Users,
    Message,
}

#[derive(Serialize, Deserialize)]
struct MessageMeta {
    from: String,
    message: String,
    time: u64,
}

#[derive(Clone)]
struct Session {
    name: String,
}

type SharedUsers = Arc<Mutex<HashMap<SocketAddr, Session>>>;

fn current_time() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

async fn notify_users(state: &SharedUsers, out: &broadcast::Sender<Message>) {
    let map = state.lock().await;
    let names: Vec<String> = map.values().map(|s| s.name.clone()).collect();
    let msg = WsEvent {
        msg_type: MsgType::Users,
        data: None,
        data_array: Some(names),
    };
    if let Ok(json) = serde_json::to_string(&msg) {
        let _ = out.send(Message::text(json));
    }
}

async fn chat_loop(
    addr: SocketAddr,
    ws: WebSocketStream<tokio::net::TcpStream>,
    users: SharedUsers,
    out: broadcast::Sender<Message>,
) {
    let (mut tx, mut rx) = ws.split();
    let mut name = format!("guest-{}", addr.port());
    let mut incoming = out.subscribe();

    // Task to push messages from broadcast to this client
    let push_task = tokio::spawn(async move {
        while let Ok(msg) = incoming.recv().await {
            if tx.send(msg).await.is_err() {
                break;
            }
        }
    });

    let user_ref = users.clone();
    let sender_ref = out.clone();

    // Task to pull messages from this client and process them
    let pull_task = tokio::spawn(async move {
        while let Some(msg_result) = rx.next().await {
            match msg_result {
                Ok(Message::Text(txt)) => {
                    if let Ok(event) = serde_json::from_str::<WsEvent>(&txt) {
                        match event.msg_type {
                            MsgType::Register => {
                                if let Some(new_name) = event.data {
                                    name = new_name;
                                    user_ref.lock().await.insert(
                                        addr,
                                        Session { name: name.clone() },
                                    );
                                    notify_users(&user_ref, &sender_ref).await;
                                }
                            }
                            MsgType::Message => {
                                let content = MessageMeta {
                                    from: name.clone(),
                                    message: event.data.unwrap_or_default(),
                                    time: current_time(),
                                };
                                
                                if let Ok(content_json) = serde_json::to_string(&content) {
                                    let packet = WsEvent {
                                        msg_type: MsgType::Message,
                                        data: Some(content_json),
                                        data_array: None,
                                    };
                                    
                                    if let Ok(packet_json) = serde_json::to_string(&packet) {
                                        let _ = sender_ref.send(Message::text(packet_json));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    println!("Client {} disconnected", addr);
                    break;
                }
                Err(e) => {
                    println!("Error from client {}: {}", addr, e);
                    break;
                }
                _ => {}
            }
        }

        // Clean up user when connection closes
        user_ref.lock().await.remove(&addr);
        notify_users(&user_ref, &sender_ref).await;
    });

    // Wait for either task to complete
    tokio::select! {
        _ = push_task => {},
        _ = pull_task => {},
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting WebSocket chat server on 127.0.0.1:8080");
    
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    let clients: SharedUsers = Arc::new(Mutex::new(HashMap::new()));
    let (broadcaster, _) = broadcast::channel(32);

    println!("📡 Server listening for connections...");

    while let Ok((stream, addr)) = listener.accept().await {
        println!("🔗 New connection from {}", addr);
        
        match ServerBuilder::new().accept(stream).await {
            Ok(ws) => {
                tokio::spawn(chat_loop(addr, ws, clients.clone(), broadcaster.clone()));
            }
            Err(e) => {
                println!("❌ Failed to accept WebSocket connection from {}: {}", addr, e);
            }
        }
    }

    Ok(())
}