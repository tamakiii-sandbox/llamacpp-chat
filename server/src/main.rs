mod config;
mod process;

use axum::{
    extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
use shared::{ChatHistory, Message, Role, ServerMessage};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::fs;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use config::AppConfig;
use process::ProcessManager;

const HISTORY_FILE: &str = "chat_history.json";
const CONFIG_FILE: &str = "models.json";

struct AppState {
    history: Mutex<ChatHistory>,
    config: AppConfig,
    process_manager: tokio::sync::Mutex<ProcessManager>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "server=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load Config
    let config = match config::load_config(CONFIG_FILE).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to load config: {}. Using default mock config.", e);
            // Fallback config could be created here, but for now let's panic or minimal init
            // Assuming config file existence is mandatory for this step
            panic!("Configuration file {} needed: {}", CONFIG_FILE, e);
        }
    };

    // Load history
    let history = if let Ok(content) = fs::read_to_string(HISTORY_FILE).await {
        serde_json::from_str(&content).unwrap_or(ChatHistory { 
            messages: vec![],
            current_model: config.default.clone(),
        })
    } else {
        ChatHistory { 
            messages: vec![],
            current_model: config.default.clone(),
        }
    };

    // Initialize ProcessManager
    let mut process_manager = ProcessManager::new();
    
    // Start default model
    if let Some(model_config) = config.models.get(&config.default) {
        if let Err(e) = process_manager.start(&model_config.path, &model_config.args) {
            tracing::warn!("Failed to start default model: {}. Running in mock mode possibly.", e);
        }
    } else {
        tracing::warn!("Default model '{}' not found in config.", config.default);
    }

    let app_state = Arc::new(AppState {
        history: Mutex::new(history),
        config,
        process_manager: tokio::sync::Mutex::new(process_manager),
    });

    let app = Router::new()
        .route("/ws", get(|ws| ws_handler(ws, app_state)));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    tracing::info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
    
    // Cleanup handled by Drop implementation ideally, but for now we rely on OS cleanup 
    // or we could signal shutdown. 
    // Since axum::serve blocks, we can't easily run shutdown code after it unless we handle signals.
    // For local dev, killing the parent usually kills the child if not detached, 
    // but explicit kill is better.
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    state: Arc<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    use shared::ClientMessage;

    // Send existing history
    // Send existing history
    {
        let history = state.history.lock().unwrap().clone();
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
                    
                    let mut success = false;
                    // Check if model exists in config
                    if let Some(model_config) = state.config.models.get(&model_name) {
                        let mut pm = state.process_manager.lock().await;
                        match pm.restart(&model_config.path, &model_config.args).await {
                            Ok(_) => {
                                tracing::info!("Model switched successfully to {}", model_name);
                                success = true;
                            }
                            Err(e) => {
                                tracing::error!("Failed to switch model: {}", e);
                            }
                        }
                    } else {
                        tracing::warn!("Model '{}' not found in config.", model_name);
                    }

                    if success {
                        {
                            let mut history = state.history.lock().unwrap();
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
                }
                ClientMessage::Text(content) => {
                    // User Message
                    let current_model = {
                        let mut history = state.history.lock().unwrap();
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
                        let mut history = state.history.lock().unwrap();
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
