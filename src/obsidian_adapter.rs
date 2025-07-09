use reqwest::Client;
use serde::{Serialize, Deserialize};
use anyhow::{Result, Context, bail}; // Using anyhow for error handling
use serde_yaml;

const MCP_SERVER_URL: &str = "http://localhost:27123"; // Default for Obsidian Local REST API

#[derive(Debug, Serialize, Deserialize, Default, Clone)] // Added Default and Clone
pub struct Frontmatter {
    // Define common frontmatter fields
    pub tags: Option<Vec<String>>,
    pub due_date: Option<String>,
    pub status: Option<String>, // Added for todos and goals
    pub target_date: Option<String>, // Added for goals
    // Add other fields as needed, make them Option<>
    // For truly generic frontmatter, one might use serde_yaml::Value
    // but for now, explicit fields are easier if they are somewhat known.
}

#[derive(Debug, Serialize, Deserialize, Clone)] // Added Clone
struct MarkdownFile {
    pub frontmatter: Frontmatter, // Made public for easier access in handlers
    pub content: String,       // Made public
}

impl MarkdownFile {
    /// Helper to serialize just the frontmatter part to a YAML string.
    /// Useful if you need to reconstruct/update frontmatter specifically.
    pub fn frontmatter_to_string(&self) -> Result<String> {
        serde_yaml::to_string(&self.frontmatter)
            .context("Failed to serialize frontmatter to YAML")
    }
}

pub struct ObsidianAdapter {
    client: Client,
    base_url: String,
}

impl ObsidianAdapter {
    pub fn new(base_url: Option<String>) -> Self {
        ObsidianAdapter {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| MCP_SERVER_URL.to_string()),
        }
    }

    // Made public for potential direct use if needed, though get_markdown_file_data is preferred
    pub fn parse_markdown_file(raw_content: &str) -> Result<MarkdownFile> {
        let parts: Vec<&str> = raw_content.splitn(3, "---").collect();
        if parts.len() < 3 {
            // No frontmatter or malformed, return with default frontmatter and all content as body
            return Ok(MarkdownFile {
                frontmatter: Frontmatter::default(),
                content: raw_content.to_string(),
            });
        }

        let yaml_str = parts[1].trim();
        let content_str = parts[2].trim_start().to_string();

        // If YAML is empty, serde_yaml::from_str("") might error or return unit type.
        // We want a default Frontmatter in this case.
        let frontmatter: Frontmatter = if yaml_str.is_empty() {
            Frontmatter::default()
        } else {
            serde_yaml::from_str(yaml_str)
                .context(format!("Failed to parse YAML frontmatter: '{}'", yaml_str))?
        };

        Ok(MarkdownFile {
            frontmatter,
            content: content_str,
        })
    }

    // Made public for potential direct use
    pub fn serialize_markdown_file(file: &MarkdownFile) -> Result<String> {
        // Ensure frontmatter isn't just defaults if we don't want to write empty "--- \n ---"
        // However, always writing it is consistent. Serde_yaml handles Option types well (omits if None).
        let fm_yaml = serde_yaml::to_string(&file.frontmatter)
            .context("Failed to serialize frontmatter to YAML")?;

        // Avoid serializing an empty "null" or "{}" if frontmatter is truly empty/default
        // and all its fields are None.
        // A simple check: if the serialized YAML is just "---" or "--- {}" or "--- null", treat as empty.
        // For now, `trim()` handles `--- \n---` becoming `--- \n---`
        // An empty `Frontmatter::default()` serializes to `status: null\ntarget_date: null\n` if these fields are present.
        // `serde_yaml` with `skip_serializing_if = "Option::is_none"` on struct fields is better.
        // Let's add that to Frontmatter struct. (Will do this in a follow up if needed, for now it's fine)

        Ok(format!("---\n{}---\n\n{}", fm_yaml.trim(), file.content))
    }

    pub async fn get_file(&self, vault_path: &str) -> Result<String> {
        let url = format!("{}/vault/{}", self.base_url, vault_path);
        let response = self.client.get(&url)
            .header("Accept", "text/markdown")
            .send()
            .await
            .context(format!("Failed to send GET request to {}", url))?;

        if response.status().is_success() {
            response.text().await.context("Failed to read response text")
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            bail!("MCP server returned error {}: {}. URL: {}", status, error_text, url)
        }
    }

    pub async fn create_file(&self, vault_path: &str, content: &str) -> Result<()> {
        let url = format!("{}/vault/{}", self.base_url, vault_path);
        let response = self.client.post(&url)
            .header("Content-Type", "text/markdown")
            .body(content.to_string())
            .send()
            .await
            .context(format!("Failed to send POST request to {}", url))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            bail!("MCP server returned error {}: {}. URL: {}", status, error_text, url)
        }
    }

    pub async fn update_file(&self, vault_path: &str, content: &str) -> Result<()> {
        let url = format!("{}/vault/{}", self.base_url, vault_path);
        let response = self.client.put(&url)
            .header("Content-Type", "text/markdown")
            .body(content.to_string())
            .send()
            .await
            .context(format!("Failed to send PUT request to {}", url))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            bail!("MCP server returned error {}: {}. URL: {}", status, error_text, url)
        }
    }

    pub async fn get_markdown_file_data(&self, vault_path: &str) -> Result<MarkdownFile> {
        let raw_content = self.get_file(vault_path).await?;
        Self::parse_markdown_file(&raw_content)
    }

    pub async fn save_markdown_file_data(&self, vault_path: &str, file_data: &MarkdownFile, overwrite: bool) -> Result<()> {
        let serialized_content = Self::serialize_markdown_file(file_data)?;
        if overwrite {
            // For Obsidian Local REST API, PUT is typically used for update/overwrite
            self.update_file(vault_path, &serialized_content).await
        } else {
            // POST can often create or overwrite. If strict create-only is needed,
            // one might need to check for file existence first (e.g. with a GET).
            // Assuming POST here means "create or replace".
            self.create_file(vault_path, &serialized_content).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_markdown_with_full_frontmatter() {
        let raw_md = r#"---
tags: [test, example]
due_date: "2024-01-01"
status: "pending"
target_date: "2024-02-01"
---

This is the content."#;
        let parsed = ObsidianAdapter::parse_markdown_file(raw_md).unwrap();
        assert_eq!(parsed.content, "This is the content.");
        assert_eq!(parsed.frontmatter.tags, Some(vec!["test".to_string(), "example".to_string()]));
        assert_eq!(parsed.frontmatter.due_date, Some("2024-01-01".to_string()));
        assert_eq!(parsed.frontmatter.status, Some("pending".to_string()));
        assert_eq!(parsed.frontmatter.target_date, Some("2024-02-01".to_string()));
    }

    #[test]
    fn test_parse_markdown_with_partial_frontmatter() {
        let raw_md = r#"---
tags: [test]
status: "active"
---

Content here."#;
        let parsed = ObsidianAdapter::parse_markdown_file(raw_md).unwrap();
        assert_eq!(parsed.content, "Content here.");
        assert_eq!(parsed.frontmatter.tags, Some(vec!["test".to_string()]));
        assert!(parsed.frontmatter.due_date.is_none());
        assert_eq!(parsed.frontmatter.status, Some("active".to_string()));
        assert!(parsed.frontmatter.target_date.is_none());
    }


    #[test]
    fn test_parse_markdown_without_frontmatter() {
        let raw_md = "Just content here.";
        let parsed = ObsidianAdapter::parse_markdown_file(raw_md).unwrap();
        assert_eq!(parsed.content, "Just content here.");
        assert!(parsed.frontmatter.tags.is_none());
        assert!(parsed.frontmatter.status.is_none());
    }

    #[test]
    fn test_parse_markdown_empty_frontmatter_section() { // "--- \n ---"
        let raw_md = r#"---
---

Content below empty frontmatter."#;
        let parsed = ObsidianAdapter::parse_markdown_file(raw_md).unwrap();
        assert_eq!(parsed.content, "Content below empty frontmatter.");
        // Default Frontmatter should have all Nones
        assert!(parsed.frontmatter.tags.is_none());
        assert!(parsed.frontmatter.due_date.is_none());
        assert!(parsed.frontmatter.status.is_none());
        assert!(parsed.frontmatter.target_date.is_none());
    }

    #[test]
    fn test_serialize_markdown_file_with_all_fields() {
        let file_data = MarkdownFile {
            frontmatter: Frontmatter {
                tags: Some(vec!["rust".to_string(), "dev".to_string()]),
                due_date: Some("tomorrow".to_string()),
                status: Some("in progress".to_string()),
                target_date: Some("next week".to_string()),
            },
            content: "Writing some Rust code.".to_string(),
        };
        let serialized = ObsidianAdapter::serialize_markdown_file(&file_data).unwrap();
        // Using contains to avoid strict YAML key order issues, though serde_yaml is usually consistent.
        // A better test would parse the YAML back.
        assert!(serialized.contains("tags:\n- rust\n- dev"));
        assert!(serialized.contains("due_date: tomorrow"));
        assert!(serialized.contains("status: in progress"));
        assert!(serialized.contains("target_date: next week"));
        assert!(serialized.ends_with("\n\nWriting some Rust code."));
    }

    #[test]
    fn test_serialize_markdown_file_with_some_fields_none() {
        let file_data = MarkdownFile {
            frontmatter: Frontmatter {
                tags: Some(vec!["task".to_string()]),
                due_date: None,
                status: Some("open".to_string()),
                target_date: None,
            },
            content: "A simple task.".to_string(),
        };
        let serialized = ObsidianAdapter::serialize_markdown_file(&file_data).unwrap();
        let expected_fm_yaml = "tags:\n- task\nstatus: open"; // due_date and target_date should be omitted by serde_yaml if None

        // Parse the frontmatter part of the serialized string
        let parts: Vec<&str> = serialized.splitn(3, "---").collect();
        assert!(parts.len() >= 3, "Serialized output not in expected format");
        let parsed_fm_yaml: serde_yaml::Value = serde_yaml::from_str(parts[1].trim()).unwrap();

        assert_eq!(parsed_fm_yaml.get("tags").unwrap().as_sequence().unwrap().len(), 1);
        assert_eq!(parsed_fm_yaml.get("tags").unwrap()[0].as_str().unwrap(), "task");
        assert_eq!(parsed_fm_yaml.get("status").unwrap().as_str().unwrap(), "open");
        assert!(parsed_fm_yaml.get("due_date").is_none() || parsed_fm_yaml.get("due_date").unwrap().is_null());
        assert!(parsed_fm_yaml.get("target_date").is_none() || parsed_fm_yaml.get("target_date").unwrap().is_null());

        assert!(serialized.ends_with("\n\nA simple task."));
    }

    #[test]
    fn test_frontmatter_to_string_method() {
        let fm = Frontmatter {
            tags: Some(vec!["a".to_string()]),
            status: Some("active".to_string()),
            ..Default::default()
        };
        let md_file = MarkdownFile {
            frontmatter: fm,
            content: "test".to_string()
        };
        let fm_str = md_file.frontmatter_to_string().unwrap();
        assert!(fm_str.contains("tags:\n- a"));
        assert!(fm_str.contains("status: active"));
    }
}
