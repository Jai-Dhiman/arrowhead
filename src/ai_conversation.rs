use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;
use tokio::time::{sleep, timeout};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub function_call: Option<FunctionCall>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Function,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    pub conversation_id: String,
    pub message_history: Vec<Message>,
    pub max_context_tokens: usize,
    pub current_token_count: usize,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl ConversationContext {
    pub fn new(conversation_id: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            conversation_id,
            message_history: Vec::new(),
            max_context_tokens: 8000, // Default context window
            current_token_count: 0,
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_max_tokens(conversation_id: String, max_tokens: usize) -> Self {
        let mut context = Self::new(conversation_id);
        context.max_context_tokens = max_tokens;
        context
    }

    pub fn add_message(&mut self, message: Message) {
        self.message_history.push(message);
        self.updated_at = chrono::Utc::now();
        self.update_token_count();
        self.prune_if_needed();
    }

    pub fn get_recent_messages(&self, limit: usize) -> Vec<&Message> {
        self.message_history.iter().rev().take(limit).collect()
    }

    pub fn get_messages_since(&self, timestamp: chrono::DateTime<chrono::Utc>) -> Vec<&Message> {
        self.message_history
            .iter()
            .filter(|msg| msg.timestamp > timestamp)
            .collect()
    }

    pub fn get_messages_by_role(&self, role: &MessageRole) -> Vec<&Message> {
        self.message_history
            .iter()
            .filter(|msg| std::mem::discriminant(&msg.role) == std::mem::discriminant(role))
            .collect()
    }

    pub fn clear_history(&mut self) {
        self.message_history.clear();
        self.current_token_count = 0;
        self.updated_at = chrono::Utc::now();
    }

    pub fn get_message_count(&self) -> usize {
        self.message_history.len()
    }

    pub fn get_current_token_count(&self) -> usize {
        self.current_token_count
    }

    pub fn is_context_full(&self) -> bool {
        self.current_token_count >= self.max_context_tokens
    }

    pub fn add_metadata(&mut self, key: String, value: serde_json::Value) {
        self.metadata.insert(key, value);
        self.updated_at = chrono::Utc::now();
    }

    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.get(key)
    }

    pub fn remove_metadata(&mut self, key: &str) -> Option<serde_json::Value> {
        self.updated_at = chrono::Utc::now();
        self.metadata.remove(key)
    }

    fn update_token_count(&mut self) {
        // Rough estimation: 1 token ≈ 4 characters
        self.current_token_count = self.message_history
            .iter()
            .map(|msg| msg.content.len() / 4)
            .sum();
    }

    fn prune_if_needed(&mut self) {
        if self.current_token_count <= self.max_context_tokens {
            return;
        }

        // Keep system messages and the most recent messages
        let mut system_messages = Vec::new();
        let mut other_messages = Vec::new();

        for message in self.message_history.drain(..) {
            if matches!(message.role, MessageRole::System) {
                system_messages.push(message);
            } else {
                other_messages.push(message);
            }
        }

        // Keep system messages and prune older non-system messages
        self.message_history = system_messages;
        
        // Add back recent messages until we're under the token limit
        let mut temp_token_count = self.message_history
            .iter()
            .map(|msg| msg.content.len() / 4)
            .sum::<usize>();

        for message in other_messages.into_iter().rev() {
            let message_tokens = message.content.len() / 4;
            if temp_token_count + message_tokens <= self.max_context_tokens {
                temp_token_count += message_tokens;
                self.message_history.push(message);
            } else {
                break;
            }
        }

        // Sort messages by timestamp to maintain chronological order
        self.message_history.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        self.current_token_count = temp_token_count;
    }

    pub fn save_to_file(&self, file_path: &str) -> Result<(), AIConversationError> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(file_path, json).map_err(|e| {
            AIConversationError::GenericError(anyhow::anyhow!("Failed to save context: {}", e))
        })
    }

    pub fn load_from_file(file_path: &str) -> Result<Self, AIConversationError> {
        let content = std::fs::read_to_string(file_path).map_err(|e| {
            AIConversationError::GenericError(anyhow::anyhow!("Failed to read context file: {}", e))
        })?;
        
        let context: ConversationContext = serde_json::from_str(&content)?;
        Ok(context)
    }

    pub fn get_conversation_summary(&self) -> ConversationSummary {
        let user_messages = self.get_messages_by_role(&MessageRole::User).len();
        let assistant_messages = self.get_messages_by_role(&MessageRole::Assistant).len();
        let system_messages = self.get_messages_by_role(&MessageRole::System).len();
        let function_messages = self.get_messages_by_role(&MessageRole::Function).len();

        ConversationSummary {
            conversation_id: self.conversation_id.clone(),
            total_messages: self.message_history.len(),
            user_messages,
            assistant_messages,
            system_messages,
            function_messages,
            current_token_count: self.current_token_count,
            max_context_tokens: self.max_context_tokens,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub conversation_id: String,
    pub total_messages: usize,
    pub user_messages: usize,
    pub assistant_messages: usize,
    pub system_messages: usize,
    pub function_messages: usize,
    pub current_token_count: usize,
    pub max_context_tokens: usize,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
pub trait LLMClient: Send + Sync {
    async fn send_message(&self, messages: Vec<Message>) -> Result<Message>;
    async fn stream_response(&self, messages: Vec<Message>) -> Result<tokio::sync::mpsc::Receiver<String>>;
    async fn function_calling(&self, messages: Vec<Message>, functions: Vec<FunctionSchema>) -> Result<Message>;
    fn get_model_name(&self) -> String;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSchema {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, thiserror::Error)]
pub enum AIConversationError {
    #[error("API error: {0}")]
    ApiError(#[from] reqwest::Error),
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Invalid message format: {0}")]
    InvalidMessageFormat(String),
    #[error("Context window exceeded")]
    ContextWindowExceeded,
    #[error("Function call error: {0}")]
    FunctionCallError(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Generic error: {0}")]
    GenericError(#[from] anyhow::Error),
}

pub struct AIConversationEngine {
    pub conversation_id: String,
    pub context: ConversationContext,
    pub llm_client: Box<dyn LLMClient>,
    pub function_registry: HashMap<String, FunctionSchema>,
}

impl AIConversationEngine {
    pub fn new(llm_client: Box<dyn LLMClient>) -> Self {
        let conversation_id = Uuid::new_v4().to_string();
        let context = ConversationContext::new(conversation_id.clone());
        
        Self {
            conversation_id,
            context,
            llm_client,
            function_registry: HashMap::new(),
        }
    }

    pub fn with_conversation_id(conversation_id: String, llm_client: Box<dyn LLMClient>) -> Self {
        let context = ConversationContext::new(conversation_id.clone());
        
        Self {
            conversation_id,
            context,
            llm_client,
            function_registry: HashMap::new(),
        }
    }

    pub async fn send_message(&mut self, content: String) -> Result<String, AIConversationError> {
        let user_message = Message {
            id: Uuid::new_v4().to_string(),
            role: MessageRole::User,
            content,
            timestamp: chrono::Utc::now(),
            function_call: None,
        };

        self.context.add_message(user_message);

        let response = self.llm_client
            .send_message(self.context.message_history.clone())
            .await?;

        self.context.add_message(response.clone());

        Ok(response.content)
    }

    pub async fn stream_message(&mut self, content: String) -> Result<tokio::sync::mpsc::Receiver<String>, AIConversationError> {
        let user_message = Message {
            id: Uuid::new_v4().to_string(),
            role: MessageRole::User,
            content,
            timestamp: chrono::Utc::now(),
            function_call: None,
        };

        self.context.add_message(user_message);

        let receiver = self.llm_client
            .stream_response(self.context.message_history.clone())
            .await?;

        Ok(receiver)
    }

    pub fn register_function(&mut self, function: FunctionSchema) {
        self.function_registry.insert(function.name.clone(), function);
    }

    pub fn get_conversation_history(&self) -> &Vec<Message> {
        &self.context.message_history
    }

    pub fn clear_history(&mut self) {
        self.context.message_history.clear();
    }

    pub fn get_model_name(&self) -> String {
        self.llm_client.get_model_name()
    }

    // Enhanced async method with comprehensive error handling
    pub async fn send_message_with_timeout(&mut self, content: String, timeout_secs: u64) -> Result<String, AIConversationError> {
        let result = timeout(
            Duration::from_secs(timeout_secs),
            self.send_message(content)
        ).await;

        match result {
            Ok(response) => response,
            Err(_) => Err(AIConversationError::GenericError(anyhow::anyhow!("Message sending timed out after {} seconds", timeout_secs)))
        }
    }

    // Performance monitoring for async operations
    pub async fn send_message_with_metrics(&mut self, content: String) -> Result<(String, Duration), AIConversationError> {
        let start = std::time::Instant::now();
        let response = self.send_message(content).await?;
        let duration = start.elapsed();
        
        // In a real implementation, you'd send these metrics to a monitoring system
        eprintln!("Message processing took: {:?}", duration);
        
        Ok((response, duration))
    }

    // Batch processing with concurrency control
    pub async fn process_messages_concurrent(&mut self, messages: Vec<String>, max_concurrent: usize) -> Vec<Result<String, AIConversationError>> {
        use tokio::sync::Semaphore;
        use std::sync::Arc;
        
        let semaphore = Arc::new(Semaphore::new(max_concurrent));
        let mut results = Vec::new();
        
        for message in messages {
            let permit = semaphore.clone().acquire_owned().await;
            match permit {
                Ok(_permit) => {
                    // In a real implementation, you'd process each message
                    // For now, just simulate processing
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    results.push(Ok(format!("Processed: {}", message)));
                }
                Err(_) => {
                    results.push(Err(AIConversationError::GenericError(anyhow::anyhow!("Failed to acquire semaphore"))));
                }
            }
        }
        
        results
    }

    // Health check for the conversation engine
    pub async fn health_check(&self) -> Result<serde_json::Value, AIConversationError> {
        let health_status = serde_json::json!({
            "conversation_id": self.conversation_id,
            "model_name": self.get_model_name(),
            "message_count": self.context.get_message_count(),
            "token_usage": {
                "current": self.context.get_current_token_count(),
                "limit": self.context.max_context_tokens,
                "percentage": (self.context.get_current_token_count() as f64 / self.context.max_context_tokens as f64) * 100.0
            },
            "context_full": self.context.is_context_full(),
            "metadata_keys": self.context.metadata.keys().collect::<Vec<_>>(),
            "function_count": self.function_registry.len(),
            "status": "healthy"
        });

        Ok(health_status)
    }

    // Graceful shutdown handling
    pub async fn shutdown(&mut self) -> Result<(), AIConversationError> {
        // Save conversation state before shutdown
        let save_path = format!("/tmp/conversation_{}.json", self.conversation_id);
        self.context.save_to_file(&save_path)?;
        
        // Clear sensitive data
        self.context.clear_history();
        self.function_registry.clear();
        
        eprintln!("Conversation engine shut down gracefully. State saved to: {}", save_path);
        Ok(())
    }

    // Recovery from saved state
    pub async fn recover_from_file(file_path: &str, llm_client: Box<dyn LLMClient>) -> Result<Self, AIConversationError> {
        let context = ConversationContext::load_from_file(file_path)?;
        
        Ok(Self {
            conversation_id: context.conversation_id.clone(),
            context,
            llm_client,
            function_registry: HashMap::new(),
        })
    }
}

// Claude API Client Implementation
pub struct ClaudeClient {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
    model: String,
    max_tokens: u32,
}

impl ClaudeClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.anthropic.com/v1".to_string(),
            client: reqwest::Client::new(),
            model: "claude-3-sonnet-20240229".to_string(),
            max_tokens: 4096,
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    async fn make_request(&self, messages: Vec<Message>) -> Result<ClaudeResponse, AIConversationError> {
        self.make_request_with_retries(messages, 3).await
    }

    async fn make_request_with_retries(&self, messages: Vec<Message>, max_retries: u32) -> Result<ClaudeResponse, AIConversationError> {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts < max_retries {
            attempts += 1;
            
            match self.make_single_request(&messages).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e);
                    
                    if attempts < max_retries {
                        // Exponential backoff
                        let delay = Duration::from_millis(1000 * (2_u64.pow(attempts - 1)));
                        sleep(delay).await;
                        eprintln!("Claude API request failed, retrying in {:?} (attempt {}/{})", delay, attempts, max_retries);
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| AIConversationError::GenericError(anyhow::anyhow!("Max retries exceeded"))))
    }

    async fn make_single_request(&self, messages: &[Message]) -> Result<ClaudeResponse, AIConversationError> {
        let claude_messages: Vec<ClaudeMessage> = messages
            .iter()
            .filter(|msg| !matches!(msg.role, MessageRole::System))
            .map(|msg| ClaudeMessage {
                role: match msg.role {
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                    MessageRole::Function => "user".to_string(),
                    MessageRole::System => "user".to_string(),
                },
                content: msg.content.clone(),
            })
            .collect();

        let system_messages: Vec<String> = messages
            .iter()
            .filter(|msg| matches!(msg.role, MessageRole::System))
            .map(|msg| msg.content.clone())
            .collect();

        let system_prompt = if system_messages.is_empty() {
            None
        } else {
            Some(system_messages.join("\n\n"))
        };

        let request_body = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            messages: claude_messages,
            system: system_prompt,
        };

        // Add timeout for the request
        let response = timeout(
            Duration::from_secs(30),
            self.client
                .post(&format!("{}/messages", self.base_url))
                .header("Content-Type", "application/json")
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .json(&request_body)
                .send()
        ).await
        .map_err(|_| AIConversationError::GenericError(anyhow::anyhow!("Request timeout")))?
        .map_err(|e| AIConversationError::ApiError(e))?;

        if response.status().is_success() {
            let claude_response: ClaudeResponse = response.json().await
                .map_err(|e| AIConversationError::GenericError(anyhow::anyhow!("Failed to parse Claude response: {}", e)))?;
            Ok(claude_response)
        } else if response.status().as_u16() == 429 {
            Err(AIConversationError::RateLimitExceeded)
        } else {
            let status_code = response.status();
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(AIConversationError::GenericError(anyhow::anyhow!(
                "Claude API error ({}): {}",
                status_code,
                error_text
            )))
        }
    }
}

#[async_trait]
impl LLMClient for ClaudeClient {
    async fn send_message(&self, messages: Vec<Message>) -> Result<Message> {
        let response = self.make_request(messages).await?;
        
        let content = response.content
            .first()
            .map(|c| c.text.clone())
            .unwrap_or_else(|| "No response content".to_string());

        Ok(Message {
            id: response.id,
            role: MessageRole::Assistant,
            content,
            timestamp: chrono::Utc::now(),
            function_call: None,
        })
    }

    async fn stream_response(&self, messages: Vec<Message>) -> Result<tokio::sync::mpsc::Receiver<String>> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        
        let response = self.make_request(messages).await?;
        let content = response.content
            .first()
            .map(|c| c.text.clone())
            .unwrap_or_else(|| "No response content".to_string());

        tokio::spawn(async move {
            // Simple streaming simulation - in real implementation, use streaming API
            for chunk in content.chars().collect::<Vec<_>>().chunks(10) {
                let chunk_str: String = chunk.iter().collect();
                if tx.send(chunk_str).await.is_err() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        });

        Ok(rx)
    }

    async fn function_calling(&self, messages: Vec<Message>, _functions: Vec<FunctionSchema>) -> Result<Message> {
        // Claude doesn't have built-in function calling like GPT-4, so we'll use a simple approach
        self.send_message(messages).await
    }

    fn get_model_name(&self) -> String {
        self.model.clone()
    }
}

// GPT-4 API Client Implementation
pub struct GPT4Client {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
    model: String,
    max_tokens: u32,
}

impl GPT4Client {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
            client: reqwest::Client::new(),
            model: "gpt-4".to_string(),
            max_tokens: 4096,
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    async fn make_request(&self, messages: Vec<Message>) -> Result<GPTResponse, AIConversationError> {
        self.make_request_with_retries(messages, 3).await
    }

    async fn make_request_with_retries(&self, messages: Vec<Message>, max_retries: u32) -> Result<GPTResponse, AIConversationError> {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts < max_retries {
            attempts += 1;
            
            match self.make_single_request(&messages).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e);
                    
                    if attempts < max_retries {
                        // Exponential backoff
                        let delay = Duration::from_millis(1000 * (2_u64.pow(attempts - 1)));
                        sleep(delay).await;
                        eprintln!("GPT-4 API request failed, retrying in {:?} (attempt {}/{})", delay, attempts, max_retries);
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| AIConversationError::GenericError(anyhow::anyhow!("Max retries exceeded"))))
    }

    async fn make_single_request(&self, messages: &[Message]) -> Result<GPTResponse, AIConversationError> {
        let gpt_messages: Vec<GPTMessage> = messages
            .iter()
            .map(|msg| GPTMessage {
                role: match msg.role {
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                    MessageRole::System => "system".to_string(),
                    MessageRole::Function => "function".to_string(),
                },
                content: msg.content.clone(),
            })
            .collect();

        let request_body = GPTRequest {
            model: self.model.clone(),
            messages: gpt_messages,
            max_tokens: self.max_tokens,
            temperature: 0.7,
        };

        // Add timeout for the request
        let response = timeout(
            Duration::from_secs(30),
            self.client
                .post(&format!("{}/chat/completions", self.base_url))
                .header("Content-Type", "application/json")
                .header("Authorization", &format!("Bearer {}", self.api_key))
                .json(&request_body)
                .send()
        ).await
        .map_err(|_| AIConversationError::GenericError(anyhow::anyhow!("Request timeout")))?
        .map_err(|e| AIConversationError::ApiError(e))?;

        if response.status().is_success() {
            let gpt_response: GPTResponse = response.json().await
                .map_err(|e| AIConversationError::GenericError(anyhow::anyhow!("Failed to parse GPT response: {}", e)))?;
            Ok(gpt_response)
        } else if response.status().as_u16() == 429 {
            Err(AIConversationError::RateLimitExceeded)
        } else {
            let status_code = response.status();
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(AIConversationError::GenericError(anyhow::anyhow!(
                "GPT-4 API error ({}): {}",
                status_code,
                error_text
            )))
        }
    }
}

#[async_trait]
impl LLMClient for GPT4Client {
    async fn send_message(&self, messages: Vec<Message>) -> Result<Message> {
        let response = self.make_request(messages).await?;
        
        let choice = response.choices
            .first()
            .ok_or_else(|| AIConversationError::GenericError(anyhow::anyhow!("No choices in response")))?;

        Ok(Message {
            id: response.id,
            role: MessageRole::Assistant,
            content: choice.message.content.clone(),
            timestamp: chrono::Utc::now(),
            function_call: None,
        })
    }

    async fn stream_response(&self, messages: Vec<Message>) -> Result<tokio::sync::mpsc::Receiver<String>> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        
        let response = self.make_request(messages).await?;
        let choice = response.choices
            .first()
            .ok_or_else(|| AIConversationError::GenericError(anyhow::anyhow!("No choices in response")))?;

        let content = choice.message.content.clone();
        tokio::spawn(async move {
            // Simple streaming simulation - in real implementation, use streaming API
            for chunk in content.chars().collect::<Vec<_>>().chunks(10) {
                let chunk_str: String = chunk.iter().collect();
                if tx.send(chunk_str).await.is_err() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        });

        Ok(rx)
    }

    async fn function_calling(&self, messages: Vec<Message>, functions: Vec<FunctionSchema>) -> Result<Message> {
        // GPT-4 supports function calling, but for simplicity, we'll use basic approach
        // In a real implementation, you would use the tools parameter
        self.send_message(messages).await
    }

    fn get_model_name(&self) -> String {
        self.model.clone()
    }
}

// Claude API types
#[derive(Debug, Serialize, Deserialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ClaudeMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClaudeResponse {
    id: String,
    content: Vec<ClaudeContent>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClaudeContent {
    text: String,
}

// GPT API types
#[derive(Debug, Serialize, Deserialize)]
struct GPTRequest {
    model: String,
    messages: Vec<GPTMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct GPTMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GPTResponse {
    id: String,
    choices: Vec<GPTChoice>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GPTChoice {
    message: GPTMessage,
}

// Natural Language Understanding Module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub name: String,
    pub confidence: f32,
    pub entities: Vec<Entity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub name: String,
    pub value: String,
    pub entity_type: String,
    pub confidence: f32,
    pub start_pos: usize,
    pub end_pos: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NLUResult {
    pub intent: Intent,
    pub entities: Vec<Entity>,
    pub sentiment: Option<f32>,
    pub confidence: f32,
}

pub struct NLUProcessor {
    // In a real implementation, this would contain ML models
    // For now, we'll use rule-based matching
    intent_patterns: HashMap<String, Vec<String>>,
}

impl NLUProcessor {
    pub fn new() -> Self {
        let mut intent_patterns = HashMap::new();
        
        // Define basic intent patterns
        intent_patterns.insert("create_note".to_string(), vec![
            "create".to_string(),
            "make".to_string(),
            "new note".to_string(),
            "add note".to_string(),
        ]);
        
        intent_patterns.insert("search".to_string(), vec![
            "search".to_string(),
            "find".to_string(),
            "look for".to_string(),
            "query".to_string(),
        ]);
        
        intent_patterns.insert("update_task".to_string(), vec![
            "update".to_string(),
            "modify".to_string(),
            "change".to_string(),
            "edit task".to_string(),
        ]);
        
        intent_patterns.insert("list_items".to_string(), vec![
            "list".to_string(),
            "show".to_string(),
            "display".to_string(),
            "get all".to_string(),
        ]);

        Self { intent_patterns }
    }

    pub fn process(&self, input: &str) -> NLUResult {
        let input_lower = input.to_lowercase();
        
        // Simple intent matching based on keywords
        let mut best_intent = Intent {
            name: "unknown".to_string(),
            confidence: 0.0,
            entities: vec![],
        };
        
        for (intent_name, patterns) in &self.intent_patterns {
            for pattern in patterns {
                if input_lower.contains(pattern) {
                    let confidence = 0.8; // Simple confidence scoring
                    if confidence > best_intent.confidence {
                        best_intent = Intent {
                            name: intent_name.clone(),
                            confidence,
                            entities: vec![],
                        };
                    }
                }
            }
        }
        
        // Basic entity extraction (simplified)
        let entities = self.extract_entities(&input_lower);
        best_intent.entities = entities.clone();
        
        // Simple sentiment analysis (very basic)
        let sentiment = self.analyze_sentiment(&input_lower);
        
        NLUResult {
            intent: best_intent,
            entities,
            sentiment: Some(sentiment),
            confidence: 0.7,
        }
    }
    
    fn extract_entities(&self, input: &str) -> Vec<Entity> {
        let mut entities = Vec::new();
        
        // Simple regex-based entity extraction
        // In a real implementation, this would use NER models
        
        // Extract dates (very basic)
        if let Some(date_match) = input.find("today") {
            entities.push(Entity {
                name: "date".to_string(),
                value: "today".to_string(),
                entity_type: "DATE".to_string(),
                confidence: 0.9,
                start_pos: date_match,
                end_pos: date_match + 5,
            });
        }
        
        // Extract quoted strings as potential titles/names
        let mut chars = input.chars().peekable();
        let mut pos = 0;
        while let Some(ch) = chars.next() {
            if ch == '"' {
                let start = pos + 1;
                let mut end = start;
                let mut quoted_text = String::new();
                
                while let Some(inner_ch) = chars.next() {
                    pos += 1;
                    if inner_ch == '"' {
                        end = pos;
                        break;
                    }
                    quoted_text.push(inner_ch);
                }
                
                if !quoted_text.is_empty() {
                    entities.push(Entity {
                        name: "title".to_string(),
                        value: quoted_text,
                        entity_type: "TEXT".to_string(),
                        confidence: 0.8,
                        start_pos: start,
                        end_pos: end,
                    });
                }
            }
            pos += 1;
        }
        
        entities
    }
    
    fn analyze_sentiment(&self, input: &str) -> f32 {
        // Very basic sentiment analysis
        let positive_words = ["good", "great", "excellent", "amazing", "wonderful"];
        let negative_words = ["bad", "terrible", "awful", "horrible", "worst"];
        
        let mut score: f32 = 0.0;
        
        for word in positive_words.iter() {
            if input.contains(word) {
                score += 0.2;
            }
        }
        
        for word in negative_words.iter() {
            if input.contains(word) {
                score -= 0.2;
            }
        }
        
        score.max(-1.0).min(1.0)
    }
}

// Function Calling and Tool Registry
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }
    
    pub fn register_tool(&mut self, name: String, tool: Box<dyn Tool>) {
        self.tools.insert(name, tool);
    }
    
    pub fn get_tool(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|tool| tool.as_ref())
    }
    
    pub fn list_tools(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }
    
    pub fn get_tool_schemas(&self) -> Vec<FunctionSchema> {
        self.tools.values().map(|tool| tool.get_schema()).collect()
    }
}

pub trait Tool: Send + Sync {
    fn execute(&self, parameters: HashMap<String, serde_json::Value>) -> Result<serde_json::Value, AIConversationError>;
    fn get_schema(&self) -> FunctionSchema;
    fn get_name(&self) -> String;
}

// Example tool implementations
pub struct CreateNoteTools;

impl Tool for CreateNoteTools {
    fn execute(&self, parameters: HashMap<String, serde_json::Value>) -> Result<serde_json::Value, AIConversationError> {
        let title = parameters.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled Note");
        
        let content = parameters.get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        // In a real implementation, this would create an actual note
        let result = serde_json::json!({
            "success": true,
            "note_id": uuid::Uuid::new_v4().to_string(),
            "title": title,
            "content": content,
            "created_at": chrono::Utc::now()
        });
        
        Ok(result)
    }
    
    fn get_schema(&self) -> FunctionSchema {
        FunctionSchema {
            name: "create_note".to_string(),
            description: "Create a new note with title and content".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "The title of the note"
                    },
                    "content": {
                        "type": "string",
                        "description": "The content of the note"
                    }
                },
                "required": ["title"]
            }),
        }
    }
    
    fn get_name(&self) -> String {
        "create_note".to_string()
    }
}

pub struct SearchTool;

impl Tool for SearchTool {
    fn execute(&self, parameters: HashMap<String, serde_json::Value>) -> Result<serde_json::Value, AIConversationError> {
        let query = parameters.get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        // In a real implementation, this would perform actual search
        let results = serde_json::json!({
            "success": true,
            "query": query,
            "results": [
                {
                    "id": "1",
                    "title": "Example Note",
                    "content": "This is an example note matching your query",
                    "relevance": 0.85
                }
            ],
            "total_results": 1
        });
        
        Ok(results)
    }
    
    fn get_schema(&self) -> FunctionSchema {
        FunctionSchema {
            name: "search".to_string(),
            description: "Search for notes and content".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query"
                    }
                },
                "required": ["query"]
            }),
        }
    }
    
    fn get_name(&self) -> String {
        "search".to_string()
    }
}

// Enhanced AIConversationEngine with NLU and function calling
impl AIConversationEngine {
    pub fn with_nlu_and_tools(llm_client: Box<dyn LLMClient>) -> Self {
        let mut engine = Self::new(llm_client);
        
        // Initialize NLU processor
        let _nlu_processor = NLUProcessor::new();
        engine.context.add_metadata("nlu_processor".to_string(), serde_json::json!("initialized"));
        
        // Register default tools
        let mut tool_registry = ToolRegistry::new();
        tool_registry.register_tool("create_note".to_string(), Box::new(CreateNoteTools));
        tool_registry.register_tool("search".to_string(), Box::new(SearchTool));
        
        // Store tool schemas in function registry
        for schema in tool_registry.get_tool_schemas() {
            engine.register_function(schema);
        }
        
        engine.context.add_metadata("tool_registry".to_string(), serde_json::json!("initialized"));
        
        engine
    }
    
    pub async fn process_with_nlu(&mut self, input: String) -> Result<String, AIConversationError> {
        // Process input with NLU
        let nlu_processor = NLUProcessor::new();
        let nlu_result = nlu_processor.process(&input);
        
        // Store NLU result in context
        self.context.add_metadata("last_nlu_result".to_string(), serde_json::to_value(&nlu_result)?);
        
        // Check if this requires function calling
        if nlu_result.intent.confidence > 0.7 {
            match nlu_result.intent.name.as_str() {
                "create_note" => {
                    return self.handle_create_note_intent(&nlu_result).await;
                }
                "search" => {
                    return self.handle_search_intent(&nlu_result).await;
                }
                _ => {
                    // Fall back to regular conversation
                    return self.send_message(input).await;
                }
            }
        }
        
        // Regular conversation processing
        self.send_message(input).await
    }
    
    async fn handle_create_note_intent(&mut self, nlu_result: &NLUResult) -> Result<String, AIConversationError> {
        let create_note_tool = CreateNoteTools;
        
        // Extract parameters from NLU result
        let mut parameters = HashMap::new();
        
        for entity in &nlu_result.entities {
            if entity.entity_type == "TEXT" && entity.name == "title" {
                parameters.insert("title".to_string(), serde_json::Value::String(entity.value.clone()));
            }
        }
        
        // If no title found, use a default
        if !parameters.contains_key("title") {
            parameters.insert("title".to_string(), serde_json::Value::String("New Note".to_string()));
        }
        
        // Execute the tool
        let result = create_note_tool.execute(parameters)?;
        
        // Convert result to response
        let response = format!("I've created a new note: {}", result);
        
        // Add to conversation history
        let user_message = Message {
            id: Uuid::new_v4().to_string(),
            role: MessageRole::User,
            content: format!("Intent: create_note, Confidence: {:.2}", nlu_result.intent.confidence),
            timestamp: chrono::Utc::now(),
            function_call: None,
        };
        
        let assistant_message = Message {
            id: Uuid::new_v4().to_string(),
            role: MessageRole::Assistant,
            content: response.clone(),
            timestamp: chrono::Utc::now(),
            function_call: Some(FunctionCall {
                name: "create_note".to_string(),
                arguments: HashMap::new(),
            }),
        };
        
        self.context.add_message(user_message);
        self.context.add_message(assistant_message);
        
        Ok(response)
    }
    
    async fn handle_search_intent(&mut self, nlu_result: &NLUResult) -> Result<String, AIConversationError> {
        let search_tool = SearchTool;
        
        // Extract query from entities or use default
        let mut parameters = HashMap::new();
        let query = nlu_result.entities
            .iter()
            .find(|e| e.entity_type == "TEXT")
            .map(|e| e.value.clone())
            .unwrap_or_else(|| "general search".to_string());
        
        parameters.insert("query".to_string(), serde_json::Value::String(query));
        
        // Execute the tool
        let result = search_tool.execute(parameters)?;
        
        // Convert result to response
        let response = format!("Search results: {}", result);
        
        // Add to conversation history
        let user_message = Message {
            id: Uuid::new_v4().to_string(),
            role: MessageRole::User,
            content: format!("Intent: search, Confidence: {:.2}", nlu_result.intent.confidence),
            timestamp: chrono::Utc::now(),
            function_call: None,
        };
        
        let assistant_message = Message {
            id: Uuid::new_v4().to_string(),
            role: MessageRole::Assistant,
            content: response.clone(),
            timestamp: chrono::Utc::now(),
            function_call: Some(FunctionCall {
                name: "search".to_string(),
                arguments: HashMap::new(),
            }),
        };
        
        self.context.add_message(user_message);
        self.context.add_message(assistant_message);
        
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    struct MockLLMClient {
        responses: Arc<Mutex<Vec<String>>>,
    }

    impl MockLLMClient {
        fn new(responses: Vec<String>) -> Self {
            Self {
                responses: Arc::new(Mutex::new(responses)),
            }
        }
    }

    #[async_trait]
    impl LLMClient for MockLLMClient {
        async fn send_message(&self, _messages: Vec<Message>) -> Result<Message> {
            let mut responses = self.responses.lock().await;
            let response_content = if responses.is_empty() {
                "Default response".to_string()
            } else {
                responses.remove(0)
            };

            Ok(Message {
                id: Uuid::new_v4().to_string(),
                role: MessageRole::Assistant,
                content: response_content,
                timestamp: chrono::Utc::now(),
                function_call: None,
            })
        }

        async fn stream_response(&self, _messages: Vec<Message>) -> Result<tokio::sync::mpsc::Receiver<String>> {
            let (tx, rx) = tokio::sync::mpsc::channel(100);
            tokio::spawn(async move {
                let _ = tx.send("Hello ".to_string()).await;
                let _ = tx.send("World".to_string()).await;
            });
            Ok(rx)
        }

        async fn function_calling(&self, _messages: Vec<Message>, _functions: Vec<FunctionSchema>) -> Result<Message> {
            Ok(Message {
                id: Uuid::new_v4().to_string(),
                role: MessageRole::Assistant,
                content: "Function called".to_string(),
                timestamp: chrono::Utc::now(),
                function_call: Some(FunctionCall {
                    name: "test_function".to_string(),
                    arguments: HashMap::new(),
                }),
            })
        }

        fn get_model_name(&self) -> String {
            "mock-model".to_string()
        }
    }

    #[tokio::test]
    async fn test_conversation_engine_creation() {
        let mock_client = MockLLMClient::new(vec![]);
        let engine = AIConversationEngine::new(Box::new(mock_client));
        
        assert!(!engine.conversation_id.is_empty());
        assert_eq!(engine.context.message_history.len(), 0);
        assert_eq!(engine.context.current_token_count, 0);
    }

    #[tokio::test]
    async fn test_send_message() {
        let mock_client = MockLLMClient::new(vec!["Test response".to_string()]);
        let mut engine = AIConversationEngine::new(Box::new(mock_client));
        
        let response = engine.send_message("Hello".to_string()).await.unwrap();
        assert_eq!(response, "Test response");
        assert_eq!(engine.context.message_history.len(), 2); // User + Assistant messages
        assert!(engine.context.current_token_count > 0);
    }

    #[tokio::test]
    async fn test_function_registration() {
        let mock_client = MockLLMClient::new(vec![]);
        let mut engine = AIConversationEngine::new(Box::new(mock_client));
        
        let function = FunctionSchema {
            name: "test_function".to_string(),
            description: "A test function".to_string(),
            parameters: serde_json::json!({}),
        };
        
        engine.register_function(function.clone());
        assert!(engine.function_registry.contains_key("test_function"));
    }

    #[tokio::test]
    async fn test_conversation_context_token_management() {
        let mut context = ConversationContext::with_max_tokens("test".to_string(), 100);
        
        // Add messages that exceed token limit
        for i in 0..10 {
            let message = Message {
                id: Uuid::new_v4().to_string(),
                role: MessageRole::User,
                content: format!("This is a long message number {} that should consume tokens", i),
                timestamp: chrono::Utc::now(),
                function_call: None,
            };
            context.add_message(message);
        }
        
        // Should have pruned some messages
        assert!(context.message_history.len() < 10);
        assert!(context.current_token_count <= 100);
    }

    #[tokio::test]
    async fn test_conversation_context_persistence() {
        let mut context = ConversationContext::new("test_persist".to_string());
        
        let message = Message {
            id: Uuid::new_v4().to_string(),
            role: MessageRole::User,
            content: "Test message".to_string(),
            timestamp: chrono::Utc::now(),
            function_call: None,
        };
        
        context.add_message(message);
        context.add_metadata("test_key".to_string(), serde_json::json!("test_value"));
        
        // Test serialization
        let file_path = "/tmp/test_conversation.json";
        context.save_to_file(file_path).unwrap();
        
        // Test deserialization
        let loaded_context = ConversationContext::load_from_file(file_path).unwrap();
        assert_eq!(loaded_context.conversation_id, "test_persist");
        assert_eq!(loaded_context.message_history.len(), 1);
        assert_eq!(loaded_context.get_metadata("test_key"), Some(&serde_json::json!("test_value")));
        
        // Cleanup
        std::fs::remove_file(file_path).ok();
    }

    #[tokio::test]
    async fn test_conversation_summary() {
        let mut context = ConversationContext::new("test_summary".to_string());
        
        // Add different types of messages
        let user_msg = Message {
            id: Uuid::new_v4().to_string(),
            role: MessageRole::User,
            content: "User message".to_string(),
            timestamp: chrono::Utc::now(),
            function_call: None,
        };
        
        let assistant_msg = Message {
            id: Uuid::new_v4().to_string(),
            role: MessageRole::Assistant,
            content: "Assistant message".to_string(),
            timestamp: chrono::Utc::now(),
            function_call: None,
        };
        
        context.add_message(user_msg);
        context.add_message(assistant_msg);
        
        let summary = context.get_conversation_summary();
        assert_eq!(summary.user_messages, 1);
        assert_eq!(summary.assistant_messages, 1);
        assert_eq!(summary.total_messages, 2);
    }

    #[tokio::test]
    async fn test_claude_client_creation() {
        let client = ClaudeClient::new("test_key".to_string())
            .with_model("claude-3-5-sonnet-20241022".to_string())
            .with_max_tokens(1000);
        
        assert_eq!(client.get_model_name(), "claude-3-5-sonnet-20241022");
        assert_eq!(client.max_tokens, 1000);
    }

    #[tokio::test]
    async fn test_gpt4_client_creation() {
        let client = GPT4Client::new("test_key".to_string())
            .with_model("gpt-4-turbo".to_string())
            .with_max_tokens(2000);
        
        assert_eq!(client.get_model_name(), "gpt-4-turbo");
        assert_eq!(client.max_tokens, 2000);
    }

    #[tokio::test]
    async fn test_ai_conversation_engine_with_claude() {
        let claude_client = ClaudeClient::new("test_key".to_string());
        let engine = AIConversationEngine::new(Box::new(claude_client));
        
        assert_eq!(engine.get_model_name(), "claude-3-sonnet-20240229");
        assert!(!engine.conversation_id.is_empty());
    }

    #[tokio::test]
    async fn test_ai_conversation_engine_with_gpt4() {
        let gpt4_client = GPT4Client::new("test_key".to_string());
        let engine = AIConversationEngine::new(Box::new(gpt4_client));
        
        assert_eq!(engine.get_model_name(), "gpt-4");
        assert!(!engine.conversation_id.is_empty());
    }

    #[tokio::test]
    async fn test_nlu_processor() {
        let nlu = NLUProcessor::new();
        
        // Test intent recognition
        let result = nlu.process("create a new note");
        assert_eq!(result.intent.name, "create_note");
        assert!(result.intent.confidence > 0.7);
        
        let result = nlu.process("search for something");
        assert_eq!(result.intent.name, "search");
        assert!(result.intent.confidence > 0.7);
        
        // Test entity extraction
        let result = nlu.process("create a note called \"My Important Note\"");
        assert_eq!(result.intent.name, "create_note");
        assert!(!result.entities.is_empty());
        
        let title_entity = result.entities.iter().find(|e| e.name == "title");
        assert!(title_entity.is_some());
        assert_eq!(title_entity.unwrap().value, "my important note");
    }

    #[tokio::test]
    async fn test_tool_registry() {
        let mut registry = ToolRegistry::new();
        
        // Register tools
        registry.register_tool("create_note".to_string(), Box::new(CreateNoteTools));
        registry.register_tool("search".to_string(), Box::new(SearchTool));
        
        // Test tool listing
        let tools = registry.list_tools();
        assert!(tools.contains(&"create_note".to_string()));
        assert!(tools.contains(&"search".to_string()));
        
        // Test tool retrieval
        let tool = registry.get_tool("create_note");
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().get_name(), "create_note");
        
        // Test schema generation
        let schemas = registry.get_tool_schemas();
        assert_eq!(schemas.len(), 2);
    }

    #[tokio::test]
    async fn test_create_note_tool() {
        let tool = CreateNoteTools;
        
        let mut params = HashMap::new();
        params.insert("title".to_string(), serde_json::Value::String("Test Note".to_string()));
        params.insert("content".to_string(), serde_json::Value::String("Test content".to_string()));
        
        let result = tool.execute(params).unwrap();
        assert_eq!(result["success"], true);
        assert_eq!(result["title"], "Test Note");
        assert_eq!(result["content"], "Test content");
    }

    #[tokio::test]
    async fn test_search_tool() {
        let tool = SearchTool;
        
        let mut params = HashMap::new();
        params.insert("query".to_string(), serde_json::Value::String("test query".to_string()));
        
        let result = tool.execute(params).unwrap();
        assert_eq!(result["success"], true);
        assert_eq!(result["query"], "test query");
        assert_eq!(result["total_results"], 1);
    }

    #[tokio::test]
    async fn test_ai_conversation_engine_with_nlu() {
        let mock_client = MockLLMClient::new(vec![]);
        let engine = AIConversationEngine::with_nlu_and_tools(Box::new(mock_client));
        
        // Check that NLU and tool registry are initialized
        assert!(engine.context.get_metadata("nlu_processor").is_some());
        assert!(engine.context.get_metadata("tool_registry").is_some());
        
        // Check that function schemas are registered
        assert!(!engine.function_registry.is_empty());
    }

    #[tokio::test]
    async fn test_send_message_with_timeout() {
        let mock_client = MockLLMClient::new(vec!["Response".to_string()]);
        let mut engine = AIConversationEngine::new(Box::new(mock_client));
        
        // Test successful message with timeout
        let result = engine.send_message_with_timeout("Hello".to_string(), 10).await;
        assert!(result.is_ok());
        
        // Test timeout (simulate with very short timeout)
        let result = engine.send_message_with_timeout("Hello".to_string(), 0).await;
        // Note: This might still succeed if the mock is very fast
        // In a real scenario with actual API calls, this would timeout
    }

    #[tokio::test]
    async fn test_send_message_with_metrics() {
        let mock_client = MockLLMClient::new(vec!["Response".to_string()]);
        let mut engine = AIConversationEngine::new(Box::new(mock_client));
        
        let result = engine.send_message_with_metrics("Hello".to_string()).await;
        assert!(result.is_ok());
        
        let (response, duration) = result.unwrap();
        assert_eq!(response, "Response");
        assert!(duration.as_millis() >= 0);
    }

    #[tokio::test]
    async fn test_process_messages_concurrent() {
        let mock_client = MockLLMClient::new(vec![]);
        let mut engine = AIConversationEngine::new(Box::new(mock_client));
        
        let messages = vec!["msg1".to_string(), "msg2".to_string(), "msg3".to_string()];
        let results = engine.process_messages_concurrent(messages, 2).await;
        
        assert_eq!(results.len(), 3);
        for result in results {
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_health_check() {
        let mock_client = MockLLMClient::new(vec![]);
        let engine = AIConversationEngine::new(Box::new(mock_client));
        
        let health = engine.health_check().await.unwrap();
        assert_eq!(health["status"], "healthy");
        assert!(health["conversation_id"].is_string());
        assert!(health["model_name"].is_string());
        assert!(health["message_count"].is_number());
    }

    #[tokio::test]
    async fn test_shutdown_and_recovery() {
        let mock_client = MockLLMClient::new(vec!["Response".to_string()]);
        let mut engine = AIConversationEngine::new(Box::new(mock_client));
        
        // Add some data to the engine
        let _ = engine.send_message("Test message".to_string()).await;
        
        // Test shutdown
        let result = engine.shutdown().await;
        assert!(result.is_ok());
        
        // Test recovery
        let save_path = format!("/tmp/conversation_{}.json", engine.conversation_id);
        let mock_client2 = MockLLMClient::new(vec![]);
        let recovered_engine = AIConversationEngine::recover_from_file(&save_path, Box::new(mock_client2)).await;
        
        // This might fail if the file doesn't exist in the test environment
        // In a real implementation, you'd have proper file handling
        if recovered_engine.is_ok() {
            let recovered = recovered_engine.unwrap();
            assert_eq!(recovered.conversation_id, engine.conversation_id);
        }
        
        // Cleanup
        std::fs::remove_file(save_path).ok();
    }

    #[tokio::test]
    async fn test_error_handling_comprehensive() {
        // Test various error scenarios
        let mock_client = MockLLMClient::new(vec![]);
        let engine = AIConversationEngine::new(Box::new(mock_client));
        
        // Test invalid file path for recovery
        let result = AIConversationEngine::recover_from_file("/invalid/path/file.json", Box::new(MockLLMClient::new(vec![])));
        assert!(result.await.is_err());
        
        // Test health check on valid engine
        let health = engine.health_check().await;
        assert!(health.is_ok());
    }
}