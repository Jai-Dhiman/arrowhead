use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
// use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use uuid::Uuid;

/// MCP Protocol version
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MCPVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl MCPVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }
    
    pub fn current() -> Self {
        Self::new(2024, 11, 5) // MCP version 2024-11-05
    }
    
    pub fn is_compatible(&self, other: &MCPVersion) -> bool {
        self.major == other.major && self.minor <= other.minor
    }
    
    /// Check if this version is backward compatible with another version
    pub fn is_backward_compatible(&self, other: &MCPVersion) -> bool {
        if self.major != other.major {
            return false;
        }
        
        if self.minor > other.minor {
            return true;
        }
        
        if self.minor == other.minor && self.patch >= other.patch {
            return true;
        }
        
        false
    }
    
    /// Parse version from string (e.g., "2024-11-05")
    pub fn from_string(version_str: &str) -> Result<Self> {
        let parts: Vec<&str> = version_str.split('-').collect();
        if parts.len() != 3 {
            anyhow::bail!("Invalid version format: {}", version_str);
        }
        
        let major = parts[0].parse::<u32>()
            .context("Invalid major version")?;
        let minor = parts[1].parse::<u32>()
            .context("Invalid minor version")?;
        let patch = parts[2].parse::<u32>()
            .context("Invalid patch version")?;
        
        Ok(Self::new(major, minor, patch))
    }
    
    /// Convert to string representation
    pub fn to_string(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.major, self.minor, self.patch)
    }
}

/// JSON-RPC 2.0 message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JSONRPCMessage {
    Request {
        jsonrpc: String,
        id: serde_json::Value,
        method: String,
        params: Option<serde_json::Value>,
    },
    Response {
        jsonrpc: String,
        id: serde_json::Value,
        result: Option<serde_json::Value>,
        error: Option<JSONRPCError>,
    },
    Notification {
        jsonrpc: String,
        method: String,
        params: Option<serde_json::Value>,
    },
}

/// JSON-RPC 2.0 error structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JSONRPCError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// MCP Protocol messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MCPMessage {
    Initialize {
        jsonrpc: String,
        id: serde_json::Value,
        method: String, // "initialize"
        params: InitializeParams,
    },
    Initialized {
        jsonrpc: String,
        method: String, // "initialized"
    },
    Ping {
        jsonrpc: String,
        id: serde_json::Value,
        method: String, // "ping"
    },
    ListTools {
        jsonrpc: String,
        id: serde_json::Value,
        method: String, // "tools/list"
    },
    CallTool {
        jsonrpc: String,
        id: serde_json::Value,
        method: String, // "tools/call"
        params: CallToolParams,
    },
    ListResources {
        jsonrpc: String,
        id: serde_json::Value,
        method: String, // "resources/list"
    },
    ReadResource {
        jsonrpc: String,
        id: serde_json::Value,
        method: String, // "resources/read"
        params: ReadResourceParams,
    },
    SetLoggingLevel {
        jsonrpc: String,
        id: serde_json::Value,
        method: String, // "logging/setLevel"
        params: SetLoggingLevelParams,
    },
    /// Generic message for handling unknown message types
    Unknown {
        jsonrpc: String,
        method: String,
        id: Option<serde_json::Value>,
        params: Option<serde_json::Value>,
    },
}

/// Initialize parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    #[serde(rename = "clientInfo")]
    pub client_info: ClientInfo,
}

/// Client capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCapabilities {
    pub roots: Option<RootsCapability>,
    pub sampling: Option<SamplingCapability>,
}

/// Roots capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootsCapability {
    #[serde(rename = "listChanged")]
    pub list_changed: bool,
}

/// Sampling capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingCapability {}

/// Client information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

/// Tool call parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolParams {
    pub name: String,
    pub arguments: Option<serde_json::Value>,
}

/// Resource read parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResourceParams {
    pub uri: String,
}

/// Set logging level parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetLoggingLevelParams {
    pub level: LoggingLevel,
}

/// Logging levels
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoggingLevel {
    Debug,
    Info,
    Notice,
    Warning,
    Error,
    Critical,
    Alert,
    Emergency,
}

/// Tool metadata structure for dynamic tool management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub schema: Option<serde_json::Value>,
    pub capabilities: Vec<String>,
    pub compatibility_version: MCPVersion,
    pub last_updated: Option<String>,
    pub provider: Option<String>,
}

/// Tool registration information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRegistration {
    pub metadata: ToolMetadata,
    pub is_available: bool,
    pub last_seen: Option<String>,
    pub error_count: u32,
    pub response_time_ms: Option<u64>,
}

/// Tool discovery result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDiscoveryResult {
    pub tools: Vec<ToolMetadata>,
    pub discovery_time_ms: u64,
    pub source: String,
    pub errors: Vec<String>,
}

/// Tool registry for managing available tools
#[derive(Debug)]
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, ToolRegistration>>>,
    discovery_cache: Arc<RwLock<HashMap<String, ToolDiscoveryResult>>>,
    cache_ttl_ms: u64,
}

impl ToolRegistry {
    /// Create a new tool registry
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            discovery_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl_ms: 300000, // 5 minutes default
        }
    }

    /// Register a tool with the registry
    pub async fn register_tool(&self, metadata: ToolMetadata) -> Result<()> {
        let registration = ToolRegistration {
            metadata: metadata.clone(),
            is_available: true,
            last_seen: Some(chrono::Utc::now().to_rfc3339()),
            error_count: 0,
            response_time_ms: None,
        };

        let mut tools = self.tools.write().await;
        tools.insert(metadata.name.clone(), registration);
        Ok(())
    }

    /// Get all registered tools
    pub async fn get_tools(&self) -> Vec<ToolRegistration> {
        let tools = self.tools.read().await;
        tools.values().cloned().collect()
    }

    /// Get a specific tool by name
    pub async fn get_tool(&self, name: &str) -> Option<ToolRegistration> {
        let tools = self.tools.read().await;
        tools.get(name).cloned()
    }

    /// Update tool availability status
    pub async fn update_tool_status(&self, name: &str, is_available: bool, response_time_ms: Option<u64>) -> Result<()> {
        let mut tools = self.tools.write().await;
        if let Some(registration) = tools.get_mut(name) {
            registration.is_available = is_available;
            registration.last_seen = Some(chrono::Utc::now().to_rfc3339());
            registration.response_time_ms = response_time_ms;
            
            if !is_available {
                registration.error_count += 1;
            }
        }
        Ok(())
    }

    /// Remove a tool from the registry
    pub async fn unregister_tool(&self, name: &str) -> Result<()> {
        let mut tools = self.tools.write().await;
        tools.remove(name);
        Ok(())
    }

    /// Get tools by capability
    pub async fn get_tools_by_capability(&self, capability: &str) -> Vec<ToolRegistration> {
        let tools = self.tools.read().await;
        tools.values()
            .filter(|reg| reg.metadata.capabilities.contains(&capability.to_string()))
            .cloned()
            .collect()
    }

    /// Check tool compatibility with a specific version
    pub async fn get_compatible_tools(&self, version: &MCPVersion) -> Vec<ToolRegistration> {
        let tools = self.tools.read().await;
        tools.values()
            .filter(|reg| reg.metadata.compatibility_version.is_compatible(version))
            .cloned()
            .collect()
    }

    /// Cache discovery results
    pub async fn cache_discovery_result(&self, source: &str, result: ToolDiscoveryResult) {
        let mut cache = self.discovery_cache.write().await;
        cache.insert(source.to_string(), result);
    }

    /// Get cached discovery results
    pub async fn get_cached_discovery(&self, source: &str) -> Option<ToolDiscoveryResult> {
        let cache = self.discovery_cache.read().await;
        cache.get(source).cloned()
    }

    /// Clear expired cache entries
    pub async fn cleanup_cache(&self) {
        let mut cache = self.discovery_cache.write().await;
        let now = chrono::Utc::now().timestamp_millis() as u64;
        
        cache.retain(|_, result| {
            now - result.discovery_time_ms < self.cache_ttl_ms
        });
    }

    /// Get registry statistics
    pub async fn get_statistics(&self) -> HashMap<String, serde_json::Value> {
        let tools = self.tools.read().await;
        let cache = self.discovery_cache.read().await;
        
        let mut stats = HashMap::new();
        stats.insert("total_tools".to_string(), serde_json::Value::Number(tools.len().into()));
        stats.insert("available_tools".to_string(), 
            serde_json::Value::Number(tools.values().filter(|t| t.is_available).count().into()));
        stats.insert("cached_discoveries".to_string(), 
            serde_json::Value::Number(cache.len().into()));
        
        let avg_response_time = tools.values()
            .filter_map(|t| t.response_time_ms)
            .sum::<u64>() as f64 / tools.len().max(1) as f64;
        stats.insert("avg_response_time_ms".to_string(), 
            serde_json::Value::Number(serde_json::Number::from_f64(avg_response_time).unwrap_or(serde_json::Number::from(0))));
        
        stats
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin system for extending MCP client functionality
pub mod plugin {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;
    
    /// Plugin lifecycle states
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum PluginState {
        Unloaded,
        Loading,
        Loaded,
        Active,
        Inactive,
        Error(String),
        Unloading,
    }
    
    /// Plugin type enumeration
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum PluginType {
        Native,        // Native Rust plugin
        External,      // External process plugin
        WebAssembly,   // WASM plugin
        Script,        // Script-based plugin
    }
    
    /// Plugin metadata
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PluginMetadata {
        pub id: String,
        pub name: String,
        pub version: String,
        pub description: Option<String>,
        pub author: Option<String>,
        pub plugin_type: PluginType,
        pub mcp_version: MCPVersion,
        pub entry_point: String,
        pub dependencies: Vec<String>,
        pub permissions: Vec<String>,
        pub configuration: Option<serde_json::Value>,
    }
    
    /// Plugin instance information
    #[derive(Debug, Clone)]
    pub struct PluginInstance {
        pub metadata: PluginMetadata,
        pub state: PluginState,
        pub load_time: Option<std::time::SystemTime>,
        pub last_activity: Option<std::time::SystemTime>,
        pub error_count: u32,
        pub resource_usage: ResourceUsage,
    }
    
    /// Resource usage tracking
    #[derive(Debug, Clone, Default)]
    pub struct ResourceUsage {
        pub memory_mb: f64,
        pub cpu_percent: f64,
        pub network_bytes: u64,
        pub disk_bytes: u64,
    }
    
    /// Plugin execution context
    #[derive(Clone)]
    pub struct PluginContext {
        pub plugin_id: String,
        pub mcp_client: Arc<MCPClient>,
        pub tool_registry: Arc<ToolRegistry>,
        pub config: serde_json::Value,
        pub permissions: Vec<String>,
        pub resource_limits: ResourceLimits,
    }
    
    impl Serialize for PluginContext {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            use serde::ser::SerializeStruct;
            let mut state = serializer.serialize_struct("PluginContext", 4)?;
            state.serialize_field("plugin_id", &self.plugin_id)?;
            state.serialize_field("config", &self.config)?;
            state.serialize_field("permissions", &self.permissions)?;
            state.serialize_field("resource_limits", &self.resource_limits)?;
            state.end()
        }
    }
    
    /// Resource limits for plugins
    #[derive(Debug, Clone, Serialize)]
    pub struct ResourceLimits {
        pub max_memory_mb: f64,
        pub max_cpu_percent: f64,
        pub max_network_bytes: u64,
        pub max_disk_bytes: u64,
        pub timeout_seconds: u64,
    }
    
    impl Default for ResourceLimits {
        fn default() -> Self {
            Self {
                max_memory_mb: 100.0,
                max_cpu_percent: 10.0,
                max_network_bytes: 1024 * 1024 * 10, // 10MB
                max_disk_bytes: 1024 * 1024 * 50,    // 50MB
                timeout_seconds: 30,
            }
        }
    }
    
    /// Plugin capability trait
    #[async_trait::async_trait]
    pub trait Plugin: Send + Sync {
        /// Get plugin metadata
        fn metadata(&self) -> &PluginMetadata;
        
        /// Initialize the plugin
        async fn initialize(&mut self, context: PluginContext) -> Result<()>;
        
        /// Activate the plugin
        async fn activate(&mut self) -> Result<()>;
        
        /// Deactivate the plugin
        async fn deactivate(&mut self) -> Result<()>;
        
        /// Shutdown the plugin
        async fn shutdown(&mut self) -> Result<()>;
        
        /// Handle MCP message
        async fn handle_message(&mut self, message: &JSONRPCMessage) -> Result<Option<JSONRPCMessage>>;
        
        /// Get plugin capabilities
        fn capabilities(&self) -> Vec<String>;
        
        /// Validate plugin configuration
        fn validate_config(&self, config: &serde_json::Value) -> Result<()>;
        
        /// Get current resource usage
        fn resource_usage(&self) -> ResourceUsage;
        
        /// Handle plugin events
        async fn on_event(&mut self, event: PluginEvent) -> Result<()>;
    }
    
    /// Plugin events
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum PluginEvent {
        ToolRegistered { tool_name: String },
        ToolUnregistered { tool_name: String },
        ConnectionEstablished,
        ConnectionLost,
        MessageReceived { message: JSONRPCMessage },
        MessageSent { message: JSONRPCMessage },
        Error { error: String },
        Custom { event_type: String, data: serde_json::Value },
    }
    
    /// Plugin loader trait for different plugin types
    #[async_trait::async_trait]
    pub trait PluginLoader: Send + Sync {
        /// Load plugin from path
        async fn load_plugin(&self, path: &PathBuf) -> Result<Box<dyn Plugin>>;
        
        /// Validate plugin before loading
        async fn validate_plugin(&self, path: &PathBuf) -> Result<PluginMetadata>;
        
        /// Check if loader supports this plugin type
        fn supports_plugin_type(&self, plugin_type: &PluginType) -> bool;
        
        /// Get loader name
        fn loader_name(&self) -> &str;
    }
    
    /// Native Rust plugin loader
    pub struct NativePluginLoader;
    
    #[async_trait::async_trait]
    impl PluginLoader for NativePluginLoader {
        async fn load_plugin(&self, path: &PathBuf) -> Result<Box<dyn Plugin>> {
            // For native plugins, we'd use dynamic loading or compilation
            // This is a placeholder implementation
            anyhow::bail!("Native plugin loading not yet implemented for path: {:?}", path);
        }
        
        async fn validate_plugin(&self, path: &PathBuf) -> Result<PluginMetadata> {
            // Validate native plugin structure
            anyhow::bail!("Native plugin validation not yet implemented for path: {:?}", path);
        }
        
        fn supports_plugin_type(&self, plugin_type: &PluginType) -> bool {
            matches!(plugin_type, PluginType::Native)
        }
        
        fn loader_name(&self) -> &str {
            "native"
        }
    }
    
    /// External process plugin loader
    pub struct ExternalPluginLoader;
    
    #[async_trait::async_trait]
    impl PluginLoader for ExternalPluginLoader {
        async fn load_plugin(&self, path: &PathBuf) -> Result<Box<dyn Plugin>> {
            // Load external process plugin
            let metadata = self.validate_plugin(path).await?;
            Ok(Box::new(ExternalPlugin::new(metadata, path.clone())))
        }
        
        async fn validate_plugin(&self, path: &PathBuf) -> Result<PluginMetadata> {
            // Read plugin manifest
            let manifest_path = path.join("plugin.json");
            if !manifest_path.exists() {
                anyhow::bail!("Plugin manifest not found: {:?}", manifest_path);
            }
            
            let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
            let metadata: PluginMetadata = serde_json::from_str(&manifest_content)?;
            
            // Validate executable exists
            let executable_path = path.join(&metadata.entry_point);
            if !executable_path.exists() {
                anyhow::bail!("Plugin executable not found: {:?}", executable_path);
            }
            
            Ok(metadata)
        }
        
        fn supports_plugin_type(&self, plugin_type: &PluginType) -> bool {
            matches!(plugin_type, PluginType::External)
        }
        
        fn loader_name(&self) -> &str {
            "external"
        }
    }
    
    /// External process plugin implementation
    pub struct ExternalPlugin {
        metadata: PluginMetadata,
        path: PathBuf,
        process: Option<tokio::process::Child>,
        state: PluginState,
        context: Option<PluginContext>,
        resource_usage: ResourceUsage,
    }
    
    impl ExternalPlugin {
        pub fn new(metadata: PluginMetadata, path: PathBuf) -> Self {
            Self {
                metadata,
                path,
                process: None,
                state: PluginState::Unloaded,
                context: None,
                resource_usage: ResourceUsage::default(),
            }
        }
        
        async fn start_process(&mut self) -> Result<()> {
            let executable_path = self.path.join(&self.metadata.entry_point);
            
            let mut command = tokio::process::Command::new(&executable_path);
            command
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .current_dir(&self.path);
            
            let child = command.spawn()
                .context("Failed to start plugin process")?;
            
            self.process = Some(child);
            Ok(())
        }
        
        async fn stop_process(&mut self) -> Result<()> {
            if let Some(mut process) = self.process.take() {
                process.kill().await?;
                let _ = process.wait().await;
            }
            Ok(())
        }
        
        async fn send_message(&mut self, message: &JSONRPCMessage) -> Result<()> {
            if let Some(process) = &mut self.process {
                if let Some(stdin) = process.stdin.as_mut() {
                    let json = serde_json::to_string(message)?;
                    use tokio::io::AsyncWriteExt;
                    stdin.write_all(json.as_bytes()).await?;
                    stdin.write_all(b"\n").await?;
                    stdin.flush().await?;
                }
            }
            Ok(())
        }
        
        async fn receive_message(&mut self) -> Result<Option<JSONRPCMessage>> {
            if let Some(process) = &mut self.process {
                if let Some(stdout) = process.stdout.as_mut() {
                    use tokio::io::{AsyncBufReadExt, BufReader};
                    let mut reader = BufReader::new(stdout);
                    let mut line = String::new();
                    match reader.read_line(&mut line).await {
                        Ok(0) => Ok(None), // EOF
                        Ok(_) => {
                            let message: JSONRPCMessage = serde_json::from_str(&line)?;
                            Ok(Some(message))
                        }
                        Err(e) => Err(e.into()),
                    }
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        }
    }
    
    #[async_trait::async_trait]
    impl Plugin for ExternalPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.metadata
        }
        
        async fn initialize(&mut self, context: PluginContext) -> Result<()> {
            self.context = Some(context);
            self.state = PluginState::Loading;
            
            self.start_process().await?;
            
            // Send initialization message
            let init_message = JSONRPCMessage::Request {
                jsonrpc: "2.0".to_string(),
                id: serde_json::Value::String(uuid::Uuid::new_v4().to_string()),
                method: "plugin/initialize".to_string(),
                params: Some(serde_json::to_value(&self.context)?),
            };
            
            self.send_message(&init_message).await?;
            self.state = PluginState::Loaded;
            Ok(())
        }
        
        async fn activate(&mut self) -> Result<()> {
            self.state = PluginState::Active;
            
            let activate_message = JSONRPCMessage::Request {
                jsonrpc: "2.0".to_string(),
                id: serde_json::Value::String(uuid::Uuid::new_v4().to_string()),
                method: "plugin/activate".to_string(),
                params: None,
            };
            
            self.send_message(&activate_message).await?;
            Ok(())
        }
        
        async fn deactivate(&mut self) -> Result<()> {
            self.state = PluginState::Inactive;
            
            let deactivate_message = JSONRPCMessage::Request {
                jsonrpc: "2.0".to_string(),
                id: serde_json::Value::String(uuid::Uuid::new_v4().to_string()),
                method: "plugin/deactivate".to_string(),
                params: None,
            };
            
            self.send_message(&deactivate_message).await?;
            Ok(())
        }
        
        async fn shutdown(&mut self) -> Result<()> {
            self.state = PluginState::Unloading;
            
            let shutdown_message = JSONRPCMessage::Request {
                jsonrpc: "2.0".to_string(),
                id: serde_json::Value::String(uuid::Uuid::new_v4().to_string()),
                method: "plugin/shutdown".to_string(),
                params: None,
            };
            
            let _ = self.send_message(&shutdown_message).await;
            self.stop_process().await?;
            self.state = PluginState::Unloaded;
            Ok(())
        }
        
        async fn handle_message(&mut self, message: &JSONRPCMessage) -> Result<Option<JSONRPCMessage>> {
            self.send_message(message).await?;
            self.receive_message().await
        }
        
        fn capabilities(&self) -> Vec<String> {
            vec!["message_handling".to_string(), "external_process".to_string()]
        }
        
        fn validate_config(&self, _config: &serde_json::Value) -> Result<()> {
            // Basic validation for external plugins
            Ok(())
        }
        
        fn resource_usage(&self) -> ResourceUsage {
            self.resource_usage.clone()
        }
        
        async fn on_event(&mut self, event: PluginEvent) -> Result<()> {
            let event_message = JSONRPCMessage::Notification {
                jsonrpc: "2.0".to_string(),
                method: "plugin/event".to_string(),
                params: Some(serde_json::to_value(&event)?),
            };
            
            self.send_message(&event_message).await?;
            Ok(())
        }
    }
    
    /// Plugin manager for loading, managing, and orchestrating plugins
    pub struct PluginManager {
        plugins: Arc<RwLock<HashMap<String, Box<dyn Plugin>>>>,
        plugin_instances: Arc<RwLock<HashMap<String, PluginInstance>>>,
        loaders: Vec<Box<dyn PluginLoader>>,
        plugin_directories: Vec<PathBuf>,
        security_manager: PluginSecurityManager,
        event_bus: PluginEventBus,
    }
    
    impl PluginManager {
        /// Create a new plugin manager
        pub fn new() -> Self {
            let mut loaders: Vec<Box<dyn PluginLoader>> = Vec::new();
            loaders.push(Box::new(NativePluginLoader));
            loaders.push(Box::new(ExternalPluginLoader));
            
            Self {
                plugins: Arc::new(RwLock::new(HashMap::new())),
                plugin_instances: Arc::new(RwLock::new(HashMap::new())),
                loaders,
                plugin_directories: Vec::new(),
                security_manager: PluginSecurityManager::new(),
                event_bus: PluginEventBus::new(),
            }
        }
        
        /// Add a plugin directory to scan
        pub fn add_plugin_directory(&mut self, path: PathBuf) {
            self.plugin_directories.push(path);
        }
        
        /// Scan for plugins in all configured directories
        pub async fn scan_plugins(&self) -> Result<Vec<PluginMetadata>> {
            let mut found_plugins = Vec::new();
            
            for directory in &self.plugin_directories {
                if directory.exists() && directory.is_dir() {
                    let mut entries = tokio::fs::read_dir(directory).await?;
                    
                    while let Some(entry) = entries.next_entry().await? {
                        let entry_path = entry.path();
                        if entry_path.is_dir() {
                            // Try to load plugin metadata from this directory
                            for loader in &self.loaders {
                                match loader.validate_plugin(&entry_path).await {
                                    Ok(metadata) => {
                                        found_plugins.push(metadata);
                                        break;
                                    }
                                    Err(_) => continue,
                                }
                            }
                        }
                    }
                }
            }
            
            Ok(found_plugins)
        }
        
        /// Load a plugin from a specific path
        pub async fn load_plugin(&self, path: &PathBuf) -> Result<String> {
            // First validate the plugin
            let metadata = self.validate_plugin_security(path).await?;
            
            // Find appropriate loader
            let loader = self.loaders.iter()
                .find(|l| l.supports_plugin_type(&metadata.plugin_type))
                .ok_or_else(|| anyhow::anyhow!("No loader found for plugin type: {:?}", metadata.plugin_type))?;
            
            // Load the plugin
            let plugin = loader.load_plugin(path).await?;
            let plugin_id = metadata.id.clone();
            
            // Create plugin instance
            let instance = PluginInstance {
                metadata: metadata.clone(),
                state: PluginState::Loaded,
                load_time: Some(std::time::SystemTime::now()),
                last_activity: None,
                error_count: 0,
                resource_usage: ResourceUsage::default(),
            };
            
            // Store plugin and instance
            {
                let mut plugins = self.plugins.write().await;
                plugins.insert(plugin_id.clone(), plugin);
            }
            
            {
                let mut instances = self.plugin_instances.write().await;
                instances.insert(plugin_id.clone(), instance);
            }
            
            // Emit event
            self.event_bus.emit_event(PluginEvent::Custom {
                event_type: "plugin_loaded".to_string(),
                data: serde_json::json!({
                    "plugin_id": plugin_id,
                    "plugin_name": metadata.name,
                    "plugin_version": metadata.version
                }),
            }).await?;
            
            Ok(plugin_id)
        }
        
        /// Unload a plugin
        pub async fn unload_plugin(&self, plugin_id: &str) -> Result<()> {
            // Get plugin and shutdown
            {
                let mut plugins = self.plugins.write().await;
                if let Some(mut plugin) = plugins.remove(plugin_id) {
                    plugin.shutdown().await?;
                }
            }
            
            // Update instance state
            {
                let mut instances = self.plugin_instances.write().await;
                if let Some(instance) = instances.get_mut(plugin_id) {
                    instance.state = PluginState::Unloaded;
                }
            }
            
            // Emit event
            self.event_bus.emit_event(PluginEvent::Custom {
                event_type: "plugin_unloaded".to_string(),
                data: serde_json::json!({
                    "plugin_id": plugin_id
                }),
            }).await?;
            
            Ok(())
        }
        
        /// Initialize a plugin
        pub async fn initialize_plugin(&self, plugin_id: &str, context: PluginContext) -> Result<()> {
            let mut plugins = self.plugins.write().await;
            if let Some(plugin) = plugins.get_mut(plugin_id) {
                plugin.initialize(context).await?;
                
                // Update instance state
                drop(plugins);
                let mut instances = self.plugin_instances.write().await;
                if let Some(instance) = instances.get_mut(plugin_id) {
                    instance.state = PluginState::Loaded;
                }
            }
            
            Ok(())
        }
        
        /// Activate a plugin
        pub async fn activate_plugin(&self, plugin_id: &str) -> Result<()> {
            let mut plugins = self.plugins.write().await;
            if let Some(plugin) = plugins.get_mut(plugin_id) {
                plugin.activate().await?;
                
                // Update instance state
                drop(plugins);
                let mut instances = self.plugin_instances.write().await;
                if let Some(instance) = instances.get_mut(plugin_id) {
                    instance.state = PluginState::Active;
                    instance.last_activity = Some(std::time::SystemTime::now());
                }
            }
            
            Ok(())
        }
        
        /// Deactivate a plugin
        pub async fn deactivate_plugin(&self, plugin_id: &str) -> Result<()> {
            let mut plugins = self.plugins.write().await;
            if let Some(plugin) = plugins.get_mut(plugin_id) {
                plugin.deactivate().await?;
                
                // Update instance state
                drop(plugins);
                let mut instances = self.plugin_instances.write().await;
                if let Some(instance) = instances.get_mut(plugin_id) {
                    instance.state = PluginState::Inactive;
                }
            }
            
            Ok(())
        }
        
        /// Send message to plugin
        pub async fn send_message_to_plugin(&self, plugin_id: &str, message: &JSONRPCMessage) -> Result<Option<JSONRPCMessage>> {
            let mut plugins = self.plugins.write().await;
            if let Some(plugin) = plugins.get_mut(plugin_id) {
                plugin.handle_message(message).await
            } else {
                anyhow::bail!("Plugin not found: {}", plugin_id);
            }
        }
        
        /// Broadcast event to all active plugins
        pub async fn broadcast_event(&self, event: PluginEvent) -> Result<()> {
            let mut plugins = self.plugins.write().await;
            let instances = self.plugin_instances.read().await;
            
            for (plugin_id, instance) in instances.iter() {
                if instance.state == PluginState::Active {
                    if let Some(plugin) = plugins.get_mut(plugin_id) {
                        if let Err(e) = plugin.on_event(event.clone()).await {
                            log::warn!("Failed to send event to plugin {}: {}", plugin_id, e);
                        }
                    }
                }
            }
            
            Ok(())
        }
        
        /// Get plugin instances
        pub async fn get_plugin_instances(&self) -> HashMap<String, PluginInstance> {
            self.plugin_instances.read().await.clone()
        }
        
        /// Get plugin instance by ID
        pub async fn get_plugin_instance(&self, plugin_id: &str) -> Option<PluginInstance> {
            self.plugin_instances.read().await.get(plugin_id).cloned()
        }
        
        /// Get plugins by state
        pub async fn get_plugins_by_state(&self, state: PluginState) -> Vec<PluginInstance> {
            let instances = self.plugin_instances.read().await;
            instances.values()
                .filter(|instance| instance.state == state)
                .cloned()
                .collect()
        }
        
        /// Validate plugin security
        async fn validate_plugin_security(&self, path: &PathBuf) -> Result<PluginMetadata> {
            // Find appropriate loader and validate
            for loader in &self.loaders {
                if let Ok(metadata) = loader.validate_plugin(path).await {
                    // Security validation
                    self.security_manager.validate_plugin(&metadata, path).await?;
                    return Ok(metadata);
                }
            }
            
            anyhow::bail!("No valid plugin found at path: {:?}", path);
        }
    }
    
    /// Plugin security manager
    #[derive(Debug)]
    pub struct PluginSecurityManager {
        allowed_permissions: Vec<String>,
        sandbox_enabled: bool,
        resource_limits: ResourceLimits,
    }
    
    impl PluginSecurityManager {
        pub fn new() -> Self {
            Self {
                allowed_permissions: vec![
                    "network.http".to_string(),
                    "filesystem.read".to_string(),
                    "mcp.tools.call".to_string(),
                ],
                sandbox_enabled: true,
                resource_limits: ResourceLimits::default(),
            }
        }
        
        /// Validate plugin security
        pub async fn validate_plugin(&self, metadata: &PluginMetadata, _path: &PathBuf) -> Result<()> {
            // Check permissions
            for permission in &metadata.permissions {
                if !self.allowed_permissions.contains(permission) {
                    anyhow::bail!("Permission not allowed: {}", permission);
                }
            }
            
            // Additional security checks can be added here
            // - Code signing verification
            // - Checksum validation
            // - Security policy compliance
            
            Ok(())
        }
        
        /// Check if plugin exceeds resource limits
        pub fn check_resource_limits(&self, usage: &ResourceUsage) -> Result<()> {
            if usage.memory_mb > self.resource_limits.max_memory_mb {
                anyhow::bail!("Memory usage exceeds limit: {} MB", usage.memory_mb);
            }
            
            if usage.cpu_percent > self.resource_limits.max_cpu_percent {
                anyhow::bail!("CPU usage exceeds limit: {}%", usage.cpu_percent);
            }
            
            Ok(())
        }
    }
    
    /// Plugin event bus for communication
    pub struct PluginEventBus {
        event_handlers: Arc<RwLock<HashMap<String, Vec<Box<dyn Fn(PluginEvent) -> Result<()> + Send + Sync>>>>>,
    }
    
    impl PluginEventBus {
        pub fn new() -> Self {
            Self {
                event_handlers: Arc::new(RwLock::new(HashMap::new())),
            }
        }
        
        /// Emit an event to all registered handlers
        pub async fn emit_event(&self, event: PluginEvent) -> Result<()> {
            let handlers = self.event_handlers.read().await;
            let event_type = match &event {
                PluginEvent::ToolRegistered { .. } => "tool_registered",
                PluginEvent::ToolUnregistered { .. } => "tool_unregistered",
                PluginEvent::ConnectionEstablished => "connection_established",
                PluginEvent::ConnectionLost => "connection_lost",
                PluginEvent::MessageReceived { .. } => "message_received",
                PluginEvent::MessageSent { .. } => "message_sent",
                PluginEvent::Error { .. } => "error",
                PluginEvent::Custom { event_type, .. } => event_type,
            };
            
            if let Some(event_handlers) = handlers.get(event_type) {
                for handler in event_handlers {
                    if let Err(e) = handler(event.clone()) {
                        log::warn!("Event handler failed: {}", e);
                    }
                }
            }
            
            Ok(())
        }
        
        /// Register an event handler
        pub async fn register_handler<F>(&self, event_type: &str, handler: F) 
        where
            F: Fn(PluginEvent) -> Result<()> + Send + Sync + 'static,
        {
            let mut handlers = self.event_handlers.write().await;
            handlers.entry(event_type.to_string())
                .or_insert_with(Vec::new)
                .push(Box::new(handler));
        }
    }
}

/// MCP Transport types
#[derive(Debug, Clone)]
pub enum MCPTransport {
    StdIO,
    TCP { host: String, port: u16 },
    WebSocket { url: String },
    Process { command: String, args: Vec<String> },
}

/// MCP Client configuration
#[derive(Debug, Clone)]
pub struct MCPClientConfig {
    pub transport: MCPTransport,
    pub protocol_version: MCPVersion,
    pub client_info: ClientInfo,
    pub timeout_ms: u64,
    pub max_retries: u32,
}

impl Default for MCPClientConfig {
    fn default() -> Self {
        Self {
            transport: MCPTransport::StdIO,
            protocol_version: MCPVersion::current(),
            client_info: ClientInfo {
                name: "arrowhead".to_string(),
                version: "0.1.0".to_string(),
            },
            timeout_ms: 30000,
            max_retries: 3,
        }
    }
}

/// MCP Client connection state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MCPConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Initialized,
    Error(String),
}

/// Server capabilities discovered during initialization
#[derive(Debug, Clone, Default)]
pub struct ServerCapabilities {
    pub protocol_version: Option<MCPVersion>,
    pub supported_methods: Vec<String>,
    pub experimental_features: Vec<String>,
    pub backwards_compatibility: bool,
    pub feature_flags: HashMap<String, bool>,
}

/// Feature flag configuration
#[derive(Debug, Clone)]
pub struct FeatureFlag {
    pub name: String,
    pub enabled: bool,
    pub min_version: Option<MCPVersion>,
    pub max_version: Option<MCPVersion>,
    pub description: Option<String>,
}

/// MCP Client implementation
#[derive(Clone)]
pub struct MCPClient {
    config: MCPClientConfig,
    state: Arc<RwLock<MCPConnectionState>>,
    pending_requests: Arc<RwLock<HashMap<String, tokio::sync::oneshot::Sender<serde_json::Value>>>>,
    child_process: Option<Arc<RwLock<Option<Child>>>>,
    tool_registry: Arc<ToolRegistry>,
    plugin_manager: Arc<RwLock<plugin::PluginManager>>,
    server_capabilities: Arc<RwLock<ServerCapabilities>>,
    feature_flags: Arc<RwLock<HashMap<String, FeatureFlag>>>,
}

impl MCPClient {
    /// Create a new MCP client with the given configuration
    pub fn new(config: MCPClientConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(MCPConnectionState::Disconnected)),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            child_process: None,
            tool_registry: Arc::new(ToolRegistry::new()),
            plugin_manager: Arc::new(RwLock::new(plugin::PluginManager::new())),
            server_capabilities: Arc::new(RwLock::new(ServerCapabilities::default())),
            feature_flags: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the current connection state
    pub async fn get_state(&self) -> MCPConnectionState {
        self.state.read().await.clone()
    }

    /// Connect to the MCP server
    pub async fn connect(&mut self) -> Result<()> {
        let mut state = self.state.write().await;
        *state = MCPConnectionState::Connecting;
        drop(state);

        match &self.config.transport {
            MCPTransport::StdIO => {
                // For StdIO, we're already connected
                let mut state = self.state.write().await;
                *state = MCPConnectionState::Connected;
                Ok(())
            }
            MCPTransport::TCP { host, port } => {
                let _stream = TcpStream::connect(format!("{}:{}", host, port)).await
                    .context("Failed to connect to TCP server")?;
                
                let mut state = self.state.write().await;
                *state = MCPConnectionState::Connected;
                Ok(())
            }
            MCPTransport::WebSocket { url: _ } => {
                // TODO: Implement WebSocket connection
                anyhow::bail!("WebSocket transport not yet implemented");
            }
            MCPTransport::Process { command, args } => {
                let child = Command::new(command)
                    .args(args)
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .context("Failed to spawn MCP server process")?;

                self.child_process = Some(Arc::new(RwLock::new(Some(child))));
                let mut state = self.state.write().await;
                *state = MCPConnectionState::Connected;
                Ok(())
            }
        }
    }

    /// Initialize the MCP protocol with automatic version negotiation
    pub async fn initialize(&self) -> Result<serde_json::Value> {
        let state = self.state.read().await;
        if *state != MCPConnectionState::Connected {
            anyhow::bail!("Client must be connected before initializing");
        }
        drop(state);

        // Perform version negotiation first
        let negotiated_version = self.negotiate_protocol_version().await?;
        
        let params = InitializeParams {
            protocol_version: negotiated_version.to_string(),
            capabilities: ClientCapabilities {
                roots: Some(RootsCapability {
                    list_changed: true,
                }),
                sampling: None,
            },
            client_info: self.config.client_info.clone(),
        };

        let request = JSONRPCMessage::Request {
            jsonrpc: "2.0".to_string(),
            id: serde_json::Value::String(Uuid::new_v4().to_string()),
            method: "initialize".to_string(),
            params: Some(serde_json::to_value(params)?),
        };

        let response = self.send_request(request).await?;
        
        // Parse server capabilities from response
        self.parse_server_capabilities(&response).await?;
        
        // Initialize feature flags based on negotiated capabilities
        self.initialize_feature_flags().await?;
        
        let mut state = self.state.write().await;
        *state = MCPConnectionState::Initialized;

        Ok(response)
    }

    /// Negotiate protocol version with the server
    pub async fn negotiate_protocol_version(&self) -> Result<MCPVersion> {
        // Start with client's preferred version
        let client_version = self.config.protocol_version.clone();
        
        // Try to detect server's supported version
        match self.detect_server_version().await {
            Ok(server_version) => {
                // Find compatible version
                if client_version.is_compatible(&server_version) {
                    // Use the newer version that both support
                    let negotiated_version = if client_version.is_backward_compatible(&server_version) {
                        client_version
                    } else {
                        server_version
                    };
                    
                    // Update server capabilities with negotiated version
                    {
                        let mut capabilities = self.server_capabilities.write().await;
                        capabilities.protocol_version = Some(negotiated_version.clone());
                        capabilities.backwards_compatibility = true;
                    }
                    
                    log::info!("Protocol version negotiated: {}", negotiated_version.to_string());
                    Ok(negotiated_version)
                } else {
                    // Versions are incompatible, try fallback
                    log::warn!("Protocol version incompatible. Client: {}, Server: {}", 
                              client_version.to_string(), server_version.to_string());
                    
                    // Try to find a compatible older version
                    let fallback_version = self.find_compatible_version(&client_version, &server_version).await?;
                    
                    {
                        let mut capabilities = self.server_capabilities.write().await;
                        capabilities.protocol_version = Some(fallback_version.clone());
                        capabilities.backwards_compatibility = true;
                    }
                    
                    log::info!("Using fallback protocol version: {}", fallback_version.to_string());
                    Ok(fallback_version)
                }
            }
            Err(e) => {
                log::warn!("Could not detect server version: {}. Using client version.", e);
                // Fall back to client version
                {
                    let mut capabilities = self.server_capabilities.write().await;
                    capabilities.protocol_version = Some(client_version.clone());
                    capabilities.backwards_compatibility = false;
                }
                Ok(client_version)
            }
        }
    }

    /// Detect server's protocol version
    async fn detect_server_version(&self) -> Result<MCPVersion> {
        // Try to send a version inquiry request
        let request = JSONRPCMessage::Request {
            jsonrpc: "2.0".to_string(),
            id: serde_json::Value::String(Uuid::new_v4().to_string()),
            method: "protocol/version".to_string(),
            params: None,
        };

        match self.send_request(request).await {
            Ok(response) => {
                // Parse version from response
                if let Some(version_str) = response.get("version").and_then(|v| v.as_str()) {
                    MCPVersion::from_string(version_str)
                } else {
                    anyhow::bail!("Invalid version response format");
                }
            }
            Err(_) => {
                // If version inquiry fails, try to infer from other methods
                // This is a fallback for servers that don't support version inquiry
                self.infer_server_version().await
            }
        }
    }

    /// Infer server version from available methods
    async fn infer_server_version(&self) -> Result<MCPVersion> {
        // Send a ping to check basic connectivity and response format
        let ping_request = JSONRPCMessage::Request {
            jsonrpc: "2.0".to_string(),
            id: serde_json::Value::String(Uuid::new_v4().to_string()),
            method: "ping".to_string(),
            params: None,
        };

        match self.send_request(ping_request).await {
            Ok(_) => {
                // Server responds to ping, assume it supports current version
                Ok(MCPVersion::current())
            }
            Err(_) => {
                // If ping fails, try older version format
                Ok(MCPVersion::new(2024, 10, 1)) // Fallback to older version
            }
        }
    }

    /// Find a compatible version between client and server
    async fn find_compatible_version(&self, client_version: &MCPVersion, server_version: &MCPVersion) -> Result<MCPVersion> {
        // List of known compatible versions in descending order
        let known_versions = vec![
            MCPVersion::new(2024, 11, 5),
            MCPVersion::new(2024, 11, 1),
            MCPVersion::new(2024, 10, 15),
            MCPVersion::new(2024, 10, 1),
        ];

        // Find the highest version that both client and server can support
        for version in known_versions {
            if client_version.is_backward_compatible(&version) && server_version.is_backward_compatible(&version) {
                return Ok(version);
            }
        }

        // If no compatible version found, use the older of the two
        if client_version.major < server_version.major ||
           (client_version.major == server_version.major && client_version.minor < server_version.minor) {
            Ok(client_version.clone())
        } else {
            Ok(server_version.clone())
        }
    }

    /// Parse server capabilities from initialization response
    async fn parse_server_capabilities(&self, response: &serde_json::Value) -> Result<()> {
        let mut capabilities = self.server_capabilities.write().await;
        
        // Parse capabilities from response
        if let Some(server_caps) = response.get("capabilities") {
            // Extract supported methods
            if let Some(methods) = server_caps.get("methods").and_then(|m| m.as_array()) {
                capabilities.supported_methods = methods.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect();
            }
            
            // Extract experimental features
            if let Some(experimental) = server_caps.get("experimental").and_then(|e| e.as_array()) {
                capabilities.experimental_features = experimental.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect();
            }
            
            // Extract feature flags
            if let Some(flags) = server_caps.get("features").and_then(|f| f.as_object()) {
                for (key, value) in flags {
                    if let Some(enabled) = value.as_bool() {
                        capabilities.feature_flags.insert(key.clone(), enabled);
                    }
                }
            }
        }
        
        // Log discovered capabilities
        log::info!("Server capabilities discovered: {} methods, {} experimental features, {} feature flags",
                  capabilities.supported_methods.len(),
                  capabilities.experimental_features.len(),
                  capabilities.feature_flags.len());
        
        Ok(())
    }

    /// Initialize feature flags based on negotiated capabilities
    async fn initialize_feature_flags(&self) -> Result<()> {
        let mut flags = self.feature_flags.write().await;
        let capabilities = self.server_capabilities.read().await;
        
        // Define default feature flags based on protocol version
        let default_flags = vec![
            FeatureFlag {
                name: "tools.list".to_string(),
                enabled: true,
                min_version: Some(MCPVersion::new(2024, 10, 1)),
                max_version: None,
                description: Some("List available tools".to_string()),
            },
            FeatureFlag {
                name: "tools.call".to_string(),
                enabled: true,
                min_version: Some(MCPVersion::new(2024, 10, 1)),
                max_version: None,
                description: Some("Call tools".to_string()),
            },
            FeatureFlag {
                name: "resources.list".to_string(),
                enabled: true,
                min_version: Some(MCPVersion::new(2024, 10, 1)),
                max_version: None,
                description: Some("List available resources".to_string()),
            },
            FeatureFlag {
                name: "resources.read".to_string(),
                enabled: true,
                min_version: Some(MCPVersion::new(2024, 10, 1)),
                max_version: None,
                description: Some("Read resources".to_string()),
            },
            FeatureFlag {
                name: "logging.setLevel".to_string(),
                enabled: true,
                min_version: Some(MCPVersion::new(2024, 11, 1)),
                max_version: None,
                description: Some("Set logging level".to_string()),
            },
            FeatureFlag {
                name: "experimental.streaming".to_string(),
                enabled: false,
                min_version: Some(MCPVersion::new(2024, 11, 5)),
                max_version: None,
                description: Some("Streaming responses".to_string()),
            },
        ];
        
        // Initialize flags based on server capabilities and version
        for flag in default_flags {
            let mut enabled = flag.enabled;
            
            // Check version compatibility
            if let Some(server_version) = &capabilities.protocol_version {
                if let Some(min_version) = &flag.min_version {
                    if !server_version.is_backward_compatible(min_version) {
                        enabled = false;
                    }
                }
                if let Some(max_version) = &flag.max_version {
                    if server_version.is_backward_compatible(max_version) {
                        enabled = false;
                    }
                }
            }
            
            // Check if server explicitly supports this feature
            if let Some(server_enabled) = capabilities.feature_flags.get(&flag.name) {
                enabled = *server_enabled;
            }
            
            // Check if it's in supported methods
            if capabilities.supported_methods.contains(&flag.name) {
                enabled = true;
            }
            
            let final_flag = FeatureFlag {
                name: flag.name.clone(),
                enabled,
                min_version: flag.min_version,
                max_version: flag.max_version,
                description: flag.description,
            };
            
            flags.insert(flag.name, final_flag);
        }
        
        log::info!("Initialized {} feature flags", flags.len());
        Ok(())
    }

    /// Send a JSON-RPC request and wait for response
    async fn send_request(&self, request: JSONRPCMessage) -> Result<serde_json::Value> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        
        // Extract request ID
        let request_id = match &request {
            JSONRPCMessage::Request { id, .. } => id.clone(),
            _ => anyhow::bail!("Invalid request message"),
        };

        // Store the response channel
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(request_id.to_string(), tx);
        }

        // Send the request
        self.send_message(request).await?;

        // Wait for response with timeout
        let response = tokio::time::timeout(
            std::time::Duration::from_millis(self.config.timeout_ms),
            rx
        ).await
        .context("Request timeout")?
        .context("Request channel closed")?;

        Ok(response)
    }

    /// Send a message (without waiting for response)
    async fn send_message(&self, message: JSONRPCMessage) -> Result<()> {
        let json = serde_json::to_string(&message)?;
        
        match &self.config.transport {
            MCPTransport::StdIO => {
                // For StdIO, write to stdout
                println!("{}", json);
                Ok(())
            }
            MCPTransport::TCP { .. } => {
                // TODO: Implement TCP message sending
                anyhow::bail!("TCP message sending not yet implemented");
            }
            MCPTransport::WebSocket { .. } => {
                // TODO: Implement WebSocket message sending
                anyhow::bail!("WebSocket message sending not yet implemented");
            }
            MCPTransport::Process { .. } => {
                // TODO: Implement process message sending
                anyhow::bail!("Process message sending not yet implemented");
            }
        }
    }

    /// List available tools with graceful degradation
    pub async fn list_tools(&self) -> Result<serde_json::Value> {
        self.call_method_with_degradation("tools/list", None).await
    }

    /// Discover and register tools from the server
    pub async fn discover_tools(&self) -> Result<ToolDiscoveryResult> {
        let start_time = std::time::Instant::now();
        let mut errors = Vec::new();
        let mut tools = Vec::new();

        // First, get tools from the server
        match self.list_tools().await {
            Ok(response) => {
                // Parse the response to extract tool metadata
                if let Some(tools_array) = response.get("tools").and_then(|t| t.as_array()) {
                    for tool_value in tools_array {
                        match self.parse_tool_metadata(tool_value) {
                            Ok(metadata) => {
                                tools.push(metadata.clone());
                                // Register the tool in our registry
                                if let Err(e) = self.tool_registry.register_tool(metadata).await {
                                    errors.push(format!("Failed to register tool: {}", e));
                                }
                            }
                            Err(e) => {
                                errors.push(format!("Failed to parse tool metadata: {}", e));
                            }
                        }
                    }
                } else {
                    errors.push("Invalid tools response format".to_string());
                }
            }
            Err(e) => {
                errors.push(format!("Failed to list tools: {}", e));
            }
        }

        let discovery_time_ms = start_time.elapsed().as_millis() as u64;
        let result = ToolDiscoveryResult {
            tools,
            discovery_time_ms,
            source: "mcp_server".to_string(),
            errors,
        };

        // Cache the discovery result
        self.tool_registry.cache_discovery_result("mcp_server", result.clone()).await;

        Ok(result)
    }

    /// Parse tool metadata from JSON response
    fn parse_tool_metadata(&self, tool_value: &serde_json::Value) -> Result<ToolMetadata> {
        let name = tool_value.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Tool name is required"))?
            .to_string();

        let description = tool_value.get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let schema = tool_value.get("inputSchema").cloned();

        // Extract capabilities from the schema or default to empty
        let capabilities = self.extract_capabilities_from_schema(&schema);

        Ok(ToolMetadata {
            name,
            version: "1.0.0".to_string(), // Default version
            description,
            schema,
            capabilities,
            compatibility_version: self.config.protocol_version.clone(),
            last_updated: Some(chrono::Utc::now().to_rfc3339()),
            provider: Some("mcp_server".to_string()),
        })
    }

    /// Extract capabilities from tool schema
    fn extract_capabilities_from_schema(&self, schema: &Option<serde_json::Value>) -> Vec<String> {
        let mut capabilities = vec!["execute".to_string()]; // Default capability

        if let Some(schema_obj) = schema {
            if let Some(properties) = schema_obj.get("properties") {
                if properties.is_object() {
                    capabilities.push("parameterized".to_string());
                }
            }
            
            if let Some(required) = schema_obj.get("required") {
                if required.is_array() {
                    capabilities.push("validated".to_string());
                }
            }
        }

        capabilities
    }

    /// Get tool registry reference
    pub fn get_tool_registry(&self) -> &Arc<ToolRegistry> {
        &self.tool_registry
    }

    /// Refresh tool registry by rediscovering tools
    pub async fn refresh_tool_registry(&self) -> Result<()> {
        let discovery_result = self.discover_tools().await?;
        
        // Log discovery results
        if !discovery_result.errors.is_empty() {
            log::warn!("Tool discovery completed with {} errors", discovery_result.errors.len());
            for error in &discovery_result.errors {
                log::warn!("Discovery error: {}", error);
            }
        }
        
        log::info!("Discovered {} tools in {}ms", 
            discovery_result.tools.len(), 
            discovery_result.discovery_time_ms);

        Ok(())
    }

    /// Get tools by capability from registry
    pub async fn get_tools_by_capability(&self, capability: &str) -> Vec<ToolRegistration> {
        self.tool_registry.get_tools_by_capability(capability).await
    }

    /// Get compatible tools from registry
    pub async fn get_compatible_tools(&self, version: &MCPVersion) -> Vec<ToolRegistration> {
        self.tool_registry.get_compatible_tools(version).await
    }

    /// Negotiate capabilities with the server
    pub async fn negotiate_capabilities(&self, required_capabilities: &[String]) -> Result<Vec<String>> {
        let discovery_result = self.discover_tools().await?;
        let mut supported_capabilities = Vec::new();

        // Check which capabilities are supported by available tools
        for required_capability in required_capabilities {
            let tools_with_capability = self.tool_registry
                .get_tools_by_capability(required_capability)
                .await;
            
            if !tools_with_capability.is_empty() {
                supported_capabilities.push(required_capability.clone());
            }
        }

        // Log negotiation results
        log::info!("Capability negotiation completed. Supported: {:?}", supported_capabilities);
        if supported_capabilities.len() < required_capabilities.len() {
            let unsupported: Vec<&String> = required_capabilities.iter()
                .filter(|cap| !supported_capabilities.contains(cap))
                .collect();
            log::warn!("Unsupported capabilities: {:?}", unsupported);
        }

        Ok(supported_capabilities)
    }

    /// Get tool registry statistics
    pub async fn get_registry_statistics(&self) -> HashMap<String, serde_json::Value> {
        self.tool_registry.get_statistics().await
    }

    /// Validate tool against required capabilities
    pub async fn validate_tool_capabilities(&self, tool_name: &str, required_capabilities: &[String]) -> Result<bool> {
        if let Some(tool_registration) = self.tool_registry.get_tool(tool_name).await {
            let tool_capabilities = &tool_registration.metadata.capabilities;
            
            for required_capability in required_capabilities {
                if !tool_capabilities.contains(required_capability) {
                    return Ok(false);
                }
            }
            
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get tool metadata by name
    pub async fn get_tool_metadata(&self, tool_name: &str) -> Option<ToolMetadata> {
        self.tool_registry.get_tool(tool_name).await.map(|reg| reg.metadata)
    }

    /// Check if tool is available and responsive
    pub async fn is_tool_available(&self, tool_name: &str) -> bool {
        if let Some(tool_registration) = self.tool_registry.get_tool(tool_name).await {
            tool_registration.is_available && tool_registration.error_count < 3
        } else {
            false
        }
    }

    /// Check version compatibility between client and server
    pub async fn check_version_compatibility(&self, server_version: &str) -> Result<bool> {
        let server_version = MCPVersion::from_string(server_version)?;
        let client_version = &self.config.protocol_version;
        
        Ok(client_version.is_compatible(&server_version))
    }

    /// Get all tools compatible with a specific version
    pub async fn get_tools_compatible_with_version(&self, version: &str) -> Result<Vec<ToolRegistration>> {
        let version = MCPVersion::from_string(version)?;
        Ok(self.tool_registry.get_compatible_tools(&version).await)
    }

    /// Update tool version information
    pub async fn update_tool_version(&self, tool_name: &str, new_version: &str) -> Result<()> {
        if let Some(mut tool_registration) = self.tool_registry.get_tool(tool_name).await {
            tool_registration.metadata.version = new_version.to_string();
            tool_registration.metadata.last_updated = Some(chrono::Utc::now().to_rfc3339());
            
            // Re-register with updated version
            self.tool_registry.register_tool(tool_registration.metadata).await?;
        }
        
        Ok(())
    }

    /// Get minimum required version for a set of tools
    pub async fn get_minimum_required_version(&self, tool_names: &[&str]) -> Result<MCPVersion> {
        let mut min_version = MCPVersion::new(0, 0, 0);
        
        for tool_name in tool_names {
            if let Some(tool_registration) = self.tool_registry.get_tool(tool_name).await {
                let tool_version = &tool_registration.metadata.compatibility_version;
                if tool_version.major > min_version.major ||
                   (tool_version.major == min_version.major && tool_version.minor > min_version.minor) ||
                   (tool_version.major == min_version.major && tool_version.minor == min_version.minor && tool_version.patch > min_version.patch) {
                    min_version = tool_version.clone();
                }
            }
        }
        
        Ok(min_version)
    }

    /// Check if client supports all required tool versions
    pub async fn check_tool_version_compatibility(&self, tool_requirements: &[(String, String)]) -> Result<Vec<String>> {
        let mut incompatible_tools = Vec::new();
        
        for (tool_name, required_version) in tool_requirements {
            if let Some(tool_registration) = self.tool_registry.get_tool(tool_name).await {
                let tool_version = MCPVersion::from_string(&tool_registration.metadata.version)?;
                let required_version = MCPVersion::from_string(required_version)?;
                
                if !tool_version.is_backward_compatible(&required_version) {
                    incompatible_tools.push(format!("{} (has: {}, requires: {})", 
                        tool_name, tool_version.to_string(), required_version.to_string()));
                }
            } else {
                incompatible_tools.push(format!("{} (not found)", tool_name));
            }
        }
        
        Ok(incompatible_tools)
    }

    /// Get plugin manager reference
    pub fn get_plugin_manager(&self) -> &Arc<RwLock<plugin::PluginManager>> {
        &self.plugin_manager
    }

    /// Load a plugin from path
    pub async fn load_plugin(&self, path: &std::path::PathBuf) -> Result<String> {
        let plugin_manager = self.plugin_manager.read().await;
        plugin_manager.load_plugin(path).await
    }

    /// Unload a plugin
    pub async fn unload_plugin(&self, plugin_id: &str) -> Result<()> {
        let plugin_manager = self.plugin_manager.read().await;
        plugin_manager.unload_plugin(plugin_id).await
    }

    /// Initialize a plugin with MCP client context
    pub async fn initialize_plugin(&self, plugin_id: &str, config: serde_json::Value) -> Result<()> {
        let context = plugin::PluginContext {
            plugin_id: plugin_id.to_string(),
            mcp_client: Arc::new(self.clone()),
            tool_registry: self.tool_registry.clone(),
            config,
            permissions: vec!["mcp.tools.call".to_string()], // Default permissions
            resource_limits: plugin::ResourceLimits::default(),
        };

        let plugin_manager = self.plugin_manager.read().await;
        plugin_manager.initialize_plugin(plugin_id, context).await
    }

    /// Activate a plugin
    pub async fn activate_plugin(&self, plugin_id: &str) -> Result<()> {
        let plugin_manager = self.plugin_manager.read().await;
        plugin_manager.activate_plugin(plugin_id).await
    }

    /// Deactivate a plugin
    pub async fn deactivate_plugin(&self, plugin_id: &str) -> Result<()> {
        let plugin_manager = self.plugin_manager.read().await;
        plugin_manager.deactivate_plugin(plugin_id).await
    }

    /// Send message to plugin
    pub async fn send_message_to_plugin(&self, plugin_id: &str, message: &JSONRPCMessage) -> Result<Option<JSONRPCMessage>> {
        let plugin_manager = self.plugin_manager.read().await;
        plugin_manager.send_message_to_plugin(plugin_id, message).await
    }

    /// Get all plugin instances
    pub async fn get_plugin_instances(&self) -> HashMap<String, plugin::PluginInstance> {
        let plugin_manager = self.plugin_manager.read().await;
        plugin_manager.get_plugin_instances().await
    }

    /// Scan for plugins in directory
    pub async fn scan_plugins(&self, directory: &std::path::PathBuf) -> Result<Vec<plugin::PluginMetadata>> {
        let mut plugin_manager = self.plugin_manager.write().await;
        plugin_manager.add_plugin_directory(directory.clone());
        plugin_manager.scan_plugins().await
    }

    /// Broadcast event to all active plugins
    pub async fn broadcast_plugin_event(&self, event: plugin::PluginEvent) -> Result<()> {
        let plugin_manager = self.plugin_manager.read().await;
        plugin_manager.broadcast_event(event).await
    }

    /// Get plugins by state
    pub async fn get_plugins_by_state(&self, state: plugin::PluginState) -> Vec<plugin::PluginInstance> {
        let plugin_manager = self.plugin_manager.read().await;
        plugin_manager.get_plugins_by_state(state).await
    }

    /// Get server capabilities
    pub async fn get_server_capabilities(&self) -> ServerCapabilities {
        self.server_capabilities.read().await.clone()
    }

    /// Get negotiated protocol version
    pub async fn get_negotiated_protocol_version(&self) -> Option<MCPVersion> {
        self.server_capabilities.read().await.protocol_version.clone()
    }

    /// Check if a feature is enabled
    pub async fn is_feature_enabled(&self, feature_name: &str) -> bool {
        let flags = self.feature_flags.read().await;
        flags.get(feature_name).map(|f| f.enabled).unwrap_or(false)
    }

    /// Get all feature flags
    pub async fn get_feature_flags(&self) -> HashMap<String, FeatureFlag> {
        self.feature_flags.read().await.clone()
    }

    /// Enable or disable a feature flag
    pub async fn set_feature_flag(&self, feature_name: &str, enabled: bool) -> Result<()> {
        let mut flags = self.feature_flags.write().await;
        if let Some(flag) = flags.get_mut(feature_name) {
            flag.enabled = enabled;
            log::info!("Feature flag '{}' set to: {}", feature_name, enabled);
        } else {
            anyhow::bail!("Feature flag '{}' not found", feature_name);
        }
        Ok(())
    }

    /// Add a custom feature flag
    pub async fn add_feature_flag(&self, flag: FeatureFlag) -> Result<()> {
        let mut flags = self.feature_flags.write().await;
        let capabilities = self.server_capabilities.read().await;
        
        // Check version compatibility
        let mut enabled = flag.enabled;
        if let Some(server_version) = &capabilities.protocol_version {
            if let Some(min_version) = &flag.min_version {
                if !server_version.is_backward_compatible(min_version) {
                    enabled = false;
                }
            }
            if let Some(max_version) = &flag.max_version {
                if server_version.is_backward_compatible(max_version) {
                    enabled = false;
                }
            }
        }
        
        let final_flag = FeatureFlag {
            name: flag.name.clone(),
            enabled,
            min_version: flag.min_version,
            max_version: flag.max_version,
            description: flag.description,
        };
        
        flags.insert(flag.name.clone(), final_flag);
        log::info!("Added custom feature flag: {}", flag.name);
        Ok(())
    }

    /// Check if server supports a specific method
    pub async fn is_method_supported(&self, method: &str) -> bool {
        let capabilities = self.server_capabilities.read().await;
        capabilities.supported_methods.contains(&method.to_string())
    }

    /// Get supported methods
    pub async fn get_supported_methods(&self) -> Vec<String> {
        self.server_capabilities.read().await.supported_methods.clone()
    }

    /// Check if server supports experimental features
    pub async fn supports_experimental_features(&self) -> bool {
        let capabilities = self.server_capabilities.read().await;
        !capabilities.experimental_features.is_empty()
    }

    /// Get experimental features
    pub async fn get_experimental_features(&self) -> Vec<String> {
        self.server_capabilities.read().await.experimental_features.clone()
    }

    /// Check backward compatibility
    pub async fn is_backward_compatible(&self) -> bool {
        self.server_capabilities.read().await.backwards_compatibility
    }

    /// Gracefully degrade functionality for older servers
    pub async fn graceful_degrade(&self, requested_method: &str) -> Result<Option<String>> {
        let capabilities = self.server_capabilities.read().await;
        
        // Check if the method is supported
        if capabilities.supported_methods.contains(&requested_method.to_string()) {
            return Ok(Some(requested_method.to_string()));
        }
        
        // Try to find a compatible alternative method
        let alternative = match requested_method {
            "tools/list" => {
                // If tools/list is not supported, try older format
                if capabilities.supported_methods.contains(&"list_tools".to_string()) {
                    Some("list_tools".to_string())
                } else {
                    None
                }
            }
            "tools/call" => {
                // If tools/call is not supported, try older format
                if capabilities.supported_methods.contains(&"call_tool".to_string()) {
                    Some("call_tool".to_string())
                } else {
                    None
                }
            }
            "resources/list" => {
                // If resources/list is not supported, try older format
                if capabilities.supported_methods.contains(&"list_resources".to_string()) {
                    Some("list_resources".to_string())
                } else {
                    None
                }
            }
            "resources/read" => {
                // If resources/read is not supported, try older format
                if capabilities.supported_methods.contains(&"read_resource".to_string()) {
                    Some("read_resource".to_string())
                } else {
                    None
                }
            }
            "logging/setLevel" => {
                // Logging may not be supported in older versions
                None
            }
            _ => None,
        };
        
        if let Some(alt_method) = &alternative {
            log::info!("Gracefully degrading from '{}' to '{}'", requested_method, alt_method);
        } else {
            log::warn!("No compatible alternative found for method '{}'", requested_method);
        }
        
        Ok(alternative)
    }

    /// Call a method with graceful degradation
    pub async fn call_method_with_degradation(&self, method: &str, params: Option<serde_json::Value>) -> Result<serde_json::Value> {
        // Check if we can call the method directly
        if self.is_method_supported(method).await {
            return self.call_method(method, params).await;
        }
        
        // Try to find a compatible alternative
        if let Some(alternative_method) = self.graceful_degrade(method).await? {
            return self.call_method(&alternative_method, params).await;
        }
        
        anyhow::bail!("Method '{}' is not supported by server and no compatible alternative found", method);
    }

    /// Call a method directly
    async fn call_method(&self, method: &str, params: Option<serde_json::Value>) -> Result<serde_json::Value> {
        let request = JSONRPCMessage::Request {
            jsonrpc: "2.0".to_string(),
            id: serde_json::Value::String(Uuid::new_v4().to_string()),
            method: method.to_string(),
            params,
        };
        
        self.send_request(request).await
    }

    /// Refresh server capabilities
    pub async fn refresh_server_capabilities(&self) -> Result<()> {
        // Try to re-negotiate protocol version
        let _version = self.negotiate_protocol_version().await?;
        
        // Send a new initialize request to get updated capabilities
        let params = InitializeParams {
            protocol_version: self.config.protocol_version.to_string(),
            capabilities: ClientCapabilities {
                roots: Some(RootsCapability {
                    list_changed: true,
                }),
                sampling: None,
            },
            client_info: self.config.client_info.clone(),
        };

        let request = JSONRPCMessage::Request {
            jsonrpc: "2.0".to_string(),
            id: serde_json::Value::String(Uuid::new_v4().to_string()),
            method: "initialize".to_string(),
            params: Some(serde_json::to_value(params)?),
        };

        let response = self.send_request(request).await?;
        
        // Parse updated capabilities
        self.parse_server_capabilities(&response).await?;
        
        // Re-initialize feature flags
        self.initialize_feature_flags().await?;
        
        log::info!("Server capabilities refreshed");
        Ok(())
    }

    /// Call a tool with graceful degradation
    pub async fn call_tool(&self, name: &str, arguments: Option<serde_json::Value>) -> Result<serde_json::Value> {
        let start_time = std::time::Instant::now();
        
        let params = CallToolParams {
            name: name.to_string(),
            arguments,
        };

        let result = self.call_method_with_degradation("tools/call", Some(serde_json::to_value(params)?)).await;
        let response_time_ms = start_time.elapsed().as_millis() as u64;
        
        // Update tool status in registry
        match &result {
            Ok(_) => {
                let _ = self.tool_registry.update_tool_status(name, true, Some(response_time_ms)).await;
            }
            Err(_) => {
                let _ = self.tool_registry.update_tool_status(name, false, Some(response_time_ms)).await;
            }
        }

        result
    }

    /// List available resources with graceful degradation
    pub async fn list_resources(&self) -> Result<serde_json::Value> {
        self.call_method_with_degradation("resources/list", None).await
    }

    /// Read a resource with graceful degradation
    pub async fn read_resource(&self, uri: &str) -> Result<serde_json::Value> {
        let params = ReadResourceParams {
            uri: uri.to_string(),
        };

        self.call_method_with_degradation("resources/read", Some(serde_json::to_value(params)?)).await
    }

    /// Send a ping to the server
    pub async fn ping(&self) -> Result<serde_json::Value> {
        let request = JSONRPCMessage::Request {
            jsonrpc: "2.0".to_string(),
            id: serde_json::Value::String(Uuid::new_v4().to_string()),
            method: "ping".to_string(),
            params: None,
        };

        self.send_request(request).await
    }

    /// Set the logging level with graceful degradation
    pub async fn set_logging_level(&self, level: LoggingLevel) -> Result<()> {
        let params = SetLoggingLevelParams { level };

        // Check if logging is supported before attempting
        if !self.is_feature_enabled("logging.setLevel").await {
            log::warn!("Logging level setting not supported by server");
            return Ok(()); // Gracefully ignore if not supported
        }

        self.call_method_with_degradation("logging/setLevel", Some(serde_json::to_value(params)?)).await?;
        Ok(())
    }

    /// Disconnect from the server
    pub async fn disconnect(&mut self) -> Result<()> {
        let mut state = self.state.write().await;
        *state = MCPConnectionState::Disconnected;

        if let Some(child_arc) = self.child_process.take() {
            let mut child_guard = child_arc.write().await;
            if let Some(mut child) = child_guard.take() {
                child.kill().await.context("Failed to kill child process")?;
            }
        }

        Ok(())
    }
}

impl Drop for MCPClient {
    fn drop(&mut self) {
        if let Some(child_arc) = self.child_process.take() {
            tokio::spawn(async move {
                let mut child_guard = child_arc.write().await;
                if let Some(mut child) = child_guard.take() {
                    let _ = child.kill().await;
                }
            });
        }
    }
}

/// Utility functions for MCP message handling
pub mod utils {
    use super::*;

    /// Create a JSON-RPC request message
    pub fn create_request(id: impl Into<serde_json::Value>, method: &str, params: Option<serde_json::Value>) -> JSONRPCMessage {
        JSONRPCMessage::Request {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            method: method.to_string(),
            params,
        }
    }

    /// Create a JSON-RPC response message
    pub fn create_response(id: impl Into<serde_json::Value>, result: Option<serde_json::Value>) -> JSONRPCMessage {
        JSONRPCMessage::Response {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            result,
            error: None,
        }
    }

    /// Create a JSON-RPC error response
    pub fn create_error_response(id: impl Into<serde_json::Value>, code: i32, message: &str) -> JSONRPCMessage {
        JSONRPCMessage::Response {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            result: None,
            error: Some(JSONRPCError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }

    /// Create a JSON-RPC notification message
    pub fn create_notification(method: &str, params: Option<serde_json::Value>) -> JSONRPCMessage {
        JSONRPCMessage::Notification {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
        }
    }

    /// Parse a JSON string into an MCP message
    pub fn parse_message(json: &str) -> Result<JSONRPCMessage> {
        serde_json::from_str(json).context("Failed to parse JSON-RPC message")
    }

    /// Serialize an MCP message to JSON string
    pub fn serialize_message(message: &JSONRPCMessage) -> Result<String> {
        serde_json::to_string(message).context("Failed to serialize JSON-RPC message")
    }

    /// Validate message format
    pub fn validate_message(message: &JSONRPCMessage) -> Result<()> {
        match message {
            JSONRPCMessage::Request { jsonrpc, id, method, .. } => {
                if jsonrpc != "2.0" {
                    anyhow::bail!("Invalid JSON-RPC version: {}", jsonrpc);
                }
                if method.is_empty() {
                    anyhow::bail!("Request method cannot be empty");
                }
                if id.is_null() {
                    anyhow::bail!("Request ID cannot be null");
                }
            }
            JSONRPCMessage::Response { jsonrpc, id, result, error } => {
                if jsonrpc != "2.0" {
                    anyhow::bail!("Invalid JSON-RPC version: {}", jsonrpc);
                }
                if id.is_null() {
                    anyhow::bail!("Response ID cannot be null");
                }
                if result.is_none() && error.is_none() {
                    anyhow::bail!("Response must have either result or error");
                }
                if result.is_some() && error.is_some() {
                    anyhow::bail!("Response cannot have both result and error");
                }
            }
            JSONRPCMessage::Notification { jsonrpc, method, .. } => {
                if jsonrpc != "2.0" {
                    anyhow::bail!("Invalid JSON-RPC version: {}", jsonrpc);
                }
                if method.is_empty() {
                    anyhow::bail!("Notification method cannot be empty");
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_version_compatibility() {
        let v1 = MCPVersion::new(2024, 11, 5);
        let v2 = MCPVersion::new(2024, 11, 6);
        let v3 = MCPVersion::new(2024, 12, 1);
        let v4 = MCPVersion::new(2025, 1, 1);

        assert!(v1.is_compatible(&v2));
        assert!(v1.is_compatible(&v3));
        assert!(!v1.is_compatible(&v4));
    }

    #[test]
    fn test_message_serialization() {
        let request = JSONRPCMessage::Request {
            jsonrpc: "2.0".to_string(),
            id: serde_json::Value::String("test-id".to_string()),
            method: "test/method".to_string(),
            params: Some(serde_json::json!({"test": "value"})),
        };

        let json = utils::serialize_message(&request).unwrap();
        let parsed = utils::parse_message(&json).unwrap();

        match parsed {
            JSONRPCMessage::Request { jsonrpc, id, method, params } => {
                assert_eq!(jsonrpc, "2.0");
                assert_eq!(id, serde_json::Value::String("test-id".to_string()));
                assert_eq!(method, "test/method");
                assert_eq!(params, Some(serde_json::json!({"test": "value"})));
            }
            _ => panic!("Expected request message"),
        }
    }

    #[test]
    fn test_message_validation() {
        let valid_request = JSONRPCMessage::Request {
            jsonrpc: "2.0".to_string(),
            id: serde_json::Value::String("test-id".to_string()),
            method: "test/method".to_string(),
            params: None,
        };

        let invalid_request = JSONRPCMessage::Request {
            jsonrpc: "2.0".to_string(),
            id: serde_json::Value::String("test-id".to_string()),
            method: "".to_string(),
            params: None,
        };

        assert!(utils::validate_message(&valid_request).is_ok());
        assert!(utils::validate_message(&invalid_request).is_err());
    }

    #[tokio::test]
    async fn test_client_creation() {
        let config = MCPClientConfig::default();
        let client = MCPClient::new(config);
        
        assert_eq!(client.get_state().await, MCPConnectionState::Disconnected);
    }

    #[tokio::test]
    async fn test_tool_registry_creation() {
        let registry = ToolRegistry::new();
        let tools = registry.get_tools().await;
        assert!(tools.is_empty());
    }

    #[tokio::test]
    async fn test_tool_registration() {
        let registry = ToolRegistry::new();
        let metadata = ToolMetadata {
            name: "test_tool".to_string(),
            version: "1.0.0".to_string(),
            description: Some("A test tool".to_string()),
            schema: None,
            capabilities: vec!["execute".to_string()],
            compatibility_version: MCPVersion::current(),
            last_updated: None,
            provider: Some("test".to_string()),
        };

        registry.register_tool(metadata.clone()).await.unwrap();
        
        let tools = registry.get_tools().await;
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].metadata.name, "test_tool");
        assert!(tools[0].is_available);
    }

    #[tokio::test]
    async fn test_tool_capability_filtering() {
        let registry = ToolRegistry::new();
        
        let metadata1 = ToolMetadata {
            name: "tool1".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            schema: None,
            capabilities: vec!["execute".to_string(), "parameterized".to_string()],
            compatibility_version: MCPVersion::current(),
            last_updated: None,
            provider: None,
        };
        
        let metadata2 = ToolMetadata {
            name: "tool2".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            schema: None,
            capabilities: vec!["execute".to_string()],
            compatibility_version: MCPVersion::current(),
            last_updated: None,
            provider: None,
        };

        registry.register_tool(metadata1).await.unwrap();
        registry.register_tool(metadata2).await.unwrap();
        
        let parameterized_tools = registry.get_tools_by_capability("parameterized").await;
        assert_eq!(parameterized_tools.len(), 1);
        assert_eq!(parameterized_tools[0].metadata.name, "tool1");
        
        let execute_tools = registry.get_tools_by_capability("execute").await;
        assert_eq!(execute_tools.len(), 2);
    }

    #[tokio::test]
    async fn test_version_compatibility() {
        let v1 = MCPVersion::new(2024, 11, 5);
        let v2 = MCPVersion::new(2024, 11, 6);
        let v3 = MCPVersion::new(2024, 12, 1);

        assert!(v1.is_backward_compatible(&v1));
        assert!(v2.is_backward_compatible(&v1));
        assert!(v3.is_backward_compatible(&v1));
        assert!(!v1.is_backward_compatible(&v2));
    }

    #[tokio::test]
    async fn test_version_parsing() {
        let version_str = "2024-11-05";
        let version = MCPVersion::from_string(version_str).unwrap();
        
        assert_eq!(version.major, 2024);
        assert_eq!(version.minor, 11);
        assert_eq!(version.patch, 5);
        assert_eq!(version.to_string(), "2024-11-05");
    }

    #[tokio::test]
    async fn test_registry_statistics() {
        let registry = ToolRegistry::new();
        
        let metadata = ToolMetadata {
            name: "test_tool".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            schema: None,
            capabilities: vec!["execute".to_string()],
            compatibility_version: MCPVersion::current(),
            last_updated: None,
            provider: None,
        };
        
        registry.register_tool(metadata).await.unwrap();
        registry.update_tool_status("test_tool", true, Some(100)).await.unwrap();
        
        let stats = registry.get_statistics().await;
        assert_eq!(stats.get("total_tools").unwrap().as_u64().unwrap(), 1);
        assert_eq!(stats.get("available_tools").unwrap().as_u64().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_plugin_manager_creation() {
        let manager = plugin::PluginManager::new();
        let instances = manager.get_plugin_instances().await;
        assert!(instances.is_empty());
    }

    #[tokio::test]
    async fn test_plugin_metadata_serialization() {
        let metadata = plugin::PluginMetadata {
            id: "test-plugin".to_string(),
            name: "Test Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: Some("A test plugin".to_string()),
            author: Some("Test Author".to_string()),
            plugin_type: plugin::PluginType::External,
            mcp_version: MCPVersion::current(),
            entry_point: "plugin.exe".to_string(),
            dependencies: vec![],
            permissions: vec!["mcp.tools.call".to_string()],
            configuration: None,
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let parsed: plugin::PluginMetadata = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.id, "test-plugin");
        assert_eq!(parsed.name, "Test Plugin");
        assert_eq!(parsed.plugin_type, plugin::PluginType::External);
    }

    #[tokio::test]
    async fn test_plugin_state_transitions() {
        let manager = plugin::PluginManager::new();
        
        // Test getting plugins by state
        let unloaded_plugins = manager.get_plugins_by_state(plugin::PluginState::Unloaded).await;
        assert!(unloaded_plugins.is_empty());
        
        let active_plugins = manager.get_plugins_by_state(plugin::PluginState::Active).await;
        assert!(active_plugins.is_empty());
    }

    #[tokio::test]
    async fn test_plugin_security_manager() {
        let security_manager = plugin::PluginSecurityManager::new();
        
        let valid_metadata = plugin::PluginMetadata {
            id: "test-plugin".to_string(),
            name: "Test Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            author: None,
            plugin_type: plugin::PluginType::External,
            mcp_version: MCPVersion::current(),
            entry_point: "plugin.exe".to_string(),
            dependencies: vec![],
            permissions: vec!["mcp.tools.call".to_string()], // Valid permission
            configuration: None,
        };
        
        let invalid_metadata = plugin::PluginMetadata {
            id: "bad-plugin".to_string(),
            name: "Bad Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            author: None,
            plugin_type: plugin::PluginType::External,
            mcp_version: MCPVersion::current(),
            entry_point: "plugin.exe".to_string(),
            dependencies: vec![],
            permissions: vec!["dangerous.permission".to_string()], // Invalid permission
            configuration: None,
        };
        
        let path = std::path::PathBuf::from("/test/path");
        
        // Valid plugin should pass validation
        assert!(security_manager.validate_plugin(&valid_metadata, &path).await.is_ok());
        
        // Invalid plugin should fail validation
        assert!(security_manager.validate_plugin(&invalid_metadata, &path).await.is_err());
    }

    #[tokio::test]
    async fn test_plugin_resource_usage() {
        let usage = plugin::ResourceUsage {
            memory_mb: 50.0,
            cpu_percent: 5.0,
            network_bytes: 1024,
            disk_bytes: 2048,
        };
        
        let security_manager = plugin::PluginSecurityManager::new();
        
        // Usage within limits should pass
        assert!(security_manager.check_resource_limits(&usage).is_ok());
        
        let excessive_usage = plugin::ResourceUsage {
            memory_mb: 200.0, // Exceeds default limit of 100MB
            cpu_percent: 5.0,
            network_bytes: 1024,
            disk_bytes: 2048,
        };
        
        // Excessive usage should fail
        assert!(security_manager.check_resource_limits(&excessive_usage).is_err());
    }

    #[tokio::test]
    async fn test_plugin_event_serialization() {
        let event = plugin::PluginEvent::ToolRegistered {
            tool_name: "test_tool".to_string(),
        };
        
        let json = serde_json::to_string(&event).unwrap();
        let parsed: plugin::PluginEvent = serde_json::from_str(&json).unwrap();
        
        match parsed {
            plugin::PluginEvent::ToolRegistered { tool_name } => {
                assert_eq!(tool_name, "test_tool");
            }
            _ => panic!("Expected ToolRegistered event"),
        }
    }

    #[tokio::test]
    async fn test_mcp_client_with_plugins() {
        let config = MCPClientConfig::default();
        let client = MCPClient::new(config);
        
        // Test plugin manager integration
        let instances = client.get_plugin_instances().await;
        assert!(instances.is_empty());
        
        // Test plugin state querying
        let active_plugins = client.get_plugins_by_state(plugin::PluginState::Active).await;
        assert!(active_plugins.is_empty());
    }

    #[tokio::test]
    async fn test_client_config_default() {
        let config = MCPClientConfig::default();
        
        assert_eq!(config.client_info.name, "arrowhead");
        assert_eq!(config.client_info.version, "0.1.0");
        assert_eq!(config.timeout_ms, 30000);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_logging_level_serialization() {
        let level = LoggingLevel::Info;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, r#""info""#);

        let parsed: LoggingLevel = serde_json::from_str(&json).unwrap();
        matches!(parsed, LoggingLevel::Info);
    }

    #[test]
    fn test_initialize_params_serialization() {
        let params = InitializeParams {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ClientCapabilities {
                roots: Some(RootsCapability {
                    list_changed: true,
                }),
                sampling: None,
            },
            client_info: ClientInfo {
                name: "test".to_string(),
                version: "1.0.0".to_string(),
            },
        };

        let json = serde_json::to_string(&params).unwrap();
        let parsed: InitializeParams = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.protocol_version, "2024-11-05");
        assert_eq!(parsed.client_info.name, "test");
        assert_eq!(parsed.client_info.version, "1.0.0");
    }

    #[test]
    fn test_call_tool_params_serialization() {
        let params = CallToolParams {
            name: "test_tool".to_string(),
            arguments: Some(serde_json::json!({"arg1": "value1", "arg2": 42})),
        };

        let json = serde_json::to_string(&params).unwrap();
        let parsed: CallToolParams = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.name, "test_tool");
        assert_eq!(parsed.arguments, Some(serde_json::json!({"arg1": "value1", "arg2": 42})));
    }

    #[tokio::test]
    async fn test_server_capabilities_default() {
        let capabilities = ServerCapabilities::default();
        
        assert!(capabilities.protocol_version.is_none());
        assert!(capabilities.supported_methods.is_empty());
        assert!(capabilities.experimental_features.is_empty());
        assert!(!capabilities.backwards_compatibility);
        assert!(capabilities.feature_flags.is_empty());
    }

    #[tokio::test]
    async fn test_feature_flag_creation() {
        let flag = FeatureFlag {
            name: "test.feature".to_string(),
            enabled: true,
            min_version: Some(MCPVersion::new(2024, 10, 1)),
            max_version: None,
            description: Some("Test feature".to_string()),
        };
        
        assert_eq!(flag.name, "test.feature");
        assert!(flag.enabled);
        assert!(flag.min_version.is_some());
        assert!(flag.max_version.is_none());
    }

    #[tokio::test]
    async fn test_client_with_capabilities() {
        let config = MCPClientConfig::default();
        let client = MCPClient::new(config);
        
        // Test initial state
        let capabilities = client.get_server_capabilities().await;
        assert!(capabilities.protocol_version.is_none());
        
        let flags = client.get_feature_flags().await;
        assert!(flags.is_empty());
        
        // Test feature flag checking
        assert!(!client.is_feature_enabled("nonexistent.feature").await);
    }

    #[tokio::test]
    async fn test_feature_flag_management() {
        let config = MCPClientConfig::default();
        let client = MCPClient::new(config);
        
        // Add a custom feature flag
        let flag = FeatureFlag {
            name: "custom.feature".to_string(),
            enabled: true,
            min_version: Some(MCPVersion::new(2024, 10, 1)),
            max_version: None,
            description: Some("Custom test feature".to_string()),
        };
        
        client.add_feature_flag(flag).await.unwrap();
        
        // Check if the flag was added
        assert!(client.is_feature_enabled("custom.feature").await);
        
        // Test setting flag state
        client.set_feature_flag("custom.feature", false).await.unwrap();
        assert!(!client.is_feature_enabled("custom.feature").await);
        
        // Test setting non-existent flag
        assert!(client.set_feature_flag("nonexistent.feature", true).await.is_err());
    }

    #[tokio::test]
    async fn test_version_negotiation_logic() {
        let client_version = MCPVersion::new(2024, 11, 5);
        let server_version = MCPVersion::new(2024, 11, 1);
        
        // Test compatibility checking
        assert!(client_version.is_compatible(&server_version));
        assert!(client_version.is_backward_compatible(&server_version));
        assert!(!server_version.is_backward_compatible(&client_version));
        
        // Test version parsing
        let parsed_version = MCPVersion::from_string("2024-11-05").unwrap();
        assert_eq!(parsed_version.major, 2024);
        assert_eq!(parsed_version.minor, 11);
        assert_eq!(parsed_version.patch, 5);
    }

    #[tokio::test]
    async fn test_graceful_degradation() {
        let config = MCPClientConfig::default();
        let client = MCPClient::new(config);
        
        // Test graceful degradation logic
        let degradation_result = client.graceful_degrade("tools/list").await.unwrap();
        
        // Should return the original method if supported methods is empty
        // or return an alternative if available
        assert!(degradation_result.is_some());
    }

    #[tokio::test]
    async fn test_method_support_checking() {
        let config = MCPClientConfig::default();
        let client = MCPClient::new(config);
        
        // Initially, no methods are supported
        assert!(!client.is_method_supported("tools/list").await);
        assert!(!client.is_method_supported("resources/read").await);
        
        let supported_methods = client.get_supported_methods().await;
        assert!(supported_methods.is_empty());
    }

    #[tokio::test]
    async fn test_experimental_features() {
        let config = MCPClientConfig::default();
        let client = MCPClient::new(config);
        
        // Initially, no experimental features
        assert!(!client.supports_experimental_features().await);
        
        let experimental_features = client.get_experimental_features().await;
        assert!(experimental_features.is_empty());
    }

    #[tokio::test]
    async fn test_backward_compatibility_flag() {
        let config = MCPClientConfig::default();
        let client = MCPClient::new(config);
        
        // Initially, backward compatibility is false
        assert!(!client.is_backward_compatible().await);
    }

    #[test]
    fn test_mcp_version_ordering() {
        let v1 = MCPVersion::new(2024, 10, 1);
        let v2 = MCPVersion::new(2024, 10, 15);
        let v3 = MCPVersion::new(2024, 11, 1);
        let v4 = MCPVersion::new(2024, 11, 5);
        
        // Test version progression
        assert!(v4.is_backward_compatible(&v3));
        assert!(v3.is_backward_compatible(&v2));
        assert!(v2.is_backward_compatible(&v1));
        
        // Test cross-version compatibility
        assert!(v4.is_backward_compatible(&v1));
        assert!(!v1.is_backward_compatible(&v4));
    }

    #[test]
    fn test_server_capabilities_serialization() {
        let mut capabilities = ServerCapabilities::default();
        capabilities.protocol_version = Some(MCPVersion::new(2024, 11, 5));
        capabilities.supported_methods = vec!["tools/list".to_string(), "tools/call".to_string()];
        capabilities.experimental_features = vec!["streaming".to_string()];
        capabilities.backwards_compatibility = true;
        capabilities.feature_flags.insert("advanced.mode".to_string(), true);
        
        // Test that all fields are properly set
        assert!(capabilities.protocol_version.is_some());
        assert_eq!(capabilities.supported_methods.len(), 2);
        assert_eq!(capabilities.experimental_features.len(), 1);
        assert!(capabilities.backwards_compatibility);
        assert_eq!(capabilities.feature_flags.len(), 1);
    }
}