use arrowhead::nl_cli_bridge::{NLCLIBridge, NLBridgeConfig};
use arrowhead::obsidian_adapter::ObsidianAdapter;
use arrowhead::gemini_client::{GeminiClient, GeminiConfig};
use arrowhead::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Testing AI agent without MockLLM");
    
    // Load configuration
    let config = Config::load().unwrap_or_default();
    
    // Create two separate AI clients
    let command_client = {
        let api_key = config.get_llm_api_key()
            .ok_or("Missing Gemini API key")?;
        
        let gemini_config = GeminiConfig {
            api_key,
            model: config.get_llm_model(),
            temperature: Some(0.7),
            max_tokens: Some(1000),
            ..Default::default()
        };
        
        Box::new(GeminiClient::new(gemini_config)?)
    };
    
    let conversation_client = {
        let api_key = config.get_llm_api_key()
            .ok_or("Missing Gemini API key")?;
        
        let gemini_config = GeminiConfig {
            api_key,
            model: config.get_llm_model(),
            temperature: Some(0.7),
            max_tokens: Some(1000),
            ..Default::default()
        };
        
        Box::new(GeminiClient::new(gemini_config)?)
    };
    
    // Create the bridge with both clients
    let bridge_config = NLBridgeConfig::default();
    let mut bridge = NLCLIBridge::new(command_client, conversation_client, bridge_config.clone())?;
    
    // Create adapter
    let adapter = ObsidianAdapter::new(Some("https://127.0.0.1:27124".to_string()), None);
    
    // Start a session
    let session_id = bridge.start_session(None).await?;
    println!("✅ Session started: {}", session_id);
    
    // Test command parsing
    let test_input = "add a todo to finish my project";
    println!("🧪 Testing command: {}", test_input);
    
    match bridge.process_input(&session_id, test_input, &adapter, &bridge_config).await {
        Ok(response) => {
            println!("✅ Response: {}", response.response_text);
            println!("📝 Commands: {:?}", response.executed_commands);
            println!("💡 Suggestions: {:?}", response.suggestions);
            
            if response.execution_successful {
                println!("✅ Execution successful!");
            } else {
                println!("⚠️  Execution had issues");
                if let Some(error) = response.error_message {
                    println!("   Error: {}", error);
                }
            }
        }
        Err(e) => {
            println!("❌ Error: {}", e);
        }
    }
    
    // Test conversational input
    let test_input2 = "hello";
    println!("\n🧪 Testing conversational input: {}", test_input2);
    
    match bridge.process_input(&session_id, test_input2, &adapter, &bridge_config).await {
        Ok(response) => {
            println!("✅ Response: {}", response.response_text);
            println!("📝 Commands: {:?}", response.executed_commands);
        }
        Err(e) => {
            println!("❌ Error: {}", e);
        }
    }
    
    // End the session
    bridge.end_session(&session_id).await?;
    
    println!("🎉 Test completed successfully!");
    
    Ok(())
}