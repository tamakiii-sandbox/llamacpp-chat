use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatHistory {
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    History(ChatHistory),
    Token(String), // For streaming response
    EndOfMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Text(String),
}
