/// Basic MCP Client Usage Example
/// 
/// This example demonstrates the fundamental operations of the MCP client:
/// - Connecting to a server
/// - Listing available tools
/// - Calling tools with different argument types
/// - Handling errors gracefully
/// - Proper cleanup

use arrowhead::mcp_api::{MCPClientBuilder, MCPError, tool_args};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging to see what's happening
    env_logger::init();

    // Create a new MCP client with stdio transport
    let mut client = MCPClientBuilder::new()
        .with_stdio_transport()
        .with_timeout(30)
        .with_client_info("basic-example", "1.0.0")
        .build()?;

    println!("🔌 Connecting to MCP server...");
    
    // Connect to the server
    match client.connect().await {
        Ok(_) => println!("✅ Connected successfully!"),
        Err(e) => {
            println!("❌ Failed to connect: {}", e);
            return Err(e.into());
        }
    }

    // Check server capabilities
    println!("\n📋 Checking server capabilities...");
    let capabilities = client.get_server_capabilities().await?;
    println!("Protocol version: {:?}", capabilities.protocol_version);
    println!("Supported methods: {:?}", capabilities.supported_methods);
    
    // List available tools
    println!("\n🔧 Listing available tools...");
    match client.list_tools().await {
        Ok(tools) => {
            println!("Found {} tools:", tools.len());
            for tool in &tools {
                println!("  - {}: {}", tool.name, tool.description.as_deref().unwrap_or("No description"));
            }
            
            // Call the first tool if available
            if let Some(first_tool) = tools.first() {
                println!("\n🚀 Calling tool: {}", first_tool.name);
                call_tool_examples(&client, &first_tool.name).await?;
            }
        }
        Err(e) => {
            println!("❌ Failed to list tools: {}", e);
        }
    }

    // List available resources
    println!("\n📁 Listing available resources...");
    match client.list_resources().await {
        Ok(resources) => {
            println!("Found {} resources:", resources.len());
            for resource in &resources {
                println!("  - {}: {}", resource.uri, resource.name.as_deref().unwrap_or("No name"));
            }
            
            // Read the first resource if available
            if let Some(first_resource) = resources.first() {
                println!("\n📖 Reading resource: {}", first_resource.uri);
                match client.read_resource(&first_resource.uri).await {
                    Ok(content) => {
                        if let Some(text) = content.text {
                            println!("Content (first 200 chars): {}", 
                                   text.chars().take(200).collect::<String>());
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to read resource: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to list resources: {}", e);
        }
    }

    // Test server health
    println!("\n💓 Testing server health...");
    match client.ping().await {
        Ok(_) => println!("✅ Server is healthy"),
        Err(e) => println!("❌ Server health check failed: {}", e),
    }

    // Show tool statistics
    println!("\n📊 Tool registry statistics:");
    let stats = client.get_tool_statistics().await?;
    for (key, value) in stats {
        println!("  {}: {}", key, value);
    }

    // Cleanup
    println!("\n🔌 Disconnecting...");
    client.disconnect().await?;
    println!("✅ Disconnected successfully!");

    Ok(())
}

/// Demonstrate different ways to call tools
async fn call_tool_examples(client: &arrowhead::mcp_api::MCPClientApi, tool_name: &str) -> Result<(), MCPError> {
    println!("Demonstrating different ways to call tools...");

    // Method 1: Using the tool_args! macro (recommended)
    println!("  📝 Method 1: Using tool_args! macro");
    match client.call_tool(tool_name, tool_args!(
        "message" => "Hello from Rust!",
        "count" => 3,
        "enabled" => true
    )).await {
        Ok(result) => println!("    Result: {}", result),
        Err(e) => println!("    Error: {}", e),
    }

    // Method 2: Using HashMap
    println!("  📝 Method 2: Using HashMap");
    let mut args = HashMap::new();
    args.insert("input".to_string(), serde_json::json!("test input"));
    args.insert("options".to_string(), serde_json::json!({"verbose": true}));
    
    match client.call_tool(tool_name, args).await {
        Ok(result) => println!("    Result: {}", result),
        Err(e) => println!("    Error: {}", e),
    }

    // Method 3: Using serde_json directly
    println!("  📝 Method 3: Using serde_json directly");
    let json_args = serde_json::json!({
        "data": [1, 2, 3, 4, 5],
        "operation": "sum"
    });
    
    match client.call_tool(tool_name, json_args).await {
        Ok(result) => println!("    Result: {}", result),
        Err(e) => println!("    Error: {}", e),
    }

    // Method 4: No arguments
    println!("  📝 Method 4: No arguments");
    match client.call_tool(tool_name, ()).await {
        Ok(result) => println!("    Result: {}", result),
        Err(e) => println!("    Error: {}", e),
    }

    Ok(())
}

/// Example of error handling patterns
async fn error_handling_example(client: &arrowhead::mcp_api::MCPClientApi) {
    use arrowhead::mcp_api::MCPError;

    println!("Demonstrating error handling...");

    match client.call_tool("nonexistent_tool", tool_args!()).await {
        Ok(result) => println!("Unexpected success: {}", result),
        Err(MCPError::Tool(msg)) => println!("Tool error (expected): {}", msg),
        Err(MCPError::Connection(msg)) => println!("Connection error: {}", msg),
        Err(MCPError::Protocol(msg)) => println!("Protocol error: {}", msg),
        Err(MCPError::Timeout(msg)) => println!("Timeout error: {}", msg),
        Err(e) => println!("Other error: {}", e),
    }
}

/// Example of feature flag usage
async fn feature_flag_example(client: &arrowhead::mcp_api::MCPClientApi) -> Result<(), MCPError> {
    println!("Checking feature flags...");

    // Check if specific features are enabled
    let features_to_check = vec![
        "tools.list",
        "tools.call", 
        "resources.list",
        "resources.read",
        "logging.setLevel",
        "experimental.streaming",
    ];

    for feature in features_to_check {
        let enabled = client.is_feature_enabled(feature).await?;
        println!("  {}: {}", feature, if enabled { "✅ enabled" } else { "❌ disabled" });
    }

    // Get all feature flags
    let all_flags = client.get_feature_flags().await?;
    println!("Total feature flags: {}", all_flags.len());

    Ok(())
}