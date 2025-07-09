use anyhow::{Result, Context};
use crate::cli::{NoteAction, NoteArgs};
use crate::obsidian_adapter::ObsidianAdapter;
use crate::utils::slugify; // Import slugify from utils
use serde::Serialize; // For serializing frontmatter

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
            println!("(Placeholder: Actual note listing logic to be implemented. Similar challenges to listing todos.)");
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
            println!("Editing note: '{}' (this might open $EDITOR)", name_or_id);
            println!("(Placeholder: Actual note editing logic to be implemented. Involves platform specifics for opening editor.)");
        }
    }
    Ok(())
}
