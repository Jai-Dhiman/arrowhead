use arrowhead::cli::Cli;
use arrowhead::obsidian_adapter::ObsidianAdapter;
use arrowhead::router::route_command;
use arrowhead::nl_cli_bridge::{NLCLIBridge, NLBridgeConfig};
use arrowhead::ai_conversation::AIConversationEngine;
use clap::Parser;
use std::io::{self, Write};

#[tokio::main]
async fn main() {
    // Check if we should run in natural language mode
    let args: Vec<String> = std::env::args().collect();
    let nl_mode = args.contains(&"--nl".to_string()) || args.contains(&"--natural-language".to_string());
    
    // Initialize Obsidian Adapter
    let api_key = std::env::var("OBSIDIAN_API_KEY").ok();
    let base_url = std::env::var("OBSIDIAN_BASE_URL")
        .ok()
        .or_else(|| Some("https://127.0.0.1:27124".to_string()));
    let adapter = ObsidianAdapter::new(base_url, api_key);

    if nl_mode {
        // Run in natural language mode
        if let Err(e) = run_natural_language_mode(&adapter).await {
            eprintln!("Error in natural language mode: {:?}", e);
            std::process::exit(1);
        }
    } else {
        // Parse CLI arguments and run in traditional mode
        let cli_args = Cli::parse();
        
        // Route command to appropriate module
        if let Err(e) = route_command(cli_args, &adapter).await {
            eprintln!("Error: {:?}", e);
            // Consider more user-friendly error reporting here
            // For example, distinguishing between client errors and internal errors.
            std::process::exit(1);
        }
    }
}

/// Run the application in natural language mode
async fn run_natural_language_mode(adapter: &ObsidianAdapter) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Welcome to Arrowhead Natural Language Interface!");
    println!("You can now use natural language to interact with your tasks, goals, and notes.");
    println!("Type 'help' for assistance, 'quit' or 'exit' to stop.\n");

    // Create AI conversation engine (using a simple mock for now)
    let llm_client = Box::new(create_mock_llm_client());
    
    // Initialize NL CLI bridge
    let bridge_config = NLBridgeConfig::default();
    let mut bridge = NLCLIBridge::new(llm_client, bridge_config.clone())?;
    
    // Start a conversation session
    let session_id = bridge.start_session(None).await?;
    
    // Main interaction loop
    loop {
        // Prompt user for input
        print!("🎯 You: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        // Check for exit commands
        if input.eq_ignore_ascii_case("quit") || input.eq_ignore_ascii_case("exit") {
            println!("👋 Goodbye!");
            break;
        }
        
        // Handle help command
        if input.eq_ignore_ascii_case("help") {
            show_help();
            continue;
        }
        
        // Skip empty input
        if input.is_empty() {
            continue;
        }
        
        // Process the input
        match bridge.process_input(&session_id, input, adapter, &bridge_config).await {
            Ok(response) => {
                println!("🤖 Assistant: {}", response.response_text);
                
                // Show suggestions if available
                if !response.suggestions.is_empty() {
                    println!("\n💡 Suggestions:");
                    for suggestion in &response.suggestions {
                        println!("   • {}", suggestion);
                    }
                }
                
                // Show help suggestions if available
                if !response.help_suggestions.is_empty() {
                    println!("\n🎯 Quick Tips:");
                    for (i, help_suggestion) in response.help_suggestions.iter().enumerate().take(3) {
                        println!("   {}. {}", i + 1, help_suggestion.text);
                        if let Some(ref example) = help_suggestion.example {
                            println!("      Try: \"{}\"", example);
                        }
                    }
                }
                
                // Show error if execution failed
                if let Some(error) = response.error_message {
                    println!("⚠️  Error: {}", error);
                }
                
                println!(); // Add blank line for readability
            }
            Err(e) => {
                println!("❌ Error processing your request: {}", e);
                println!("Please try rephrasing your request or type 'help' for assistance.\n");
            }
        }
    }
    
    // End the session
    bridge.end_session(&session_id).await?;
    
    Ok(())
}

/// Show help information
fn show_help() {
    println!("🔧 Arrowhead Natural Language Interface Help\n");
    println!("You can use natural language to:");
    println!("  📝 Manage todos: 'Add a todo to finish the project'");
    println!("  🎯 Manage goals: 'Create a goal to learn Rust'");
    println!("  📓 Manage notes: 'Create a note about the meeting'");
    println!("  📋 View items: 'Show me my todos' or 'List my goals'");
    println!("  ✅ Complete tasks: 'Mark task 5 as done'");
    println!("\nSpecial commands:");
    println!("  'help' - Show this help message");
    println!("  'quit' or 'exit' - Exit the application");
    println!("\nTips:");
    println!("  • Be specific about what you want to do");
    println!("  • You can include details like due dates and tags");
    println!("  • If something is unclear, I'll ask for clarification");
    println!();
}

/// Create a mock LLM client for demonstration
fn create_mock_llm_client() -> impl arrowhead::ai_conversation::LLMClient {
    use arrowhead::ai_conversation::{LLMClient, Message, MessageRole};
    use async_trait::async_trait;
    use chrono::Utc;
    use anyhow::Result;
    
    struct MockLLMClient;
    
    #[async_trait]
    impl LLMClient for MockLLMClient {
        async fn send_message(&self, messages: Vec<Message>) -> Result<Message> {
            // Simple response based on input content
            let content = messages.last().unwrap().content.to_lowercase();
            
            let response_content = if content.contains("add") && content.contains("todo") {
                r#"{"intent": "add_todo", "entities": {"description": "finish the project"}, "confidence": 0.9, "alternatives": [], "needs_disambiguation": false}"#
            } else if content.contains("list") && content.contains("todo") {
                r#"{"intent": "list_todos", "entities": {}, "confidence": 0.9, "alternatives": [], "needs_disambiguation": false}"#
            } else if content.contains("add") && content.contains("goal") {
                r#"{"intent": "add_goal", "entities": {"title": "learn Rust"}, "confidence": 0.9, "alternatives": [], "needs_disambiguation": false}"#
            } else if content.contains("create") && content.contains("note") {
                r#"{"intent": "create_note", "entities": {"title": "meeting notes"}, "confidence": 0.9, "alternatives": [], "needs_disambiguation": false}"#
            } else {
                "I understand. How can I help you with your tasks, goals, or notes?"
            };
            
            Ok(Message {
                id: "mock".to_string(),
                role: MessageRole::Assistant,
                content: response_content.to_string(),
                timestamp: Utc::now(),
                function_call: None,
            })
        }

        async fn stream_response(&self, messages: Vec<Message>) -> Result<tokio::sync::mpsc::Receiver<String>> {
            let (tx, rx) = tokio::sync::mpsc::channel(100);
            let response = self.send_message(messages).await?;
            let _ = tx.send(response.content).await;
            Ok(rx)
        }

        async fn function_calling(&self, messages: Vec<Message>, _functions: Vec<arrowhead::ai_conversation::FunctionSchema>) -> Result<Message> {
            self.send_message(messages).await
        }

        fn get_model_name(&self) -> String {
            "mock-conversational-model".to_string()
        }
    }
    
    MockLLMClient
}
