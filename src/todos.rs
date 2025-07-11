use anyhow::{Result, Context};
use crate::cli::{TodoAction, TodoArgs};
use crate::obsidian_adapter::ObsidianAdapter;
use crate::utils::slugify; // Import slugify from utils
use serde::Serialize; // For serializing frontmatter

#[derive(Serialize)]
struct TodoFrontmatter {
    due_date: Option<String>,
    tags: Option<Vec<String>>,
    status: String,
}

pub async fn handle_todo_command(args: TodoArgs, adapter: &ObsidianAdapter) -> Result<()> {
    let todos_dir = "Todos"; // Define a base directory for todos

    match args.action {
        TodoAction::Add { description, due_date, tags } => {
            println!("Attempting to add todo: '{}'", description);

            let fm_tags = if tags.is_empty() { None } else { Some(tags.clone()) };
            let frontmatter = TodoFrontmatter {
                due_date: due_date.clone(),
                tags: fm_tags,
                status: "open".to_string(),
            };

            let fm_yaml = serde_yaml::to_string(&frontmatter)
                .context("Failed to serialize todo frontmatter to YAML")?;

            let content = format!("---\n{}---\n\n- [ ] {}", fm_yaml.trim(), description);

            let slug = slugify(&description);
            // Ensure slug is not too long for a filename
            let max_slug_len = 50;
            let truncated_slug = if slug.len() > max_slug_len {
                slug.chars().take(max_slug_len).collect()
            } else {
                slug
            };

            let file_name = format!("{}/{}.md", todos_dir, truncated_slug);

            adapter.create_file(&file_name, &content).await
                .context(format!("Failed to create todo file '{}'", file_name))?;

            println!("Todo '{}' created as '{}'.", description, file_name);
        }
        TodoAction::List { status } => {
            println!("Listing todos...");
            if let Some(ref s) = status {
                println!("  Status filter: {}", s);
            }
            
            // Get all files in the Todos directory
            match adapter.list_files_in_folder(todos_dir).await {
                Ok(files) => {
                    if files.is_empty() {
                        println!("No todos found in {}/", todos_dir);
                        return Ok(());
                    }
                    
                    println!("Found {} todo files:", files.len());
                    
                    // Filter and display todos
                    let mut found_todos = 0;
                    
                    for file in files {
                        if !file.ends_with(".md") {
                            continue;
                        }
                        
                        let file_path = format!("{}/{}", todos_dir, file);
                        
                        // Try to get the file content
                        match adapter.get_file(&file_path).await {
                            Ok(content) => {
                                // Parse the todo content
                                let todo_status = extract_todo_status(&content);
                                let todo_checkbox = extract_checkbox_status(&content);
                                let todo_description = extract_todo_description(&content);
                                let todo_due_date = extract_due_date(&content);
                                let todo_tags = extract_tags(&content);
                                
                                // Apply status filter if provided
                                let should_show = if let Some(filter_status) = &status {
                                    match filter_status.as_str() {
                                        "open" => todo_status == "open" || todo_checkbox == "[ ]",
                                        "done" => todo_status == "done" || todo_checkbox == "[x]",
                                        _ => true,
                                    }
                                } else {
                                    true
                                };
                                
                                if should_show {
                                    found_todos += 1;
                                    
                                    // Extract the file name without extension for display
                                    let display_name = file.strip_suffix(".md").unwrap_or(&file);
                                    
                                    // Format the output
                                    let status_indicator = if todo_checkbox == "[x]" || todo_status == "done" {
                                        "✓"
                                    } else {
                                        "•"
                                    };
                                    
                                    println!("{} {} ({})", status_indicator, display_name, todo_description);
                                    
                                    if let Some(due_date) = todo_due_date {
                                        println!("    Due: {}", due_date);
                                    }
                                    
                                    if !todo_tags.is_empty() {
                                        println!("    Tags: {}", todo_tags.join(", "));
                                    }
                                }
                            }
                            Err(e) => {
                                println!("Warning: Could not read file {}: {}", file_path, e);
                            }
                        }
                    }
                    
                    if found_todos == 0 {
                        if let Some(filter_status) = &status {
                            println!("No todos found with status: {}", filter_status);
                        } else {
                            println!("No todos found.");
                        }
                    } else {
                        println!("\nTotal todos shown: {}", found_todos);
                    }
                }
                Err(e) => {
                    println!("Error listing todos: {}", e);
                    println!("Make sure the Todos directory exists and the MCP server is running.");
                }
            }
        }
        TodoAction::Done { id } => {
            // 'id' here would likely be the filename (e.g., "my-important-task") or a unique ID from frontmatter
            // For now, let's assume 'id' is the slugified filename part.
            let file_name = format!("{}/{}.md", todos_dir, id);
            println!("Attempting to mark todo '{}' as done.", file_name);

            let current_content = adapter.get_file(&file_name).await
                .context(format!("Failed to retrieve todo '{}' for marking as done.", file_name))?;

            // This is a naive replacement. A more robust solution would parse the Markdown.
            let updated_content = current_content.replacen("- [ ]", "- [x]", 1);

            if current_content == updated_content {
                 println!("Todo '{}' might already be marked as done or checkbox not found.", file_name);
            } else {
                // Ideally, also update status in frontmatter
                // For simplicity now, just updating the checkbox
                adapter.update_file(&file_name, &updated_content).await
                    .context(format!("Failed to update todo '{}' to done.", file_name))?;
                println!("Todo '{}' marked as done.", file_name);
            }
        }
        TodoAction::View { id } => {
            // Assume 'id' is the slugified filename part.
            let file_name = format!("{}/{}.md", todos_dir, id);
            println!("Viewing todo '{}'.", file_name);

            let content = adapter.get_file(&file_name).await
                .context(format!("Failed to retrieve todo '{}' for viewing.", file_name))?;

            println!("--- Content of {} ---", file_name);
            println!("{}", content);
            println!("--- End of Content ---");
        }
    }
    Ok(())
}

// Helper functions for parsing todo content

fn extract_todo_status(content: &str) -> String {
    // Look for status in frontmatter
    if let Some(frontmatter_end) = content.find("---\n") {
        if let Some(second_frontmatter_end) = content[frontmatter_end + 4..].find("---\n") {
            let frontmatter = &content[frontmatter_end + 4..frontmatter_end + 4 + second_frontmatter_end];
            
            // Simple parsing - look for "status: " line
            for line in frontmatter.lines() {
                if line.trim().starts_with("status:") {
                    return line.split(':').nth(1).unwrap_or("unknown").trim().to_string();
                }
            }
        }
    }
    "unknown".to_string()
}

fn extract_checkbox_status(content: &str) -> String {
    // Look for checkbox patterns in the content
    if content.contains("- [x]") {
        "[x]".to_string()
    } else if content.contains("- [ ]") {
        "[ ]".to_string()
    } else {
        "none".to_string()
    }
}

fn extract_todo_description(content: &str) -> String {
    // Look for the todo description after the checkbox
    for line in content.lines() {
        if line.contains("- [") {
            // Extract text after the checkbox
            if let Some(pos) = line.find(']') {
                return line[pos + 1..].trim().to_string();
            }
        }
    }
    
    // Fallback: look for the first non-frontmatter line
    let mut in_frontmatter = false;
    let mut frontmatter_ended = false;
    
    for line in content.lines() {
        if line.trim() == "---" {
            if !in_frontmatter {
                in_frontmatter = true;
            } else {
                frontmatter_ended = true;
            }
            continue;
        }
        
        if frontmatter_ended && !line.trim().is_empty() {
            return line.trim().to_string();
        }
    }
    
    "No description found".to_string()
}

fn extract_due_date(content: &str) -> Option<String> {
    // Look for due_date in frontmatter
    if let Some(frontmatter_end) = content.find("---\n") {
        if let Some(second_frontmatter_end) = content[frontmatter_end + 4..].find("---\n") {
            let frontmatter = &content[frontmatter_end + 4..frontmatter_end + 4 + second_frontmatter_end];
            
            // Simple parsing - look for "due_date: " line
            for line in frontmatter.lines() {
                if line.trim().starts_with("due_date:") {
                    let due_date = line.split(':').nth(1).unwrap_or("").trim();
                    if !due_date.is_empty() && due_date != "null" {
                        return Some(due_date.to_string());
                    }
                }
            }
        }
    }
    None
}

fn extract_tags(content: &str) -> Vec<String> {
    // Look for tags in frontmatter
    if let Some(frontmatter_end) = content.find("---\n") {
        if let Some(second_frontmatter_end) = content[frontmatter_end + 4..].find("---\n") {
            let frontmatter = &content[frontmatter_end + 4..frontmatter_end + 4 + second_frontmatter_end];
            
            // Simple parsing - look for "tags: " line
            for line in frontmatter.lines() {
                if line.trim().starts_with("tags:") {
                    let tags_str = line.split(':').nth(1).unwrap_or("").trim();
                    if !tags_str.is_empty() && tags_str != "null" {
                        // Handle both array format and simple format
                        if tags_str.starts_with('[') && tags_str.ends_with(']') {
                            // Array format: [tag1, tag2, tag3]
                            let tags_content = &tags_str[1..tags_str.len() - 1];
                            return tags_content
                                .split(',')
                                .map(|s| s.trim().trim_matches('"').to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                        } else {
                            // Simple format: tag1, tag2, tag3
                            return tags_str
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                        }
                    }
                }
            }
        }
    }
    Vec::new()
}
