use arrowhead::config::Config;
use arrowhead::gemini_client::GeminiClient;
use arrowhead::ai_conversation::{LLMClient, Message, MessageRole};
use chrono::Utc;
use std::env;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Check if API key is available
    let api_key = env::var("GEMINI_API_KEY").map_err(|_| {
        "GEMINI_API_KEY environment variable not set. Please set it to test the Gemini client."
    })?;

    // Create Gemini client configuration
    let config = arrowhead::gemini_client::GeminiConfig {
        api_key,
        model: "gemini-1.5-flash".to_string(),
        temperature: Some(0.7),
        max_tokens: Some(1024),
        ..Default::default()
    };

    // Create the client
    let client = GeminiClient::new(config)?;

    // Test message for natural language command parsing
    let test_message = Message {
        id: Uuid::new_v4().to_string(),
        role: MessageRole::User,
        content: "Add a todo to finish the project by Friday".to_string(),
        timestamp: Utc::now(),
        function_call: None,
    };

    // System message for command parsing
    let system_message = Message {
        id: Uuid::new_v4().to_string(),
        role: MessageRole::System,
        content: r#"You are a natural language command parser. Parse the given command and return a structured JSON response with intent, entities, confidence score, and alternatives for disambiguation.

Example response format:
{
  "intent": "add_todo",
  "entities": {
    "description": "finish the project",
    "due_date": "Friday"
  },
  "confidence": 0.9,
  "alternatives": [],
  "needs_disambiguation": false
}"#.to_string(),
        timestamp: Utc::now(),
        function_call: None,
    };

    println!("Testing Gemini 1.5 Flash client...");
    println!("User message: {}", test_message.content);
    println!("Sending request to Gemini API...");

    // Send the message
    let messages = vec![system_message, test_message];
    match client.send_message(messages).await {
        Ok(response) => {
            println!("✅ Success! Gemini API response:");
            println!("Model: {}", client.get_model_name());
            println!("Response: {}", response.content);
            
            // Try to parse the JSON response
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&response.content) {
                println!("✅ Valid JSON response received!");
                println!("Parsed JSON: {}", serde_json::to_string_pretty(&parsed)?);
            } else {
                println!("⚠️  Response is not valid JSON, but API call was successful");
            }
        }
        Err(e) => {
            eprintln!("❌ Error calling Gemini API: {}", e);
            return Err(e.into());
        }
    }

    println!("\n🎉 Gemini client test completed successfully!");
    Ok(())
}