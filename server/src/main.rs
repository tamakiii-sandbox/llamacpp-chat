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
        serde_json::from_str(&content).unwrap_or(ChatHistory { messages: vec![] })
    } else {
        ChatHistory { messages: vec![] }
    };

    let app_state = Arc::new(Mutex::new(history));

    let app = Router::new()
        .route("/ws", get(|ws| ws_handler(ws, app_state)));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    state: Arc<Mutex<ChatHistory>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<Mutex<ChatHistory>>) {
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

            // User Message
            {
                let mut history = state.lock().unwrap();
                history.messages.push(Message {
                    role: Role::User,
                    content: text.clone(),
                });
                // Save to disk (sync for now, better async in real app but mutex makes it tricky)
                if let Ok(json) = serde_json::to_string(&*history) {
                    let _ = std::fs::write(HISTORY_FILE, json);
                }
            }

            // Mock response: Stream back the received message
            let response_prefix = "Echo: ";
            
            // Stream prefix
            // In a real app we'd construct a full assistant message gradually
            // Here we just stream tokens
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
            for char in text.chars() {
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
