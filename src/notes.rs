use anyhow::{Result, Context};
use crate::cli::{NoteAction, NoteArgs};
use crate::obsidian_adapter::ObsidianAdapter;
use crate::utils::slugify; // Import slugify from utils
use serde::Serialize; // For serializing frontmatter
use std::env;
use std::fs;
use std::process::Command;

#[derive(Serialize)]
struct NoteFrontmatter {
    tags: Option<Vec<String>>,
}

pub async fn handle_note_command(args: NoteArgs, adapter: &ObsidianAdapter) -> Result<()> {
    let notes_dir = "Notes"; // Define a base directory for notes

    match args.action {
        NoteAction::Create { title, content, tags } => {
            println!("Attempting to create note: '{}'", title);

            let fm_tags = if tags.is_empty() { None } else { Some(tags.clone()) };
            let frontmatter = NoteFrontmatter {
                tags: fm_tags,
            };

            let fm_yaml = serde_yaml::to_string(&frontmatter)
                .context("Failed to serialize note frontmatter to YAML")?;

            let note_body = content.unwrap_or_default();
            let full_content = format!("---\n{}---\n\n{}", fm_yaml.trim(), note_body);

            let slug = slugify(&title);
            let max_slug_len = 50; // Keep consistent with todos
            let truncated_slug = if slug.len() > max_slug_len {
                slug.chars().take(max_slug_len).collect()
            } else {
                slug
            };

            let file_name = format!("{}/{}.md", notes_dir, truncated_slug);

            adapter.create_file(&file_name, &full_content).await
                .context(format!("Failed to create note file '{}'", file_name))?;

            println!("Note '{}' created as '{}'.", title, file_name);
        }
        NoteAction::List { tags } => {
            println!("Listing notes...");
            if !tags.is_empty() {
                println!("  Tags filter: {:?}", tags);
            }
            
            // Get all files in the Notes directory
            match adapter.list_files_in_folder(notes_dir).await {
                Ok(files) => {
                    if files.is_empty() {
                        println!("No notes found in {}/", notes_dir);
                        return Ok(());
                    }
                    
                    println!("Found {} note files:", files.len());
                    
                    // Filter and display notes
                    let mut found_notes = 0;
                    
                    for file in files {
                        if !file.ends_with(".md") {
                            continue;
                        }
                        
                        let file_path = format!("{}/{}", notes_dir, file);
                        
                        // Try to get the file content
                        match adapter.get_file(&file_path).await {
                            Ok(content) => {
                                // Parse the note content
                                let note_tags = extract_note_tags(&content);
                                let note_title = extract_note_title(&content, &file);
                                let note_preview = extract_note_preview(&content);
                                
                                // Apply tags filter if provided
                                let should_show = if !tags.is_empty() {
                                    // Check if any of the requested tags match the note's tags
                                    tags.iter().any(|filter_tag| {
                                        note_tags.iter().any(|note_tag| {
                                            note_tag.to_lowercase().contains(&filter_tag.to_lowercase())
                                        })
                                    })
                                } else {
                                    true
                                };
                                
                                if should_show {
                                    found_notes += 1;
                                    
                                    // Extract the file name without extension for display
                                    let display_name = file.strip_suffix(".md").unwrap_or(&file);
                                    
                                    // Format the output
                                    println!("📝 {} ({})", display_name, note_title);
                                    
                                    if !note_tags.is_empty() {
                                        println!("    Tags: {}", note_tags.join(", "));
                                    }
                                    
                                    if !note_preview.is_empty() {
                                        println!("    Preview: {}", note_preview);
                                    }
                                    
                                    println!(); // Add spacing between notes
                                }
                            }
                            Err(e) => {
                                println!("Warning: Could not read file {}: {}", file_path, e);
                            }
                        }
                    }
                    
                    if found_notes == 0 {
                        if !tags.is_empty() {
                            println!("No notes found with tags: {:?}", tags);
                        } else {
                            println!("No notes found.");
                        }
                    } else {
                        println!("Total notes shown: {}", found_notes);
                    }
                }
                Err(e) => {
                    println!("Error listing notes: {}", e);
                    println!("Make sure the Notes directory exists and the MCP server is running.");
                }
            }
        }
        NoteAction::View { name_or_id } => {
            // Assume name_or_id is the slugified filename part
            let file_name = format!("{}/{}.md", notes_dir, slugify(&name_or_id));
            println!("Viewing note: '{}'", file_name);

            let file_content = adapter.get_file(&file_name).await
                .context(format!("Failed to retrieve note '{}' for viewing.", file_name))?;

            println!("--- Content of {} ---", file_name);
            println!("{}", file_content);
            println!("--- End of Content ---");
        }
        NoteAction::Append { name_or_id, content } => {
            let file_name = format!("{}/{}.md", notes_dir, slugify(&name_or_id));
            println!("Appending to note '{}': '{}'", file_name, content.chars().take(50).collect::<String>() + "...");

            let mut current_content = adapter.get_file(&file_name).await
                .context(format!("Failed to retrieve note '{}' for appending.", file_name))?;

            current_content.push_str("\n\n"); // Add some spacing
            current_content.push_str(&content);

            adapter.update_file(&file_name, &current_content).await
                .context(format!("Failed to append content to note '{}'.", file_name))?;

            println!("Content appended to note '{}'.", file_name);
        }
        NoteAction::Edit { name_or_id } => {
            println!("Editing note: '{}'", name_or_id);
            
            // Find the note file
            let file_name = if name_or_id.ends_with(".md") {
                format!("{}/{}", notes_dir, name_or_id)
            } else {
                format!("{}/{}.md", notes_dir, name_or_id)
            };
            
            // Get current content
            let current_content = adapter.get_file(&file_name).await
                .context(format!("Failed to retrieve note '{}' for editing", file_name))?;
            
            // Create backup (for future backup functionality)
            let _backup_content = current_content.clone();
            
            // Edit the note
            match edit_note_content(&current_content).await {
                Ok(Some(new_content)) => {
                    // Only update if content changed
                    if new_content != current_content {
                        adapter.update_file(&file_name, &new_content).await
                            .context(format!("Failed to save edited note '{}'", file_name))?;
                        println!("Note '{}' updated successfully.", file_name);
                    } else {
                        println!("Note '{}' unchanged.", file_name);
                    }
                }
                Ok(None) => {
                    println!("Editing cancelled.");
                }
                Err(e) => {
                    eprintln!("Error editing note: {}", e);
                    eprintln!("Note content remains unchanged.");
                }
            }
        }
    }
    Ok(())
}

/// Opens an external editor to edit the note content
/// Returns Ok(Some(content)) if content was edited and saved
/// Returns Ok(None) if editing was cancelled
/// Returns Err if there was an error during editing
async fn edit_note_content(current_content: &str) -> Result<Option<String>> {
    // Create a temporary file for editing
    let temp_dir = env::temp_dir();
    let temp_file_path = temp_dir.join(format!("arrowhead_note_edit_{}.md", 
        std::process::id()));
    
    // Write current content to temp file
    fs::write(&temp_file_path, current_content)
        .context("Failed to create temporary file for editing")?;
    
    // Determine which editor to use
    let editor = get_editor_command()?;
    
    // Launch the editor
    let mut editor_cmd = Command::new(&editor.program);
    editor_cmd.args(&editor.args);
    editor_cmd.arg(&temp_file_path);
    
    println!("Opening editor: {} (save and exit when done)", editor.program);
    
    let status = editor_cmd.status()
        .context(format!("Failed to launch editor: {}", editor.program))?;
    
    if !status.success() {
        // Clean up temp file
        let _ = fs::remove_file(&temp_file_path);
        return Err(anyhow::anyhow!("Editor exited with non-zero status: {}", status));
    }
    
    // Read the edited content
    let edited_content = fs::read_to_string(&temp_file_path)
        .context("Failed to read edited content from temporary file")?;
    
    // Clean up temp file
    fs::remove_file(&temp_file_path)
        .context("Failed to remove temporary file")?;
    
    Ok(Some(edited_content))
}

/// Represents an editor command with program and arguments
struct EditorCommand {
    program: String,
    args: Vec<String>,
}

/// Determines which editor to use based on environment and platform
fn get_editor_command() -> Result<EditorCommand> {
    // First, try the EDITOR environment variable
    if let Ok(editor_env) = env::var("EDITOR") {
        if !editor_env.is_empty() {
            // Handle editors with arguments (e.g., "code --wait")
            let parts: Vec<&str> = editor_env.split_whitespace().collect();
            if let Some(program) = parts.first() {
                return Ok(EditorCommand {
                    program: program.to_string(),
                    args: parts.iter().skip(1).map(|s| s.to_string()).collect(),
                });
            }
        }
    }
    
    // Platform-specific fallbacks
    #[cfg(target_os = "windows")]
    {
        // On Windows, try notepad as a fallback
        Ok(EditorCommand {
            program: "notepad".to_string(),
            args: vec![],
        })
    }
    
    #[cfg(target_os = "macos")]
    {
        // On macOS, try to use the default text editor
        if Command::new("which").arg("code").output().is_ok() {
            // VS Code is available and commonly used
            Ok(EditorCommand {
                program: "code".to_string(),
                args: vec!["--wait".to_string()],
            })
        } else if Command::new("which").arg("nano").output().is_ok() {
            Ok(EditorCommand {
                program: "nano".to_string(),
                args: vec![],
            })
        } else {
            // Fall back to vim
            Ok(EditorCommand {
                program: "vim".to_string(),
                args: vec![],
            })
        }
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        // On Linux and other Unix-like systems
        if Command::new("which").arg("nano").output().is_ok() {
            Ok(EditorCommand {
                program: "nano".to_string(),
                args: vec![],
            })
        } else if Command::new("which").arg("vim").output().is_ok() {
            Ok(EditorCommand {
                program: "vim".to_string(),
                args: vec![],
            })
        } else {
            // Last resort - try vi (should be available on most Unix systems)
            Ok(EditorCommand {
                program: "vi".to_string(),
                args: vec![],
            })
        }
    }
}

// Helper functions for parsing note content

fn extract_note_tags(content: &str) -> Vec<String> {
    // Look for tags in frontmatter (similar to todo tags extraction)
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
    
    // Also look for inline tags (e.g., #tag in content)
    let mut inline_tags = Vec::new();
    for line in content.lines() {
        // Skip frontmatter lines
        if line.starts_with("---") {
            continue;
        }
        
        // Find hashtags in the line
        let words: Vec<&str> = line.split_whitespace().collect();
        for word in words {
            if word.starts_with('#') && word.len() > 1 {
                let tag = word[1..].trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                if !tag.is_empty() {
                    inline_tags.push(tag.to_string());
                }
            }
        }
    }
    
    inline_tags
}

fn extract_note_title(content: &str, filename: &str) -> String {
    // First, try to find a title in the content
    // Look for H1 headings (# Title)
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") && trimmed.len() > 2 {
            return trimmed[2..].trim().to_string();
        }
    }
    
    // Try to find title in frontmatter
    if let Some(frontmatter_end) = content.find("---\n") {
        if let Some(second_frontmatter_end) = content[frontmatter_end + 4..].find("---\n") {
            let frontmatter = &content[frontmatter_end + 4..frontmatter_end + 4 + second_frontmatter_end];
            
            for line in frontmatter.lines() {
                if line.trim().starts_with("title:") {
                    let title = line.split(':').nth(1).unwrap_or("").trim();
                    if !title.is_empty() && title != "null" {
                        return title.trim_matches('"').to_string();
                    }
                }
            }
        }
    }
    
    // Fallback: use filename without extension
    filename.strip_suffix(".md").unwrap_or(filename).to_string()
}

fn extract_note_preview(content: &str) -> String {
    // Extract the first few lines of actual content (not frontmatter)
    let mut in_frontmatter = false;
    let mut frontmatter_ended = false;
    let mut preview_lines = Vec::new();
    
    for line in content.lines() {
        if line.trim() == "---" {
            if !in_frontmatter {
                in_frontmatter = true;
            } else {
                frontmatter_ended = true;
            }
            continue;
        }
        
        if frontmatter_ended {
            let trimmed = line.trim();
            
            // Skip empty lines and headings
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                preview_lines.push(trimmed);
                
                // Stop after collecting some content
                if preview_lines.len() >= 2 {
                    break;
                }
            }
        }
    }
    
    if preview_lines.is_empty() {
        return "No preview available".to_string();
    }
    
    // Join the preview lines and truncate if too long
    let preview = preview_lines.join(" ");
    if preview.len() > 100 {
        format!("{}...", &preview[..100])
    } else {
        preview
    }
}
