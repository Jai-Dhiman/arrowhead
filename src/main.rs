use arrowhead::cli::Cli;
use arrowhead::obsidian_adapter::ObsidianAdapter;
use arrowhead::router::route_command;
use arrowhead::config::Config;
use arrowhead::gemini_client::{GeminiClient, GeminiConfig};
use arrowhead::openai_client::{OpenAIClient, OpenAIConfig};
use arrowhead::ai_conversation::{Message, MessageRole, AIConversationEngine};
use clap::Parser;
use std::io::{self, Write};
use chrono::Utc;
use uuid::Uuid;
use serde_json;

#[tokio::main]
async fn main() {
    // Initialize Obsidian Adapter
    let api_key = std::env::var("OBSIDIAN_API_KEY").ok();
    let base_url = std::env::var("OBSIDIAN_BASE_URL")
        .ok()
        .or_else(|| Some("https://127.0.0.1:27124".to_string()));
    let adapter = ObsidianAdapter::new(base_url, api_key);

    // Parse CLI arguments
    let cli_args = Cli::parse();
    
    // Check if a specific command was provided
    if cli_args.command.is_some() {
        // Traditional CLI mode - execute the specific command
        if let Err(e) = route_command(cli_args, &adapter).await {
            eprintln!("Error: {:?}", e);
            std::process::exit(1);
        }
    } else {
        // No command provided - start interactive chat mode
        if let Err(e) = run_interactive_chat_mode(&adapter).await {
            eprintln!("Error in interactive mode: {:?}", e);
            std::process::exit(1);
        }
    }
}

/// Run the application in interactive chat mode (similar to Claude Code)
async fn run_interactive_chat_mode(_adapter: &ObsidianAdapter) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Welcome to Arrowhead!");
    println!("I'm your AI-powered productivity assistant. Ask me anything about your tasks, goals, and notes.");
    println!("You can also use traditional commands like 'arrowhead todo add' in another terminal.");
    println!("Type 'help' for assistance, or 'quit'/'exit' to stop.\n");

    // Load configuration
    let config = Config::load().unwrap_or_default();
    
    // Create AI client for conversation
    let llm_client = match create_llm_client(&config) {
        Ok(client) => client,
        Err(e) => {
            show_setup_guide(&e);
            std::process::exit(1);
        }
    };
    
    // Create AI conversation engine
    let mut ai_engine = AIConversationEngine::new(llm_client);
    
    // Add system message to provide context
    let system_message = Message {
        id: Uuid::new_v4().to_string(),
        role: MessageRole::System,
        content: "You are Arrowhead, an AI-powered productivity assistant. You help users manage their tasks, goals, and notes through natural conversation. Be helpful, concise, and friendly. If users ask about specific productivity features, you can explain what Arrowhead can do, but focus on having a natural conversation.".to_string(),
        timestamp: Utc::now(),
        function_call: None,
    };
    ai_engine.context.add_message(system_message);
    
    // Main interaction loop
    loop {
        // Prompt user for input (Claude Code style)
        print!("💬 ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        let bytes_read = io::stdin().read_line(&mut input)?;
        
        // Handle EOF (Ctrl+D or piped input ending)
        if bytes_read == 0 {
            println!("\n👋 Goodbye!");
            break;
        }
        
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
        
        // Handle setup command
        if input.eq_ignore_ascii_case("setup") {
            println!("🔧 Configuration Setup:");
            println!("Run: arrowhead config --init");
            println!("Then set: export GEMINI_API_KEY=\"your_key\"");
            continue;
        }
        
        // Skip empty input
        if input.is_empty() {
            continue;
        }
        
        // Show loading spinner
        print!("⏳ ");
        io::stdout().flush()?;
        
        // Send directly to LLM
        match ai_engine.send_message(input.to_string()).await {
            Ok(response) => {
                // Clear the loading spinner line
                print!("\r   \r");
                io::stdout().flush()?;
                
                // Try to parse and clean up the response if it's JSON
                let clean_response = if response.starts_with('{') && response.ends_with('}') {
                    match serde_json::from_str::<serde_json::Value>(&response) {
                        Ok(json) => {
                            // Try to extract common response fields
                            json.get("response")
                                .or_else(|| json.get("message"))
                                .or_else(|| json.get("content"))
                                .or_else(|| json.get("text"))
                                .and_then(|v| v.as_str())
                                .unwrap_or(&response)
                                .to_string()
                        }
                        Err(_) => response
                    }
                } else {
                    response
                };
                
                println!("{}", clean_response);
                println!(); // Add blank line for readability
            }
            Err(e) => {
                // Clear the loading spinner line
                print!("\r   \r");
                io::stdout().flush()?;
                
                let error_msg = e.to_string();
                if error_msg.contains("503") || error_msg.contains("Service Unavailable") || error_msg.contains("overloaded") {
                    println!("🔄 The AI service is temporarily busy. This usually resolves in a few minutes.");
                    println!("💡 Tips while waiting:");
                    println!("   • Try again in 30 seconds");
                    println!("   • Use traditional commands: `arrowhead todo list`");
                    println!("   • Check service status: The API may be experiencing high demand");
                } else {
                    println!("❌ Error processing your request: {}", e);
                    println!("Please try rephrasing your request or type 'help' for assistance.");
                }
                println!();
            }
        }
    }
    
    Ok(())
}

/// Show help information
fn show_help() {
    println!("# Arrowhead Help\n");
    println!("I can help you with productivity tasks using natural language. Here are some examples:\n");
    println!("**Task Management:**");
    println!("• \"Add a todo to finish the project by Friday\"");
    println!("• \"Show me my overdue tasks\"");
    println!("• \"Mark the project review task as done\"\n");
    println!("**Goals & Planning:**");
    println!("• \"Create a goal to learn Rust\"");
    println!("• \"Show my progress on current goals\"");
    println!("• \"Update my learning goal status\"\n");
    println!("**Notes & Knowledge:**");
    println!("• \"Create a note about today's meeting\"");
    println!("• \"Find notes related to project planning\"");
    println!("• \"Add content to my meeting notes\"\n");
    println!("**Commands:**");
    println!("• `help` - Show this help");
    println!("• `setup` - Show configuration setup guide");
    println!("• `quit` or `exit` - Exit interactive mode");
    println!("• Traditional CLI: `arrowhead todo list`, `arrowhead goal add`, etc.\n");
    println!("Just ask me naturally what you'd like to do - I'll figure out the right command!\n");
}

/// Create LLM client based on configuration
fn create_llm_client(config: &Config) -> Result<Box<dyn arrowhead::ai_conversation::LLMClient>, Box<dyn std::error::Error>> {
    // Validate configuration
    config.validate()?;
    
    match config.llm.provider.as_str() {
        "gemini" => {
            let api_key = config.get_llm_api_key()
                .ok_or("Missing Gemini API key")?;
            
            let gemini_config = GeminiConfig {
                api_key,
                model: config.get_llm_model(),
                temperature: Some(config.get_llm_temperature()),
                max_tokens: Some(config.get_llm_max_tokens()),
                ..Default::default()
            };
            
            let client = GeminiClient::new(gemini_config)?;
            Ok(Box::new(client))
        }
        "openai" => {
            let api_key = config.get_llm_api_key()
                .ok_or("Missing OpenAI API key")?;
            
            let openai_config = OpenAIConfig {
                api_key,
                model: config.get_llm_model(),
                temperature: Some(config.get_llm_temperature()),
                max_tokens: Some(config.get_llm_max_tokens()),
                ..Default::default()
            };
            
            let client = OpenAIClient::new(openai_config)?;
            Ok(Box::new(client))
        }
        _ => {
            Err(format!("Unsupported LLM provider: {}", config.llm.provider).into())
        }
    }
}

/// Show setup guide when API key is not configured
fn show_setup_guide(error: &Box<dyn std::error::Error>) {
    println!();
    println!("🚀 Welcome to Arrowhead!");
    println!("═══════════════════════════════════════════════════════════════");
    println!();
    println!("❌ Setup Required: {}", error);
    println!();
    println!("📋 Quick Setup Guide:");
    println!();
    println!("1️⃣  Get a Gemini API key (FREE):");
    println!("   → Visit: https://aistudio.google.com/app/apikey");
    println!("   → Sign in with Google account");
    println!("   → Click 'Create API Key'");
    println!();
    println!("2️⃣  Set your API key:");
    println!();
    println!("   Option A - Environment Variable (Recommended):");
    println!("   export GEMINI_API_KEY=\"your_api_key_here\"");
    println!();
    println!("   Option B - Configuration File:");
    println!("   arrowhead config --init");
    println!("   # Then edit ~/.config/arrowhead/config.toml");
    println!();
    println!("3️⃣  Test the setup:");
    println!("   arrowhead config --show");
    println!();
    println!("4️⃣  Start using Arrowhead:");
    println!("   arrowhead  # Interactive mode");
    println!();
    println!("💡 Examples of what you can do:");
    println!("   • \"Add a todo to finish the project by Friday\"");
    println!("   • \"Show me my overdue tasks\"");
    println!("   • \"Create a goal to learn Rust\"");
    println!("   • \"Make a note about today's meeting\"");
    println!();
    println!("💰 Why Gemini? It's cost-effective (~1.5x cheaper than GPT-4)");
    println!("   and has a huge 1M token context window!");
    println!();
    println!("🆘 Need help? Run: arrowhead config --init");
    println!("═══════════════════════════════════════════════════════════════");
}
