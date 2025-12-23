use anyhow::{Result, Context};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use futures::StreamExt;
use std::pin::Pin;
use futures::Stream;

#[derive(Serialize, Debug)]
pub struct ChatRequest {
    pub messages: Vec<Message>,
    pub stream: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize, Debug)]
struct ChatCompletionChunk {
    choices: Vec<Choice>,
}

#[derive(Deserialize, Debug)]
struct Choice {
    delta: Delta,
    finish_reason: Option<String>,
}

#[derive(Deserialize, Debug)]
struct Delta {
    content: Option<String>,
}

pub struct OAIClient {
    client: Client,
    base_url: String,
}

impl OAIClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
        }
    }

    pub async fn chat_stream(
        &self,
        messages: Vec<Message>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let request = ChatRequest {
            messages,
            stream: true,
        };

        let res = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to llama-server")?;

        if !res.status().is_success() {
             let text = res.text().await.unwrap_or_default();
             return Err(anyhow::anyhow!("API Error: {}", text));
        }

        let stream = res.bytes_stream().map(|item| {
            match item {
                Ok(bytes) => {
                    let s = String::from_utf8_lossy(&bytes);
                    Ok(s.to_string())
                }
                Err(e) => Err(anyhow::anyhow!(e)),
            }
        });

        // This is a naive line/chunk parser. 
        // Real SSE parsing is slightly more complex (data: ...).
        // For robustness, we should use a proper SSE parser or just strip prefixes.
        // Let's implement a simple transformer here.
        
        let sse_stream = stream.map(|chunk_res| {
             match chunk_res {
                 Ok(chunk) => {
                     // The chunk might contain multiple "data: {...}\n\n" lines
                     let mut tokens = String::new();
                     for line in chunk.lines() {
                         if line.starts_with("data: ") {
                             let data = &line[6..];
                             if data == "[DONE]" {
                                 continue;
                             }
                             if let Ok(json) = serde_json::from_str::<ChatCompletionChunk>(data) {
                                  if let Some(choice) = json.choices.first() {
                                      if let Some(content) = &choice.delta.content {
                                          tokens.push_str(content);
                                      }
                                  }
                             }
                         }
                     }
                     Ok(tokens)
                 }
                 Err(e) => Err(e),
             }
        });

        Ok(Box::pin(sse_stream))
    }
}
