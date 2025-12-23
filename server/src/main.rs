use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "server=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = Router::new().route("/ws", get(ws_handler));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            tracing::debug!("received: {}", text);

            // Mock response: Stream back the received message in chunks
            let response_prefix = "Echo: ";
            if socket
                .send(Message::Text(response_prefix.into()))
                .await
                .is_err()
            {
                return;
            }

            // Simulate streaming
            for char in text.chars() {
                if socket
                    .send(Message::Text(char.to_string()))
                    .await
                    .is_err()
                {
                    return;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }
            
            // Send a newline to indicate end of message (simple protocol for now)
            if socket.send(Message::Text("\n".into())).await.is_err() {
                return;
            }
        }
    }
}
