use axum::{
    extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
// use futures::{sink::SinkExt, stream::StreamExt}; // Removed unused imports
use shared::{ChatHistory, Message, Role, ServerMessage};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::fs;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const HISTORY_FILE: &str = "chat_history.json";

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "server=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load history
    let history = if let Ok(content) = fs::read_to_string(HISTORY_FILE).await {
        serde_json::from_str(&content).unwrap_or(ChatHistory { 
            messages: vec![],
            current_model: "llama-2-7b".to_string(),
        })
    } else {
        ChatHistory { 
            messages: vec![],
            current_model: "llama-2-7b".to_string(),
        }
    };

    let app_state = Arc::new(Mutex::new(history));

    let app = Router::new()
        .route("/ws", get(|ws| ws_handler(ws, app_state)));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    tracing::info!("listening on {}", addr);
    
    // Attempt to start llama.cpp server
    // Assuming 'llama-server' is in PATH. If not, this will error but we'll catch it.
    tracing::info!("Attempting to start llama.cpp server...");
    let mut llama_process = tokio::process::Command::new("llama-server")
        .arg("--port")
        .arg("8080") // Standard llama.cpp port
        .stdout(std::process::Stdio::null()) // Suppress output for clean TUI, or redirect
        .stderr(std::process::Stdio::null())
        .spawn();

    if let Ok(_) = &mut llama_process {
        tracing::info!("llama.cpp server started (or attempted) on port 8080");
    } else {
        tracing::warn!("Failed to start llama-server. Ensure it is in your PATH. Running in mock mode.");
    }

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
    
    // Kill child on exit
    if let Ok(mut child) = llama_process {
        let _ = child.kill().await;
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    state: Arc<Mutex<ChatHistory>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<Mutex<ChatHistory>>) {
    use shared::ClientMessage;

    // Send existing history
    {
        let history = state.lock().unwrap().clone();
        let msg = ServerMessage::History(history);
        if let Ok(json) = serde_json::to_string(&msg) {
             if socket.send(WsMessage::Text(json.into())).await.is_err() {
                 return;
             }
        }
    }

    while let Some(Ok(msg)) = socket.recv().await {
        if let WsMessage::Text(text) = msg {
            tracing::debug!("received: {}", text);

            let client_msg: ClientMessage = match serde_json::from_str(&text) {
                Ok(m) => m,
                Err(_) => {
                    // Fallback for raw text if client not fully updated (or manual testing)
                    ClientMessage::Text(text)
                }
            };

            match client_msg {
                ClientMessage::SetModel(model_name) => {
                    tracing::info!("Switching model to: {}", model_name);
                    {
                        let mut history = state.lock().unwrap();
                        history.current_model = model_name.clone();
                         // Save to disk
                        if let Ok(json) = serde_json::to_string(&*history) {
                            let _ = std::fs::write(HISTORY_FILE, json);
                        }
                    }
                    
                    let confirm = ServerMessage::ModelChanged(model_name);
                    if let Ok(json) = serde_json::to_string(&confirm) {
                        if socket.send(WsMessage::Text(json.into())).await.is_err() {
                            return;
                        }
                    }
                }
                ClientMessage::Text(content) => {
                    // User Message
                    let current_model = {
                        let mut history = state.lock().unwrap();
                        history.messages.push(Message {
                            role: Role::User,
                            content: content.clone(),
                        });
                        // Save to disk 
                        if let Ok(json) = serde_json::to_string(&*history) {
                            let _ = std::fs::write(HISTORY_FILE, json);
                        }
                        history.current_model.clone()
                    };

                    // Mock response: Stream back
                    // If model is "gpt-4" (simulated), prefix differently
                    let response_prefix = if current_model.contains("4") {
                        format!("Echo [Smart {}]: ", current_model)
                    } else {
                        format!("Echo [Fast {}]: ", current_model)
                    };
                    
                    // Stream prefix
                    for char in response_prefix.chars() {
                         let token_msg = ServerMessage::Token(char.to_string());
                          if let Ok(json) = serde_json::to_string(&token_msg) {
                            if socket.send(WsMessage::Text(json.into())).await.is_err() {
                                return;
                            }
                          }
                    }
                    
                     // Simulate streaming echo
                    let mut assistant_content = String::from(response_prefix);
                    for char in content.chars() {
                        let token_msg = ServerMessage::Token(char.to_string());
                         if let Ok(json) = serde_json::to_string(&token_msg) {
                            if socket.send(WsMessage::Text(json.into())).await.is_err() {
                                return;
                            }
                         }
                        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                        assistant_content.push(char);
                    }

                     // End of message
                    let eom = ServerMessage::EndOfMessage;
                     if let Ok(json) = serde_json::to_string(&eom) {
                        if socket.send(WsMessage::Text(json.into())).await.is_err() {
                            return;
                        }
                     }
                     
                     // Save Assistant Message
                     {
                        let mut history = state.lock().unwrap();
                        history.messages.push(Message {
                            role: Role::Assistant,
                            content: assistant_content,
                        });
                        if let Ok(json) = serde_json::to_string(&*history) {
                            let _ = std::fs::write(HISTORY_FILE, json);
                        }
                     }
                }
            }
        }
    }
}
