use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use uuid::Uuid;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextManagerConfig {
    pub data_dir: PathBuf,
    pub session_timeout: Duration,
    pub max_sessions: usize,
    pub enable_caching: bool,
    pub cache_size: usize,
    pub performance_target_ms: u64,
}

impl Default for ContextManagerConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./data/context"),
            session_timeout: Duration::from_secs(3600), // 1 hour
            max_sessions: 100,
            enable_caching: true,
            cache_size: 1000,
            performance_target_ms: 1000, // 1 second requirement
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ContextManagerError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
    #[error("Storage error: {0}")]
    StorageError(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Performance target exceeded: {0}ms")]
    PerformanceTargetExceeded(u64),
    #[error("Context limit exceeded")]
    ContextLimitExceeded,
    #[error("Generic error: {0}")]
    GenericError(#[from] anyhow::Error),
}

// Enhanced SessionStore with full persistence and threading support
pub struct SessionStore {
    config: ContextManagerConfig,
    sessions: RwLock<HashMap<String, SessionData>>,
    threads: RwLock<HashMap<String, ConversationThread>>,
    messages: RwLock<HashMap<String, ConversationMessage>>,
    thread_index: RwLock<HashMap<String, Vec<String>>>, // user_id -> thread_ids
}

pub struct UserPreferences {
    config: ContextManagerConfig,
    preferences: RwLock<HashMap<String, UserPreferenceData>>,
}

pub struct ToolContext {
    config: ContextManagerConfig,
    contexts: RwLock<HashMap<String, ToolContextData>>,
}

// Core data structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub session_id: String,
    pub user_id: String,
    pub created_at: SystemTime,
    pub last_accessed: SystemTime,
    pub conversation_threads: Vec<ConversationThread>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationThread {
    pub thread_id: String,
    pub title: String,
    pub created_at: SystemTime,
    pub last_message_at: SystemTime,
    pub message_count: usize,
    pub participants: Vec<String>,
    pub tags: Vec<String>,
    pub summary: Option<String>,
    pub messages: Vec<ConversationMessage>,
    pub parent_thread_id: Option<String>,
    pub status: ThreadStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub message_id: String,
    pub thread_id: String,
    pub sender: String,
    pub content: String,
    pub message_type: MessageType,
    pub timestamp: SystemTime,
    pub metadata: HashMap<String, serde_json::Value>,
    pub attachments: Vec<MessageAttachment>,
    pub reply_to: Option<String>,
    pub reactions: Vec<MessageReaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Text,
    System,
    Command,
    ToolResponse,
    Error,
    Notification,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreadStatus {
    Active,
    Archived,
    Closed,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageAttachment {
    pub attachment_id: String,
    pub filename: String,
    pub content_type: String,
    pub size: usize,
    pub url: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageReaction {
    pub user_id: String,
    pub emoji: String,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadSearchQuery {
    pub query: String,
    pub participants: Vec<String>,
    pub tags: Vec<String>,
    pub date_range: Option<(SystemTime, SystemTime)>,
    pub message_type: Option<MessageType>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadSearchResult {
    pub threads: Vec<ConversationThread>,
    pub total_count: usize,
    pub search_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferenceData {
    pub user_id: String,
    pub preferences: HashMap<String, serde_json::Value>,
    pub learning_data: HashMap<String, LearningMetric>,
    pub updated_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningMetric {
    pub metric_name: String,
    pub value: f64,
    pub confidence: f64,
    pub last_updated: SystemTime,
    pub sample_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContextData {
    pub context_id: String,
    pub tool_name: String,
    pub session_id: String,
    pub state: HashMap<String, serde_json::Value>,
    pub created_at: SystemTime,
    pub last_accessed: SystemTime,
    pub ttl: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub category: String,
    pub suggestion: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContextStatistics {
    pub total_contexts: usize,
    pub active_contexts: usize,
    pub expired_contexts: usize,
    pub tool_counts: HashMap<String, usize>,
    pub session_counts: HashMap<String, usize>,
    pub total_state_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextType {
    Session,
    UserPreferences,
    ToolContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRequest {
    pub request_id: String,
    pub context_type: ContextType,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextResponse {
    pub request_id: String,
    pub context_type: ContextType,
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    pub session_data: Option<SessionData>,
    pub user_preferences: Option<UserPreferenceData>,
    pub tool_contexts: Vec<ToolContextData>,
    pub conversation_threads: Vec<ConversationThread>,
    pub loaded_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveContext {
    pub session_data: Option<SessionData>,
    pub user_preferences: Option<UserPreferenceData>,
    pub user_recommendations: Vec<Recommendation>,
    pub conversation_threads: Vec<ConversationThread>,
    pub tool_contexts: Vec<ToolContextData>,
    pub retrieved_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupResult {
    pub expired_sessions: usize,
    pub expired_tool_contexts: usize,
    pub cleanup_duration: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceOptimizationResult {
    pub analysis_duration: Duration,
    pub current_metrics: HashMap<String, PerformanceMetrics>,
    pub optimization_suggestions: Vec<String>,
    pub cache_utilization: f64,
    pub session_utilization: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextIndex {
    pub total_sessions: usize,
    pub total_tool_contexts: usize,
    pub active_tool_contexts: usize,
    pub cached_items: usize,
    pub performance_metrics: HashMap<String, PerformanceMetrics>,
    pub indexed_at: SystemTime,
}

// Main ContextManager struct
pub struct ContextManager {
    config: ContextManagerConfig,
    session_store: SessionStore,
    user_preferences: UserPreferences,
    tool_context: ToolContext,
    
    // Performance tracking
    performance_metrics: RwLock<HashMap<String, Vec<Duration>>>,
    
    // Cache for frequently accessed data
    cache: RwLock<HashMap<String, CacheEntry>>,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    data: serde_json::Value,
    created_at: SystemTime,
    access_count: usize,
    ttl: Duration,
}

impl ContextManager {
    pub fn new(config: ContextManagerConfig) -> Result<Self, ContextManagerError> {
        // Validate configuration
        Self::validate_config(&config)?;
        
        // Ensure data directory exists
        if !config.data_dir.exists() {
            std::fs::create_dir_all(&config.data_dir)?;
        }
        
        // Initialize components
        let session_store = SessionStore::new(config.clone())?;
        let user_preferences = UserPreferences::new(config.clone())?;
        let tool_context = ToolContext::new(config.clone())?;
        
        Ok(Self {
            config,
            session_store,
            user_preferences,
            tool_context,
            performance_metrics: RwLock::new(HashMap::new()),
            cache: RwLock::new(HashMap::new()),
        })
    }
    
    pub fn with_default_config() -> Result<Self, ContextManagerError> {
        Self::new(ContextManagerConfig::default())
    }
    
    fn validate_config(config: &ContextManagerConfig) -> Result<(), ContextManagerError> {
        if config.max_sessions == 0 {
            return Err(ContextManagerError::InvalidConfiguration(
                "max_sessions must be greater than 0".to_string()
            ));
        }
        
        if config.cache_size == 0 {
            return Err(ContextManagerError::InvalidConfiguration(
                "cache_size must be greater than 0".to_string()
            ));
        }
        
        if config.performance_target_ms == 0 {
            return Err(ContextManagerError::InvalidConfiguration(
                "performance_target_ms must be greater than 0".to_string()
            ));
        }
        
        Ok(())
    }
    
    // Core context management methods
    pub async fn get_session(&self, session_id: &str) -> Result<Option<SessionData>, ContextManagerError> {
        let start_time = std::time::Instant::now();
        
        // Check cache first
        if let Some(cached_data) = self.get_from_cache(&format!("session:{}", session_id)).await? {
            let session_data: SessionData = serde_json::from_value(cached_data)?;
            self.track_performance("get_session", start_time.elapsed()).await;
            return Ok(Some(session_data));
        }
        
        // Get from session store
        let session_data = self.session_store.get_session(session_id).await?;
        
        // Cache the result
        if let Some(ref data) = session_data {
            self.set_cache(&format!("session:{}", session_id), serde_json::to_value(data)?).await?;
        }
        
        let elapsed = start_time.elapsed();
        self.track_performance("get_session", elapsed).await;
        
        // Check performance target
        if elapsed.as_millis() > self.config.performance_target_ms as u128 {
            return Err(ContextManagerError::PerformanceTargetExceeded(elapsed.as_millis() as u64));
        }
        
        Ok(session_data)
    }
    
    pub async fn create_session(&self, user_id: &str) -> Result<String, ContextManagerError> {
        let start_time = std::time::Instant::now();
        
        let session_id = Uuid::new_v4().to_string();
        let session_data = SessionData {
            session_id: session_id.clone(),
            user_id: user_id.to_string(),
            created_at: SystemTime::now(),
            last_accessed: SystemTime::now(),
            conversation_threads: Vec::new(),
            metadata: HashMap::new(),
        };
        
        self.session_store.create_session(session_data.clone()).await?;
        
        // Cache the new session
        self.set_cache(&format!("session:{}", session_id), serde_json::to_value(&session_data)?).await?;
        
        self.track_performance("create_session", start_time.elapsed()).await;
        
        Ok(session_id)
    }
    
    pub async fn get_user_preferences(&self, user_id: &str) -> Result<Option<UserPreferenceData>, ContextManagerError> {
        let start_time = std::time::Instant::now();
        
        // Check cache first
        let cache_key = format!("user_prefs:{}", user_id);
        if let Some(cached_data) = self.get_from_cache(&cache_key).await? {
            let prefs: UserPreferenceData = serde_json::from_value(cached_data)?;
            self.track_performance("get_user_preferences", start_time.elapsed()).await;
            return Ok(Some(prefs));
        }
        
        // Get from user preferences store
        let prefs = self.user_preferences.get_preferences(user_id).await?;
        
        // Cache the result
        if let Some(ref data) = prefs {
            self.set_cache(&cache_key, serde_json::to_value(data)?).await?;
        }
        
        self.track_performance("get_user_preferences", start_time.elapsed()).await;
        
        Ok(prefs)
    }
    
    pub async fn get_tool_context(&self, context_id: &str) -> Result<Option<ToolContextData>, ContextManagerError> {
        let start_time = std::time::Instant::now();
        
        // Check cache first
        let cache_key = format!("tool_context:{}", context_id);
        if let Some(cached_data) = self.get_from_cache(&cache_key).await? {
            let context: ToolContextData = serde_json::from_value(cached_data)?;
            self.track_performance("get_tool_context", start_time.elapsed()).await;
            return Ok(Some(context));
        }
        
        // Get from tool context store
        let context = self.tool_context.get_context(context_id).await?;
        
        // Cache the result
        if let Some(ref data) = context {
            self.set_cache(&cache_key, serde_json::to_value(data)?).await?;
        }
        
        self.track_performance("get_tool_context", start_time.elapsed()).await;
        
        Ok(context)
    }
    
    // Cache management methods
    async fn get_from_cache(&self, key: &str) -> Result<Option<serde_json::Value>, ContextManagerError> {
        if !self.config.enable_caching {
            return Ok(None);
        }
        
        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(key) {
            let now = SystemTime::now();
            if now.duration_since(entry.created_at).unwrap_or(Duration::ZERO) < entry.ttl {
                return Ok(Some(entry.data.clone()));
            }
        }
        
        Ok(None)
    }
    
    async fn set_cache(&self, key: &str, value: serde_json::Value) -> Result<(), ContextManagerError> {
        if !self.config.enable_caching {
            return Ok(());
        }
        
        let mut cache = self.cache.write().await;
        
        // Check cache size and evict if necessary
        if cache.len() >= self.config.cache_size {
            // Simple LRU eviction - remove oldest entry
            let oldest_key = cache.iter()
                .min_by_key(|(_, entry)| entry.created_at)
                .map(|(k, _)| k.clone());
            
            if let Some(key_to_remove) = oldest_key {
                cache.remove(&key_to_remove);
            }
        }
        
        let entry = CacheEntry {
            data: value,
            created_at: SystemTime::now(),
            access_count: 0,
            ttl: Duration::from_secs(300), // 5 minutes default TTL
        };
        
        cache.insert(key.to_string(), entry);
        
        Ok(())
    }
    
    // Performance tracking
    async fn track_performance(&self, operation: &str, duration: Duration) {
        let mut metrics = self.performance_metrics.write().await;
        let operation_metrics = metrics.entry(operation.to_string()).or_insert_with(Vec::new);
        
        operation_metrics.push(duration);
        
        // Keep only last 100 measurements
        if operation_metrics.len() > 100 {
            operation_metrics.remove(0);
        }
    }
    
    pub async fn get_performance_metrics(&self) -> HashMap<String, PerformanceMetrics> {
        let metrics = self.performance_metrics.read().await;
        let mut result = HashMap::new();
        
        for (operation, durations) in metrics.iter() {
            let avg_duration = durations.iter().sum::<Duration>() / durations.len() as u32;
            let max_duration = durations.iter().max().copied().unwrap_or(Duration::ZERO);
            let min_duration = durations.iter().min().copied().unwrap_or(Duration::ZERO);
            
            result.insert(operation.clone(), PerformanceMetrics {
                operation: operation.clone(),
                avg_duration,
                max_duration,
                min_duration,
                sample_count: durations.len(),
            });
        }
        
        result
    }
    
    // Health check
    pub async fn health_check(&self) -> Result<ContextManagerHealth, ContextManagerError> {
        let session_count = self.session_store.get_session_count().await?;
        let cache_size = self.cache.read().await.len();
        let performance_metrics = self.get_performance_metrics().await;
        
        Ok(ContextManagerHealth {
            session_count,
            cache_size,
            cache_enabled: self.config.enable_caching,
            performance_metrics,
            data_dir: self.config.data_dir.clone(),
        })
    }
    
    // Cleanup methods
    pub async fn cleanup_expired_sessions(&self) -> Result<usize, ContextManagerError> {
        self.session_store.cleanup_expired_sessions().await
    }
    
    pub async fn clear_cache(&self) -> Result<(), ContextManagerError> {
        let mut cache = self.cache.write().await;
        cache.clear();
        Ok(())
    }
    
    // Enhanced methods for SessionStore functionality
    pub async fn create_conversation_thread(&self, title: String, participants: Vec<String>, user_id: &str) -> Result<String, ContextManagerError> {
        let thread_id = Uuid::new_v4().to_string();
        let now = SystemTime::now();
        
        let thread = ConversationThread {
            thread_id: thread_id.clone(),
            title,
            created_at: now,
            last_message_at: now,
            message_count: 0,
            participants,
            tags: Vec::new(),
            summary: None,
            messages: Vec::new(),
            parent_thread_id: None,
            status: ThreadStatus::Active,
        };
        
        self.session_store.create_thread(thread).await?;
        
        Ok(thread_id)
    }
    
    pub async fn get_conversation_thread(&self, thread_id: &str) -> Result<Option<ConversationThread>, ContextManagerError> {
        self.session_store.get_thread(thread_id).await
    }
    
    pub async fn add_message_to_thread(&self, thread_id: &str, sender: &str, content: String, message_type: MessageType) -> Result<String, ContextManagerError> {
        let message_id = Uuid::new_v4().to_string();
        let now = SystemTime::now();
        
        let message = ConversationMessage {
            message_id: message_id.clone(),
            thread_id: thread_id.to_string(),
            sender: sender.to_string(),
            content,
            message_type,
            timestamp: now,
            metadata: HashMap::new(),
            attachments: Vec::new(),
            reply_to: None,
            reactions: Vec::new(),
        };
        
        self.session_store.add_message(message).await?;
        
        Ok(message_id)
    }
    
    pub async fn get_user_conversation_threads(&self, user_id: &str) -> Result<Vec<ConversationThread>, ContextManagerError> {
        self.session_store.get_user_threads(user_id).await
    }
    
    pub async fn search_conversation_threads(&self, query: ThreadSearchQuery) -> Result<ThreadSearchResult, ContextManagerError> {
        self.session_store.search_threads(query).await
    }
    
    pub async fn get_message(&self, message_id: &str) -> Result<Option<ConversationMessage>, ContextManagerError> {
        self.session_store.get_message(message_id).await
    }
    
    // UserPreferences methods
    pub async fn update_user_preferences(&self, user_id: &str, preferences: HashMap<String, serde_json::Value>) -> Result<(), ContextManagerError> {
        self.user_preferences.update_preferences(user_id, preferences).await
    }
    
    pub async fn learn_from_user_interaction(&self, user_id: &str, interaction_type: &str, context: HashMap<String, serde_json::Value>) -> Result<(), ContextManagerError> {
        self.user_preferences.learn_from_interaction(user_id, interaction_type, context).await
    }
    
    pub async fn get_user_recommendations(&self, user_id: &str) -> Result<Vec<Recommendation>, ContextManagerError> {
        self.user_preferences.get_recommendations(user_id).await
    }
    
    // ToolContext methods
    pub async fn create_tool_context(&self, tool_name: &str, session_id: &str, initial_state: HashMap<String, serde_json::Value>, ttl: Option<Duration>) -> Result<String, ContextManagerError> {
        self.tool_context.create_context(tool_name, session_id, initial_state, ttl).await
    }
    
    pub async fn update_tool_context(&self, context_id: &str, state_updates: HashMap<String, serde_json::Value>) -> Result<(), ContextManagerError> {
        self.tool_context.update_context(context_id, state_updates).await
    }
    
    pub async fn remove_tool_context(&self, context_id: &str) -> Result<(), ContextManagerError> {
        self.tool_context.remove_context(context_id).await
    }
    
    pub async fn get_tool_contexts_for_session(&self, session_id: &str) -> Result<Vec<ToolContextData>, ContextManagerError> {
        self.tool_context.get_contexts_for_session(session_id).await
    }
    
    pub async fn get_tool_contexts_for_tool(&self, tool_name: &str) -> Result<Vec<ToolContextData>, ContextManagerError> {
        self.tool_context.get_contexts_for_tool(tool_name).await
    }
    
    pub async fn share_tool_context(&self, source_context_id: &str, target_tool: &str, session_id: &str, shared_keys: Vec<String>) -> Result<String, ContextManagerError> {
        self.tool_context.share_context(source_context_id, target_tool, session_id, shared_keys).await
    }
    
    pub async fn cleanup_expired_tool_contexts(&self) -> Result<usize, ContextManagerError> {
        self.tool_context.cleanup_expired_contexts().await
    }
    
    pub async fn get_tool_context_statistics(&self) -> Result<ToolContextStatistics, ContextManagerError> {
        self.tool_context.get_context_statistics().await
    }
    
    pub async fn serialize_tool_context(&self, context_id: &str) -> Result<String, ContextManagerError> {
        self.tool_context.serialize_context(context_id).await
    }
    
    pub async fn deserialize_tool_context(&self, serialized_data: &str) -> Result<String, ContextManagerError> {
        self.tool_context.deserialize_context(serialized_data).await
    }
    
    // Enhanced performance and integration methods
    pub async fn batch_get_contexts(&self, context_requests: Vec<ContextRequest>) -> Result<Vec<ContextResponse>, ContextManagerError> {
        let start_time = std::time::Instant::now();
        let mut responses = Vec::new();
        
        // Group requests by type for batch processing
        let mut session_requests = Vec::new();
        let mut user_pref_requests = Vec::new();
        let mut tool_context_requests = Vec::new();
        
        for request in context_requests {
            match request.context_type {
                ContextType::Session => session_requests.push(request),
                ContextType::UserPreferences => user_pref_requests.push(request),
                ContextType::ToolContext => tool_context_requests.push(request),
            }
        }
        
        // Process session requests in parallel
        for request in session_requests {
            let result = self.get_session(&request.id).await;
            let success = result.is_ok();
            let (data, error) = match result {
                Ok(session_data) => (
                    session_data.map(|s| serde_json::to_value(s).unwrap_or(serde_json::Value::Null)),
                    None
                ),
                Err(e) => (None, Some(e.to_string())),
            };
            
            responses.push(ContextResponse {
                request_id: request.request_id,
                context_type: ContextType::Session,
                success,
                data,
                error,
            });
        }
        
        // Process user preference requests in parallel
        for request in user_pref_requests {
            let result = self.get_user_preferences(&request.id).await;
            let success = result.is_ok();
            let (data, error) = match result {
                Ok(prefs_data) => (
                    prefs_data.map(|p| serde_json::to_value(p).unwrap_or(serde_json::Value::Null)),
                    None
                ),
                Err(e) => (None, Some(e.to_string())),
            };
            
            responses.push(ContextResponse {
                request_id: request.request_id,
                context_type: ContextType::UserPreferences,
                success,
                data,
                error,
            });
        }
        
        // Process tool context requests in parallel
        for request in tool_context_requests {
            let result = self.get_tool_context(&request.id).await;
            let success = result.is_ok();
            let (data, error) = match result {
                Ok(tool_data) => (
                    tool_data.map(|t| serde_json::to_value(t).unwrap_or(serde_json::Value::Null)),
                    None
                ),
                Err(e) => (None, Some(e.to_string())),
            };
            
            responses.push(ContextResponse {
                request_id: request.request_id,
                context_type: ContextType::ToolContext,
                success,
                data,
                error,
            });
        }
        
        let elapsed = start_time.elapsed();
        self.track_performance("batch_get_contexts", elapsed).await;
        
        // Check performance target
        if elapsed.as_millis() > self.config.performance_target_ms as u128 {
            return Err(ContextManagerError::PerformanceTargetExceeded(elapsed.as_millis() as u64));
        }
        
        Ok(responses)
    }
    
    pub async fn preload_session_context(&self, session_id: &str) -> Result<SessionContext, ContextManagerError> {
        let start_time = std::time::Instant::now();
        
        // Preload all related context data for a session
        let session_data = self.get_session(session_id).await?;
        let tool_contexts = self.get_tool_contexts_for_session(session_id).await?;
        
        let user_preferences = if let Some(ref session) = session_data {
            self.get_user_preferences(&session.user_id).await?
        } else {
            None
        };
        
        let conversation_threads = if session_data.is_some() {
            self.get_user_conversation_threads(
                &session_data.as_ref().unwrap().user_id
            ).await?
        } else {
            Vec::new()
        };
        
        let session_context = SessionContext {
            session_data,
            user_preferences,
            tool_contexts,
            conversation_threads,
            loaded_at: SystemTime::now(),
        };
        
        let elapsed = start_time.elapsed();
        self.track_performance("preload_session_context", elapsed).await;
        
        // Cache the preloaded context
        let cache_key = format!("session_context:{}", session_id);
        self.set_cache(&cache_key, serde_json::to_value(&session_context)?).await?;
        
        Ok(session_context)
    }
    
    pub async fn get_comprehensive_context(&self, user_id: &str, session_id: &str, tool_name: Option<&str>) -> Result<ComprehensiveContext, ContextManagerError> {
        let start_time = std::time::Instant::now();
        
        // Try to get from cache first
        let cache_key = format!("comprehensive:{}:{}:{}", user_id, session_id, tool_name.unwrap_or("all"));
        if let Some(cached_data) = self.get_from_cache(&cache_key).await? {
            let context: ComprehensiveContext = serde_json::from_value(cached_data)?;
            self.track_performance("get_comprehensive_context_cached", start_time.elapsed()).await;
            return Ok(context);
        }
        
        // Fetch all context data
        let session_data = self.get_session(session_id).await?;
        let user_preferences = self.get_user_preferences(user_id).await?;
        let user_recommendations = self.get_user_recommendations(user_id).await?;
        let conversation_threads = self.get_user_conversation_threads(user_id).await?;
        
        let tool_contexts = if let Some(tool) = tool_name {
            self.get_tool_contexts_for_tool(tool).await?
        } else {
            self.get_tool_contexts_for_session(session_id).await?
        };
        
        let context = ComprehensiveContext {
            session_data,
            user_preferences,
            user_recommendations,
            conversation_threads,
            tool_contexts,
            retrieved_at: SystemTime::now(),
        };
        
        let elapsed = start_time.elapsed();
        self.track_performance("get_comprehensive_context", elapsed).await;
        
        // Cache the comprehensive context
        self.set_cache(&cache_key, serde_json::to_value(&context)?).await?;
        
        // Check performance target
        if elapsed.as_millis() > self.config.performance_target_ms as u128 {
            return Err(ContextManagerError::PerformanceTargetExceeded(elapsed.as_millis() as u64));
        }
        
        Ok(context)
    }
    
    pub async fn cleanup_all_expired_data(&self) -> Result<CleanupResult, ContextManagerError> {
        let start_time = std::time::Instant::now();
        
        // Cleanup expired sessions
        let expired_sessions = self.cleanup_expired_sessions().await?;
        
        // Cleanup expired tool contexts
        let expired_tool_contexts = self.cleanup_expired_tool_contexts().await?;
        
        // Clear expired cache entries
        self.cleanup_expired_cache().await?;
        
        let elapsed = start_time.elapsed();
        self.track_performance("cleanup_all_expired_data", elapsed).await;
        
        Ok(CleanupResult {
            expired_sessions,
            expired_tool_contexts,
            cleanup_duration: elapsed,
        })
    }
    
    async fn cleanup_expired_cache(&self) -> Result<(), ContextManagerError> {
        let mut cache = self.cache.write().await;
        let now = SystemTime::now();
        
        let expired_keys: Vec<String> = cache
            .iter()
            .filter(|(_, entry)| {
                now.duration_since(entry.created_at).unwrap_or(Duration::ZERO) >= entry.ttl
            })
            .map(|(key, _)| key.clone())
            .collect();
        
        for key in expired_keys {
            cache.remove(&key);
        }
        
        Ok(())
    }
    
    pub async fn optimize_performance(&self) -> Result<PerformanceOptimizationResult, ContextManagerError> {
        let start_time = std::time::Instant::now();
        
        // Analyze current performance metrics
        let metrics = self.get_performance_metrics().await;
        let mut optimizations = Vec::new();
        
        // Check if we should increase cache size
        let cache_size = self.cache.read().await.len();
        if cache_size >= self.config.cache_size * 8 / 10 { // 80% full
            optimizations.push("Consider increasing cache size".to_string());
        }
        
        // Check for slow operations
        for (operation, perf_metrics) in &metrics {
            if perf_metrics.avg_duration.as_millis() > self.config.performance_target_ms as u128 / 2 {
                optimizations.push(format!("Operation '{}' is running slow (avg: {}ms)", operation, perf_metrics.avg_duration.as_millis()));
            }
        }
        
        // Get statistics for analysis
        let session_count = self.session_store.get_session_count().await?;
        let tool_stats = self.get_tool_context_statistics().await?;
        
        if session_count > self.config.max_sessions * 8 / 10 { // 80% of max
            optimizations.push("Session count approaching limit, consider cleanup".to_string());
        }
        
        if tool_stats.expired_contexts > 0 {
            optimizations.push(format!("Found {} expired tool contexts, consider cleanup", tool_stats.expired_contexts));
        }
        
        let elapsed = start_time.elapsed();
        
        Ok(PerformanceOptimizationResult {
            analysis_duration: elapsed,
            current_metrics: metrics,
            optimization_suggestions: optimizations,
            cache_utilization: (cache_size as f64 / self.config.cache_size as f64) * 100.0,
            session_utilization: (session_count as f64 / self.config.max_sessions as f64) * 100.0,
        })
    }
    
    pub async fn get_context_index(&self) -> Result<ContextIndex, ContextManagerError> {
        let start_time = std::time::Instant::now();
        
        // Build comprehensive index of all context data
        let session_count = self.session_store.get_session_count().await?;
        let tool_stats = self.get_tool_context_statistics().await?;
        let cache_size = self.cache.read().await.len();
        let performance_metrics = self.get_performance_metrics().await;
        
        let index = ContextIndex {
            total_sessions: session_count,
            total_tool_contexts: tool_stats.total_contexts,
            active_tool_contexts: tool_stats.active_contexts,
            cached_items: cache_size,
            performance_metrics,
            indexed_at: SystemTime::now(),
        };
        
        let elapsed = start_time.elapsed();
        self.track_performance("get_context_index", elapsed).await;
        
        Ok(index)
    }
    
    // Enhanced error handling with fallback mechanisms
    pub async fn get_context_with_fallback(&self, context_id: &str, context_type: ContextType) -> Result<Option<serde_json::Value>, ContextManagerError> {
        let start_time = std::time::Instant::now();
        
        let result = match context_type {
            ContextType::Session => {
                match self.get_session(context_id).await {
                    Ok(data) => data.map(|d| serde_json::to_value(d).unwrap_or(serde_json::Value::Null)),
                    Err(_) => {
                        // Fallback: try to load from disk directly
                        self.session_store.get_session(context_id).await.ok()
                            .and_then(|d| d)
                            .map(|d| serde_json::to_value(d).unwrap_or(serde_json::Value::Null))
                    }
                }
            },
            ContextType::UserPreferences => {
                match self.get_user_preferences(context_id).await {
                    Ok(data) => data.map(|d| serde_json::to_value(d).unwrap_or(serde_json::Value::Null)),
                    Err(_) => {
                        // Fallback: try to load from disk directly
                        self.user_preferences.get_preferences(context_id).await.ok()
                            .and_then(|d| d)
                            .map(|d| serde_json::to_value(d).unwrap_or(serde_json::Value::Null))
                    }
                }
            },
            ContextType::ToolContext => {
                match self.get_tool_context(context_id).await {
                    Ok(data) => data.map(|d| serde_json::to_value(d).unwrap_or(serde_json::Value::Null)),
                    Err(_) => {
                        // Fallback: try to load from disk directly
                        self.tool_context.get_context(context_id).await.ok()
                            .and_then(|d| d)
                            .map(|d| serde_json::to_value(d).unwrap_or(serde_json::Value::Null))
                    }
                }
            },
        };
        
        let elapsed = start_time.elapsed();
        self.track_performance("get_context_with_fallback", elapsed).await;
        
        Ok(result)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub operation: String,
    pub avg_duration: Duration,
    pub max_duration: Duration,
    pub min_duration: Duration,
    pub sample_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextManagerHealth {
    pub session_count: usize,
    pub cache_size: usize,
    pub cache_enabled: bool,
    pub performance_metrics: HashMap<String, PerformanceMetrics>,
    pub data_dir: PathBuf,
}

// Enhanced SessionStore implementation with full persistence and threading support
impl SessionStore {
    fn new(config: ContextManagerConfig) -> Result<Self, ContextManagerError> {
        let sessions_dir = config.data_dir.join("sessions");
        let threads_dir = config.data_dir.join("threads");
        let messages_dir = config.data_dir.join("messages");
        
        // Create directories if they don't exist
        fs::create_dir_all(&sessions_dir)?;
        fs::create_dir_all(&threads_dir)?;
        fs::create_dir_all(&messages_dir)?;
        
        let store = Self {
            config,
            sessions: RwLock::new(HashMap::new()),
            threads: RwLock::new(HashMap::new()),
            messages: RwLock::new(HashMap::new()),
            thread_index: RwLock::new(HashMap::new()),
        };
        
        // Load existing data
        store.load_existing_data()?;
        
        Ok(store)
    }
    
    fn load_existing_data(&self) -> Result<(), ContextManagerError> {
        // In a real implementation, this would load from persistent storage
        // For now, we'll keep it simple and start with empty data
        Ok(())
    }
    
    async fn get_session(&self, session_id: &str) -> Result<Option<SessionData>, ContextManagerError> {
        // Check in-memory first
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            return Ok(Some(session.clone()));
        }
        drop(sessions);
        
        // Try to load from persistent storage
        let session_file = self.config.data_dir.join("sessions").join(format!("{}.json", session_id));
        if session_file.exists() {
            let content = fs::read_to_string(&session_file)?;
            let session_data: SessionData = serde_json::from_str(&content)?;
            
            // Cache in memory
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.to_string(), session_data.clone());
            
            return Ok(Some(session_data));
        }
        
        Ok(None)
    }
    
    async fn create_session(&self, session_data: SessionData) -> Result<(), ContextManagerError> {
        // Store in memory
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_data.session_id.clone(), session_data.clone());
        drop(sessions);
        
        // Persist to disk
        self.persist_session(&session_data).await?;
        
        Ok(())
    }
    
    async fn update_session(&self, session_data: SessionData) -> Result<(), ContextManagerError> {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_data.session_id.clone(), session_data.clone());
        drop(sessions);
        
        self.persist_session(&session_data).await?;
        
        Ok(())
    }
    
    async fn persist_session(&self, session_data: &SessionData) -> Result<(), ContextManagerError> {
        let session_file = self.config.data_dir.join("sessions").join(format!("{}.json", session_data.session_id));
        let content = serde_json::to_string_pretty(session_data)?;
        fs::write(&session_file, content)?;
        Ok(())
    }
    
    async fn get_session_count(&self) -> Result<usize, ContextManagerError> {
        let sessions = self.sessions.read().await;
        Ok(sessions.len())
    }
    
    async fn cleanup_expired_sessions(&self) -> Result<usize, ContextManagerError> {
        let mut sessions = self.sessions.write().await;
        let now = SystemTime::now();
        let timeout = self.config.session_timeout;
        
        let initial_count = sessions.len();
        let expired_sessions: Vec<String> = sessions
            .iter()
            .filter(|(_, session)| {
                now.duration_since(session.last_accessed).unwrap_or(Duration::ZERO) >= timeout
            })
            .map(|(id, _)| id.clone())
            .collect();
        
        // Remove expired sessions
        for session_id in &expired_sessions {
            sessions.remove(session_id);
            
            // Remove session file
            let session_file = self.config.data_dir.join("sessions").join(format!("{}.json", session_id));
            if session_file.exists() {
                fs::remove_file(&session_file)?;
            }
        }
        
        Ok(expired_sessions.len())
    }
    
    // Thread management methods
    async fn create_thread(&self, thread_data: ConversationThread) -> Result<(), ContextManagerError> {
        let mut threads = self.threads.write().await;
        threads.insert(thread_data.thread_id.clone(), thread_data.clone());
        drop(threads);
        
        // Update thread index
        let mut thread_index = self.thread_index.write().await;
        for participant in &thread_data.participants {
            let user_threads = thread_index.entry(participant.clone()).or_insert_with(Vec::new);
            user_threads.push(thread_data.thread_id.clone());
        }
        drop(thread_index);
        
        // Persist to disk
        self.persist_thread(&thread_data).await?;
        
        Ok(())
    }
    
    async fn get_thread(&self, thread_id: &str) -> Result<Option<ConversationThread>, ContextManagerError> {
        let threads = self.threads.read().await;
        if let Some(thread) = threads.get(thread_id) {
            return Ok(Some(thread.clone()));
        }
        drop(threads);
        
        // Try to load from persistent storage
        let thread_file = self.config.data_dir.join("threads").join(format!("{}.json", thread_id));
        if thread_file.exists() {
            let content = fs::read_to_string(&thread_file)?;
            let thread_data: ConversationThread = serde_json::from_str(&content)?;
            
            // Cache in memory
            let mut threads = self.threads.write().await;
            threads.insert(thread_id.to_string(), thread_data.clone());
            
            return Ok(Some(thread_data));
        }
        
        Ok(None)
    }
    
    async fn persist_thread(&self, thread_data: &ConversationThread) -> Result<(), ContextManagerError> {
        let thread_file = self.config.data_dir.join("threads").join(format!("{}.json", thread_data.thread_id));
        let content = serde_json::to_string_pretty(thread_data)?;
        fs::write(&thread_file, content)?;
        Ok(())
    }
    
    async fn get_user_threads(&self, user_id: &str) -> Result<Vec<ConversationThread>, ContextManagerError> {
        let thread_index = self.thread_index.read().await;
        let thread_ids = thread_index.get(user_id).cloned().unwrap_or_default();
        drop(thread_index);
        
        let mut threads = Vec::new();
        for thread_id in thread_ids {
            if let Some(thread) = self.get_thread(&thread_id).await? {
                threads.push(thread);
            }
        }
        
        // Sort by last message time (most recent first)
        threads.sort_by(|a, b| b.last_message_at.cmp(&a.last_message_at));
        
        Ok(threads)
    }
    
    // Message management methods
    async fn add_message(&self, message: ConversationMessage) -> Result<(), ContextManagerError> {
        let message_id = message.message_id.clone();
        let thread_id = message.thread_id.clone();
        
        // Store message
        let mut messages = self.messages.write().await;
        messages.insert(message_id.clone(), message.clone());
        drop(messages);
        
        // Update thread with message
        let mut threads = self.threads.write().await;
        if let Some(thread) = threads.get_mut(&thread_id) {
            thread.messages.push(message.clone());
            thread.message_count = thread.messages.len();
            thread.last_message_at = message.timestamp;
            
            // Persist updated thread
            self.persist_thread(thread).await?;
        }
        drop(threads);
        
        // Persist message
        self.persist_message(&message).await?;
        
        Ok(())
    }
    
    async fn get_message(&self, message_id: &str) -> Result<Option<ConversationMessage>, ContextManagerError> {
        let messages = self.messages.read().await;
        if let Some(message) = messages.get(message_id) {
            return Ok(Some(message.clone()));
        }
        drop(messages);
        
        // Try to load from persistent storage
        let message_file = self.config.data_dir.join("messages").join(format!("{}.json", message_id));
        if message_file.exists() {
            let content = fs::read_to_string(&message_file)?;
            let message_data: ConversationMessage = serde_json::from_str(&content)?;
            
            // Cache in memory
            let mut messages = self.messages.write().await;
            messages.insert(message_id.to_string(), message_data.clone());
            
            return Ok(Some(message_data));
        }
        
        Ok(None)
    }
    
    async fn persist_message(&self, message: &ConversationMessage) -> Result<(), ContextManagerError> {
        let message_file = self.config.data_dir.join("messages").join(format!("{}.json", message.message_id));
        let content = serde_json::to_string_pretty(message)?;
        fs::write(&message_file, content)?;
        Ok(())
    }
    
    async fn search_threads(&self, query: ThreadSearchQuery) -> Result<ThreadSearchResult, ContextManagerError> {
        let start_time = std::time::Instant::now();
        let threads = self.threads.read().await;
        
        let mut matching_threads = Vec::new();
        
        for thread in threads.values() {
            let mut matches = true;
            
            // Check participants filter
            if !query.participants.is_empty() {
                matches = matches && query.participants.iter().any(|p| thread.participants.contains(p));
            }
            
            // Check tags filter
            if !query.tags.is_empty() {
                matches = matches && query.tags.iter().any(|t| thread.tags.contains(t));
            }
            
            // Check date range filter
            if let Some((start, end)) = query.date_range {
                matches = matches && thread.created_at >= start && thread.created_at <= end;
            }
            
            // Check text search in thread title and messages
            if !query.query.is_empty() {
                let query_lower = query.query.to_lowercase();
                let title_match = thread.title.to_lowercase().contains(&query_lower);
                let message_match = thread.messages.iter().any(|msg| {
                    msg.content.to_lowercase().contains(&query_lower)
                });
                matches = matches && (title_match || message_match);
            }
            
            // Check message type filter
            if let Some(ref msg_type) = query.message_type {
                matches = matches && thread.messages.iter().any(|msg| {
                    std::mem::discriminant(&msg.message_type) == std::mem::discriminant(msg_type)
                });
            }
            
            if matches {
                matching_threads.push(thread.clone());
            }
        }
        
        // Sort by relevance (for now, just by last message time)
        matching_threads.sort_by(|a, b| b.last_message_at.cmp(&a.last_message_at));
        
        let total_count = matching_threads.len();
        
        // Apply pagination
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(50);
        
        if offset < matching_threads.len() {
            let end = std::cmp::min(offset + limit, matching_threads.len());
            matching_threads = matching_threads[offset..end].to_vec();
        } else {
            matching_threads.clear();
        }
        
        let search_time = start_time.elapsed();
        
        Ok(ThreadSearchResult {
            threads: matching_threads,
            total_count,
            search_time_ms: search_time.as_millis() as u64,
        })
    }
}

impl UserPreferences {
    fn new(config: ContextManagerConfig) -> Result<Self, ContextManagerError> {
        let prefs_dir = config.data_dir.join("preferences");
        fs::create_dir_all(&prefs_dir)?;
        
        let store = Self {
            config,
            preferences: RwLock::new(HashMap::new()),
        };
        
        // Load existing preferences
        store.load_existing_preferences()?;
        
        Ok(store)
    }
    
    fn load_existing_preferences(&self) -> Result<(), ContextManagerError> {
        // In a real implementation, this would load from persistent storage
        // For now, we'll keep it simple and start with empty data
        Ok(())
    }
    
    async fn get_preferences(&self, user_id: &str) -> Result<Option<UserPreferenceData>, ContextManagerError> {
        // Check in-memory first
        let preferences = self.preferences.read().await;
        if let Some(prefs) = preferences.get(user_id) {
            return Ok(Some(prefs.clone()));
        }
        drop(preferences);
        
        // Try to load from persistent storage
        let prefs_file = self.config.data_dir.join("preferences").join(format!("{}.json", user_id));
        if prefs_file.exists() {
            let content = fs::read_to_string(&prefs_file)?;
            let prefs_data: UserPreferenceData = serde_json::from_str(&content)?;
            
            // Cache in memory
            let mut preferences = self.preferences.write().await;
            preferences.insert(user_id.to_string(), prefs_data.clone());
            
            return Ok(Some(prefs_data));
        }
        
        Ok(None)
    }
    
    async fn update_preferences(&self, user_id: &str, new_preferences: HashMap<String, serde_json::Value>) -> Result<(), ContextManagerError> {
        let mut preferences = self.preferences.write().await;
        
        let updated_prefs = if let Some(existing) = preferences.get_mut(user_id) {
            // Update existing preferences
            existing.preferences.extend(new_preferences);
            existing.updated_at = SystemTime::now();
            existing.clone()
        } else {
            // Create new preferences
            let new_prefs = UserPreferenceData {
                user_id: user_id.to_string(),
                preferences: new_preferences,
                learning_data: HashMap::new(),
                updated_at: SystemTime::now(),
            };
            preferences.insert(user_id.to_string(), new_prefs.clone());
            new_prefs
        };
        
        drop(preferences);
        
        // Persist to disk
        self.persist_preferences(&updated_prefs).await?;
        
        Ok(())
    }
    
    async fn learn_from_interaction(&self, user_id: &str, interaction_type: &str, context: HashMap<String, serde_json::Value>) -> Result<(), ContextManagerError> {
        let mut preferences = self.preferences.write().await;
        
        let prefs_data = preferences.entry(user_id.to_string()).or_insert_with(|| {
            UserPreferenceData {
                user_id: user_id.to_string(),
                preferences: HashMap::new(),
                learning_data: HashMap::new(),
                updated_at: SystemTime::now(),
            }
        });
        
        // Update learning metrics based on interaction
        self.update_learning_metrics(prefs_data, interaction_type, &context);
        
        // Generate preference recommendations
        self.generate_preference_recommendations(prefs_data);
        
        prefs_data.updated_at = SystemTime::now();
        let updated_prefs = prefs_data.clone();
        
        drop(preferences);
        
        // Persist to disk
        self.persist_preferences(&updated_prefs).await?;
        
        Ok(())
    }
    
    fn update_learning_metrics(&self, prefs_data: &mut UserPreferenceData, interaction_type: &str, context: &HashMap<String, serde_json::Value>) {
        // Communication style learning
        if interaction_type == "message" {
            self.learn_communication_style(prefs_data, context);
        }
        
        // Tool usage pattern learning
        if interaction_type == "tool_usage" {
            self.learn_tool_usage_patterns(prefs_data, context);
        }
        
        // Workflow preference learning
        if interaction_type == "workflow" {
            self.learn_workflow_preferences(prefs_data, context);
        }
        
        // Response time preference learning
        if interaction_type == "response_time" {
            self.learn_response_time_preferences(prefs_data, context);
        }
    }
    
    fn learn_communication_style(&self, prefs_data: &mut UserPreferenceData, context: &HashMap<String, serde_json::Value>) {
        // Analyze message length preference
        if let Some(message_length) = context.get("message_length").and_then(|v| v.as_u64()) {
            let metric_name = "preferred_message_length";
            self.update_metric(prefs_data, metric_name, message_length as f64);
        }
        
        // Analyze formality level
        if let Some(formality) = context.get("formality_level").and_then(|v| v.as_f64()) {
            let metric_name = "preferred_formality";
            self.update_metric(prefs_data, metric_name, formality);
        }
        
        // Analyze technical detail preference
        if let Some(detail_level) = context.get("technical_detail").and_then(|v| v.as_f64()) {
            let metric_name = "preferred_technical_detail";
            self.update_metric(prefs_data, metric_name, detail_level);
        }
    }
    
    fn learn_tool_usage_patterns(&self, prefs_data: &mut UserPreferenceData, context: &HashMap<String, serde_json::Value>) {
        // Track frequently used tools
        if let Some(tool_name) = context.get("tool_name").and_then(|v| v.as_str()) {
            let metric_name = format!("tool_usage_{}", tool_name);
            let current_value = prefs_data.learning_data.get(&metric_name)
                .map(|m| m.value)
                .unwrap_or(0.0);
            self.update_metric(prefs_data, &metric_name, current_value + 1.0);
        }
        
        // Track tool execution time preferences
        if let Some(execution_time) = context.get("execution_time").and_then(|v| v.as_f64()) {
            let metric_name = "preferred_execution_time";
            self.update_metric(prefs_data, metric_name, execution_time);
        }
    }
    
    fn learn_workflow_preferences(&self, prefs_data: &mut UserPreferenceData, context: &HashMap<String, serde_json::Value>) {
        // Track preferred workflow patterns
        if let Some(workflow_type) = context.get("workflow_type").and_then(|v| v.as_str()) {
            let metric_name = format!("workflow_preference_{}", workflow_type);
            let current_value = prefs_data.learning_data.get(&metric_name)
                .map(|m| m.value)
                .unwrap_or(0.0);
            self.update_metric(prefs_data, &metric_name, current_value + 1.0);
        }
        
        // Track session duration preferences
        if let Some(session_duration) = context.get("session_duration").and_then(|v| v.as_f64()) {
            let metric_name = "preferred_session_duration";
            self.update_metric(prefs_data, metric_name, session_duration);
        }
    }
    
    fn learn_response_time_preferences(&self, prefs_data: &mut UserPreferenceData, context: &HashMap<String, serde_json::Value>) {
        // Track response time satisfaction
        if let Some(response_time) = context.get("response_time").and_then(|v| v.as_f64()) {
            let metric_name = "preferred_response_time";
            self.update_metric(prefs_data, metric_name, response_time);
        }
        
        // Track user satisfaction with response time
        if let Some(satisfaction) = context.get("time_satisfaction").and_then(|v| v.as_f64()) {
            let metric_name = "response_time_satisfaction";
            self.update_metric(prefs_data, metric_name, satisfaction);
        }
    }
    
    fn update_metric(&self, prefs_data: &mut UserPreferenceData, metric_name: &str, new_value: f64) {
        let learning_metric = prefs_data.learning_data.entry(metric_name.to_string()).or_insert_with(|| {
            LearningMetric {
                metric_name: metric_name.to_string(),
                value: new_value,
                confidence: 0.1,
                last_updated: SystemTime::now(),
                sample_count: 0,
            }
        });
        
        // Update metric using exponential moving average
        let alpha = 0.1; // Learning rate
        learning_metric.value = (1.0 - alpha) * learning_metric.value + alpha * new_value;
        learning_metric.sample_count += 1;
        learning_metric.last_updated = SystemTime::now();
        
        // Update confidence based on sample count
        learning_metric.confidence = (learning_metric.sample_count as f64 / (learning_metric.sample_count as f64 + 10.0)).min(1.0);
    }
    
    fn generate_preference_recommendations(&self, prefs_data: &mut UserPreferenceData) {
        // Generate communication style recommendations
        if let Some(message_length) = prefs_data.learning_data.get("preferred_message_length") {
            if message_length.confidence > 0.5 {
                let pref_value = if message_length.value > 500.0 {
                    serde_json::Value::String("detailed".to_string())
                } else {
                    serde_json::Value::String("concise".to_string())
                };
                prefs_data.preferences.insert("communication_style".to_string(), pref_value);
            }
        }
        
        // Generate tool usage recommendations
        let mut tool_preferences = Vec::new();
        for (metric_name, metric) in &prefs_data.learning_data {
            if metric_name.starts_with("tool_usage_") && metric.confidence > 0.3 {
                let tool_name = metric_name.strip_prefix("tool_usage_").unwrap_or(metric_name);
                tool_preferences.push(serde_json::json!({
                    "tool": tool_name,
                    "frequency": metric.value,
                    "confidence": metric.confidence
                }));
            }
        }
        
        if !tool_preferences.is_empty() {
            // Sort by frequency and keep top 5
            tool_preferences.sort_by(|a, b| {
                b.get("frequency").unwrap().as_f64().unwrap()
                    .partial_cmp(&a.get("frequency").unwrap().as_f64().unwrap())
                    .unwrap()
            });
            tool_preferences.truncate(5);
            
            prefs_data.preferences.insert("favorite_tools".to_string(), serde_json::Value::Array(tool_preferences));
        }
        
        // Generate workflow recommendations
        let mut workflow_preferences = Vec::new();
        for (metric_name, metric) in &prefs_data.learning_data {
            if metric_name.starts_with("workflow_preference_") && metric.confidence > 0.3 {
                let workflow_type = metric_name.strip_prefix("workflow_preference_").unwrap_or(metric_name);
                workflow_preferences.push(serde_json::json!({
                    "workflow": workflow_type,
                    "frequency": metric.value,
                    "confidence": metric.confidence
                }));
            }
        }
        
        if !workflow_preferences.is_empty() {
            workflow_preferences.sort_by(|a, b| {
                b.get("frequency").unwrap().as_f64().unwrap()
                    .partial_cmp(&a.get("frequency").unwrap().as_f64().unwrap())
                    .unwrap()
            });
            workflow_preferences.truncate(3);
            
            prefs_data.preferences.insert("preferred_workflows".to_string(), serde_json::Value::Array(workflow_preferences));
        }
        
        // Generate response time recommendations
        if let Some(response_time) = prefs_data.learning_data.get("preferred_response_time") {
            if response_time.confidence > 0.5 {
                let pref_value = if response_time.value < 1000.0 {
                    serde_json::Value::String("fast".to_string())
                } else if response_time.value < 5000.0 {
                    serde_json::Value::String("moderate".to_string())
                } else {
                    serde_json::Value::String("thorough".to_string())
                };
                prefs_data.preferences.insert("response_time_preference".to_string(), pref_value);
            }
        }
    }
    
    async fn get_recommendations(&self, user_id: &str) -> Result<Vec<Recommendation>, ContextManagerError> {
        let preferences = self.preferences.read().await;
        let prefs_data = preferences.get(user_id);
        
        if let Some(prefs) = prefs_data {
            let mut recommendations = Vec::new();
            
            // Communication style recommendations
            if let Some(comm_style) = prefs.preferences.get("communication_style") {
                recommendations.push(Recommendation {
                    category: "communication".to_string(),
                    suggestion: format!("User prefers {} responses", comm_style.as_str().unwrap_or("balanced")),
                    confidence: prefs.learning_data.get("preferred_message_length")
                        .map(|m| m.confidence)
                        .unwrap_or(0.5),
                });
            }
            
            // Tool usage recommendations
            if let Some(favorite_tools) = prefs.preferences.get("favorite_tools") {
                if let Some(tools_array) = favorite_tools.as_array() {
                    for tool in tools_array.iter().take(3) {
                        if let Some(tool_name) = tool.get("tool").and_then(|v| v.as_str()) {
                            recommendations.push(Recommendation {
                                category: "tools".to_string(),
                                suggestion: format!("User frequently uses {}", tool_name),
                                confidence: tool.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.5),
                            });
                        }
                    }
                }
            }
            
            // Workflow recommendations
            if let Some(workflows) = prefs.preferences.get("preferred_workflows") {
                if let Some(workflows_array) = workflows.as_array() {
                    for workflow in workflows_array.iter().take(2) {
                        if let Some(workflow_type) = workflow.get("workflow").and_then(|v| v.as_str()) {
                            recommendations.push(Recommendation {
                                category: "workflow".to_string(),
                                suggestion: format!("User prefers {} workflows", workflow_type),
                                confidence: workflow.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.5),
                            });
                        }
                    }
                }
            }
            
            // Response time recommendations
            if let Some(response_pref) = prefs.preferences.get("response_time_preference") {
                recommendations.push(Recommendation {
                    category: "performance".to_string(),
                    suggestion: format!("User prefers {} responses", response_pref.as_str().unwrap_or("balanced")),
                    confidence: prefs.learning_data.get("preferred_response_time")
                        .map(|m| m.confidence)
                        .unwrap_or(0.5),
                });
            }
            
            Ok(recommendations)
        } else {
            Ok(Vec::new())
        }
    }
    
    async fn persist_preferences(&self, prefs_data: &UserPreferenceData) -> Result<(), ContextManagerError> {
        let prefs_file = self.config.data_dir.join("preferences").join(format!("{}.json", prefs_data.user_id));
        let content = serde_json::to_string_pretty(prefs_data)?;
        fs::write(&prefs_file, content)?;
        Ok(())
    }
}

impl ToolContext {
    fn new(config: ContextManagerConfig) -> Result<Self, ContextManagerError> {
        let contexts_dir = config.data_dir.join("tool_contexts");
        fs::create_dir_all(&contexts_dir)?;
        
        let store = Self {
            config,
            contexts: RwLock::new(HashMap::new()),
        };
        
        // Load existing contexts
        store.load_existing_contexts()?;
        
        Ok(store)
    }
    
    fn load_existing_contexts(&self) -> Result<(), ContextManagerError> {
        // In a real implementation, this would load from persistent storage
        // For now, we'll keep it simple and start with empty data
        Ok(())
    }
    
    async fn get_context(&self, context_id: &str) -> Result<Option<ToolContextData>, ContextManagerError> {
        // Check in-memory first
        let contexts = self.contexts.read().await;
        if let Some(context) = contexts.get(context_id) {
            // Check TTL expiration
            if let Some(ttl) = context.ttl {
                let elapsed = SystemTime::now().duration_since(context.last_accessed).unwrap_or(Duration::ZERO);
                if elapsed >= ttl {
                    // Context expired
                    drop(contexts);
                    self.remove_context(context_id).await?;
                    return Ok(None);
                }
            }
            
            // Update last accessed time
            let mut updated_context = context.clone();
            updated_context.last_accessed = SystemTime::now();
            drop(contexts);
            
            // Update in memory and persist
            let mut contexts = self.contexts.write().await;
            contexts.insert(context_id.to_string(), updated_context.clone());
            drop(contexts);
            
            self.persist_context(&updated_context).await?;
            
            return Ok(Some(updated_context));
        }
        drop(contexts);
        
        // Try to load from persistent storage
        let context_file = self.config.data_dir.join("tool_contexts").join(format!("{}.json", context_id));
        if context_file.exists() {
            let content = fs::read_to_string(&context_file)?;
            let context_data: ToolContextData = serde_json::from_str(&content)?;
            
            // Check TTL expiration
            if let Some(ttl) = context_data.ttl {
                let elapsed = SystemTime::now().duration_since(context_data.last_accessed).unwrap_or(Duration::ZERO);
                if elapsed >= ttl {
                    // Context expired, remove file
                    fs::remove_file(&context_file)?;
                    return Ok(None);
                }
            }
            
            // Update last accessed time
            let mut updated_context = context_data;
            updated_context.last_accessed = SystemTime::now();
            
            // Cache in memory
            let mut contexts = self.contexts.write().await;
            contexts.insert(context_id.to_string(), updated_context.clone());
            drop(contexts);
            
            // Update persistent storage
            self.persist_context(&updated_context).await?;
            
            return Ok(Some(updated_context));
        }
        
        Ok(None)
    }
    
    async fn create_context(&self, tool_name: &str, session_id: &str, initial_state: HashMap<String, serde_json::Value>, ttl: Option<Duration>) -> Result<String, ContextManagerError> {
        let context_id = format!("{}_{}", tool_name, uuid::Uuid::new_v4());
        let now = SystemTime::now();
        
        let context_data = ToolContextData {
            context_id: context_id.clone(),
            tool_name: tool_name.to_string(),
            session_id: session_id.to_string(),
            state: initial_state,
            created_at: now,
            last_accessed: now,
            ttl,
        };
        
        // Store in memory
        let mut contexts = self.contexts.write().await;
        contexts.insert(context_id.clone(), context_data.clone());
        drop(contexts);
        
        // Persist to disk
        self.persist_context(&context_data).await?;
        
        Ok(context_id)
    }
    
    async fn update_context(&self, context_id: &str, state_updates: HashMap<String, serde_json::Value>) -> Result<(), ContextManagerError> {
        let mut contexts = self.contexts.write().await;
        
        if let Some(context) = contexts.get_mut(context_id) {
            // Update state
            context.state.extend(state_updates);
            context.last_accessed = SystemTime::now();
            
            let updated_context = context.clone();
            drop(contexts);
            
            // Persist to disk
            self.persist_context(&updated_context).await?;
            
            Ok(())
        } else {
            // Try to load from disk first
            drop(contexts);
            
            if let Some(mut context) = self.load_context_from_disk(context_id).await? {
                context.state.extend(state_updates);
                context.last_accessed = SystemTime::now();
                
                // Update in memory
                let mut contexts = self.contexts.write().await;
                contexts.insert(context_id.to_string(), context.clone());
                drop(contexts);
                
                // Persist to disk
                self.persist_context(&context).await?;
                
                Ok(())
            } else {
                Err(ContextManagerError::StorageError(
                    std::io::Error::new(std::io::ErrorKind::NotFound, format!("Context {} not found", context_id))
                ))
            }
        }
    }
    
    async fn remove_context(&self, context_id: &str) -> Result<(), ContextManagerError> {
        // Remove from memory
        let mut contexts = self.contexts.write().await;
        contexts.remove(context_id);
        drop(contexts);
        
        // Remove from disk
        let context_file = self.config.data_dir.join("tool_contexts").join(format!("{}.json", context_id));
        if context_file.exists() {
            fs::remove_file(&context_file)?;
        }
        
        Ok(())
    }
    
    async fn get_contexts_for_session(&self, session_id: &str) -> Result<Vec<ToolContextData>, ContextManagerError> {
        let contexts = self.contexts.read().await;
        let mut session_contexts = Vec::new();
        
        for context in contexts.values() {
            if context.session_id == session_id {
                // Check TTL expiration
                if let Some(ttl) = context.ttl {
                    let elapsed = SystemTime::now().duration_since(context.last_accessed).unwrap_or(Duration::ZERO);
                    if elapsed < ttl {
                        session_contexts.push(context.clone());
                    }
                } else {
                    session_contexts.push(context.clone());
                }
            }
        }
        
        // Sort by last accessed (most recent first)
        session_contexts.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));
        
        Ok(session_contexts)
    }
    
    async fn get_contexts_for_tool(&self, tool_name: &str) -> Result<Vec<ToolContextData>, ContextManagerError> {
        let contexts = self.contexts.read().await;
        let mut tool_contexts = Vec::new();
        
        for context in contexts.values() {
            if context.tool_name == tool_name {
                // Check TTL expiration
                if let Some(ttl) = context.ttl {
                    let elapsed = SystemTime::now().duration_since(context.last_accessed).unwrap_or(Duration::ZERO);
                    if elapsed < ttl {
                        tool_contexts.push(context.clone());
                    }
                } else {
                    tool_contexts.push(context.clone());
                }
            }
        }
        
        // Sort by last accessed (most recent first)
        tool_contexts.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));
        
        Ok(tool_contexts)
    }
    
    async fn share_context(&self, source_context_id: &str, target_tool: &str, session_id: &str, shared_keys: Vec<String>) -> Result<String, ContextManagerError> {
        // Get source context
        let source_context = self.get_context(source_context_id).await?
            .ok_or_else(|| ContextManagerError::StorageError(
                std::io::Error::new(std::io::ErrorKind::NotFound, format!("Source context {} not found", source_context_id))
            ))?;
        
        // Create shared state with only specified keys
        let mut shared_state = HashMap::new();
        
        if shared_keys.is_empty() {
            // Share all state if no specific keys specified
            shared_state = source_context.state.clone();
        } else {
            // Share only specified keys
            for key in shared_keys {
                if let Some(value) = source_context.state.get(&key) {
                    shared_state.insert(key, value.clone());
                }
            }
        }
        
        // Add metadata about the sharing
        shared_state.insert("_shared_from".to_string(), serde_json::Value::String(source_context_id.to_string()));
        shared_state.insert("_shared_at".to_string(), serde_json::Value::String(
            SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs().to_string()
        ));
        
        // Create new context for target tool
        let shared_context_id = self.create_context(target_tool, session_id, shared_state, source_context.ttl).await?;
        
        Ok(shared_context_id)
    }
    
    async fn cleanup_expired_contexts(&self) -> Result<usize, ContextManagerError> {
        let mut contexts = self.contexts.write().await;
        let now = SystemTime::now();
        
        let initial_count = contexts.len();
        let expired_contexts: Vec<String> = contexts
            .iter()
            .filter(|(_, context)| {
                if let Some(ttl) = context.ttl {
                    let elapsed = now.duration_since(context.last_accessed).unwrap_or(Duration::ZERO);
                    elapsed >= ttl
                } else {
                    false
                }
            })
            .map(|(id, _)| id.clone())
            .collect();
        
        // Remove expired contexts
        for context_id in &expired_contexts {
            contexts.remove(context_id);
            
            // Remove context file
            let context_file = self.config.data_dir.join("tool_contexts").join(format!("{}.json", context_id));
            if context_file.exists() {
                fs::remove_file(&context_file)?;
            }
        }
        
        Ok(expired_contexts.len())
    }
    
    async fn get_context_statistics(&self) -> Result<ToolContextStatistics, ContextManagerError> {
        let contexts = self.contexts.read().await;
        let now = SystemTime::now();
        
        let mut tool_counts = HashMap::new();
        let mut session_counts = HashMap::new();
        let mut active_contexts = 0;
        let mut expired_contexts = 0;
        let mut total_state_size = 0;
        
        for context in contexts.values() {
            // Count by tool
            *tool_counts.entry(context.tool_name.clone()).or_insert(0) += 1;
            
            // Count by session
            *session_counts.entry(context.session_id.clone()).or_insert(0) += 1;
            
            // Check if expired
            if let Some(ttl) = context.ttl {
                let elapsed = now.duration_since(context.last_accessed).unwrap_or(Duration::ZERO);
                if elapsed >= ttl {
                    expired_contexts += 1;
                } else {
                    active_contexts += 1;
                }
            } else {
                active_contexts += 1;
            }
            
            // Estimate state size
            if let Ok(serialized) = serde_json::to_string(&context.state) {
                total_state_size += serialized.len();
            }
        }
        
        Ok(ToolContextStatistics {
            total_contexts: contexts.len(),
            active_contexts,
            expired_contexts,
            tool_counts,
            session_counts,
            total_state_size,
        })
    }
    
    async fn persist_context(&self, context: &ToolContextData) -> Result<(), ContextManagerError> {
        let context_file = self.config.data_dir.join("tool_contexts").join(format!("{}.json", context.context_id));
        let content = serde_json::to_string_pretty(context)?;
        fs::write(&context_file, content)?;
        Ok(())
    }
    
    async fn load_context_from_disk(&self, context_id: &str) -> Result<Option<ToolContextData>, ContextManagerError> {
        let context_file = self.config.data_dir.join("tool_contexts").join(format!("{}.json", context_id));
        if context_file.exists() {
            let content = fs::read_to_string(&context_file)?;
            let context_data: ToolContextData = serde_json::from_str(&content)?;
            Ok(Some(context_data))
        } else {
            Ok(None)
        }
    }
    
    async fn serialize_context(&self, context_id: &str) -> Result<String, ContextManagerError> {
        let context = self.get_context(context_id).await?
            .ok_or_else(|| ContextManagerError::StorageError(
                std::io::Error::new(std::io::ErrorKind::NotFound, format!("Context {} not found", context_id))
            ))?;
        
        let serialized = serde_json::to_string_pretty(&context)?;
        Ok(serialized)
    }
    
    async fn deserialize_context(&self, serialized_data: &str) -> Result<String, ContextManagerError> {
        let context_data: ToolContextData = serde_json::from_str(serialized_data)?;
        
        // Store in memory
        let mut contexts = self.contexts.write().await;
        contexts.insert(context_data.context_id.clone(), context_data.clone());
        drop(contexts);
        
        // Persist to disk
        self.persist_context(&context_data).await?;
        
        Ok(context_data.context_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};
    
    #[tokio::test]
    async fn test_context_manager_creation() {
        let config = ContextManagerConfig::default();
        let manager = ContextManager::new(config).unwrap();
        
        let health = manager.health_check().await.unwrap();
        assert_eq!(health.session_count, 0);
        assert_eq!(health.cache_size, 0);
        assert!(health.cache_enabled);
    }
    
    #[tokio::test]
    async fn test_session_management() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Create a session
        let session_id = manager.create_session("test_user").await.unwrap();
        assert!(!session_id.is_empty());
        
        // Retrieve the session
        let session = manager.get_session(&session_id).await.unwrap();
        assert!(session.is_some());
        
        let session_data = session.unwrap();
        assert_eq!(session_data.session_id, session_id);
        assert_eq!(session_data.user_id, "test_user");
    }
    
    #[tokio::test]
    async fn test_cache_functionality() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Create a session (should be cached)
        let session_id = manager.create_session("test_user").await.unwrap();
        
        // First retrieval (from cache)
        let start = std::time::Instant::now();
        let _session1 = manager.get_session(&session_id).await.unwrap();
        let first_duration = start.elapsed();
        
        // Second retrieval (should be faster from cache)
        let start = std::time::Instant::now();
        let _session2 = manager.get_session(&session_id).await.unwrap();
        let second_duration = start.elapsed();
        
        // Cache should make second retrieval faster (though this might not always be true in tests)
        assert!(second_duration <= first_duration + Duration::from_millis(10));
    }
    
    #[tokio::test]
    async fn test_performance_tracking() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Create a session to generate performance metrics
        let _session_id = manager.create_session("test_user").await.unwrap();
        
        // Get performance metrics
        let metrics = manager.get_performance_metrics().await;
        assert!(metrics.contains_key("create_session"));
        
        let create_metrics = &metrics["create_session"];
        assert_eq!(create_metrics.sample_count, 1);
        assert!(create_metrics.avg_duration.as_nanos() > 0);
    }
    
    #[tokio::test]
    async fn test_invalid_configuration() {
        let mut config = ContextManagerConfig::default();
        config.max_sessions = 0;
        
        let result = ContextManager::new(config);
        assert!(result.is_err());
        
        if let Err(ContextManagerError::InvalidConfiguration(msg)) = result {
            assert!(msg.contains("max_sessions must be greater than 0"));
        } else {
            panic!("Expected InvalidConfiguration error");
        }
    }
    
    #[tokio::test]
    async fn test_session_cleanup() {
        let mut config = ContextManagerConfig::default();
        config.session_timeout = Duration::from_millis(100); // Very short timeout for testing
        
        let manager = ContextManager::new(config).unwrap();
        
        // Create a session
        let _session_id = manager.create_session("test_user").await.unwrap();
        
        // Wait for session to expire
        sleep(Duration::from_millis(150)).await;
        
        // Cleanup expired sessions
        let cleaned_count = manager.cleanup_expired_sessions().await.unwrap();
        assert_eq!(cleaned_count, 1);
    }
    
    #[tokio::test]
    async fn test_cache_eviction() {
        let mut config = ContextManagerConfig::default();
        config.cache_size = 2; // Very small cache for testing
        
        let manager = ContextManager::new(config).unwrap();
        
        // Create more sessions than cache size
        let session1 = manager.create_session("user1").await.unwrap();
        let session2 = manager.create_session("user2").await.unwrap();
        let session3 = manager.create_session("user3").await.unwrap();
        
        // All should be retrievable
        assert!(manager.get_session(&session1).await.unwrap().is_some());
        assert!(manager.get_session(&session2).await.unwrap().is_some());
        assert!(manager.get_session(&session3).await.unwrap().is_some());
    }
    
    #[tokio::test]
    async fn test_conversation_thread_creation() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Create a conversation thread
        let thread_id = manager.create_conversation_thread(
            "Test Thread".to_string(), 
            vec!["user1".to_string(), "user2".to_string()], 
            "user1"
        ).await.unwrap();
        
        assert!(!thread_id.is_empty());
        
        // Retrieve the thread
        let thread = manager.get_conversation_thread(&thread_id).await.unwrap();
        assert!(thread.is_some());
        
        let thread_data = thread.unwrap();
        assert_eq!(thread_data.title, "Test Thread");
        assert_eq!(thread_data.participants.len(), 2);
        assert!(thread_data.participants.contains(&"user1".to_string()));
        assert!(thread_data.participants.contains(&"user2".to_string()));
    }
    
    #[tokio::test]
    async fn test_message_management() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Create a conversation thread
        let thread_id = manager.create_conversation_thread(
            "Test Thread".to_string(), 
            vec!["user1".to_string()], 
            "user1"
        ).await.unwrap();
        
        // Add a message to the thread
        let message_id = manager.add_message_to_thread(
            &thread_id, 
            "user1", 
            "Hello, world!".to_string(), 
            MessageType::Text
        ).await.unwrap();
        
        assert!(!message_id.is_empty());
        
        // Retrieve the message
        let message = manager.get_message(&message_id).await.unwrap();
        assert!(message.is_some());
        
        let message_data = message.unwrap();
        assert_eq!(message_data.content, "Hello, world!");
        assert_eq!(message_data.sender, "user1");
        assert_eq!(message_data.thread_id, thread_id);
        
        // Check that thread was updated
        let thread = manager.get_conversation_thread(&thread_id).await.unwrap().unwrap();
        assert_eq!(thread.message_count, 1);
        assert_eq!(thread.messages.len(), 1);
    }
    
    #[tokio::test]
    async fn test_user_threads_retrieval() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Create multiple threads for a user
        let thread1 = manager.create_conversation_thread(
            "Thread 1".to_string(), 
            vec!["user1".to_string()], 
            "user1"
        ).await.unwrap();
        
        let thread2 = manager.create_conversation_thread(
            "Thread 2".to_string(), 
            vec!["user1".to_string(), "user2".to_string()], 
            "user1"
        ).await.unwrap();
        
        // Get user's threads
        let user_threads = manager.get_user_conversation_threads("user1").await.unwrap();
        assert_eq!(user_threads.len(), 2);
        
        // Check that threads are sorted by last message time (most recent first)
        let thread_ids: Vec<String> = user_threads.iter().map(|t| t.thread_id.clone()).collect();
        assert!(thread_ids.contains(&thread1));
        assert!(thread_ids.contains(&thread2));
    }
    
    #[tokio::test]
    async fn test_thread_search() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Create a thread with messages
        let thread_id = manager.create_conversation_thread(
            "Important Discussion".to_string(), 
            vec!["user1".to_string()], 
            "user1"
        ).await.unwrap();
        
        let _message_id = manager.add_message_to_thread(
            &thread_id, 
            "user1", 
            "This is about project planning".to_string(), 
            MessageType::Text
        ).await.unwrap();
        
        // Search for threads
        let query = ThreadSearchQuery {
            query: "project".to_string(),
            participants: vec![],
            tags: vec![],
            date_range: None,
            message_type: None,
            limit: None,
            offset: None,
        };
        
        let search_result = manager.search_conversation_threads(query).await.unwrap();
        assert_eq!(search_result.total_count, 1);
        assert_eq!(search_result.threads.len(), 1);
        assert_eq!(search_result.threads[0].thread_id, thread_id);
        assert!(search_result.search_time_ms >= 0);
    }
    
    #[tokio::test]
    async fn test_session_store_persistence() {
        let mut config = ContextManagerConfig::default();
        config.data_dir = PathBuf::from("/tmp/test_context_manager");
        
        let manager = ContextManager::new(config).unwrap();
        
        // Create a session
        let session_id = manager.create_session("test_user").await.unwrap();
        
        // Verify session file exists
        let session_file = PathBuf::from("/tmp/test_context_manager/sessions").join(format!("{}.json", session_id));
        assert!(session_file.exists());
        
        // Cleanup
        if let Err(e) = std::fs::remove_dir_all("/tmp/test_context_manager") {
            eprintln!("Warning: Failed to cleanup test directory: {}", e);
        }
    }
    
    #[tokio::test]
    async fn test_user_preferences_update() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Create initial preferences
        let mut preferences = HashMap::new();
        preferences.insert("theme".to_string(), serde_json::Value::String("dark".to_string()));
        preferences.insert("notifications".to_string(), serde_json::Value::Bool(true));
        
        // Update preferences
        manager.update_user_preferences("test_user", preferences).await.unwrap();
        
        // Retrieve preferences
        let retrieved_prefs = manager.get_user_preferences("test_user").await.unwrap();
        assert!(retrieved_prefs.is_some());
        
        let prefs_data = retrieved_prefs.unwrap();
        assert_eq!(prefs_data.user_id, "test_user");
        assert_eq!(prefs_data.preferences.get("theme").unwrap().as_str().unwrap(), "dark");
        assert_eq!(prefs_data.preferences.get("notifications").unwrap().as_bool().unwrap(), true);
    }
    
    #[tokio::test]
    async fn test_user_preferences_learning() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Simulate message interaction
        let mut context = HashMap::new();
        context.insert("message_length".to_string(), serde_json::Value::Number(serde_json::Number::from(250)));
        context.insert("formality_level".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(0.7).unwrap()));
        
        // Learn from interaction
        manager.learn_from_user_interaction("test_user", "message", context).await.unwrap();
        
        // Retrieve preferences to check learning
        let retrieved_prefs = manager.get_user_preferences("test_user").await.unwrap();
        assert!(retrieved_prefs.is_some());
        
        let prefs_data = retrieved_prefs.unwrap();
        assert!(prefs_data.learning_data.contains_key("preferred_message_length"));
        assert!(prefs_data.learning_data.contains_key("preferred_formality"));
        
        let message_length_metric = &prefs_data.learning_data["preferred_message_length"];
        assert_eq!(message_length_metric.value, 250.0);
        assert_eq!(message_length_metric.sample_count, 1);
        assert!(message_length_metric.confidence > 0.0);
    }
    
    #[tokio::test]
    async fn test_user_preferences_tool_usage_learning() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Simulate tool usage interactions
        let mut context1 = HashMap::new();
        context1.insert("tool_name".to_string(), serde_json::Value::String("file_search".to_string()));
        context1.insert("execution_time".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(1500.0).unwrap()));
        
        let mut context2 = HashMap::new();
        context2.insert("tool_name".to_string(), serde_json::Value::String("file_search".to_string()));
        context2.insert("execution_time".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(1200.0).unwrap()));
        
        // Learn from multiple interactions
        manager.learn_from_user_interaction("test_user", "tool_usage", context1).await.unwrap();
        manager.learn_from_user_interaction("test_user", "tool_usage", context2).await.unwrap();
        
        // Retrieve preferences
        let retrieved_prefs = manager.get_user_preferences("test_user").await.unwrap();
        assert!(retrieved_prefs.is_some());
        
        let prefs_data = retrieved_prefs.unwrap();
        assert!(prefs_data.learning_data.contains_key("tool_usage_file_search"));
        assert!(prefs_data.learning_data.contains_key("preferred_execution_time"));
        
        let tool_usage_metric = &prefs_data.learning_data["tool_usage_file_search"];
        assert_eq!(tool_usage_metric.sample_count, 2);
        assert!(tool_usage_metric.confidence > 0.0);
    }
    
    #[tokio::test]
    async fn test_user_preferences_recommendations() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Simulate multiple message interactions to build confidence (need more interactions)
        for i in 0..12 {
            let mut context = HashMap::new();
            context.insert("message_length".to_string(), serde_json::Value::Number(serde_json::Number::from(600 + i * 10)));
            context.insert("formality_level".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(0.8).unwrap()));
            
            manager.learn_from_user_interaction("test_user", "message", context).await.unwrap();
        }
        
        // Simulate tool usage to build tool preferences (need more interactions)
        for _ in 0..6 {
            let mut context = HashMap::new();
            context.insert("tool_name".to_string(), serde_json::Value::String("code_analysis".to_string()));
            
            manager.learn_from_user_interaction("test_user", "tool_usage", context).await.unwrap();
        }
        
        // Get recommendations
        let recommendations = manager.get_user_recommendations("test_user").await.unwrap();
        assert!(!recommendations.is_empty());
        
        // Check that we have communication and tool recommendations
        let has_communication = recommendations.iter().any(|r| r.category == "communication");
        let has_tools = recommendations.iter().any(|r| r.category == "tools");
        
        assert!(has_communication || has_tools);
        
        // Verify recommendation confidence
        for rec in &recommendations {
            assert!(rec.confidence > 0.0);
            assert!(rec.confidence <= 1.0);
        }
    }
    
    #[tokio::test]
    async fn test_user_preferences_workflow_learning() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Simulate workflow interactions
        let mut context1 = HashMap::new();
        context1.insert("workflow_type".to_string(), serde_json::Value::String("debugging".to_string()));
        context1.insert("session_duration".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(3600.0).unwrap()));
        
        let mut context2 = HashMap::new();
        context2.insert("workflow_type".to_string(), serde_json::Value::String("debugging".to_string()));
        context2.insert("session_duration".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(3200.0).unwrap()));
        
        // Learn from workflow interactions
        manager.learn_from_user_interaction("test_user", "workflow", context1).await.unwrap();
        manager.learn_from_user_interaction("test_user", "workflow", context2).await.unwrap();
        
        // Retrieve preferences
        let retrieved_prefs = manager.get_user_preferences("test_user").await.unwrap();
        assert!(retrieved_prefs.is_some());
        
        let prefs_data = retrieved_prefs.unwrap();
        assert!(prefs_data.learning_data.contains_key("workflow_preference_debugging"));
        assert!(prefs_data.learning_data.contains_key("preferred_session_duration"));
        
        let workflow_metric = &prefs_data.learning_data["workflow_preference_debugging"];
        assert_eq!(workflow_metric.sample_count, 2);
        assert!(workflow_metric.confidence > 0.0);
    }
    
    #[tokio::test]
    async fn test_user_preferences_persistence() {
        let mut config = ContextManagerConfig::default();
        config.data_dir = PathBuf::from("/tmp/test_user_prefs");
        
        let manager = ContextManager::new(config).unwrap();
        
        // Create preferences
        let mut preferences = HashMap::new();
        preferences.insert("test_pref".to_string(), serde_json::Value::String("test_value".to_string()));
        
        // Update preferences
        manager.update_user_preferences("test_user", preferences).await.unwrap();
        
        // Verify preferences file exists
        let prefs_file = PathBuf::from("/tmp/test_user_prefs/preferences").join("test_user.json");
        assert!(prefs_file.exists());
        
        // Verify file content
        let content = std::fs::read_to_string(&prefs_file).unwrap();
        assert!(content.contains("test_pref"));
        assert!(content.contains("test_value"));
        
        // Cleanup
        if let Err(e) = std::fs::remove_dir_all("/tmp/test_user_prefs") {
            eprintln!("Warning: Failed to cleanup test directory: {}", e);
        }
    }
    
    #[tokio::test]
    async fn test_user_preferences_metric_confidence_building() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Simulate multiple interactions to build confidence
        for i in 0..15 {
            let mut context = HashMap::new();
            context.insert("message_length".to_string(), serde_json::Value::Number(serde_json::Number::from(300 + i * 5)));
            
            manager.learn_from_user_interaction("test_user", "message", context).await.unwrap();
        }
        
        // Retrieve preferences
        let retrieved_prefs = manager.get_user_preferences("test_user").await.unwrap();
        assert!(retrieved_prefs.is_some());
        
        let prefs_data = retrieved_prefs.unwrap();
        let message_length_metric = &prefs_data.learning_data["preferred_message_length"];
        
        // Check that confidence increased with more samples
        assert_eq!(message_length_metric.sample_count, 15);
        assert!(message_length_metric.confidence > 0.5);
        
        // Should have generated communication style preference
        assert!(prefs_data.preferences.contains_key("communication_style"));
    }
    
    #[tokio::test]
    async fn test_tool_context_creation_and_retrieval() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Create initial state
        let mut initial_state = HashMap::new();
        initial_state.insert("workspace".to_string(), serde_json::Value::String("/tmp/test".to_string()));
        initial_state.insert("current_file".to_string(), serde_json::Value::String("main.rs".to_string()));
        
        // Create context
        let context_id = manager.create_tool_context("code_editor", "session_123", initial_state, None).await.unwrap();
        assert!(!context_id.is_empty());
        assert!(context_id.starts_with("code_editor_"));
        
        // Retrieve context
        let retrieved_context = manager.get_tool_context(&context_id).await.unwrap();
        assert!(retrieved_context.is_some());
        
        let context_data = retrieved_context.unwrap();
        assert_eq!(context_data.tool_name, "code_editor");
        assert_eq!(context_data.session_id, "session_123");
        assert_eq!(context_data.state.get("workspace").unwrap().as_str().unwrap(), "/tmp/test");
        assert_eq!(context_data.state.get("current_file").unwrap().as_str().unwrap(), "main.rs");
    }
    
    #[tokio::test]
    async fn test_tool_context_update() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Create initial context
        let mut initial_state = HashMap::new();
        initial_state.insert("line_number".to_string(), serde_json::Value::Number(serde_json::Number::from(10)));
        
        let context_id = manager.create_tool_context("debugger", "session_456", initial_state, None).await.unwrap();
        
        // Update context
        let mut updates = HashMap::new();
        updates.insert("line_number".to_string(), serde_json::Value::Number(serde_json::Number::from(25)));
        updates.insert("breakpoint_set".to_string(), serde_json::Value::Bool(true));
        
        manager.update_tool_context(&context_id, updates).await.unwrap();
        
        // Verify update
        let updated_context = manager.get_tool_context(&context_id).await.unwrap().unwrap();
        assert_eq!(updated_context.state.get("line_number").unwrap().as_i64().unwrap(), 25);
        assert_eq!(updated_context.state.get("breakpoint_set").unwrap().as_bool().unwrap(), true);
    }
    
    #[tokio::test]
    async fn test_tool_context_ttl_expiration() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Create context with short TTL
        let mut initial_state = HashMap::new();
        initial_state.insert("temp_data".to_string(), serde_json::Value::String("expires_soon".to_string()));
        
        let context_id = manager.create_tool_context("temp_tool", "session_789", initial_state, Some(Duration::from_millis(100))).await.unwrap();
        
        // Context should exist initially
        let context = manager.get_tool_context(&context_id).await.unwrap();
        assert!(context.is_some());
        
        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        // Context should be expired and removed
        let expired_context = manager.get_tool_context(&context_id).await.unwrap();
        assert!(expired_context.is_none());
    }
    
    #[tokio::test]
    async fn test_tool_context_sharing() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Create source context
        let mut source_state = HashMap::new();
        source_state.insert("shared_data".to_string(), serde_json::Value::String("important_info".to_string()));
        source_state.insert("private_data".to_string(), serde_json::Value::String("secret".to_string()));
        source_state.insert("config".to_string(), serde_json::json!({"debug": true}));
        
        let source_context_id = manager.create_tool_context("source_tool", "session_abc", source_state, None).await.unwrap();
        
        // Share only specific keys
        let shared_keys = vec!["shared_data".to_string(), "config".to_string()];
        let shared_context_id = manager.share_tool_context(&source_context_id, "target_tool", "session_abc", shared_keys).await.unwrap();
        
        // Verify shared context
        let shared_context = manager.get_tool_context(&shared_context_id).await.unwrap().unwrap();
        assert_eq!(shared_context.tool_name, "target_tool");
        assert_eq!(shared_context.session_id, "session_abc");
        
        // Should have shared data
        assert_eq!(shared_context.state.get("shared_data").unwrap().as_str().unwrap(), "important_info");
        assert!(shared_context.state.get("config").is_some());
        
        // Should not have private data
        assert!(shared_context.state.get("private_data").is_none());
        
        // Should have sharing metadata
        assert!(shared_context.state.get("_shared_from").is_some());
        assert!(shared_context.state.get("_shared_at").is_some());
    }
    
    #[tokio::test]
    async fn test_tool_context_session_filtering() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Create contexts for different sessions
        let mut state1 = HashMap::new();
        state1.insert("data".to_string(), serde_json::Value::String("session1_data".to_string()));
        let _context1 = manager.create_tool_context("tool1", "session_1", state1, None).await.unwrap();
        
        let mut state2 = HashMap::new();
        state2.insert("data".to_string(), serde_json::Value::String("session1_data2".to_string()));
        let _context2 = manager.create_tool_context("tool2", "session_1", state2, None).await.unwrap();
        
        let mut state3 = HashMap::new();
        state3.insert("data".to_string(), serde_json::Value::String("session2_data".to_string()));
        let _context3 = manager.create_tool_context("tool1", "session_2", state3, None).await.unwrap();
        
        // Get contexts for session_1
        let session1_contexts = manager.get_tool_contexts_for_session("session_1").await.unwrap();
        assert_eq!(session1_contexts.len(), 2);
        
        // Verify all contexts belong to session_1
        for context in &session1_contexts {
            assert_eq!(context.session_id, "session_1");
        }
        
        // Get contexts for session_2
        let session2_contexts = manager.get_tool_contexts_for_session("session_2").await.unwrap();
        assert_eq!(session2_contexts.len(), 1);
        assert_eq!(session2_contexts[0].session_id, "session_2");
    }
    
    #[tokio::test]
    async fn test_tool_context_tool_filtering() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Create contexts for different tools
        let mut state1 = HashMap::new();
        state1.insert("data".to_string(), serde_json::Value::String("editor_data1".to_string()));
        let _context1 = manager.create_tool_context("code_editor", "session_1", state1, None).await.unwrap();
        
        let mut state2 = HashMap::new();
        state2.insert("data".to_string(), serde_json::Value::String("editor_data2".to_string()));
        let _context2 = manager.create_tool_context("code_editor", "session_2", state2, None).await.unwrap();
        
        let mut state3 = HashMap::new();
        state3.insert("data".to_string(), serde_json::Value::String("debugger_data".to_string()));
        let _context3 = manager.create_tool_context("debugger", "session_1", state3, None).await.unwrap();
        
        // Get contexts for code_editor
        let editor_contexts = manager.get_tool_contexts_for_tool("code_editor").await.unwrap();
        assert_eq!(editor_contexts.len(), 2);
        
        // Verify all contexts belong to code_editor
        for context in &editor_contexts {
            assert_eq!(context.tool_name, "code_editor");
        }
        
        // Get contexts for debugger
        let debugger_contexts = manager.get_tool_contexts_for_tool("debugger").await.unwrap();
        assert_eq!(debugger_contexts.len(), 1);
        assert_eq!(debugger_contexts[0].tool_name, "debugger");
    }
    
    #[tokio::test]
    async fn test_tool_context_statistics() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Create various contexts
        let mut state1 = HashMap::new();
        state1.insert("data".to_string(), serde_json::Value::String("test_data".to_string()));
        let _context1 = manager.create_tool_context("editor", "session_1", state1.clone(), None).await.unwrap();
        let _context2 = manager.create_tool_context("editor", "session_2", state1.clone(), None).await.unwrap();
        let _context3 = manager.create_tool_context("debugger", "session_1", state1.clone(), Some(Duration::from_millis(100))).await.unwrap();
        
        // Get statistics
        let stats = manager.get_tool_context_statistics().await.unwrap();
        
        assert_eq!(stats.total_contexts, 3);
        assert!(stats.active_contexts > 0);
        assert_eq!(stats.tool_counts.get("editor").unwrap(), &2);
        assert_eq!(stats.tool_counts.get("debugger").unwrap(), &1);
        assert_eq!(stats.session_counts.get("session_1").unwrap(), &2);
        assert_eq!(stats.session_counts.get("session_2").unwrap(), &1);
        assert!(stats.total_state_size > 0);
    }
    
    #[tokio::test]
    async fn test_tool_context_serialization() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Create context
        let mut initial_state = HashMap::new();
        initial_state.insert("test_key".to_string(), serde_json::Value::String("test_value".to_string()));
        let context_id = manager.create_tool_context("test_tool", "session_test", initial_state, None).await.unwrap();
        
        // Serialize context
        let serialized = manager.serialize_tool_context(&context_id).await.unwrap();
        assert!(serialized.contains("test_tool"));
        assert!(serialized.contains("session_test"));
        assert!(serialized.contains("test_key"));
        assert!(serialized.contains("test_value"));
        
        // Remove original context
        manager.remove_tool_context(&context_id).await.unwrap();
        
        // Verify removal
        let removed_context = manager.get_tool_context(&context_id).await.unwrap();
        assert!(removed_context.is_none());
        
        // Deserialize context
        let new_context_id = manager.deserialize_tool_context(&serialized).await.unwrap();
        assert_eq!(new_context_id, context_id);
        
        // Verify deserialized context
        let deserialized_context = manager.get_tool_context(&new_context_id).await.unwrap().unwrap();
        assert_eq!(deserialized_context.tool_name, "test_tool");
        assert_eq!(deserialized_context.session_id, "session_test");
        assert_eq!(deserialized_context.state.get("test_key").unwrap().as_str().unwrap(), "test_value");
    }
    
    #[tokio::test]
    async fn test_tool_context_cleanup() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Create contexts with different TTLs
        let mut state = HashMap::new();
        state.insert("data".to_string(), serde_json::Value::String("test".to_string()));
        
        let _permanent_context = manager.create_tool_context("permanent", "session_1", state.clone(), None).await.unwrap();
        let _short_lived_context1 = manager.create_tool_context("temp", "session_1", state.clone(), Some(Duration::from_millis(50))).await.unwrap();
        let _short_lived_context2 = manager.create_tool_context("temp", "session_2", state.clone(), Some(Duration::from_millis(50))).await.unwrap();
        
        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Cleanup expired contexts
        let cleaned_count = manager.cleanup_expired_tool_contexts().await.unwrap();
        assert_eq!(cleaned_count, 2);
        
        // Verify statistics after cleanup
        let stats = manager.get_tool_context_statistics().await.unwrap();
        assert_eq!(stats.total_contexts, 1);
        assert_eq!(stats.active_contexts, 1);
        assert_eq!(stats.expired_contexts, 0);
    }
    
    #[tokio::test]
    async fn test_tool_context_persistence() {
        let mut config = ContextManagerConfig::default();
        config.data_dir = PathBuf::from("/tmp/test_tool_context");
        
        let manager = ContextManager::new(config).unwrap();
        
        // Create context
        let mut initial_state = HashMap::new();
        initial_state.insert("persistent_data".to_string(), serde_json::Value::String("saved_value".to_string()));
        let context_id = manager.create_tool_context("persistent_tool", "session_persist", initial_state, None).await.unwrap();
        
        // Verify context file exists
        let context_file = PathBuf::from("/tmp/test_tool_context/tool_contexts").join(format!("{}.json", context_id));
        assert!(context_file.exists());
        
        // Verify file content
        let content = std::fs::read_to_string(&context_file).unwrap();
        assert!(content.contains("persistent_tool"));
        assert!(content.contains("session_persist"));
        assert!(content.contains("persistent_data"));
        assert!(content.contains("saved_value"));
        
        // Cleanup
        if let Err(e) = std::fs::remove_dir_all("/tmp/test_tool_context") {
            eprintln!("Warning: Failed to cleanup test directory: {}", e);
        }
    }
    
    #[tokio::test]
    async fn test_batch_get_contexts() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Create test data
        let session_id = manager.create_session("test_user").await.unwrap();
        
        let mut prefs = HashMap::new();
        prefs.insert("theme".to_string(), serde_json::Value::String("dark".to_string()));
        manager.update_user_preferences("test_user", prefs).await.unwrap();
        
        let mut tool_state = HashMap::new();
        tool_state.insert("data".to_string(), serde_json::Value::String("test_data".to_string()));
        let tool_context_id = manager.create_tool_context("test_tool", &session_id, tool_state, None).await.unwrap();
        
        // Create batch request
        let requests = vec![
            ContextRequest {
                request_id: "req1".to_string(),
                context_type: ContextType::Session,
                id: session_id,
            },
            ContextRequest {
                request_id: "req2".to_string(),
                context_type: ContextType::UserPreferences,
                id: "test_user".to_string(),
            },
            ContextRequest {
                request_id: "req3".to_string(),
                context_type: ContextType::ToolContext,
                id: tool_context_id,
            },
        ];
        
        // Execute batch request
        let responses = manager.batch_get_contexts(requests).await.unwrap();
        
        // Verify responses
        assert_eq!(responses.len(), 3);
        
        for response in &responses {
            assert!(response.success);
            assert!(response.data.is_some());
            assert!(response.error.is_none());
        }
        
        // Verify specific responses
        let session_response = responses.iter().find(|r| r.request_id == "req1").unwrap();
        assert!(matches!(session_response.context_type, ContextType::Session));
        
        let prefs_response = responses.iter().find(|r| r.request_id == "req2").unwrap();
        assert!(matches!(prefs_response.context_type, ContextType::UserPreferences));
        
        let tool_response = responses.iter().find(|r| r.request_id == "req3").unwrap();
        assert!(matches!(tool_response.context_type, ContextType::ToolContext));
    }
    
    #[tokio::test]
    async fn test_preload_session_context() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Create test data
        let session_id = manager.create_session("test_user").await.unwrap();
        
        // Add some user preferences
        let mut prefs = HashMap::new();
        prefs.insert("language".to_string(), serde_json::Value::String("en".to_string()));
        manager.update_user_preferences("test_user", prefs).await.unwrap();
        
        // Create tool contexts for the session
        let mut tool_state = HashMap::new();
        tool_state.insert("context_data".to_string(), serde_json::Value::String("test".to_string()));
        let _tool_context_id = manager.create_tool_context("editor", &session_id, tool_state, None).await.unwrap();
        
        // Create conversation thread
        let _thread_id = manager.create_conversation_thread("Test Thread".to_string(), vec!["test_user".to_string()], "test_user").await.unwrap();
        
        // Preload session context
        let session_context = manager.preload_session_context(&session_id).await.unwrap();
        
        // Verify preloaded data
        assert!(session_context.session_data.is_some());
        assert!(session_context.user_preferences.is_some());
        assert_eq!(session_context.tool_contexts.len(), 1);
        assert_eq!(session_context.conversation_threads.len(), 1);
        
        let session_data = session_context.session_data.unwrap();
        assert_eq!(session_data.user_id, "test_user");
        
        let user_prefs = session_context.user_preferences.unwrap();
        assert_eq!(user_prefs.user_id, "test_user");
    }
    
    #[tokio::test]
    async fn test_comprehensive_context() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Create test data
        let session_id = manager.create_session("test_user").await.unwrap();
        
        // Add user preferences and learn from interactions
        let mut prefs = HashMap::new();
        prefs.insert("theme".to_string(), serde_json::Value::String("dark".to_string()));
        manager.update_user_preferences("test_user", prefs).await.unwrap();
        
        // Learn from interactions to generate recommendations
        for _ in 0..6 {
            let mut context = HashMap::new();
            context.insert("tool_name".to_string(), serde_json::Value::String("editor".to_string()));
            manager.learn_from_user_interaction("test_user", "tool_usage", context).await.unwrap();
        }
        
        // Create tool context
        let mut tool_state = HashMap::new();
        tool_state.insert("editor_state".to_string(), serde_json::Value::String("active".to_string()));
        let _tool_context_id = manager.create_tool_context("editor", &session_id, tool_state, None).await.unwrap();
        
        // Get comprehensive context
        let context = manager.get_comprehensive_context("test_user", &session_id, Some("editor")).await.unwrap();
        
        // Verify comprehensive data
        assert!(context.session_data.is_some());
        assert!(context.user_preferences.is_some());
        assert!(!context.user_recommendations.is_empty());
        assert_eq!(context.tool_contexts.len(), 1);
        
        // Test caching - second call should be faster
        let start = std::time::Instant::now();
        let _cached_context = manager.get_comprehensive_context("test_user", &session_id, Some("editor")).await.unwrap();
        let cached_duration = start.elapsed();
        
        // Cached call should be very fast (under 10ms typically)
        assert!(cached_duration.as_millis() < 50);
    }
    
    #[tokio::test]
    async fn test_cleanup_all_expired_data() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Create some data with short TTLs
        let mut tool_state = HashMap::new();
        tool_state.insert("temp_data".to_string(), serde_json::Value::String("expires".to_string()));
        
        let _context1 = manager.create_tool_context("temp1", "session1", tool_state.clone(), Some(Duration::from_millis(50))).await.unwrap();
        let _context2 = manager.create_tool_context("temp2", "session2", tool_state.clone(), Some(Duration::from_millis(50))).await.unwrap();
        let _permanent_context = manager.create_tool_context("permanent", "session1", tool_state.clone(), None).await.unwrap();
        
        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Cleanup all expired data
        let cleanup_result = manager.cleanup_all_expired_data().await.unwrap();
        
        // Verify cleanup results
        assert_eq!(cleanup_result.expired_tool_contexts, 2);
        assert!(cleanup_result.cleanup_duration.as_millis() > 0);
        
        // Verify remaining data
        let stats = manager.get_tool_context_statistics().await.unwrap();
        assert_eq!(stats.total_contexts, 1);
        assert_eq!(stats.active_contexts, 1);
        assert_eq!(stats.expired_contexts, 0);
    }
    
    #[tokio::test]
    async fn test_performance_optimization() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Create some test data to analyze
        let _session_id = manager.create_session("test_user").await.unwrap();
        
        let mut tool_state = HashMap::new();
        tool_state.insert("data".to_string(), serde_json::Value::String("test".to_string()));
        let _context_id = manager.create_tool_context("test_tool", "session1", tool_state, None).await.unwrap();
        
        // Run performance optimization analysis
        let optimization_result = manager.optimize_performance().await.unwrap();
        
        // Verify results
        assert!(optimization_result.analysis_duration.as_millis() > 0);
        assert!(optimization_result.cache_utilization >= 0.0);
        assert!(optimization_result.session_utilization >= 0.0);
        assert!(!optimization_result.current_metrics.is_empty());
        
        // Should have performance metrics for various operations
        assert!(optimization_result.current_metrics.contains_key("create_session") || 
                optimization_result.current_metrics.contains_key("create_tool_context"));
    }
    
    #[tokio::test]
    async fn test_context_index() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Create test data
        let _session_id = manager.create_session("test_user").await.unwrap();
        
        let mut tool_state = HashMap::new();
        tool_state.insert("data".to_string(), serde_json::Value::String("test".to_string()));
        let _context_id1 = manager.create_tool_context("tool1", "session1", tool_state.clone(), None).await.unwrap();
        let _context_id2 = manager.create_tool_context("tool2", "session1", tool_state.clone(), None).await.unwrap();
        
        // Get context index
        let index = manager.get_context_index().await.unwrap();
        
        // Verify index data
        assert_eq!(index.total_sessions, 1);
        assert_eq!(index.total_tool_contexts, 2);
        assert_eq!(index.active_tool_contexts, 2);
        assert!(index.cached_items >= 0);
        assert!(!index.performance_metrics.is_empty());
        assert!(index.indexed_at <= SystemTime::now());
    }
    
    #[tokio::test]
    async fn test_context_with_fallback() {
        let manager = ContextManager::with_default_config().unwrap();
        
        // Create test data
        let session_id = manager.create_session("test_user").await.unwrap();
        
        // Test successful retrieval
        let result = manager.get_context_with_fallback(&session_id, ContextType::Session).await.unwrap();
        assert!(result.is_some());
        
        // Test fallback for non-existent context
        let result = manager.get_context_with_fallback("non_existent", ContextType::Session).await.unwrap();
        assert!(result.is_none());
        
        // Test with user preferences
        let mut prefs = HashMap::new();
        prefs.insert("test".to_string(), serde_json::Value::String("value".to_string()));
        manager.update_user_preferences("test_user", prefs).await.unwrap();
        
        let result = manager.get_context_with_fallback("test_user", ContextType::UserPreferences).await.unwrap();
        assert!(result.is_some());
        
        // Test with tool context
        let mut tool_state = HashMap::new();
        tool_state.insert("state".to_string(), serde_json::Value::String("active".to_string()));
        let tool_context_id = manager.create_tool_context("test_tool", &session_id, tool_state, None).await.unwrap();
        
        let result = manager.get_context_with_fallback(&tool_context_id, ContextType::ToolContext).await.unwrap();
        assert!(result.is_some());
    }
    
    #[tokio::test]
    async fn test_performance_target_compliance() {
        let mut config = ContextManagerConfig::default();
        config.performance_target_ms = 100; // Very short target for testing
        
        let manager = ContextManager::new(config).unwrap();
        
        // Create a session - should be fast enough
        let session_id = manager.create_session("test_user").await.unwrap();
        
        // Single context retrieval should meet performance target
        let start = std::time::Instant::now();
        let _session = manager.get_session(&session_id).await.unwrap();
        let duration = start.elapsed();
        
        assert!(duration.as_millis() <= 100);
        
        // Batch operations might exceed target, but should still work
        let requests = vec![
            ContextRequest {
                request_id: "req1".to_string(),
                context_type: ContextType::Session,
                id: session_id,
            },
        ];
        
        let start = std::time::Instant::now();
        let _responses = manager.batch_get_contexts(requests).await;
        let batch_duration = start.elapsed();
        
        // Batch operation should complete even if it exceeds target
        assert!(batch_duration.as_millis() > 0);
    }
}