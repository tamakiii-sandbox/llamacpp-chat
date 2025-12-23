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
    pub current_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    History(ChatHistory),
    Token(String), // For streaming response
    EndOfMessage,
    ModelChanged(String),
    AvailableModels(Vec<String>),
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Text(String),
    SetModel(String),
}
