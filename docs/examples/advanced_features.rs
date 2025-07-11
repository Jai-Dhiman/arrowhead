/// Advanced MCP Client Features Example
/// 
/// This example demonstrates advanced features of the MCP client:
/// - Multiple transport types
/// - Plugin management
/// - Protocol version negotiation
/// - Graceful degradation
/// - Performance monitoring
/// - Custom error handling

use arrowhead::mcp_api::{MCPClientBuilder, MCPError, tool_args};
use arrowhead::mcp_client::{MCPVersion, plugin::{PluginMetadata, PluginType}};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::time::{sleep, Duration, timeout};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    // Demonstrate different client configurations
    different_transports_example().await?;
    
    // Demonstrate protocol version negotiation
    protocol_negotiation_example().await?;
    
    // Demonstrate plugin management
    plugin_management_example().await?;
    
    // Demonstrate performance monitoring
    performance_monitoring_example().await?;
    
    // Demonstrate error recovery
    error_recovery_example().await?;

    Ok(())
}

/// Demonstrate different transport configurations
async fn different_transports_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Different Transport Examples");
    println!("==============================");

    // 1. StdIO Transport (most common)
    println!("\n📡 1. StdIO Transport");
    let stdio_client = MCPClientBuilder::new()
        .with_stdio_transport()
        .with_client_info("stdio-client", "1.0.0")
        .build()?;
    println!("   ✅ StdIO client created");

    // 2. TCP Transport
    println!("\n📡 2. TCP Transport");
    let tcp_client = MCPClientBuilder::new()
        .with_tcp_transport("localhost", 8080)
        .with_timeout(30)
        .with_client_info("tcp-client", "1.0.0")
        .build()?;
    println!("   ✅ TCP client created (localhost:8080)");

    // 3. WebSocket Transport
    println!("\n📡 3. WebSocket Transport");
    let ws_client = MCPClientBuilder::new()
        .with_websocket_transport("ws://localhost:8081/mcp")
        .with_timeout(30)
        .with_client_info("ws-client", "1.0.0")
        .build()?;
    println!("   ✅ WebSocket client created");

    // 4. Process Transport
    println!("\n📡 4. Process Transport");
    let process_client = MCPClientBuilder::new()
        .with_process_transport("python", vec![
            "-m".to_string(), 
            "my_mcp_server".to_string(),
            "--config".to_string(),
            "config.json".to_string()
        ])
        .with_timeout(60)
        .with_max_retries(5)
        .with_client_info("process-client", "1.0.0")
        .build()?;
    println!("   ✅ Process client created");

    Ok(())
}

/// Demonstrate protocol version negotiation
async fn protocol_negotiation_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔄 Protocol Version Negotiation");
    println!("==============================");

    // Create clients with different protocol versions
    let versions = vec![
        MCPVersion::new(2024, 11, 5),
        MCPVersion::new(2024, 11, 1),
        MCPVersion::new(2024, 10, 15),
    ];

    for version in versions {
        println!("\n📋 Testing with protocol version: {}", version.to_string());
        
        let mut client = MCPClientBuilder::new()
            .with_stdio_transport()
            .with_protocol_version(version.clone())
            .with_client_info("version-test", "1.0.0")
            .build()?;

        // Attempt connection (would fail in real scenario without matching server)
        match timeout(Duration::from_secs(5), client.connect()).await {
            Ok(Ok(_)) => {
                println!("   ✅ Connected with version {}", version.to_string());
                
                // Check negotiated version
                if let Ok(Some(negotiated)) = client.get_protocol_version().await {
                    println!("   📋 Negotiated version: {}", negotiated.to_string());
                }
                
                // Check backward compatibility
                let is_compatible = client.is_backward_compatible().await.unwrap_or(false);
                println!("   🔄 Backward compatible: {}", is_compatible);
                
                client.disconnect().await?;
            }
            Ok(Err(e)) => {
                println!("   ❌ Connection failed: {}", e);
            }
            Err(_) => {
                println!("   ⏰ Connection timeout");
            }
        }
    }

    Ok(())
}

/// Demonstrate plugin management
async fn plugin_management_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔌 Plugin Management");
    println!("===================");

    let mut client = MCPClientBuilder::new()
        .with_stdio_transport()
        .with_client_info("plugin-manager", "1.0.0")
        .build()?;

    // Connect (would fail without real server)
    if let Ok(_) = client.connect().await {
        println!("✅ Connected to server");

        // Load a plugin (example path)
        let plugin_path = PathBuf::from("/path/to/plugin");
        match client.load_plugin(&plugin_path).await {
            Ok(plugin_id) => {
                println!("✅ Loaded plugin: {}", plugin_id);
                
                // Plugin would be automatically initialized here
                
                // Later, unload the plugin
                if let Err(e) = client.unload_plugin(&plugin_id).await {
                    println!("❌ Failed to unload plugin: {}", e);
                } else {
                    println!("✅ Unloaded plugin: {}", plugin_id);
                }
            }
            Err(e) => {
                println!("❌ Failed to load plugin: {}", e);
            }
        }

        client.disconnect().await?;
    } else {
        println!("❌ Could not connect to server (expected in this example)");
    }

    Ok(())
}

/// Demonstrate performance monitoring
async fn performance_monitoring_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📊 Performance Monitoring");
    println!("========================");

    let mut client = MCPClientBuilder::new()
        .with_stdio_transport()
        .with_timeout(30)
        .with_client_info("perf-monitor", "1.0.0")
        .build()?;

    if let Ok(_) = client.connect().await {
        println!("✅ Connected to server");

        // Monitor tool calls with timing
        let start = std::time::Instant::now();
        
        // Simulate multiple tool calls
        for i in 0..5 {
            let call_start = std::time::Instant::now();
            
            match client.call_tool("example_tool", tool_args!(
                "iteration" => i,
                "timestamp" => chrono::Utc::now().to_rfc3339()
            )).await {
                Ok(_) => {
                    let call_duration = call_start.elapsed();
                    println!("   Call {}: {}ms", i + 1, call_duration.as_millis());
                }
                Err(e) => {
                    println!("   Call {} failed: {}", i + 1, e);
                }
            }
        }

        let total_duration = start.elapsed();
        println!("📊 Total time: {}ms", total_duration.as_millis());

        // Get tool registry statistics
        match client.get_tool_statistics().await {
            Ok(stats) => {
                println!("\n📈 Tool Registry Statistics:");
                for (key, value) in stats {
                    println!("   {}: {}", key, value);
                }
            }
            Err(e) => {
                println!("❌ Failed to get statistics: {}", e);
            }
        }

        // Check tool availability before calling
        let tools_to_check = vec!["calculator", "file_reader", "web_scraper"];
        for tool in tools_to_check {
            match client.is_tool_available(tool).await {
                Ok(true) => {
                    println!("   ✅ {} is available", tool);
                    
                    // Get tool metadata
                    if let Ok(Some(metadata)) = client.get_tool_metadata(tool).await {
                        println!("      Version: {}", metadata.version);
                        println!("      Capabilities: {:?}", metadata.capabilities);
                    }
                }
                Ok(false) => {
                    println!("   ❌ {} is not available", tool);
                }
                Err(e) => {
                    println!("   ⚠️ Error checking {}: {}", tool, e);
                }
            }
        }

        client.disconnect().await?;
    }

    Ok(())
}

/// Demonstrate error recovery and retry mechanisms
async fn error_recovery_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔄 Error Recovery Examples");
    println!("=========================");

    let mut client = MCPClientBuilder::new()
        .with_stdio_transport()
        .with_timeout(10) // Short timeout to trigger errors
        .with_max_retries(3)
        .with_client_info("error-recovery", "1.0.0")
        .build()?;

    // Demonstrate connection retry
    println!("\n🔄 Connection Retry Logic");
    for attempt in 1..=3 {
        println!("   Attempt {}/3", attempt);
        match client.connect().await {
            Ok(_) => {
                println!("   ✅ Connected on attempt {}", attempt);
                break;
            }
            Err(e) => {
                println!("   ❌ Attempt {} failed: {}", attempt, e);
                if attempt < 3 {
                    sleep(Duration::from_millis(1000 * attempt as u64)).await;
                }
            }
        }
    }

    // Demonstrate tool call with retry logic
    if client.is_connected() {
        println!("\n🔄 Tool Call Retry Logic");
        
        let result = call_tool_with_retry(
            &client,
            "flaky_tool",
            tool_args!("test" => "data"),
            3
        ).await;

        match result {
            Ok(response) => println!("   ✅ Tool call succeeded: {}", response),
            Err(e) => println!("   ❌ Tool call failed after retries: {}", e),
        }

        // Demonstrate graceful degradation
        println!("\n🔄 Graceful Degradation");
        
        // Try modern method first, fallback to older methods
        let tools = match client.list_tools().await {
            Ok(tools) => {
                println!("   ✅ Modern tools/list succeeded");
                tools
            }
            Err(_) => {
                println!("   ⚠️ Modern method failed, trying fallback...");
                // In a real implementation, the client would automatically
                // try alternative methods
                Vec::new()
            }
        };

        println!("   📋 Found {} tools", tools.len());

        client.disconnect().await?;
    }

    Ok(())
}

/// Helper function for tool calls with retry logic
async fn call_tool_with_retry(
    client: &arrowhead::mcp_api::MCPClientApi,
    tool_name: &str,
    args: impl Into<arrowhead::mcp_api::ToolArguments> + Clone,
    max_retries: u32,
) -> Result<serde_json::Value, MCPError> {
    let mut retries = 0;
    let args = args.into();

    loop {
        match client.call_tool(tool_name, args.clone()).await {
            Ok(result) => return Ok(result),
            Err(MCPError::Timeout(_)) | Err(MCPError::Connection(_)) if retries < max_retries => {
                retries += 1;
                println!("   🔄 Retry {}/{}", retries, max_retries);
                sleep(Duration::from_millis(1000 * retries as u64)).await;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}

/// Demonstrate feature flag management
async fn feature_flag_management_example(client: &arrowhead::mcp_api::MCPClientApi) -> Result<(), MCPError> {
    println!("\n🏁 Feature Flag Management");
    println!("=========================");

    // Check current feature flags
    let flags = client.get_feature_flags().await?;
    println!("📋 Current feature flags:");
    for (name, flag) in &flags {
        println!("   {}: {} ({})", 
                name, 
                if flag.enabled { "✅" } else { "❌" },
                flag.description.as_deref().unwrap_or("No description"));
    }

    // Enable an experimental feature
    if let Err(e) = client.set_feature_flag("experimental.streaming", true).await {
        println!("⚠️ Could not enable experimental feature: {}", e);
    } else {
        println!("✅ Enabled experimental streaming");
    }

    // Check if specific features are supported
    let critical_features = vec![
        "tools.list",
        "tools.call",
        "resources.read",
    ];

    println!("\n🔍 Checking critical features:");
    for feature in critical_features {
        let enabled = client.is_feature_enabled(feature).await?;
        if enabled {
            println!("   ✅ {}", feature);
        } else {
            println!("   ❌ {} (not supported)", feature);
        }
    }

    Ok(())
}