use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use uuid::Uuid;

use crate::ai_conversation::{FunctionCall, FunctionSchema, LLMClient, Message, MessageRole};

/// Gemini API client configuration
#[derive(Debug, Clone)]
pub struct GeminiConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,
}

impl Default for GeminiConfig {
    fn default() -> Self {
        Self {
            api_key: env::var("GEMINI_API_KEY").unwrap_or_default(),
            model: "gemini-2.0-flash".to_string(),
            base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
            max_tokens: Some(8192),
            temperature: Some(0.7),
            top_p: Some(0.9),
            top_k: Some(40),
        }
    }
}

/// Gemini LLM client
pub struct GeminiClient {
    config: GeminiConfig,
    client: Client,
}

impl GeminiClient {
    pub fn new(config: GeminiConfig) -> Result<Self> {
        if config.api_key.is_empty() {
            return Err(anyhow::anyhow!(
                "Gemini API key is required. Set GEMINI_API_KEY environment variable."
            ));
        }

        let client = Client::new();
        Ok(Self { config, client })
    }

    pub fn with_api_key(api_key: String) -> Result<Self> {
        let config = GeminiConfig {
            api_key,
            ..GeminiConfig::default()
        };
        Self::new(config)
    }

    /// Convert our Message format to Gemini API format
    fn convert_messages_to_gemini_format(&self, messages: &[Message]) -> Vec<GeminiMessage> {
        messages
            .iter()
            .filter_map(|msg| {
                match msg.role {
                    MessageRole::User => Some(GeminiMessage {
                        role: "user".to_string(),
                        parts: vec![GeminiPart {
                            text: msg.content.clone(),
                        }],
                    }),
                    MessageRole::Assistant => Some(GeminiMessage {
                        role: "model".to_string(),
                        parts: vec![GeminiPart {
                            text: msg.content.clone(),
                        }],
                    }),
                    MessageRole::System => {
                        // System messages are handled differently in Gemini
                        // We'll prepend them to the first user message
                        None
                    }
                    MessageRole::Function => {
                        // Function messages are not directly supported
                        // We'll convert them to user messages for now
                        Some(GeminiMessage {
                            role: "user".to_string(),
                            parts: vec![GeminiPart {
                                text: format!("Function result: {}", msg.content),
                            }],
                        })
                    }
                }
            })
            .collect()
    }

    /// Extract system message content to use as system instruction
    fn extract_system_instruction(&self, messages: &[Message]) -> Option<String> {
        messages
            .iter()
            .find(|msg| matches!(msg.role, MessageRole::System))
            .map(|msg| msg.content.clone())
    }

    /// Make a request to the Gemini API
    async fn make_request(&self, request: GeminiRequest) -> Result<GeminiResponse> {
        let url = format!(
            "{}/models/{}:generateContent?key={}",
            self.config.base_url, self.config.model, self.config.api_key
        );

        let max_retries = 3;
        let mut last_error = None;

        for attempt in 1..=max_retries {
            let response = self
                .client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await?;

            if response.status().is_success() {
                let gemini_response: GeminiResponse = response.json().await?;
                return Ok(gemini_response);
            }

            let status = response.status();
            let error_text = response.text().await?;
            
            // Handle 503 Service Unavailable with retry
            if status == 503 {
                let error_msg = format!("Gemini API temporarily unavailable (attempt {}/{}): {}", 
                    attempt, max_retries, error_text);
                last_error = Some(anyhow::anyhow!(error_msg));
                
                if attempt < max_retries {
                    // Exponential backoff: 1s, 2s, 4s
                    let delay = std::time::Duration::from_secs(2_u64.pow(attempt - 1));
                    eprintln!("⏳ API overloaded, retrying in {}s... (attempt {}/{})", 
                        delay.as_secs(), attempt, max_retries);
                    tokio::time::sleep(delay).await;
                    continue;
                }
            } else {
                // For non-503 errors, fail immediately
                return Err(anyhow::anyhow!(
                    "Gemini API request failed: {} - {}",
                    status,
                    error_text
                ));
            }
        }

        // If we get here, all retries failed
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All retry attempts failed")))
    }

    /// Convert Gemini response to our Message format
    fn convert_gemini_response_to_message(&self, response: GeminiResponse) -> Result<Message> {
        let candidate = response
            .candidates
            .first()
            .ok_or_else(|| anyhow::anyhow!("No candidates in response"))?;

        let first_part = candidate
            .content
            .parts
            .first()
            .ok_or_else(|| anyhow::anyhow!("No parts in candidate content"))?;

        // Check if this is a function call
        if let Some(function_call) = &first_part.function_call {
            // Convert args from serde_json::Value to HashMap
            let arguments = if let serde_json::Value::Object(map) = &function_call.args {
                map.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<HashMap<String, serde_json::Value>>()
            } else {
                HashMap::new()
            };

            Ok(Message {
                id: Uuid::new_v4().to_string(),
                role: MessageRole::Assistant,
                content: format!("Function call: {}", function_call.name),
                timestamp: Utc::now(),
                function_call: Some(FunctionCall {
                    name: function_call.name.clone(),
                    arguments,
                }),
            })
        } else {
            // Regular text response
            let content = first_part.text.clone();
            Ok(Message {
                id: Uuid::new_v4().to_string(),
                role: MessageRole::Assistant,
                content,
                timestamp: Utc::now(),
                function_call: None,
            })
        }
    }
}

#[async_trait]
impl LLMClient for GeminiClient {
    async fn send_message(&self, messages: Vec<Message>) -> Result<Message> {
        let gemini_messages = self.convert_messages_to_gemini_format(&messages);
        let system_instruction = self.extract_system_instruction(&messages);

        let request = GeminiRequest {
            contents: gemini_messages,
            generation_config: Some(GeminiGenerationConfig {
                temperature: self.config.temperature,
                top_p: self.config.top_p,
                top_k: self.config.top_k,
                max_output_tokens: self.config.max_tokens,
                response_mime_type: None, // Don't force JSON for normal conversations
            }),
            system_instruction: system_instruction.map(|instruction| GeminiSystemInstruction {
                parts: vec![GeminiPart { text: instruction }],
            }),
            safety_settings: Some(vec![
                GeminiSafetySetting {
                    category: "HARM_CATEGORY_HARASSMENT".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
                GeminiSafetySetting {
                    category: "HARM_CATEGORY_HATE_SPEECH".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
                GeminiSafetySetting {
                    category: "HARM_CATEGORY_SEXUALLY_EXPLICIT".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
                GeminiSafetySetting {
                    category: "HARM_CATEGORY_DANGEROUS_CONTENT".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
            ]),
            tools: None,
        };

        let response = self.make_request(request).await?;
        self.convert_gemini_response_to_message(response)
    }

    async fn stream_response(&self, messages: Vec<Message>) -> Result<tokio::sync::mpsc::Receiver<String>> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        
        let gemini_messages = self.convert_messages_to_gemini_format(&messages);
        let system_instruction = self.extract_system_instruction(&messages);

        let request = GeminiRequest {
            contents: gemini_messages,
            generation_config: Some(GeminiGenerationConfig {
                temperature: self.config.temperature,
                top_p: self.config.top_p,
                top_k: self.config.top_k,
                max_output_tokens: self.config.max_tokens,
                response_mime_type: None, // Don't force JSON for streaming
            }),
            system_instruction: system_instruction.map(|instruction| GeminiSystemInstruction {
                parts: vec![GeminiPart { text: instruction }],
            }),
            safety_settings: Some(vec![
                GeminiSafetySetting {
                    category: "HARM_CATEGORY_HARASSMENT".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
                GeminiSafetySetting {
                    category: "HARM_CATEGORY_HATE_SPEECH".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
                GeminiSafetySetting {
                    category: "HARM_CATEGORY_SEXUALLY_EXPLICIT".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
                GeminiSafetySetting {
                    category: "HARM_CATEGORY_DANGEROUS_CONTENT".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
            ]),
            tools: None,
        };

        let url = format!(
            "{}/models/{}:streamGenerateContent?alt=sse&key={}",
            self.config.base_url, self.config.model, self.config.api_key
        );

        let client = self.client.clone();
        tokio::spawn(async move {
            let response = client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await;

            match response {
                Ok(mut response) => {
                    if !response.status().is_success() {
                        if let Ok(error_text) = response.text().await {
                            eprintln!("Gemini streaming API error: {}", error_text);
                        }
                        return;
                    }

                    // Read the SSE stream
                    while let Some(chunk) = response.chunk().await.unwrap_or(None) {
                        let chunk_str = String::from_utf8_lossy(&chunk);
                        
                        // Parse SSE format: "data: {json}\n\n"
                        for line in chunk_str.lines() {
                            if line.starts_with("data: ") {
                                let json_str = &line[6..]; // Remove "data: " prefix
                                if json_str.trim().is_empty() {
                                    continue;
                                }
                                
                                if let Ok(stream_response) = serde_json::from_str::<GeminiResponse>(json_str) {
                                    if let Some(candidate) = stream_response.candidates.first() {
                                        if let Some(part) = candidate.content.parts.first() {
                                            if !part.text.is_empty() {
                                                if tx.send(part.text.clone()).await.is_err() {
                                                    return; // Receiver dropped
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to make streaming request: {}", e);
                }
            }
        });

        Ok(rx)
    }

    async fn function_calling(&self, messages: Vec<Message>, functions: Vec<FunctionSchema>) -> Result<Message> {
        if functions.is_empty() {
            return self.send_message(messages).await;
        }

        let gemini_messages = self.convert_messages_to_gemini_format(&messages);
        let system_instruction = self.extract_system_instruction(&messages);

        // Convert function schemas to Gemini format
        let function_declarations = functions
            .into_iter()
            .map(|func| GeminiFunctionDeclaration {
                name: func.name,
                description: func.description,
                parameters: func.parameters,
            })
            .collect();

        let tools = vec![GeminiTool {
            function_declarations,
        }];

        let request = GeminiRequest {
            contents: gemini_messages,
            generation_config: Some(GeminiGenerationConfig {
                temperature: self.config.temperature,
                top_p: self.config.top_p,
                top_k: self.config.top_k,
                max_output_tokens: self.config.max_tokens,
                response_mime_type: Some("application/json".to_string()),
            }),
            system_instruction: system_instruction.map(|instruction| GeminiSystemInstruction {
                parts: vec![GeminiPart { text: instruction }],
            }),
            safety_settings: Some(vec![
                GeminiSafetySetting {
                    category: "HARM_CATEGORY_HARASSMENT".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
                GeminiSafetySetting {
                    category: "HARM_CATEGORY_HATE_SPEECH".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
                GeminiSafetySetting {
                    category: "HARM_CATEGORY_SEXUALLY_EXPLICIT".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
                GeminiSafetySetting {
                    category: "HARM_CATEGORY_DANGEROUS_CONTENT".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
            ]),
            tools: Some(tools),
        };

        let response = self.make_request(request).await?;
        self.convert_gemini_response_to_message(response)
    }

    fn get_model_name(&self) -> String {
        self.config.model.clone()
    }
}

// Gemini API request/response structures
#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiGenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiSystemInstruction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    safety_settings: Option<Vec<GeminiSafetySetting>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<GeminiTool>>,
}

#[derive(Debug, Serialize)]
struct GeminiMessage {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_mime_type: Option<String>,
}

#[derive(Debug, Serialize)]
struct GeminiSystemInstruction {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiSafetySetting {
    category: String,
    threshold: String,
}

#[derive(Debug, Serialize)]
struct GeminiTool {
    #[serde(rename = "functionDeclarations")]
    function_declarations: Vec<GeminiFunctionDeclaration>,
}

#[derive(Debug, Serialize)]
struct GeminiFunctionDeclaration {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    #[serde(default)]
    usage_metadata: Option<GeminiUsageMetadata>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
    #[serde(default)]
    finish_reason: Option<String>,
    #[serde(default)]
    safety_ratings: Option<Vec<GeminiSafetyRating>>,
}

#[derive(Debug, Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiResponsePart>,
    #[serde(default)]
    role: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponsePart {
    #[serde(default)]
    text: String,
    #[serde(rename = "functionCall")]
    function_call: Option<GeminiFunctionCall>,
}

#[derive(Debug, Deserialize)]
struct GeminiFunctionCall {
    name: String,
    args: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct GeminiUsageMetadata {
    #[serde(default)]
    prompt_token_count: Option<u32>,
    #[serde(default)]
    candidates_token_count: Option<u32>,
    #[serde(default)]
    total_token_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GeminiSafetyRating {
    category: String,
    probability: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn test_gemini_client_creation() {
        let config = GeminiConfig {
            api_key: "test_key".to_string(),
            ..GeminiConfig::default()
        };
        let client = GeminiClient::new(config);
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_gemini_client_requires_api_key() {
        let config = GeminiConfig {
            api_key: "".to_string(),
            ..GeminiConfig::default()
        };
        let client = GeminiClient::new(config);
        assert!(client.is_err());
    }

    #[tokio::test]
    async fn test_message_conversion() {
        let config = GeminiConfig {
            api_key: "test_key".to_string(),
            ..GeminiConfig::default()
        };
        let client = GeminiClient::new(config).unwrap();

        let messages = vec![
            Message {
                id: "1".to_string(),
                role: MessageRole::System,
                content: "You are a helpful assistant".to_string(),
                timestamp: Utc::now(),
                function_call: None,
            },
            Message {
                id: "2".to_string(),
                role: MessageRole::User,
                content: "Hello".to_string(),
                timestamp: Utc::now(),
                function_call: None,
            },
        ];

        let gemini_messages = client.convert_messages_to_gemini_format(&messages);
        assert_eq!(gemini_messages.len(), 1); // System message is filtered out
        assert_eq!(gemini_messages[0].role, "user");
        assert_eq!(gemini_messages[0].parts[0].text, "Hello");

        let system_instruction = client.extract_system_instruction(&messages);
        assert_eq!(system_instruction, Some("You are a helpful assistant".to_string()));
    }
}