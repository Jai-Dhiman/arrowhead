use anyhow::Result;
use crate::cli::{Cli, Commands};
use crate::obsidian_adapter::ObsidianAdapter;
use crate::todos::handle_todo_command;
use crate::notes::handle_note_command;
use crate::goals::handle_goal_command;
use crate::config::Config;

pub async fn route_command(cli: Cli, adapter: &ObsidianAdapter) -> Result<()> {
    match cli.command {
        Some(Commands::Todo(todo_args)) => {
            handle_todo_command(todo_args, adapter).await
        }
        Some(Commands::Note(note_args)) => {
            handle_note_command(note_args, adapter).await
        }
        Some(Commands::Goal(goal_args)) => {
            handle_goal_command(goal_args, adapter).await
        }
        Some(Commands::Config(config_args)) => {
            handle_config_command(config_args).await
        }
        None => {
            // No command provided, this will be handled in main.rs by starting interactive mode
            Ok(())
        }
    }
}

async fn handle_config_command(config_args: crate::cli::ConfigArgs) -> Result<()> {
    if config_args.init {
        println!("Creating sample configuration file...");
        Config::create_sample_config()?;
        println!("✅ Sample configuration created!");
        println!("Set your API key with: export GEMINI_API_KEY=\"your_key_here\"");
        println!("Or edit the config file to add your API key directly.");
    } else if config_args.show {
        println!("Current configuration:");
        match Config::load() {
            Ok(config) => {
                println!("LLM Provider: {}", config.llm.provider);
                println!("Model: {}", config.get_llm_model());
                println!("Temperature: {}", config.get_llm_temperature());
                println!("Max Tokens: {}", config.get_llm_max_tokens());
                println!("Obsidian URL: {}", config.obsidian.base_url);
                
                // Don't print API keys for security
                println!("Gemini API Key: {}", 
                    if config.llm.gemini.api_key.is_some() { "Set ✅" } else { "Not set ❌" });
                println!("Obsidian API Key: {}", 
                    if config.obsidian.api_key.is_some() { "Set ✅" } else { "Not set ❌" });
            }
            Err(e) => {
                println!("Error loading configuration: {}", e);
                println!("Use 'arrowhead config --init' to create a sample configuration.");
            }
        }
    } else if let (Some(key), Some(value)) = (config_args.set.as_ref(), config_args.value.as_ref()) {
        // Handle --set command
        let mut config = Config::load().unwrap_or_default();
        
        match config.set_value(key, value) {
            Ok(()) => {
                config.save()?;
                
                // Special handling for API keys - don't show the actual value
                if key.ends_with(".api_key") {
                    println!("✅ Configuration updated: {} = [REDACTED]", key);
                } else {
                    println!("✅ Configuration updated: {} = {}", key, value);
                }
            }
            Err(e) => {
                println!("❌ Error setting configuration: {}", e);
                println!("\nAvailable configuration keys:");
                for available_key in Config::get_available_keys() {
                    println!("  {}", available_key);
                }
            }
        }
    } else if config_args.set.is_some() {
        println!("❌ Error: --set requires both --set and --value arguments");
        println!("\nUsage:");
        println!("  arrowhead config --set gemini.api_key --value \"your_api_key\"");
        println!("\nAvailable configuration keys:");
        for key in Config::get_available_keys() {
            println!("  {}", key);
        }
    } else {
        println!("Usage:");
        println!("  arrowhead config --init                              Create sample configuration");
        println!("  arrowhead config --show                              Show current configuration");
        println!("  arrowhead config --set <key> --value <value>         Set configuration value");
        println!("\nExamples:");
        println!("  arrowhead config --set gemini.api_key --value \"your_api_key\"");
        println!("  arrowhead config --set provider --value \"openai\"");
        println!("  arrowhead config --set gemini.temperature --value \"0.8\"");
        println!("\nAvailable configuration keys:");
        for key in Config::get_available_keys() {
            println!("  {}", key);
        }
    }
    Ok(())
}
