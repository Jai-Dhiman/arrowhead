use arrowhead::conversational_interface::{ConversationalInterface, ConversationalInterfaceConfig};
use arrowhead::nl_command_parser::{NLCommandParser, NLParserConfig};
use arrowhead::gemini_client::{GeminiClient, GeminiConfig};
use arrowhead::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = Config::load().unwrap_or_default();
    
    // Create Gemini client
    let api_key = config.get_llm_api_key()
        .ok_or("Missing Gemini API key")?;
    
    let gemini_config = GeminiConfig {
        api_key,
        model: config.get_llm_model(),
        temperature: Some(0.7),
        max_tokens: Some(1000),
        ..Default::default()
    };
    
    let gemini_client = GeminiClient::new(gemini_config)?;
    
    // Create parser
    let nl_parser = NLCommandParser::new(Box::new(gemini_client));
    
    // Create conversational interface
    let conv_config = ConversationalInterfaceConfig::default();
    let mut conv_interface = ConversationalInterface::new(nl_parser, conv_config)?;
    
    // Start a session
    let session_id = conv_interface.start_session(None).await?;
    
    // Test processing
    let test_input = "add a todo to finish my project";
    println!("Testing input: {}", test_input);
    
    match conv_interface.process_input(&session_id, test_input).await {
        Ok(response) => {
            println!("Response text: {}", response.text);
            println!("Commands: {:?}", response.commands);
            println!("Suggestions: {:?}", response.suggestions);
        }
        Err(e) => {
            println!("Processing failed: {}", e);
        }
    }
    
    Ok(())
}