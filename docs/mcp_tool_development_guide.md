# MCP Tool Development Guide

## Overview

This guide covers creating custom tools and plugins for the MCP Client Framework. It includes best practices, examples, and integration patterns for extending the framework's capabilities.

## Table of Contents

1. [Tool Creation](#tool-creation)
2. [Plugin Development](#plugin-development)
3. [Integration Patterns](#integration-patterns)
4. [Testing Tools](#testing-tools)
5. [Best Practices](#best-practices)
6. [Deployment](#deployment)

## Tool Creation

### Basic Tool Structure

Tools in the MCP framework are defined by their metadata and implementation. Here's a basic example:

```rust
use arrowhead::mcp_client::{ToolMetadata, MCPVersion};
use serde_json::{json, Value};

pub struct CalculatorTool {
    metadata: ToolMetadata,
}

impl CalculatorTool {
    pub fn new() -> Self {
        let metadata = ToolMetadata {
            name: "calculator".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Performs basic arithmetic operations".to_string()),
            schema: Some(json!({
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "enum": ["add", "subtract", "multiply", "divide"],
                        "description": "The operation to perform"
                    },
                    "a": {
                        "type": "number",
                        "description": "First operand"
                    },
                    "b": {
                        "type": "number",
                        "description": "Second operand"
                    }
                },
                "required": ["operation", "a", "b"]
            })),
            capabilities: vec!["arithmetic".to_string()],
            compatibility_version: MCPVersion::current(),
            last_updated: Some(chrono::Utc::now().to_rfc3339()),
            provider: Some("my-company".to_string()),
        };

        Self { metadata }
    }

    pub fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    pub async fn execute(&self, args: Value) -> Result<Value, Box<dyn std::error::Error>> {
        let operation = args["operation"].as_str().ok_or("Missing operation")?;
        let a = args["a"].as_f64().ok_or("Invalid number a")?;
        let b = args["b"].as_f64().ok_or("Invalid number b")?;

        let result = match operation {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            "divide" => {
                if b == 0.0 {
                    return Err("Division by zero".into());
                }
                a / b
            }
            _ => return Err("Unknown operation".into()),
        };

        Ok(json!({
            "result": result,
            "operation": operation,
            "operands": [a, b]
        }))
    }
}
```

### Tool Registration

Tools need to be registered with the MCP client's tool registry:

```rust
use arrowhead::mcp_client::{MCPClient, ToolRegistration};

async fn register_calculator_tool(client: &MCPClient) -> Result<(), Box<dyn std::error::Error>> {
    let tool = CalculatorTool::new();
    
    let registration = ToolRegistration {
        metadata: tool.metadata().clone(),
        is_available: true,
        response_time_ms: None,
        last_used: None,
        usage_count: 0,
    };

    client.register_tool(registration.metadata).await?;
    Ok(())
}
```

### Advanced Tool Features

#### Tool with State Management

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct StatefulTool {
    metadata: ToolMetadata,
    state: Arc<RwLock<HashMap<String, Value>>>,
}

impl StatefulTool {
    pub fn new() -> Self {
        let metadata = ToolMetadata {
            name: "stateful_counter".to_string(),
            version: "1.0.0".to_string(),
            description: Some("A counter that maintains state".to_string()),
            schema: Some(json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["increment", "decrement", "get", "reset"],
                        "description": "Action to perform"
                    },
                    "key": {
                        "type": "string",
                        "description": "Counter key"
                    },
                    "amount": {
                        "type": "number",
                        "description": "Amount to increment/decrement",
                        "default": 1
                    }
                },
                "required": ["action", "key"]
            })),
            capabilities: vec!["state_management".to_string()],
            compatibility_version: MCPVersion::current(),
            last_updated: Some(chrono::Utc::now().to_rfc3339()),
            provider: Some("my-company".to_string()),
        };

        Self {
            metadata,
            state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn execute(&self, args: Value) -> Result<Value, Box<dyn std::error::Error>> {
        let action = args["action"].as_str().ok_or("Missing action")?;
        let key = args["key"].as_str().ok_or("Missing key")?;
        let amount = args["amount"].as_f64().unwrap_or(1.0);

        let mut state = self.state.write().await;
        
        match action {
            "increment" => {
                let current = state.get(key).and_then(|v| v.as_f64()).unwrap_or(0.0);
                let new_value = current + amount;
                state.insert(key.to_string(), json!(new_value));
                Ok(json!({
                    "key": key,
                    "value": new_value,
                    "action": "increment",
                    "amount": amount
                }))
            }
            "decrement" => {
                let current = state.get(key).and_then(|v| v.as_f64()).unwrap_or(0.0);
                let new_value = current - amount;
                state.insert(key.to_string(), json!(new_value));
                Ok(json!({
                    "key": key,
                    "value": new_value,
                    "action": "decrement",
                    "amount": amount
                }))
            }
            "get" => {
                let value = state.get(key).and_then(|v| v.as_f64()).unwrap_or(0.0);
                Ok(json!({
                    "key": key,
                    "value": value,
                    "action": "get"
                }))
            }
            "reset" => {
                state.insert(key.to_string(), json!(0.0));
                Ok(json!({
                    "key": key,
                    "value": 0.0,
                    "action": "reset"
                }))
            }
            _ => Err("Unknown action".into()),
        }
    }
}
```

#### Tool with External Dependencies

```rust
use reqwest;
use serde_json::{json, Value};

pub struct HttpTool {
    metadata: ToolMetadata,
    client: reqwest::Client,
}

impl HttpTool {
    pub fn new() -> Self {
        let metadata = ToolMetadata {
            name: "http_request".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Makes HTTP requests".to_string()),
            schema: Some(json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "URL to request"
                    },
                    "method": {
                        "type": "string",
                        "enum": ["GET", "POST", "PUT", "DELETE"],
                        "default": "GET",
                        "description": "HTTP method"
                    },
                    "headers": {
                        "type": "object",
                        "description": "HTTP headers"
                    },
                    "body": {
                        "type": "string",
                        "description": "Request body"
                    }
                },
                "required": ["url"]
            })),
            capabilities: vec!["http_requests".to_string()],
            compatibility_version: MCPVersion::current(),
            last_updated: Some(chrono::Utc::now().to_rfc3339()),
            provider: Some("my-company".to_string()),
        };

        Self {
            metadata,
            client: reqwest::Client::new(),
        }
    }

    pub async fn execute(&self, args: Value) -> Result<Value, Box<dyn std::error::Error>> {
        let url = args["url"].as_str().ok_or("Missing URL")?;
        let method = args["method"].as_str().unwrap_or("GET");
        let headers = args["headers"].as_object();
        let body = args["body"].as_str();

        let mut request = match method {
            "GET" => self.client.get(url),
            "POST" => self.client.post(url),
            "PUT" => self.client.put(url),
            "DELETE" => self.client.delete(url),
            _ => return Err("Unsupported HTTP method".into()),
        };

        if let Some(headers) = headers {
            for (key, value) in headers {
                if let Some(value_str) = value.as_str() {
                    request = request.header(key, value_str);
                }
            }
        }

        if let Some(body) = body {
            request = request.body(body.to_string());
        }

        let response = request.send().await?;
        let status = response.status().as_u16();
        let headers: HashMap<String, String> = response.headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = response.text().await?;

        Ok(json!({
            "status": status,
            "headers": headers,
            "body": body,
            "url": url,
            "method": method
        }))
    }
}
```

## Plugin Development

### Basic Plugin Structure

Plugins provide a way to extend the MCP client with custom functionality:

```rust
use arrowhead::mcp_client::plugin::{Plugin, PluginMetadata, PluginType, PluginContext, PluginEvent};
use arrowhead::mcp_client::{JSONRPCMessage, MCPVersion};
use async_trait::async_trait;
use anyhow::Result;

pub struct MyPlugin {
    metadata: PluginMetadata,
    context: Option<PluginContext>,
    tools: Vec<Box<dyn CustomTool>>,
}

impl MyPlugin {
    pub fn new() -> Self {
        let metadata = PluginMetadata {
            id: "my-plugin".to_string(),
            name: "My Custom Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: Some("A custom plugin for specialized operations".to_string()),
            author: Some("Developer Name".to_string()),
            plugin_type: PluginType::Native,
            mcp_version: MCPVersion::current(),
            entry_point: "main".to_string(),
            dependencies: vec![],
            permissions: vec!["mcp.tools.register".to_string(), "mcp.tools.call".to_string()],
            configuration: Some(json!({
                "settings": {
                    "timeout": 30,
                    "retry_count": 3
                }
            })),
        };

        Self {
            metadata,
            context: None,
            tools: vec![
                Box::new(CalculatorTool::new()),
                Box::new(HttpTool::new()),
            ],
        }
    }
}

#[async_trait]
impl Plugin for MyPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn initialize(&mut self, context: PluginContext) -> Result<()> {
        self.context = Some(context);
        
        // Register tools
        for tool in &self.tools {
            // Register tool with MCP client
            println!("Registering tool: {}", tool.metadata().name);
        }
        
        Ok(())
    }

    async fn activate(&mut self) -> Result<()> {
        println!("Plugin {} activated", self.metadata.name);
        Ok(())
    }

    async fn deactivate(&mut self) -> Result<()> {
        println!("Plugin {} deactivated", self.metadata.name);
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        println!("Plugin {} shutting down", self.metadata.name);
        Ok(())
    }

    async fn handle_message(&mut self, message: &JSONRPCMessage) -> Result<Option<JSONRPCMessage>> {
        match message {
            JSONRPCMessage::Request { method, params, .. } => {
                match method.as_str() {
                    "plugin.get_tools" => {
                        let tool_names: Vec<String> = self.tools.iter()
                            .map(|t| t.metadata().name.clone())
                            .collect();
                        
                        Ok(Some(JSONRPCMessage::Response {
                            jsonrpc: "2.0".to_string(),
                            id: message.id().clone(),
                            result: Some(json!({
                                "tools": tool_names
                            })),
                            error: None,
                        }))
                    }
                    _ => Ok(None),
                }
            }
            _ => Ok(None),
        }
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "tool_registration".to_string(),
            "message_handling".to_string(),
            "configuration".to_string(),
        ]
    }

    fn validate_config(&self, config: &serde_json::Value) -> Result<()> {
        // Validate plugin configuration
        if let Some(settings) = config.get("settings") {
            if let Some(timeout) = settings.get("timeout") {
                if !timeout.is_number() {
                    return Err(anyhow::anyhow!("Invalid timeout configuration"));
                }
            }
        }
        Ok(())
    }

    fn resource_usage(&self) -> arrowhead::mcp_client::plugin::ResourceUsage {
        arrowhead::mcp_client::plugin::ResourceUsage {
            memory_mb: 10,
            cpu_percent: 5,
            disk_mb: 1,
        }
    }

    async fn on_event(&mut self, event: PluginEvent) -> Result<()> {
        match event {
            PluginEvent::ServerConnected => {
                println!("Server connected - plugin ready");
            }
            PluginEvent::ServerDisconnected => {
                println!("Server disconnected - plugin standby");
            }
            PluginEvent::ToolCalled { tool_name, .. } => {
                println!("Tool {} was called", tool_name);
            }
            _ => {}
        }
        Ok(())
    }
}

// Helper trait for custom tools
trait CustomTool {
    fn metadata(&self) -> &ToolMetadata;
    async fn execute(&self, args: Value) -> Result<Value, Box<dyn std::error::Error>>;
}
```

### Plugin Configuration

Plugins can have configuration files for customization:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginConfig {
    pub enabled: bool,
    pub timeout_ms: u64,
    pub max_retries: u32,
    pub custom_settings: HashMap<String, Value>,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout_ms: 30000,
            max_retries: 3,
            custom_settings: HashMap::new(),
        }
    }
}

impl MyPlugin {
    pub fn load_config(&mut self, config_path: &Path) -> Result<()> {
        let config_str = std::fs::read_to_string(config_path)?;
        let config: PluginConfig = toml::from_str(&config_str)?;
        
        // Apply configuration
        self.apply_config(config)?;
        Ok(())
    }

    fn apply_config(&mut self, config: PluginConfig) -> Result<()> {
        // Apply configuration to plugin
        println!("Applying configuration: {:?}", config);
        Ok(())
    }
}
```

## Integration Patterns

### Tool Factory Pattern

```rust
use std::collections::HashMap;

pub struct ToolFactory {
    builders: HashMap<String, Box<dyn ToolBuilder>>,
}

impl ToolFactory {
    pub fn new() -> Self {
        let mut factory = Self {
            builders: HashMap::new(),
        };
        
        // Register tool builders
        factory.register_builder("calculator", Box::new(CalculatorToolBuilder));
        factory.register_builder("http", Box::new(HttpToolBuilder));
        
        factory
    }

    pub fn register_builder(&mut self, name: &str, builder: Box<dyn ToolBuilder>) {
        self.builders.insert(name.to_string(), builder);
    }

    pub fn create_tool(&self, name: &str, config: Value) -> Result<Box<dyn CustomTool>, Box<dyn std::error::Error>> {
        let builder = self.builders.get(name)
            .ok_or_else(|| format!("Unknown tool type: {}", name))?;
        
        builder.build(config)
    }
}

trait ToolBuilder {
    fn build(&self, config: Value) -> Result<Box<dyn CustomTool>, Box<dyn std::error::Error>>;
}

struct CalculatorToolBuilder;

impl ToolBuilder for CalculatorToolBuilder {
    fn build(&self, _config: Value) -> Result<Box<dyn CustomTool>, Box<dyn std::error::Error>> {
        Ok(Box::new(CalculatorTool::new()))
    }
}

struct HttpToolBuilder;

impl ToolBuilder for HttpToolBuilder {
    fn build(&self, _config: Value) -> Result<Box<dyn CustomTool>, Box<dyn std::error::Error>> {
        Ok(Box::new(HttpTool::new()))
    }
}
```

### Plugin Manager Pattern

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct CustomPluginManager {
    plugins: Arc<RwLock<HashMap<String, Box<dyn Plugin>>>>,
    plugin_factory: ToolFactory,
}

impl CustomPluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            plugin_factory: ToolFactory::new(),
        }
    }

    pub async fn load_plugin_from_config(&self, config_path: &Path) -> Result<String> {
        let config_str = std::fs::read_to_string(config_path)?;
        let config: PluginConfig = toml::from_str(&config_str)?;
        
        let mut plugin = MyPlugin::new();
        plugin.load_config(config_path)?;
        
        let plugin_id = plugin.metadata().id.clone();
        let mut plugins = self.plugins.write().await;
        plugins.insert(plugin_id.clone(), Box::new(plugin));
        
        Ok(plugin_id)
    }

    pub async fn activate_plugin(&self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        if let Some(plugin) = plugins.get_mut(plugin_id) {
            plugin.activate().await?;
        }
        Ok(())
    }

    pub async fn deactivate_plugin(&self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        if let Some(plugin) = plugins.get_mut(plugin_id) {
            plugin.deactivate().await?;
        }
        Ok(())
    }
}
```

## Testing Tools

### Unit Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_calculator_tool() {
        let tool = CalculatorTool::new();
        
        // Test addition
        let result = tool.execute(json!({
            "operation": "add",
            "a": 5,
            "b": 3
        })).await.unwrap();
        
        assert_eq!(result["result"], 8);
        assert_eq!(result["operation"], "add");
        assert_eq!(result["operands"], [5, 3]);
    }

    #[tokio::test]
    async fn test_calculator_tool_division_by_zero() {
        let tool = CalculatorTool::new();
        
        let result = tool.execute(json!({
            "operation": "divide",
            "a": 10,
            "b": 0
        })).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Division by zero"));
    }

    #[tokio::test]
    async fn test_stateful_tool() {
        let tool = StatefulTool::new();
        
        // Test increment
        let result = tool.execute(json!({
            "action": "increment",
            "key": "counter1",
            "amount": 5
        })).await.unwrap();
        
        assert_eq!(result["value"], 5);
        
        // Test get
        let result = tool.execute(json!({
            "action": "get",
            "key": "counter1"
        })).await.unwrap();
        
        assert_eq!(result["value"], 5);
    }
}
```

### Integration Testing

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use arrowhead::mcp_api::MCPClientBuilder;

    #[tokio::test]
    async fn test_tool_integration() {
        // Create mock MCP server or use test server
        let mut client = MCPClientBuilder::new()
            .with_stdio_transport()
            .build()
            .unwrap();

        // Would need actual server for full integration test
        // This is a placeholder for integration testing structure
        
        assert!(!client.is_connected());
    }
}
```

## Best Practices

### 1. Tool Design

- **Single Responsibility**: Each tool should have a clear, single purpose
- **Comprehensive Schema**: Provide detailed JSON schemas for input validation
- **Error Handling**: Return meaningful error messages
- **Documentation**: Include descriptions and examples

### 2. State Management

- **Thread Safety**: Use appropriate synchronization primitives
- **Persistence**: Consider whether state should persist across restarts
- **Cleanup**: Implement proper cleanup of resources

### 3. Configuration

- **Validation**: Always validate configuration parameters
- **Defaults**: Provide sensible default values
- **Documentation**: Document all configuration options

### 4. Testing

- **Unit Tests**: Test individual tools in isolation
- **Integration Tests**: Test tools with the MCP client
- **Error Cases**: Test error conditions and edge cases

### 5. Performance

- **Async Operations**: Use async/await for I/O operations
- **Resource Management**: Monitor memory and CPU usage
- **Caching**: Cache expensive operations where appropriate

## Deployment

### Plugin Packaging

```toml
# plugin.toml
[plugin]
name = "my-plugin"
version = "1.0.0"
description = "A custom plugin for specialized operations"
author = "Developer Name"
entry_point = "main"

[plugin.dependencies]
tokio = "1.0"
serde = "1.0"
reqwest = "0.11"

[plugin.permissions]
required = ["mcp.tools.register", "mcp.tools.call"]
optional = ["mcp.resources.read"]

[plugin.configuration]
timeout_ms = 30000
max_retries = 3
```

### Distribution

```bash
# Build plugin
cargo build --release

# Package plugin
tar -czf my-plugin-1.0.0.tar.gz target/release/libmy_plugin.so plugin.toml README.md

# Install plugin
mkdir -p ~/.mcp/plugins/my-plugin
tar -xzf my-plugin-1.0.0.tar.gz -C ~/.mcp/plugins/my-plugin
```

### Configuration Management

```rust
// ~/.mcp/config.toml
[plugins]
enabled = ["my-plugin"]

[plugins.my-plugin]
path = "~/.mcp/plugins/my-plugin"
config = "~/.mcp/plugins/my-plugin/config.toml"
auto_start = true
```

This comprehensive guide provides everything needed to create, test, and deploy custom tools and plugins for the MCP Client Framework. For more advanced use cases, refer to the API documentation and example implementations.