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
    let parser_config = NLParserConfig::default();
    let mut parser = NLCommandParser::new(Box::new(gemini_client));
    
    // Test parsing
    let test_input = "add a todo to finish my project";
    println!("Testing input: {}", test_input);
    
    match parser.parse_command(test_input).await {
        Ok(parsed) => {
            println!("Parsed successfully!");
            println!("Intent: {}", parsed.intent);
            println!("Entities: {:?}", parsed.entities);
            println!("Confidence: {}", parsed.confidence);
        }
        Err(e) => {
            println!("Parsing failed: {}", e);
        }
    }
    
    Ok(())
}