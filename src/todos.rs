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
            if let Some(s) = status {
                println!("  Status filter: {}", s);
            }
            // This will require listing files in a directory and parsing them.
            // The Obsidian Local REST API doesn't directly support listing files in a directory.
            // A common workaround is to GET /vault/ which lists all files, then filter by path.
            // Or, if the API supports searching by tag, that could be an option if all todos have a common tag.
            // For now, we'll leave this as a more complex implementation detail.
            println!("(Placeholder: Actual todo listing logic to be implemented. This requires directory listing or a full vault scan and filter.)");
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
