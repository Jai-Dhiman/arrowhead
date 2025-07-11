# MCP Client Framework Documentation

## Overview

The Arrowhead MCP (Model Context Protocol) Client Framework provides a comprehensive Rust implementation for communicating with MCP servers. It includes automatic protocol version negotiation, capability discovery, feature flag management, and graceful degradation for backward compatibility.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Architecture](#architecture)
3. [Configuration](#configuration)
4. [Client API](#client-api)
5. [Tool Management](#tool-management)
6. [Plugin System](#plugin-system)
7. [Advanced Features](#advanced-features)
8. [Examples](#examples)
9. [Error Handling](#error-handling)
10. [Best Practices](#best-practices)

## Quick Start

### Basic Usage

```rust
use arrowhead::mcp_api::{MCPClientBuilder, MCPError, tool_args};

#[tokio::main]
async fn main() -> Result<(), MCPError> {
    // Create and connect to an MCP server
    let mut client = MCPClientBuilder::new()
        .with_stdio_transport()
        .with_timeout(30)
        .with_client_info("my-app", "1.0.0")
        .build()?;

    client.connect().await?;

    // List available tools
    let tools = client.list_tools().await?;
    println!("Available tools: {:#?}", tools);

    // Call a tool
    let result = client.call_tool("echo", tool_args!(
        "message" => "Hello, MCP!"
    )).await?;
    
    println!("Tool result: {:#?}", result);

    client.disconnect().await?;
    Ok(())
}
```

### Using Process Transport

```rust
use arrowhead::mcp_api::MCPClientBuilder;

let mut client = MCPClientBuilder::new()
    .with_process_transport("python", vec!["-m", "my_mcp_server"].into_iter().map(String::from).collect())
    .with_timeout(60)
    .build()?;

client.connect().await?;
```

### Using TCP Transport

```rust
use arrowhead::mcp_api::MCPClientBuilder;

let mut client = MCPClientBuilder::new()
    .with_tcp_transport("localhost", 8080)
    .with_timeout(30)
    .build()?;

client.connect().await?;
```

## Architecture

The MCP Client Framework is built with several key components:

### Core Components

1. **MCPClient**: Low-level protocol implementation with JSON-RPC 2.0 support
2. **MCPClientApi**: High-level, user-friendly API with error handling
3. **ToolRegistry**: Dynamic tool discovery and management with caching
4. **PluginManager**: Extensible plugin system with lifecycle management
5. **ServerCapabilities**: Protocol version negotiation and feature detection
6. **Feature Flags**: Dynamic feature management with backward compatibility

### Protocol Flow

```
Client Connect → Version Negotiation → Capability Discovery → Feature Flag Initialization → Tool Registration → Ready for Operations
```

### Key Features

- **Automatic Protocol Version Negotiation**: Seamlessly handles different protocol versions
- **Graceful Degradation**: Falls back to compatible methods when newer features aren't supported
- **Plugin Architecture**: Extensible system for custom tool implementations
- **Connection Pooling**: Efficient resource management for multiple operations
- **Comprehensive Error Handling**: Detailed error types with context
- **Feature Flag System**: Dynamic feature management and server compatibility checking

## Configuration

### Client Configuration Options

```rust
use arrowhead::mcp_client::{MCPClientConfig, MCPTransport, MCPVersion, ClientInfo};

let config = MCPClientConfig {
    transport: MCPTransport::StdIO,
    protocol_version: MCPVersion::current(),
    client_info: ClientInfo {
        name: "my-client".to_string(),
        version: "1.0.0".to_string(),
    },
    timeout_ms: 30000,
    max_retries: 3,
};
```

### Transport Configuration

#### StdIO Transport
```rust
MCPTransport::StdIO
```

#### TCP Transport
```rust
MCPTransport::TCP { 
    host: "localhost".to_string(), 
    port: 8080 
}
```

#### WebSocket Transport
```rust
MCPTransport::WebSocket { 
    url: "ws://localhost:8080/mcp".to_string() 
}
```

#### Process Transport
```rust
MCPTransport::Process { 
    command: "python".to_string(), 
    args: vec!["-m".to_string(), "my_server".to_string()] 
}
```

## Client API

### Connection Management

```rust
// Connect to server
client.connect().await?;

// Check connection status
if client.is_connected() {
    println!("Connected to MCP server");
}

// Disconnect
client.disconnect().await?;
```

### Tool Operations

#### Listing Tools

```rust
let tools = client.list_tools().await?;
for tool in tools {
    println!("Tool: {} - {}", tool.name, tool.description.unwrap_or_default());
}
```

#### Calling Tools

```rust
// Using the tool_args! macro
let result = client.call_tool("calculator", tool_args!(
    "operation" => "add",
    "a" => 5,
    "b" => 3
)).await?;

// Using a HashMap
use std::collections::HashMap;
let mut args = HashMap::new();
args.insert("query".to_string(), serde_json::json!("hello world"));
let result = client.call_tool("search", args).await?;

// Using serde_json directly
let result = client.call_tool("format", serde_json::json!({
    "template": "Hello, {name}!",
    "name": "Alice"
})).await?;
```

#### Tool Availability and Metadata

```rust
// Check if a tool is available
if client.is_tool_available("my_tool").await? {
    let metadata = client.get_tool_metadata("my_tool").await?;
    if let Some(metadata) = metadata {
        println!("Tool: {} v{}", metadata.name, metadata.version);
        println!("Capabilities: {:?}", metadata.capabilities);
    }
}
```

### Resource Operations

#### Listing Resources

```rust
let resources = client.list_resources().await?;
for resource in resources {
    println!("Resource: {} - {}", resource.uri, resource.name.unwrap_or_default());
}
```

#### Reading Resources

```rust
let content = client.read_resource("file:///path/to/file.txt").await?;
if let Some(text) = content.text {
    println!("Content: {}", text);
}
```

### Server Capabilities and Features

#### Checking Server Capabilities

```rust
let capabilities = client.get_server_capabilities().await?;
println!("Protocol version: {:?}", capabilities.protocol_version);
println!("Supported methods: {:?}", capabilities.supported_methods);
println!("Experimental features: {:?}", capabilities.experimental_features);
```

#### Feature Flag Management

```rust
// Check if a feature is enabled
if client.is_feature_enabled("experimental.streaming").await? {
    println!("Streaming is supported");
}

// Get all feature flags
let flags = client.get_feature_flags().await?;
for (name, flag) in flags {
    println!("Feature {}: {}", name, if flag.enabled { "enabled" } else { "disabled" });
}

// Set a feature flag
client.set_feature_flag("tools.parallel_execution", true).await?;
```

## Tool Management

### Tool Registry Statistics

```rust
let stats = client.get_tool_statistics().await?;
println!("Tool registry statistics: {:#?}", stats);
```

### Refreshing Tools

```rust
// Refresh the tool registry to discover new tools
client.refresh_tools().await?;
```

## Plugin System

### Loading Plugins

```rust
use std::path::PathBuf;

// Load a plugin from a directory
let plugin_path = PathBuf::from("/path/to/plugin");
let plugin_id = client.load_plugin(&plugin_path).await?;
println!("Loaded plugin: {}", plugin_id);
```

### Managing Plugins

```rust
// Unload a plugin
client.unload_plugin("my-plugin-id").await?;
```

## Advanced Features

### Protocol Version Negotiation

The client automatically negotiates the best compatible protocol version with the server:

```rust
// Get the negotiated protocol version
let version = client.get_protocol_version().await?;
if let Some(version) = version {
    println!("Using protocol version: {}", version.to_string());
}
```

### Graceful Degradation

The client automatically handles compatibility issues:

```rust
// The client will automatically try alternative methods if the server
// doesn't support the latest protocol version
let tools = client.list_tools().await?; // Might use "list_tools" or "tools/list"
```

### Health Monitoring

```rust
// Send a ping to check server health
client.ping().await?;
```

## Examples

### Example 1: File Processing Tool

```rust
use arrowhead::mcp_api::{MCPClientBuilder, tool_args};

async fn process_files() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = MCPClientBuilder::new()
        .with_process_transport("python", vec!["-m", "file_processor_mcp"].into_iter().map(String::from).collect())
        .build()?;

    client.connect().await?;

    // List files in a directory
    let files = client.call_tool("list_files", tool_args!(
        "path" => "/path/to/directory",
        "pattern" => "*.txt"
    )).await?;

    println!("Found files: {:#?}", files);

    // Process each file
    if let Some(file_list) = files.get("files").and_then(|f| f.as_array()) {
        for file in file_list {
            if let Some(file_path) = file.as_str() {
                let result = client.call_tool("process_file", tool_args!(
                    "path" => file_path,
                    "operation" => "analyze"
                )).await?;
                
                println!("Processed {}: {:#?}", file_path, result);
            }
        }
    }

    client.disconnect().await?;
    Ok(())
}
```

### Example 2: Database Query Tool

```rust
use arrowhead::mcp_api::{MCPClientBuilder, tool_args};

async fn query_database() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = MCPClientBuilder::new()
        .with_tcp_transport("db-server", 8080)
        .with_timeout(60)
        .build()?;

    client.connect().await?;

    // Execute a query
    let result = client.call_tool("sql_query", tool_args!(
        "query" => "SELECT * FROM users WHERE active = true",
        "limit" => 100
    )).await?;

    println!("Query result: {:#?}", result);

    // Execute a parameterized query
    let result = client.call_tool("sql_query", tool_args!(
        "query" => "SELECT * FROM orders WHERE user_id = ? AND date > ?",
        "params" => [123, "2024-01-01"]
    )).await?;

    println!("Parameterized query result: {:#?}", result);

    client.disconnect().await?;
    Ok(())
}
```

### Example 3: Multi-Server Setup

```rust
use arrowhead::mcp_api::MCPClientBuilder;
use futures::future::try_join_all;

async fn multi_server_example() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to multiple MCP servers
    let mut clients = Vec::new();

    // File server
    let mut file_client = MCPClientBuilder::new()
        .with_process_transport("python", vec!["-m", "file_mcp"].into_iter().map(String::from).collect())
        .with_client_info("file-client", "1.0.0")
        .build()?;

    // Database server
    let mut db_client = MCPClientBuilder::new()
        .with_tcp_transport("localhost", 8080)
        .with_client_info("db-client", "1.0.0")
        .build()?;

    // Web server
    let mut web_client = MCPClientBuilder::new()
        .with_websocket_transport("ws://localhost:8081/mcp")
        .with_client_info("web-client", "1.0.0")
        .build()?;

    // Connect all clients
    file_client.connect().await?;
    db_client.connect().await?;
    web_client.connect().await?;

    // Use different servers for different operations
    let files = file_client.list_tools().await?;
    let db_tools = db_client.list_tools().await?;
    let web_tools = web_client.list_tools().await?;

    println!("File server tools: {:#?}", files);
    println!("Database server tools: {:#?}", db_tools);
    println!("Web server tools: {:#?}", web_tools);

    // Cleanup
    file_client.disconnect().await?;
    db_client.disconnect().await?;
    web_client.disconnect().await?;

    Ok(())
}
```

### Example 4: Plugin Development

```rust
use arrowhead::mcp_client::plugin::{Plugin, PluginMetadata, PluginType, PluginContext, PluginEvent};
use arrowhead::mcp_client::{JSONRPCMessage, MCPVersion};
use async_trait::async_trait;
use anyhow::Result;

// Example custom plugin
pub struct MyCustomPlugin {
    metadata: PluginMetadata,
    context: Option<PluginContext>,
}

impl MyCustomPlugin {
    pub fn new() -> Self {
        let metadata = PluginMetadata {
            id: "my-custom-plugin".to_string(),
            name: "My Custom Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: Some("A custom plugin example".to_string()),
            author: Some("Developer".to_string()),
            plugin_type: PluginType::Native,
            mcp_version: MCPVersion::current(),
            entry_point: "main".to_string(),
            dependencies: vec![],
            permissions: vec!["mcp.tools.call".to_string()],
            configuration: None,
        };

        Self {
            metadata,
            context: None,
        }
    }
}

#[async_trait]
impl Plugin for MyCustomPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn initialize(&mut self, context: PluginContext) -> Result<()> {
        self.context = Some(context);
        println!("Plugin initialized");
        Ok(())
    }

    async fn activate(&mut self) -> Result<()> {
        println!("Plugin activated");
        Ok(())
    }

    async fn deactivate(&mut self) -> Result<()> {
        println!("Plugin deactivated");
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        println!("Plugin shutdown");
        Ok(())
    }

    async fn handle_message(&mut self, message: &JSONRPCMessage) -> Result<Option<JSONRPCMessage>> {
        println!("Plugin received message: {:#?}", message);
        Ok(None)
    }

    fn capabilities(&self) -> Vec<String> {
        vec!["message_handling".to_string()]
    }

    fn validate_config(&self, _config: &serde_json::Value) -> Result<()> {
        Ok(())
    }

    fn resource_usage(&self) -> arrowhead::mcp_client::plugin::ResourceUsage {
        Default::default()
    }

    async fn on_event(&mut self, event: PluginEvent) -> Result<()> {
        println!("Plugin received event: {:#?}", event);
        Ok(())
    }
}
```

## Error Handling

The MCP Client API uses the `MCPError` type for error handling:

```rust
use arrowhead::mcp_api::MCPError;

match client.call_tool("nonexistent", tool_args!()).await {
    Ok(result) => println!("Success: {:#?}", result),
    Err(MCPError::Tool(msg)) => println!("Tool error: {}", msg),
    Err(MCPError::Connection(msg)) => println!("Connection error: {}", msg),
    Err(MCPError::Protocol(msg)) => println!("Protocol error: {}", msg),
    Err(MCPError::Timeout(msg)) => println!("Timeout error: {}", msg),
    Err(e) => println!("Other error: {}", e),
}
```

### Retry Logic

```rust
use tokio::time::{sleep, Duration};

async fn call_tool_with_retry(
    client: &arrowhead::mcp_api::MCPClientApi,
    tool_name: &str,
    args: impl Into<arrowhead::mcp_api::ToolArguments>,
    max_retries: u32,
) -> Result<serde_json::Value, MCPError> {
    let mut retries = 0;
    let args = args.into();

    loop {
        match client.call_tool(tool_name, args.clone()).await {
            Ok(result) => return Ok(result),
            Err(MCPError::Timeout(_)) | Err(MCPError::Connection(_)) if retries < max_retries => {
                retries += 1;
                sleep(Duration::from_millis(1000 * retries as u64)).await;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}
```

## Best Practices

### 1. Connection Management

- Always disconnect when done
- Use connection pooling for multiple operations
- Handle connection failures gracefully

```rust
struct MCPClientPool {
    clients: Vec<MCPClientApi>,
    current: usize,
}

impl MCPClientPool {
    async fn get_client(&mut self) -> &mut MCPClientApi {
        let client = &mut self.clients[self.current];
        self.current = (self.current + 1) % self.clients.len();
        
        if !client.is_connected() {
            let _ = client.connect().await;
        }
        
        client
    }
}
```

### 2. Error Handling

- Always handle connection and protocol errors
- Implement retry logic for transient failures
- Log errors for debugging

### 3. Feature Detection

- Check feature flags before using advanced features
- Implement fallbacks for older servers

```rust
async fn advanced_operation(client: &MCPClientApi) -> Result<(), MCPError> {
    if client.is_feature_enabled("experimental.streaming").await? {
        // Use streaming API
        println!("Using streaming");
    } else {
        // Use standard API
        println!("Using standard API");
    }
    Ok(())
}
```

### 4. Resource Management

- Close connections properly
- Monitor resource usage for plugins
- Implement timeouts for long-running operations

### 5. Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use arrowhead::mcp_api::MCPClientBuilder;

    #[tokio::test]
    async fn test_mcp_client() {
        let client = MCPClientBuilder::new()
            .with_stdio_transport()
            .build()
            .expect("Failed to build client");

        // Test without actually connecting
        assert!(!client.is_connected());
    }

    #[tokio::test]
    async fn test_tool_args_macro() {
        let args = tool_args!("key" => "value", "number" => 42);
        let json = args.to_json().unwrap();
        
        assert_eq!(json["key"], "value");
        assert_eq!(json["number"], 42);
    }
}
```

## Troubleshooting

### Common Issues

1. **Connection Timeout**: Increase timeout or check server availability
2. **Protocol Version Mismatch**: Update client or server to compatible version
3. **Tool Not Found**: Verify tool name and refresh tool registry
4. **Permission Denied**: Check plugin permissions and security settings

### Debug Logging

```rust
env_logger::init();
log::info!("MCP Client starting");
```

### Performance Monitoring

```rust
let stats = client.get_tool_statistics().await?;
println!("Tool registry stats: {:#?}", stats);
```

This comprehensive guide covers all aspects of using the MCP Client Framework. For additional help, refer to the API documentation or check the examples in the repository.