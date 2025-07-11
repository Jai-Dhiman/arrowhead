use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use uuid::Uuid;

use crate::ai_conversation::{FunctionCall, FunctionSchema, LLMClient, Message, MessageRole};

/// OpenAI API client configuration
#[derive(Debug, Clone)]
pub struct OpenAIConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
}

impl Default for OpenAIConfig {
    fn default() -> Self {
        Self {
            api_key: env::var("OPENAI_API_KEY").unwrap_or_default(),
            model: "gpt-3.5-turbo".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            max_tokens: Some(4096),
            temperature: Some(0.7),
            top_p: Some(1.0),
        }
    }
}

/// OpenAI API client
#[derive(Debug, Clone)]
pub struct OpenAIClient {
    config: OpenAIConfig,
    client: Client,
}

impl OpenAIClient {
    /// Create a new OpenAI client
    pub fn new(config: OpenAIConfig) -> Result<Self> {
        let client = Client::new();
        
        // Validate API key
        if config.api_key.is_empty() {
            return Err(anyhow::anyhow!("OpenAI API key is required"));
        }
        
        Ok(Self { config, client })
    }

    /// Convert our Message format to OpenAI's format
    fn convert_messages_to_openai(&self, messages: Vec<Message>) -> Vec<OpenAIMessage> {
        messages.into_iter().map(|msg| OpenAIMessage {
            role: match msg.role {
                MessageRole::User => "user".to_string(),
                MessageRole::Assistant => "assistant".to_string(),
                MessageRole::System => "system".to_string(),
                MessageRole::Function => "function".to_string(),
            },
            content: msg.content,
            name: None,
            function_call: msg.function_call.map(|fc| OpenAIFunctionCall {
                name: fc.name,
                arguments: serde_json::to_string(&fc.arguments).unwrap_or_default(),
            }),
        }).collect()
    }

    /// Convert OpenAI response back to our Message format
    fn convert_openai_message_to_message(&self, openai_msg: OpenAIMessage) -> Message {
        Message {
            id: Uuid::new_v4().to_string(),
            role: match openai_msg.role.as_str() {
                "user" => MessageRole::User,
                "assistant" => MessageRole::Assistant,
                "system" => MessageRole::System,
                "function" => MessageRole::Function,
                _ => MessageRole::Assistant,
            },
            content: openai_msg.content,
            timestamp: Utc::now(),
            function_call: openai_msg.function_call.map(|fc| FunctionCall {
                name: fc.name,
                arguments: serde_json::from_str(&fc.arguments).unwrap_or_default(),
            }),
        }
    }
}

#[async_trait]
impl LLMClient for OpenAIClient {
    async fn send_message(&self, messages: Vec<Message>) -> Result<Message> {
        let openai_messages = self.convert_messages_to_openai(messages);
        
        let request_body = OpenAIRequest {
            model: self.config.model.clone(),
            messages: openai_messages,
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            top_p: self.config.top_p,
            stream: Some(false),
        };

        let response = self.client
            .post(&format!("{}/chat/completions", self.config.base_url))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("OpenAI API error: {}", error_text));
        }

        let openai_response: OpenAIResponse = response.json().await?;
        
        if let Some(choice) = openai_response.choices.first() {
            Ok(self.convert_openai_message_to_message(choice.message.clone()))
        } else {
            Err(anyhow::anyhow!("No response from OpenAI"))
        }
    }

    async fn stream_response(&self, messages: Vec<Message>) -> Result<tokio::sync::mpsc::Receiver<String>> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        
        let openai_messages = self.convert_messages_to_openai(messages);
        
        let request_body = OpenAIRequest {
            model: self.config.model.clone(),
            messages: openai_messages,
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            top_p: self.config.top_p,
            stream: Some(true),
        };

        let client = self.client.clone();
        let config = self.config.clone();
        
        tokio::spawn(async move {
            let response = client
                .post(&format!("{}/chat/completions", config.base_url))
                .header("Authorization", format!("Bearer {}", config.api_key))
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        let _ = tx.send(format!("Error: {}", resp.status())).await;
                        return;
                    }
                    
                    let mut stream = resp.bytes_stream();
                    use futures::StreamExt;
                    
                    while let Some(chunk) = stream.next().await {
                        match chunk {
                            Ok(bytes) => {
                                let chunk_str = String::from_utf8_lossy(&bytes);
                                // Parse SSE format: "data: {...}\n\n"
                                for line in chunk_str.lines() {
                                    if line.starts_with("data: ") {
                                        let data = &line[6..];
                                        if data == "[DONE]" {
                                            break;
                                        }
                                        if let Ok(parsed) = serde_json::from_str::<OpenAIStreamResponse>(data) {
                                            if let Some(choice) = parsed.choices.first() {
                                                if let Some(content) = &choice.delta.content {
                                                    let _ = tx.send(content.clone()).await;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(format!("Stream error: {}", e)).await;
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(format!("Request error: {}", e)).await;
                }
            }
        });

        Ok(rx)
    }

    async fn function_calling(&self, messages: Vec<Message>, functions: Vec<FunctionSchema>) -> Result<Message> {
        let openai_messages = self.convert_messages_to_openai(messages);
        let openai_functions: Vec<OpenAIFunction> = functions.into_iter().map(|f| OpenAIFunction {
            name: f.name,
            description: f.description,
            parameters: f.parameters,
        }).collect();
        
        let request_body = OpenAIFunctionRequest {
            model: self.config.model.clone(),
            messages: openai_messages,
            functions: openai_functions,
            function_call: Some("auto".to_string()),
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            top_p: self.config.top_p,
        };

        let response = self.client
            .post(&format!("{}/chat/completions", self.config.base_url))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("OpenAI API error: {}", error_text));
        }

        let openai_response: OpenAIResponse = response.json().await?;
        
        if let Some(choice) = openai_response.choices.first() {
            Ok(self.convert_openai_message_to_message(choice.message.clone()))
        } else {
            Err(anyhow::anyhow!("No response from OpenAI"))
        }
    }

    fn get_model_name(&self) -> String {
        self.config.model.clone()
    }
}

// OpenAI API request/response structures
#[derive(Debug, Serialize, Deserialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIFunctionRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    functions: Vec<OpenAIFunction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    function_call: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    function_call: Option<OpenAIFunctionCall>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamResponse {
    choices: Vec<OpenAIStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChoice {
    delta: OpenAIStreamDelta,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamDelta {
    content: Option<String>,
}