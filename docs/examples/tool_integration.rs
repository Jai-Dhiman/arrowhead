/// Tool Integration Examples
/// 
/// This example demonstrates how to integrate with various types of MCP tools:
/// - File processing tools
/// - Database tools
/// - Web scraping tools
/// - AI/ML tools
/// - Custom business logic tools

use arrowhead::mcp_api::{MCPClientBuilder, MCPError, tool_args};
use serde_json::{json, Value};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let mut client = MCPClientBuilder::new()
        .with_stdio_transport()
        .with_timeout(60) // Longer timeout for complex operations
        .with_client_info("tool-integration", "1.0.0")
        .build()?;

    println!("🔌 Connecting to MCP server...");
    client.connect().await?;

    // Demonstrate different tool integration patterns
    file_processing_example(&client).await?;
    database_integration_example(&client).await?;
    web_scraping_example(&client).await?;
    ai_ml_integration_example(&client).await?;
    business_logic_example(&client).await?;
    workflow_orchestration_example(&client).await?;

    client.disconnect().await?;
    println!("✅ All examples completed successfully!");

    Ok(())
}

/// File processing tool integration
async fn file_processing_example(client: &arrowhead::mcp_api::MCPClientApi) -> Result<(), MCPError> {
    println!("\n📁 File Processing Tools");
    println!("========================");

    // 1. List files in a directory
    println!("📋 Listing files...");
    match client.call_tool("list_files", tool_args!(
        "path" => "/home/user/documents",
        "pattern" => "*.txt",
        "recursive" => true
    )).await {
        Ok(result) => {
            println!("✅ Found files: {}", result);
            
            // Process each file if files were returned
            if let Some(files) = result.get("files").and_then(|f| f.as_array()) {
                for file in files.iter().take(3) { // Process first 3 files
                    if let Some(file_path) = file.as_str() {
                        process_single_file(client, file_path).await?;
                    }
                }
            }
        }
        Err(e) => println!("❌ Failed to list files: {}", e),
    }

    // 2. Create a new file
    println!("\n📝 Creating a new file...");
    match client.call_tool("create_file", tool_args!(
        "path" => "/tmp/example.txt",
        "content" => "Hello from MCP client!",
        "encoding" => "utf-8"
    )).await {
        Ok(result) => println!("✅ File created: {}", result),
        Err(e) => println!("❌ Failed to create file: {}", e),
    }

    // 3. File operations with metadata
    println!("\n📊 Getting file metadata...");
    match client.call_tool("file_info", tool_args!(
        "path" => "/tmp/example.txt"
    )).await {
        Ok(result) => {
            println!("✅ File info: {}", result);
            
            // Parse file metadata
            if let Some(size) = result.get("size") {
                println!("   Size: {} bytes", size);
            }
            if let Some(modified) = result.get("modified") {
                println!("   Modified: {}", modified);
            }
        }
        Err(e) => println!("❌ Failed to get file info: {}", e),
    }

    Ok(())
}

/// Helper function to process a single file
async fn process_single_file(client: &arrowhead::mcp_api::MCPClientApi, file_path: &str) -> Result<(), MCPError> {
    println!("🔄 Processing file: {}", file_path);

    // Read file content
    let content = client.call_tool("read_file", tool_args!(
        "path" => file_path,
        "encoding" => "utf-8"
    )).await?;

    // Analyze content
    let analysis = client.call_tool("analyze_text", tool_args!(
        "text" => content.get("content").unwrap_or(&json!("")),
        "options" => {
            "word_count": true,
            "sentiment": true,
            "language": true
        }
    )).await?;

    println!("   📊 Analysis: {}", analysis);
    Ok(())
}

/// Database integration example
async fn database_integration_example(client: &arrowhead::mcp_api::MCPClientApi) -> Result<(), MCPError> {
    println!("\n🗄️ Database Integration");
    println!("======================");

    // 1. Execute a simple query
    println!("📋 Executing query...");
    match client.call_tool("sql_query", tool_args!(
        "query" => "SELECT COUNT(*) as user_count FROM users WHERE active = true",
        "database" => "production"
    )).await {
        Ok(result) => {
            println!("✅ Query result: {}", result);
            
            // Process query results
            if let Some(rows) = result.get("rows").and_then(|r| r.as_array()) {
                for (i, row) in rows.iter().enumerate() {
                    println!("   Row {}: {}", i + 1, row);
                }
            }
        }
        Err(e) => println!("❌ Query failed: {}", e),
    }

    // 2. Parameterized query
    println!("\n🔍 Parameterized query...");
    match client.call_tool("sql_query", tool_args!(
        "query" => "SELECT * FROM orders WHERE user_id = ? AND date >= ? LIMIT ?",
        "params" => [123, "2024-01-01", 10],
        "database" => "production"
    )).await {
        Ok(result) => println!("✅ Parameterized query result: {}", result),
        Err(e) => println!("❌ Parameterized query failed: {}", e),
    }

    // 3. Transaction example
    println!("\n💾 Transaction example...");
    match client.call_tool("sql_transaction", tool_args!(
        "statements" => [
            {
                "query": "INSERT INTO users (name, email) VALUES (?, ?)",
                "params": ["John Doe", "john@example.com"]
            },
            {
                "query": "INSERT INTO user_profiles (user_id, bio) VALUES (LAST_INSERT_ID(), ?)",
                "params": ["Software developer"]
            }
        ],
        "database" => "production"
    )).await {
        Ok(result) => println!("✅ Transaction completed: {}", result),
        Err(e) => println!("❌ Transaction failed: {}", e),
    }

    // 4. Database schema information
    println!("\n📋 Schema information...");
    match client.call_tool("describe_table", tool_args!(
        "table" => "users",
        "database" => "production"
    )).await {
        Ok(result) => {
            println!("✅ Table schema: {}", result);
            
            // Process column information
            if let Some(columns) = result.get("columns").and_then(|c| c.as_array()) {
                println!("   Columns:");
                for column in columns {
                    if let Some(name) = column.get("name") {
                        let col_type = column.get("type").unwrap_or(&json!("unknown"));
                        let nullable = column.get("nullable").unwrap_or(&json!(false));
                        println!("     - {}: {} (nullable: {})", name, col_type, nullable);
                    }
                }
            }
        }
        Err(e) => println!("❌ Failed to get schema: {}", e),
    }

    Ok(())
}

/// Web scraping tool integration
async fn web_scraping_example(client: &arrowhead::mcp_api::MCPClientApi) -> Result<(), MCPError> {
    println!("\n🌐 Web Scraping Integration");
    println!("===========================");

    // 1. Simple web page fetch
    println!("🌍 Fetching web page...");
    match client.call_tool("fetch_url", tool_args!(
        "url" => "https://example.com",
        "timeout" => 30,
        "user_agent" => "MCP-Client/1.0"
    )).await {
        Ok(result) => {
            println!("✅ Page fetched");
            
            // Extract specific data
            if let Some(content) = result.get("content") {
                extract_page_data(client, content).await?;
            }
        }
        Err(e) => println!("❌ Failed to fetch page: {}", e),
    }

    // 2. Extract structured data
    println!("\n📊 Extracting structured data...");
    match client.call_tool("scrape_data", tool_args!(
        "url" => "https://news.example.com",
        "selectors" => {
            "title": "h1.title",
            "author": ".author",
            "date": ".publish-date",
            "content": ".article-body"
        },
        "format" => "json"
    )).await {
        Ok(result) => {
            println!("✅ Structured data extracted: {}", result);
            
            // Process extracted data
            if let Some(articles) = result.get("articles").and_then(|a| a.as_array()) {
                for (i, article) in articles.iter().enumerate() {
                    println!("   Article {}: {}", i + 1, article.get("title").unwrap_or(&json!("No title")));
                }
            }
        }
        Err(e) => println!("❌ Failed to extract data: {}", e),
    }

    // 3. Take screenshot
    println!("\n📸 Taking screenshot...");
    match client.call_tool("screenshot", tool_args!(
        "url" => "https://example.com",
        "width" => 1920,
        "height" => 1080,
        "format" => "png",
        "quality" => 90
    )).await {
        Ok(result) => {
            println!("✅ Screenshot taken");
            if let Some(image_data) = result.get("image_data") {
                println!("   Image size: {} bytes", image_data.as_str().unwrap_or("").len());
            }
        }
        Err(e) => println!("❌ Failed to take screenshot: {}", e),
    }

    Ok(())
}

/// Helper function to extract data from web page content
async fn extract_page_data(client: &arrowhead::mcp_api::MCPClientApi, content: &Value) -> Result<(), MCPError> {
    println!("🔍 Extracting page data...");

    match client.call_tool("extract_links", tool_args!(
        "html" => content,
        "base_url" => "https://example.com",
        "filter" => {
            "internal_only": true,
            "exclude_anchors": true
        }
    )).await {
        Ok(result) => {
            if let Some(links) = result.get("links").and_then(|l| l.as_array()) {
                println!("   Found {} links", links.len());
                for link in links.iter().take(5) {
                    println!("     - {}", link.get("url").unwrap_or(&json!("No URL")));
                }
            }
        }
        Err(e) => println!("   ❌ Failed to extract links: {}", e),
    }

    Ok(())
}

/// AI/ML tool integration
async fn ai_ml_integration_example(client: &arrowhead::mcp_api::MCPClientApi) -> Result<(), MCPError> {
    println!("\n🤖 AI/ML Integration");
    println!("===================");

    // 1. Text analysis
    println!("📝 Text analysis...");
    let sample_text = "The quick brown fox jumps over the lazy dog. This is a sample text for analysis.";
    
    match client.call_tool("analyze_text", tool_args!(
        "text" => sample_text,
        "features" => ["sentiment", "entities", "keywords", "language"]
    )).await {
        Ok(result) => {
            println!("✅ Text analysis: {}", result);
            
            // Process analysis results
            if let Some(sentiment) = result.get("sentiment") {
                println!("   Sentiment: {}", sentiment);
            }
            if let Some(entities) = result.get("entities").and_then(|e| e.as_array()) {
                println!("   Entities: {:?}", entities);
            }
        }
        Err(e) => println!("❌ Text analysis failed: {}", e),
    }

    // 2. Image classification
    println!("\n🖼️ Image classification...");
    match client.call_tool("classify_image", tool_args!(
        "image_path" => "/path/to/image.jpg",
        "model" => "resnet50",
        "top_k" => 5
    )).await {
        Ok(result) => {
            println!("✅ Image classification: {}", result);
            
            if let Some(predictions) = result.get("predictions").and_then(|p| p.as_array()) {
                for (i, pred) in predictions.iter().enumerate() {
                    let label = pred.get("label").unwrap_or(&json!("Unknown"));
                    let confidence = pred.get("confidence").unwrap_or(&json!(0.0));
                    println!("   {}: {} ({:.2}%)", i + 1, label, confidence.as_f64().unwrap_or(0.0) * 100.0);
                }
            }
        }
        Err(e) => println!("❌ Image classification failed: {}", e),
    }

    // 3. Generate embeddings
    println!("\n🔢 Generating embeddings...");
    match client.call_tool("generate_embeddings", tool_args!(
        "texts" => [
            "Machine learning is fascinating",
            "I love programming in Rust",
            "The weather is nice today"
        ],
        "model" => "sentence-transformers/all-MiniLM-L6-v2"
    )).await {
        Ok(result) => {
            println!("✅ Embeddings generated");
            if let Some(embeddings) = result.get("embeddings").and_then(|e| e.as_array()) {
                println!("   Generated {} embeddings", embeddings.len());
                for (i, embedding) in embeddings.iter().enumerate() {
                    if let Some(vector) = embedding.as_array() {
                        println!("   Text {}: {} dimensions", i + 1, vector.len());
                    }
                }
            }
        }
        Err(e) => println!("❌ Embedding generation failed: {}", e),
    }

    Ok(())
}

/// Business logic integration example
async fn business_logic_example(client: &arrowhead::mcp_api::MCPClientApi) -> Result<(), MCPError> {
    println!("\n💼 Business Logic Integration");
    println!("============================");

    // 1. Calculate pricing
    println!("💰 Calculating pricing...");
    match client.call_tool("calculate_pricing", tool_args!(
        "items" => [
            {"id": "item1", "quantity": 2, "unit_price": 19.99},
            {"id": "item2", "quantity": 1, "unit_price": 49.99}
        ],
        "customer_tier" => "premium",
        "currency" => "USD",
        "apply_tax" => true
    )).await {
        Ok(result) => {
            println!("✅ Pricing calculated: {}", result);
            
            if let Some(total) = result.get("total") {
                println!("   Total: ${}", total);
            }
            if let Some(breakdown) = result.get("breakdown") {
                println!("   Breakdown: {}", breakdown);
            }
        }
        Err(e) => println!("❌ Pricing calculation failed: {}", e),
    }

    // 2. Validate business rules
    println!("\n✅ Validating business rules...");
    match client.call_tool("validate_order", tool_args!(
        "order" => {
            "customer_id": 12345,
            "items": [
                {"product_id": "ABC123", "quantity": 5}
            ],
            "shipping_address": {
                "country": "US",
                "state": "CA",
                "zip": "90210"
            }
        },
        "rules" => ["inventory_check", "shipping_restrictions", "customer_limits"]
    )).await {
        Ok(result) => {
            println!("✅ Validation result: {}", result);
            
            if let Some(valid) = result.get("valid") {
                if valid.as_bool().unwrap_or(false) {
                    println!("   ✅ Order is valid");
                } else {
                    println!("   ❌ Order validation failed");
                    if let Some(errors) = result.get("errors") {
                        println!("   Errors: {}", errors);
                    }
                }
            }
        }
        Err(e) => println!("❌ Validation failed: {}", e),
    }

    // 3. Generate reports
    println!("\n📊 Generating reports...");
    match client.call_tool("generate_report", tool_args!(
        "type" => "sales_summary",
        "period" => "last_30_days",
        "format" => "json",
        "filters" => {
            "product_category": "electronics",
            "region": "north_america"
        }
    )).await {
        Ok(result) => {
            println!("✅ Report generated: {}", result);
            
            if let Some(summary) = result.get("summary") {
                println!("   Summary: {}", summary);
            }
        }
        Err(e) => println!("❌ Report generation failed: {}", e),
    }

    Ok(())
}

/// Workflow orchestration example
async fn workflow_orchestration_example(client: &arrowhead::mcp_api::MCPClientApi) -> Result<(), MCPError> {
    println!("\n🔄 Workflow Orchestration");
    println!("========================");

    // Complex workflow: Process customer order
    println!("📋 Processing customer order workflow...");
    
    // Step 1: Validate order
    let validation_result = client.call_tool("validate_order", tool_args!(
        "order_id" => "ORD-12345",
        "customer_id" => 67890
    )).await?;
    
    if validation_result.get("valid").and_then(|v| v.as_bool()).unwrap_or(false) {
        println!("   ✅ Step 1: Order validated");
        
        // Step 2: Check inventory
        let inventory_result = client.call_tool("check_inventory", tool_args!(
            "order_id" => "ORD-12345"
        )).await?;
        
        if inventory_result.get("available").and_then(|v| v.as_bool()).unwrap_or(false) {
            println!("   ✅ Step 2: Inventory available");
            
            // Step 3: Process payment
            let payment_result = client.call_tool("process_payment", tool_args!(
                "order_id" => "ORD-12345",
                "amount" => 99.99,
                "currency" => "USD"
            )).await?;
            
            if payment_result.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
                println!("   ✅ Step 3: Payment processed");
                
                // Step 4: Create shipment
                let shipment_result = client.call_tool("create_shipment", tool_args!(
                    "order_id" => "ORD-12345",
                    "carrier" => "UPS",
                    "service_level" => "ground"
                )).await?;
                
                if let Some(tracking_number) = shipment_result.get("tracking_number") {
                    println!("   ✅ Step 4: Shipment created - Tracking: {}", tracking_number);
                    
                    // Step 5: Send confirmation
                    let _ = client.call_tool("send_notification", tool_args!(
                        "customer_id" => 67890,
                        "type" => "order_confirmation",
                        "data" => {
                            "order_id": "ORD-12345",
                            "tracking_number": tracking_number
                        }
                    )).await?;
                    
                    println!("   ✅ Step 5: Confirmation sent");
                    println!("🎉 Workflow completed successfully!");
                } else {
                    println!("   ❌ Step 4: Shipment creation failed");
                }
            } else {
                println!("   ❌ Step 3: Payment failed");
            }
        } else {
            println!("   ❌ Step 2: Insufficient inventory");
        }
    } else {
        println!("   ❌ Step 1: Order validation failed");
    }

    Ok(())
}