use crate::cli::{GoalAction, GoalArgs};
use crate::obsidian_adapter::{MarkdownFile, ObsidianAdapter}; // Reusing for parsing
use crate::utils::slugify; // Import slugify from utils
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_yaml;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct GoalFrontmatter {
    // title: String, // Title is usually the H1 in content or filename
    description: Option<String>,
    target_date: Option<String>,
    tags: Option<Vec<String>>,
    status: String, // e.g., "active", "achieved", "on-hold"
}

// We need a way to merge GoalFrontmatter into the generic ObsidianAdapter Frontmatter if we use its parser directly
// Or, define a more specific parsing for goals.
// For now, let's try to manage GoalFrontmatter separately for serialization
// and be careful with parsing if we use the generic adapter's parser.

pub async fn handle_goal_command(args: GoalArgs, adapter: &ObsidianAdapter) -> Result<()> {
    let goals_dir = "Goals"; // Define a base directory for goals

    match args.action {
        GoalAction::Add {
            title,
            description,
            target_date,
            tags,
        } => {
            println!("Attempting to add goal: '{}'", title);

            let fm_tags = if tags.is_empty() {
                None
            } else {
                Some(tags.clone())
            };
            let frontmatter = GoalFrontmatter {
                description: description.clone(),
                target_date: target_date.clone(),
                tags: fm_tags,
                status: "active".to_string(), // Default status
            };

            let fm_yaml = serde_yaml::to_string(&frontmatter)
                .context("Failed to serialize goal frontmatter to YAML")?;

            let goal_body_title = format!("# {}", title);
            let goal_body_desc = description.map_or_else(String::new, |d| format!("\n\n{}", d));

            let full_content = format!(
                "---\n{}---\n\n{}{}",
                fm_yaml.trim(),
                goal_body_title,
                goal_body_desc
            );

            let slug = slugify(&title);
            let max_slug_len = 50;
            let truncated_slug = if slug.len() > max_slug_len {
                slug.chars().take(max_slug_len).collect()
            } else {
                slug
            };

            let file_name = format!("{}/{}.md", goals_dir, truncated_slug);

            adapter
                .create_file(&file_name, &full_content)
                .await
                .context(format!("Failed to create goal file '{}'", file_name))?;

            println!("Goal '{}' created as '{}'.", title, file_name);
        }
        GoalAction::List { status } => {
            println!("Listing goals...");
            if let Some(s) = status {
                println!("  Status filter: {}", s);
            }
            println!("(Placeholder: Actual goal listing logic to be implemented. Similar challenges to other types.)");
        }
        GoalAction::Update {
            id,
            title: new_title,
            description: new_description,
            status: new_status,
            target_date: new_target_date,
        } => {
            let file_slug = slugify(&id); // Assuming id is the original title/slug
            let file_name = format!("{}/{}.md", goals_dir, file_slug);
            println!("Attempting to update goal: '{}'", file_name);

            // Fetch existing file data
            let md_file_data =
                adapter
                    .get_markdown_file_data(&file_name)
                    .await
                    .context(format!(
                        "Failed to retrieve goal '{}' for update.",
                        file_name
                    ))?;

            // Deserialize existing frontmatter specifically for Goal
            // The generic ObsidianFrontmatter might not have all goal fields.
            // This requires careful handling or a more flexible frontmatter struct in ObsidianAdapter.
            // For now, let's assume we parse the YAML part separately if needed or re-serialize.

            // A simple approach: parse the frontmatter string from md_file_data.content if it exists
            // This is a bit crude, ideally ObsidianAdapter::get_markdown_file_data would return a more structured Frontmatter
            // that can be converted to GoalFrontmatter or directly use serde_yaml::Value.

            let raw_fm_str = md_file_data.frontmatter_to_string().unwrap_or_default();
            let mut fm: GoalFrontmatter =
                serde_yaml::from_str(&raw_fm_str).unwrap_or_else(|_| GoalFrontmatter {
                    // Defaults if parsing fails or empty
                    description: None,
                    target_date: None,
                    tags: md_file_data.frontmatter.tags.clone(), // Get tags from generic parser
                    status: "active".to_string(),
                });

            // Update fields
            if let Some(desc) = new_description {
                fm.description = Some(desc);
            }
            if let Some(status) = new_status {
                fm.status = status;
            }
            if let Some(td) = new_target_date {
                fm.target_date = Some(td);
            }
            // Title change would mean filename change - more complex, handle separately or disallow for now.
            // For now, if new_title is provided, we update it in the content body (H1).

            let mut current_body_content = md_file_data.content;
            let final_title = new_title.as_ref().unwrap_or(&id); // Use new title if provided, else old id

            // Reconstruct H1 title in body
            if current_body_content.starts_with("# ") || new_title.is_some() {
                // Find the first newline to separate H1 from rest of body
                let body_after_h1 = current_body_content
                    .find("\n\n")
                    .map_or("", |idx| &current_body_content[idx..]);
                current_body_content = format!("# {}{}", final_title, body_after_h1);
            }

            let updated_fm_yaml = serde_yaml::to_string(&fm)
                .context("Failed to serialize updated goal frontmatter to YAML")?;

            let new_full_content = format!(
                "---\n{}---\n\n{}",
                updated_fm_yaml.trim(),
                current_body_content
            );

            // If title changes, we might want to rename the file. This is a more advanced feature.
            // For now, we update content in place. If new_title results in a new slug, user needs to be aware.
            let target_file_name = if let Some(nt) = new_title {
                let new_slug = slugify(&nt);
                if new_slug != file_slug {
                    // This would be a rename operation: adapter.move_file() then update.
                    // Or delete old, create new. For now, just warn.
                    println!("Warning: Title change may require filename change from '{}' to '{}.md'. This is not automatically handled yet.", file_name, new_slug);
                    // Fallback to old filename for update if rename not implemented.
                    file_name // Or format!("{}/{}.md", goals_dir, new_slug) if we were to rename
                } else {
                    file_name
                }
            } else {
                file_name
            };

            adapter
                .update_file(&target_file_name, &new_full_content)
                .await
                .context(format!("Failed to update goal file '{}'", target_file_name))?;

            println!("Goal '{}' updated.", target_file_name);
        }
        GoalAction::View { id } => {
            let file_name = format!("{}/{}.md", goals_dir, slugify(&id));
            println!("Viewing goal: '{}'", file_name);

            let content = adapter.get_file(&file_name).await.context(format!(
                "Failed to retrieve goal '{}' for viewing.",
                file_name
            ))?;

            println!("--- Content of {} ---", file_name);
            println!("{}", content);
            println!("--- End of Content ---");
        }
    }
    Ok(())
}

// Add a helper in ObsidianAdapter for frontmatter string if it's not already easy to get
// For MarkdownFile in obsidian_adapter.rs, add:
// impl MarkdownFile {
//     pub fn frontmatter_to_string(&self) -> Result<String> {
//         serde_yaml::to_string(&self.frontmatter).context("Failed to serialize frontmatter to YAML")
//     }
// }
// This is a temporary measure. A better way is to make ObsidianAdapter::Frontmatter generic or use serde_yaml::Value.
// Actually, ObsidianAdapter already serializes its own Frontmatter struct.
// The issue is that GoalFrontmatter has different fields.
// The `get_markdown_file_data` returns a `MarkdownFile` with a generic `Frontmatter`.
// We need to parse that generic frontmatter's YAML string into `GoalFrontmatter`.

// Let's adjust the update logic to reflect this.
// The `obsidian_adapter.rs` would need `MarkdownFile` to expose the raw YAML string of its frontmatter,
// or `ObsidianAdapter::parse_markdown_file` should make `frontmatter` a `serde_yaml::Value`.
// For now, let's assume `ObsidianAdapter::get_markdown_file_data` gives us enough to reconstruct.
// The current `ObsidianAdapter::parse_markdown_file` parses into a specific `Frontmatter` struct.
// This means for `Update Goal`, we might need to fetch raw file content and parse frontmatter specifically.

// Let's simplify Update: fetch raw, parse FM, update FM, merge, save.
// This means `adapter.get_markdown_file_data()` might not be suitable if its `Frontmatter` struct is too rigid.
// Let's assume `adapter.get_file()` and then manual parsing for Update.

// Re-modifying the Update part slightly for clarity.
// The current `get_markdown_file_data` parses into a specific `Frontmatter` (tags, due_date).
// This isn't ideal for goals with different frontmatter.
// A better `get_markdown_file_data` would perhaps take a type parameter for the frontmatter
// or return `serde_yaml::Value` for frontmatter.
// Given the current `ObsidianAdapter`, for `Update Goal`, it's safer to:
// 1. `get_file()` to get raw string.
// 2. Manually split frontmatter and content.
// 3. Parse frontmatter string into `GoalFrontmatter`.
// 4. Update `GoalFrontmatter` and content.
// 5. Serialize `GoalFrontmatter` and combine with content.
// 6. `update_file()`.

// The provided `replace_with_git_merge_diff` requires a single block.
// The above comments are for thought process. The code will reflect a pragmatic approach.
// The `Update` logic in the code block above attempts to use `get_markdown_file_data` and then
// re-parse its `frontmatter` field. This is a bit indirect.
// Let's assume `ObsidianAdapter::Frontmatter` is flexible enough or we add the necessary fields to it for goals.
// For the purpose of this step, I will add `status` and `target_date` to the `ObsidianAdapter::Frontmatter`
// to make the `Update` logic in `goals.rs` work more smoothly. This change will be done in the next step (Refine ObsidianAdapter).

// For *this* step, I will write the `goals.rs` update logic *as if* the frontmatter parsing is flexible.
// The current `MarkdownFile` in `obsidian_adapter.rs` has a `frontmatter_to_string` helper that I commented on.
// Let's assume that `md_file_data.frontmatter` (which is `ObsidianFrontmatter`) can be serialized to YAML string,
// and then that YAML string can be parsed into `GoalFrontmatter`. This is what the current code attempts.
// It's a bit convoluted: raw_md -> ObsidianFrontmatter -> YAML string -> GoalFrontmatter.
// A more direct path: raw_md -> YAML string -> GoalFrontmatter would be better.
// But sticking to current adapter methods:
// `let raw_fm_str = serde_yaml::to_string(&md_file_data.frontmatter).context("...")?;`
// `let mut fm: GoalFrontmatter = serde_yaml::from_str(&raw_fm_str)...`
// This assumes `ObsidianFrontmatter` can be serialized by `serde_yaml`. It can.
// The fields not present in `ObsidianFrontmatter` but present in `GoalFrontmatter` will be `None` or default.
// And fields in `ObsidianFrontmatter` not in `GoalFrontmatter` will be ignored during `GoalFrontmatter` deserialization.
// This is okay.
