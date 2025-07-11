/// # MCP Client API
///
/// This module provides a simplified, user-friendly interface to the MCP (Model Context Protocol) client.
/// It abstracts the complexity of the underlying MCP protocol implementation and provides clean,
/// intuitive methods for common operations.
///
/// ## Quick Start
///
/// ```rust
/// use arrowhead::mcp_api::{MCPClientBuilder, MCPError};
/// use std::collections::HashMap;
///
/// #[tokio::main]
/// async fn main() -> Result<(), MCPError> {
///     // Create a new MCP client
///     let client = MCPClientBuilder::new()
///         .with_stdio_transport()
///         .with_timeout(30)
///         .build()?;
///
///     // Connect and initialize
///     client.connect().await?;
///     
///     // List available tools
///     let tools = client.list_tools().await?;
///     println!("Available tools: {:?}", tools);
///
///     // Call a tool
///     let mut args = HashMap::new();
///     args.insert("query", "Hello world");
///     let result = client.call_tool("echo", args).await?;
///     println!("Tool result: {:?}", result);
///
///     Ok(())
/// }
/// ```

use crate::mcp_client::{MCPClient, MCPClientConfig, MCPTransport, MCPVersion, ServerCapabilities, FeatureFlag};
use anyhow::Result;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::PathBuf;

/// Simplified error type for MCP API operations
#[derive(Debug, thiserror::Error)]
pub enum MCPError {
    #[error("Connection error: {0}")]
    Connection(String),
    
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    #[error("Tool error: {0}")]
    Tool(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Timeout error: {0}")]
    Timeout(String),
    
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

/// Builder for creating MCP clients with a fluent interface
pub struct MCPClientBuilder {
    transport: Option<MCPTransport>,
    timeout_seconds: Option<u64>,
    max_retries: Option<u32>,
    client_name: Option<String>,
    client_version: Option<String>,
    protocol_version: Option<MCPVersion>,
}

impl MCPClientBuilder {
    /// Create a new builder instance
    pub fn new() -> Self {
        Self {
            transport: None,
            timeout_seconds: None,
            max_retries: None,
            client_name: None,
            client_version: None,
            protocol_version: None,
        }
    }

    /// Configure the client to use stdio transport
    pub fn with_stdio_transport(mut self) -> Self {
        self.transport = Some(MCPTransport::StdIO);
        self
    }

    /// Configure the client to use TCP transport
    pub fn with_tcp_transport(mut self, host: &str, port: u16) -> Self {
        self.transport = Some(MCPTransport::TCP {
            host: host.to_string(),
            port,
        });
        self
    }

    /// Configure the client to use WebSocket transport
    pub fn with_websocket_transport(mut self, url: &str) -> Self {
        self.transport = Some(MCPTransport::WebSocket {
            url: url.to_string(),
        });
        self
    }

    /// Configure the client to use process transport
    pub fn with_process_transport(mut self, command: &str, args: Vec<String>) -> Self {
        self.transport = Some(MCPTransport::Process {
            command: command.to_string(),
            args,
        });
        self
    }

    /// Set the timeout for operations in seconds
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = Some(timeout_seconds);
        self
    }

    /// Set the maximum number of retries for failed operations
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = Some(max_retries);
        self
    }

    /// Set the client name and version
    pub fn with_client_info(mut self, name: &str, version: &str) -> Self {
        self.client_name = Some(name.to_string());
        self.client_version = Some(version.to_string());
        self
    }

    /// Set the protocol version to use
    pub fn with_protocol_version(mut self, version: MCPVersion) -> Self {
        self.protocol_version = Some(version);
        self
    }

    /// Build the MCP client with the configured options
    pub fn build(self) -> Result<MCPClientApi, MCPError> {
        let transport = self.transport.unwrap_or(MCPTransport::StdIO);
        let timeout_ms = self.timeout_seconds.unwrap_or(30) * 1000;
        let max_retries = self.max_retries.unwrap_or(3);
        let client_name = self.client_name.unwrap_or_else(|| "arrowhead".to_string());
        let client_version = self.client_version.unwrap_or_else(|| "0.1.0".to_string());
        let protocol_version = self.protocol_version.unwrap_or_else(MCPVersion::current);

        let config = MCPClientConfig {
            transport,
            protocol_version,
            client_info: crate::mcp_client::ClientInfo {
                name: client_name,
                version: client_version,
            },
            timeout_ms,
            max_retries,
        };

        let client = MCPClient::new(config);
        
        Ok(MCPClientApi {
            client,
            connected: false,
        })
    }
}

impl Default for MCPClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// High-level MCP client API
pub struct MCPClientApi {
    client: MCPClient,
    connected: bool,
}

impl MCPClientApi {
    /// Create a new MCP client with default configuration
    pub fn new() -> Result<Self, MCPError> {
        MCPClientBuilder::new().build()
    }

    /// Create a new MCP client with custom configuration
    pub fn with_config(config: MCPClientConfig) -> Self {
        Self {
            client: MCPClient::new(config),
            connected: false,
        }
    }

    /// Connect to the MCP server
    pub async fn connect(&mut self) -> Result<(), MCPError> {
        self.client.connect().await
            .map_err(|e| MCPError::Connection(e.to_string()))?;
        
        self.client.initialize().await
            .map_err(|e| MCPError::Protocol(e.to_string()))?;
        
        self.connected = true;
        Ok(())
    }

    /// Check if the client is connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Disconnect from the MCP server
    pub async fn disconnect(&mut self) -> Result<(), MCPError> {
        self.client.disconnect().await
            .map_err(|e| MCPError::Connection(e.to_string()))?;
        
        self.connected = false;
        Ok(())
    }

    /// List all available tools
    pub async fn list_tools(&self) -> Result<Vec<ToolInfo>, MCPError> {
        self.ensure_connected()?;
        
        let response = self.client.list_tools().await
            .map_err(|e| MCPError::Tool(e.to_string()))?;
        
        let tools = self.parse_tools_response(&response)?;
        Ok(tools)
    }

    /// Call a tool with the given name and arguments
    pub async fn call_tool<T>(&self, name: &str, arguments: T) -> Result<Value, MCPError>
    where
        T: Into<ToolArguments>,
    {
        self.ensure_connected()?;
        
        let args = arguments.into().to_json();
        let response = self.client.call_tool(name, args).await
            .map_err(|e| MCPError::Tool(e.to_string()))?;
        
        Ok(response)
    }

    /// List all available resources
    pub async fn list_resources(&self) -> Result<Vec<ResourceInfo>, MCPError> {
        self.ensure_connected()?;
        
        let response = self.client.list_resources().await
            .map_err(|e| MCPError::Tool(e.to_string()))?;
        
        let resources = self.parse_resources_response(&response)?;
        Ok(resources)
    }

    /// Read a resource by URI
    pub async fn read_resource(&self, uri: &str) -> Result<ResourceContent, MCPError> {
        self.ensure_connected()?;
        
        let response = self.client.read_resource(uri).await
            .map_err(|e| MCPError::Tool(e.to_string()))?;
        
        let content = self.parse_resource_content(&response)?;
        Ok(content)
    }

    /// Send a ping to the server
    pub async fn ping(&self) -> Result<(), MCPError> {
        self.ensure_connected()?;
        
        self.client.ping().await
            .map_err(|e| MCPError::Protocol(e.to_string()))?;
        
        Ok(())
    }

    /// Get server capabilities
    pub async fn get_server_capabilities(&self) -> Result<ServerCapabilities, MCPError> {
        self.ensure_connected()?;
        
        let capabilities = self.client.get_server_capabilities().await;
        Ok(capabilities)
    }

    /// Get negotiated protocol version
    pub async fn get_protocol_version(&self) -> Result<Option<MCPVersion>, MCPError> {
        self.ensure_connected()?;
        
        let version = self.client.get_negotiated_protocol_version().await;
        Ok(version)
    }

    /// Check if a feature is enabled
    pub async fn is_feature_enabled(&self, feature: &str) -> Result<bool, MCPError> {
        self.ensure_connected()?;
        
        let enabled = self.client.is_feature_enabled(feature).await;
        Ok(enabled)
    }

    /// Get all feature flags
    pub async fn get_feature_flags(&self) -> Result<HashMap<String, FeatureFlag>, MCPError> {
        self.ensure_connected()?;
        
        let flags = self.client.get_feature_flags().await;
        Ok(flags)
    }

    /// Set a feature flag
    pub async fn set_feature_flag(&self, feature: &str, enabled: bool) -> Result<(), MCPError> {
        self.ensure_connected()?;
        
        self.client.set_feature_flag(feature, enabled).await
            .map_err(|e| MCPError::Configuration(e.to_string()))?;
        
        Ok(())
    }

    /// Load a plugin from the given path
    pub async fn load_plugin(&self, path: &PathBuf) -> Result<String, MCPError> {
        self.ensure_connected()?;
        
        let plugin_id = self.client.load_plugin(path).await
            .map_err(|e| MCPError::Configuration(e.to_string()))?;
        
        Ok(plugin_id)
    }

    /// Unload a plugin
    pub async fn unload_plugin(&self, plugin_id: &str) -> Result<(), MCPError> {
        self.ensure_connected()?;
        
        self.client.unload_plugin(plugin_id).await
            .map_err(|e| MCPError::Configuration(e.to_string()))?;
        
        Ok(())
    }

    /// Get tool registry statistics
    pub async fn get_tool_statistics(&self) -> Result<HashMap<String, Value>, MCPError> {
        self.ensure_connected()?;
        
        let stats = self.client.get_registry_statistics().await;
        Ok(stats)
    }

    /// Refresh tool registry
    pub async fn refresh_tools(&self) -> Result<(), MCPError> {
        self.ensure_connected()?;
        
        self.client.refresh_tool_registry().await
            .map_err(|e| MCPError::Tool(e.to_string()))?;
        
        Ok(())
    }

    /// Check if a tool is available
    pub async fn is_tool_available(&self, tool_name: &str) -> Result<bool, MCPError> {
        self.ensure_connected()?;
        
        let available = self.client.is_tool_available(tool_name).await;
        Ok(available)
    }

    /// Get tool metadata
    pub async fn get_tool_metadata(&self, tool_name: &str) -> Result<Option<ToolMetadata>, MCPError> {
        self.ensure_connected()?;
        
        let metadata = self.client.get_tool_metadata(tool_name).await;
        Ok(metadata.map(|m| ToolMetadata::from_internal(m)))
    }

    /// Private helper methods

    fn ensure_connected(&self) -> Result<(), MCPError> {
        if !self.connected {
            return Err(MCPError::Connection("Not connected to MCP server".to_string()));
        }
        Ok(())
    }

    fn parse_tools_response(&self, response: &Value) -> Result<Vec<ToolInfo>, MCPError> {
        let tools = response.get("tools")
            .and_then(|t| t.as_array())
            .ok_or_else(|| MCPError::Protocol("Invalid tools response format".to_string()))?;
        
        let mut result = Vec::new();
        for tool in tools {
            let name = tool.get("name")
                .and_then(|n| n.as_str())
                .ok_or_else(|| MCPError::Protocol("Tool missing name".to_string()))?
                .to_string();
            
            let description = tool.get("description")
                .and_then(|d| d.as_str())
                .map(|s| s.to_string());
            
            let schema = tool.get("inputSchema").cloned();
            
            result.push(ToolInfo {
                name,
                description,
                schema,
            });
        }
        
        Ok(result)
    }

    fn parse_resources_response(&self, response: &Value) -> Result<Vec<ResourceInfo>, MCPError> {
        let resources = response.get("resources")
            .and_then(|r| r.as_array())
            .ok_or_else(|| MCPError::Protocol("Invalid resources response format".to_string()))?;
        
        let mut result = Vec::new();
        for resource in resources {
            let uri = resource.get("uri")
                .and_then(|u| u.as_str())
                .ok_or_else(|| MCPError::Protocol("Resource missing URI".to_string()))?
                .to_string();
            
            let name = resource.get("name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string());
            
            let description = resource.get("description")
                .and_then(|d| d.as_str())
                .map(|s| s.to_string());
            
            let mime_type = resource.get("mimeType")
                .and_then(|m| m.as_str())
                .map(|s| s.to_string());
            
            result.push(ResourceInfo {
                uri,
                name,
                description,
                mime_type,
            });
        }
        
        Ok(result)
    }

    fn parse_resource_content(&self, response: &Value) -> Result<ResourceContent, MCPError> {
        let contents = response.get("contents")
            .and_then(|c| c.as_array())
            .ok_or_else(|| MCPError::Protocol("Invalid resource content format".to_string()))?;
        
        if contents.is_empty() {
            return Err(MCPError::Protocol("Empty resource content".to_string()));
        }
        
        let content = &contents[0];
        let uri = content.get("uri")
            .and_then(|u| u.as_str())
            .ok_or_else(|| MCPError::Protocol("Content missing URI".to_string()))?
            .to_string();
        
        let mime_type = content.get("mimeType")
            .and_then(|m| m.as_str())
            .map(|s| s.to_string());
        
        let text = content.get("text")
            .and_then(|t| t.as_str())
            .map(|s| s.to_string());
        
        let blob = content.get("blob")
            .and_then(|b| b.as_str())
            .map(|s| s.to_string());
        
        Ok(ResourceContent {
            uri,
            mime_type,
            text,
            blob,
        })
    }
}

/// Information about a tool
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: Option<String>,
    pub schema: Option<Value>,
}

/// Information about a resource
#[derive(Debug, Clone)]
pub struct ResourceInfo {
    pub uri: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

/// Content of a resource
#[derive(Debug, Clone)]
pub struct ResourceContent {
    pub uri: String,
    pub mime_type: Option<String>,
    pub text: Option<String>,
    pub blob: Option<String>,
}

/// Tool metadata
#[derive(Debug, Clone)]
pub struct ToolMetadata {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub schema: Option<Value>,
    pub capabilities: Vec<String>,
    pub provider: Option<String>,
}

impl ToolMetadata {
    fn from_internal(metadata: crate::mcp_client::ToolMetadata) -> Self {
        Self {
            name: metadata.name,
            version: metadata.version,
            description: metadata.description,
            schema: metadata.schema,
            capabilities: metadata.capabilities,
            provider: metadata.provider,
        }
    }
}

/// Extension trait for JSONRPCMessage to get ID
trait JSONRPCMessageExt {
    fn id(&self) -> serde_json::Value;
}

impl JSONRPCMessageExt for crate::mcp_client::JSONRPCMessage {
    fn id(&self) -> serde_json::Value {
        match self {
            crate::mcp_client::JSONRPCMessage::Request { id, .. } => id.clone(),
            crate::mcp_client::JSONRPCMessage::Response { id, .. } => id.clone(),
            crate::mcp_client::JSONRPCMessage::Notification { .. } => serde_json::Value::Null,
        }
    }
}

/// Arguments for tool calls
#[derive(Debug, Clone)]
pub enum ToolArguments {
    None,
    Map(HashMap<String, Value>),
    Object(Map<String, Value>),
    Json(Value),
}

impl ToolArguments {
    pub fn to_json(self) -> Option<Value> {
        match self {
            ToolArguments::None => None,
            ToolArguments::Map(map) => Some(Value::Object(map.into_iter().collect())),
            ToolArguments::Object(obj) => Some(Value::Object(obj)),
            ToolArguments::Json(json) => Some(json),
        }
    }
}

impl From<()> for ToolArguments {
    fn from(_: ()) -> Self {
        ToolArguments::None
    }
}

impl From<HashMap<String, Value>> for ToolArguments {
    fn from(map: HashMap<String, Value>) -> Self {
        ToolArguments::Map(map)
    }
}

impl From<HashMap<&str, Value>> for ToolArguments {
    fn from(map: HashMap<&str, Value>) -> Self {
        let map = map.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
        ToolArguments::Map(map)
    }
}

impl From<HashMap<String, &str>> for ToolArguments {
    fn from(map: HashMap<String, &str>) -> Self {
        let map = map.into_iter().map(|(k, v)| (k, Value::String(v.to_string()))).collect();
        ToolArguments::Map(map)
    }
}

impl From<HashMap<&str, &str>> for ToolArguments {
    fn from(map: HashMap<&str, &str>) -> Self {
        let map = map.into_iter().map(|(k, v)| (k.to_string(), Value::String(v.to_string()))).collect();
        ToolArguments::Map(map)
    }
}

impl From<Map<String, Value>> for ToolArguments {
    fn from(obj: Map<String, Value>) -> Self {
        ToolArguments::Object(obj)
    }
}

impl From<Value> for ToolArguments {
    fn from(json: Value) -> Self {
        ToolArguments::Json(json)
    }
}

/// Convenience macros for creating tool arguments
#[macro_export]
macro_rules! tool_args {
    () => {
        $crate::mcp_api::ToolArguments::None
    };
    ($($key:expr => $value:expr),+ $(,)?) => {
        {
            let mut map = std::collections::HashMap::new();
            $(
                map.insert($key.to_string(), serde_json::json!($value));
            )+
            $crate::mcp_api::ToolArguments::Map(map)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_builder_pattern() {
        let client = MCPClientBuilder::new()
            .with_stdio_transport()
            .with_timeout(60)
            .with_max_retries(5)
            .with_client_info("test-client", "1.0.0")
            .build();
        
        assert!(client.is_ok());
    }

    #[test]
    fn test_tool_arguments_from_hashmap() {
        let mut map = HashMap::new();
        map.insert("key1", "value1");
        map.insert("key2", "value2");
        
        let args = ToolArguments::from(map);
        let json = args.to_json();
        
        assert!(json.is_some());
        let json = json.unwrap();
        assert_eq!(json["key1"], "value1");
        assert_eq!(json["key2"], "value2");
    }

    #[test]
    fn test_tool_arguments_macro() {
        let args = tool_args!("name" => "test", "count" => 42);
        let json = args.to_json().unwrap();
        
        assert_eq!(json["name"], "test");
        assert_eq!(json["count"], 42);
    }

    #[test]
    fn test_tool_arguments_empty() {
        let args = tool_args!();
        let json = args.to_json();
        
        assert!(json.is_none());
    }

    #[test]
    fn test_error_display() {
        let error = MCPError::Connection("Test error".to_string());
        assert_eq!(error.to_string(), "Connection error: Test error");
    }

    #[tokio::test]
    async fn test_client_api_creation() {
        let client = MCPClientApi::new();
        assert!(client.is_ok());
        
        let client = client.unwrap();
        assert!(!client.is_connected());
    }

    #[test]
    fn test_tool_info_creation() {
        let tool = ToolInfo {
            name: "test_tool".to_string(),
            description: Some("A test tool".to_string()),
            schema: Some(json!({"type": "object"})),
        };
        
        assert_eq!(tool.name, "test_tool");
        assert!(tool.description.is_some());
        assert!(tool.schema.is_some());
    }

    #[test]
    fn test_resource_info_creation() {
        let resource = ResourceInfo {
            uri: "file:///test.txt".to_string(),
            name: Some("test.txt".to_string()),
            description: Some("Test file".to_string()),
            mime_type: Some("text/plain".to_string()),
        };
        
        assert_eq!(resource.uri, "file:///test.txt");
        assert_eq!(resource.name, Some("test.txt".to_string()));
        assert_eq!(resource.mime_type, Some("text/plain".to_string()));
    }

    #[test]
    fn test_resource_content_creation() {
        let content = ResourceContent {
            uri: "file:///test.txt".to_string(),
            mime_type: Some("text/plain".to_string()),
            text: Some("Hello, world!".to_string()),
            blob: None,
        };
        
        assert_eq!(content.uri, "file:///test.txt");
        assert_eq!(content.text, Some("Hello, world!".to_string()));
        assert!(content.blob.is_none());
    }
}