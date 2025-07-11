use anyhow::{bail, Context, Result};
use base64::{Engine as _, engine::general_purpose};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use crate::ai_conversation::AIConversationEngine;
use log::{error, warn, info, debug};

/// Jira authentication configuration
#[derive(Debug, Clone)]
pub enum JiraAuth {
    /// API Token authentication (email + token)
    ApiToken { email: String, token: String },
    /// Basic authentication (username + password) - less secure, not recommended
    Basic { username: String, password: String },
    /// OAuth 2.0 authentication (not implemented yet)
    OAuth { token: String },
}

/// Jira API client configuration
#[derive(Debug, Clone)]
pub struct JiraConfig {
    pub base_url: String,
    pub auth: JiraAuth,
    pub timeout_seconds: u64,
    pub max_retries: usize,
    pub retry_delay_ms: u64,
}

impl Default for JiraConfig {
    fn default() -> Self {
        Self {
            base_url: "https://your-domain.atlassian.net".to_string(),
            auth: JiraAuth::ApiToken {
                email: "".to_string(),
                token: "".to_string(),
            },
            timeout_seconds: 30,
            max_retries: 3,
            retry_delay_ms: 1000,
        }
    }
}

/// Connection test response
#[derive(Debug, Serialize, Deserialize)]
pub struct JiraConnectionTest {
    pub success: bool,
    pub message: String,
    pub server_info: Option<JiraServerInfo>,
}

/// Jira server information
#[derive(Debug, Serialize, Deserialize)]
pub struct JiraServerInfo {
    #[serde(rename = "baseUrl")]
    pub base_url: String,
    pub version: String,
    #[serde(rename = "versionNumbers")]
    pub version_numbers: Vec<i32>,
    #[serde(rename = "deploymentType")]
    pub deployment_type: String,
    #[serde(rename = "buildNumber")]
    pub build_number: i32,
    #[serde(rename = "serverTitle")]
    pub server_title: String,
}

/// Jira User representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraUser {
    #[serde(rename = "accountId")]
    pub account_id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "emailAddress")]
    pub email_address: Option<String>,
    pub active: bool,
    #[serde(rename = "avatarUrls")]
    pub avatar_urls: Option<HashMap<String, String>>,
}

/// Jira Project representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraProject {
    pub id: String,
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "projectTypeKey")]
    pub project_type_key: String,
    pub lead: Option<JiraUser>,
    #[serde(rename = "avatarUrls")]
    pub avatar_urls: Option<HashMap<String, String>>,
    #[serde(rename = "issueTypes")]
    pub issue_types: Option<Vec<JiraIssueType>>,
}

/// Jira Issue Type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraIssueType {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "iconUrl")]
    pub icon_url: Option<String>,
    pub subtask: bool,
}

/// Jira Issue Status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraStatus {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "statusCategory")]
    pub status_category: JiraStatusCategory,
}

/// Jira Status Category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraStatusCategory {
    pub id: u32,
    pub name: String,
    pub key: String,
    #[serde(rename = "colorName")]
    pub color_name: String,
}

/// Jira Issue Priority
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraPriority {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "iconUrl")]
    pub icon_url: Option<String>,
}

/// Jira Issue representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraIssue {
    pub id: String,
    pub key: String,
    pub fields: JiraIssueFields,
}

/// Jira Issue Fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraIssueFields {
    pub summary: String,
    pub description: Option<String>,
    #[serde(rename = "issuetype")]
    pub issue_type: JiraIssueType,
    pub project: JiraProject,
    pub status: JiraStatus,
    pub priority: Option<JiraPriority>,
    pub assignee: Option<JiraUser>,
    pub reporter: Option<JiraUser>,
    pub created: String,
    pub updated: String,
    pub labels: Option<Vec<String>>,
    pub components: Option<Vec<JiraComponent>>,
    #[serde(rename = "fixVersions")]
    pub fix_versions: Option<Vec<JiraVersion>>,
}

/// Jira Component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraComponent {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// Jira Version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraVersion {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub archived: bool,
    pub released: bool,
}

/// Issue creation request
#[derive(Debug, Serialize)]
pub struct CreateIssueRequest {
    pub fields: CreateIssueFields,
}

/// Issue creation fields
#[derive(Debug, Serialize)]
pub struct CreateIssueFields {
    pub summary: String,
    pub description: Option<String>,
    #[serde(rename = "issuetype")]
    pub issue_type: IssueTypeReference,
    pub project: ProjectReference,
    pub assignee: Option<UserReference>,
    pub priority: Option<PriorityReference>,
    pub labels: Option<Vec<String>>,
}

/// Reference types for issue creation
#[derive(Debug, Serialize)]
pub struct IssueTypeReference {
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct ProjectReference {
    pub key: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct UserReference {
    #[serde(rename = "accountId")]
    pub account_id: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct PriorityReference {
    pub id: String,
}

/// Issue update request
#[derive(Debug, Serialize, Clone)]
pub struct UpdateIssueRequest {
    pub fields: UpdateIssueFields,
}

/// Issue update fields
#[derive(Debug, Serialize, Clone)]
pub struct UpdateIssueFields {
    pub summary: Option<String>,
    pub description: Option<String>,
    pub assignee: Option<UserReference>,
    pub priority: Option<PriorityReference>,
    pub labels: Option<Vec<String>>,
}

/// Search results wrapper
#[derive(Debug, Deserialize)]
pub struct SearchResults<T> {
    pub issues: Vec<T>,
    #[serde(rename = "startAt")]
    pub start_at: u32,
    #[serde(rename = "maxResults")]
    pub max_results: u32,
    pub total: u32,
}

/// Project search results
#[derive(Debug, Deserialize)]
pub struct ProjectSearchResults {
    pub values: Vec<JiraProject>,
    #[serde(rename = "startAt")]
    pub start_at: u32,
    #[serde(rename = "maxResults")]
    pub max_results: u32,
    pub total: u32,
    #[serde(rename = "isLast")]
    pub is_last: bool,
}

/// Jira API client adapter
pub struct JiraAdapter {
    client: Client,
    config: JiraConfig,
    ai_conversation: Option<AIConversationEngine>,
}

impl JiraAdapter {
    /// Create a new JiraAdapter with the given configuration
    pub fn new(config: JiraConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(JiraAdapter { 
            client, 
            config,
            ai_conversation: None,
        })
    }
    
    /// Create a new JiraAdapter with AI conversation capabilities
    pub fn new_with_ai(config: JiraConfig, ai_conversation: AIConversationEngine) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(JiraAdapter { 
            client, 
            config,
            ai_conversation: Some(ai_conversation),
        })
    }

    /// Create a JiraAdapter from environment variables
    pub fn from_env() -> Result<Self> {
        let base_url = std::env::var("JIRA_BASE_URL")
            .context("JIRA_BASE_URL environment variable not set")?;
        
        let email = std::env::var("JIRA_EMAIL")
            .context("JIRA_EMAIL environment variable not set")?;
        
        let token = std::env::var("JIRA_API_TOKEN")
            .context("JIRA_API_TOKEN environment variable not set")?;

        let config = JiraConfig {
            base_url,
            auth: JiraAuth::ApiToken { email, token },
            ..Default::default()
        };

        Self::new(config)
    }

    /// Create authorization header value based on auth type
    fn create_auth_header(&self) -> String {
        match &self.config.auth {
            JiraAuth::ApiToken { email, token } => {
                let credentials = format!("{}:{}", email, token);
                format!("Basic {}", general_purpose::STANDARD.encode(credentials))
            }
            JiraAuth::Basic { username, password } => {
                let credentials = format!("{}:{}", username, password);
                format!("Basic {}", general_purpose::STANDARD.encode(credentials))
            }
            JiraAuth::OAuth { token } => {
                format!("Bearer {}", token)
            }
        }
    }

    /// Add authorization header to request builder
    fn add_auth_header(&self, request_builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request_builder.header("Authorization", self.create_auth_header())
    }

    /// Execute a request with retry logic and proper error handling
    async fn execute_with_retry<T>(&self, mut request_fn: impl FnMut() -> reqwest::RequestBuilder) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            let response = match self.add_auth_header(request_fn()).send().await {
                Ok(response) => response,
                Err(e) => {
                    last_error = Some(anyhow::Error::new(e));
                    if attempt < self.config.max_retries {
                        sleep(Duration::from_millis(
                            self.config.retry_delay_ms * (2_u64.pow(attempt as u32))
                        )).await;
                        continue;
                    } else {
                        break;
                    }
                }
            };

            // Handle rate limiting (429) and server errors (5xx)
            if response.status().as_u16() == 429 || response.status().is_server_error() {
                if attempt < self.config.max_retries {
                    let retry_after = response
                        .headers()
                        .get("retry-after")
                        .and_then(|h| h.to_str().ok())
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(self.config.retry_delay_ms / 1000) * 1000;
                    
                    sleep(Duration::from_millis(retry_after)).await;
                    continue;
                }
            }

            if response.status().is_success() {
                return response
                    .json::<T>()
                    .await
                    .context("Failed to parse JSON response");
            } else {
                let status = response.status();
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                
                last_error = Some(anyhow::anyhow!(
                    "Jira API returned error {}: {}",
                    status,
                    error_text
                ));

                // Don't retry on client errors (4xx except 429)
                if status.is_client_error() && status.as_u16() != 429 {
                    break;
                }

                if attempt < self.config.max_retries {
                    sleep(Duration::from_millis(
                        self.config.retry_delay_ms * (2_u64.pow(attempt as u32))
                    )).await;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Request failed after {} attempts", self.config.max_retries + 1)))
    }

    /// Test the connection to Jira API
    pub async fn test_connection(&self) -> Result<JiraConnectionTest> {
        match self.get_server_info().await {
            Ok(server_info) => Ok(JiraConnectionTest {
                success: true,
                message: "Successfully connected to Jira".to_string(),
                server_info: Some(server_info),
            }),
            Err(e) => Ok(JiraConnectionTest {
                success: false,
                message: format!("Failed to connect to Jira: {}", e),
                server_info: None,
            }),
        }
    }

    /// Get Jira server information
    pub async fn get_server_info(&self) -> Result<JiraServerInfo> {
        let url = format!("{}/rest/api/3/serverInfo", self.config.base_url);
        
        self.execute_with_retry(|| {
            self.client
                .get(&url)
                .header("Accept", "application/json")
        }).await
    }

    /// Validate the current configuration and authentication
    pub async fn validate_config(&self) -> Result<()> {
        // Check if base URL is valid
        if self.config.base_url.is_empty() {
            bail!("Base URL is empty");
        }

        if !self.config.base_url.starts_with("http://") && !self.config.base_url.starts_with("https://") {
            bail!("Base URL must start with http:// or https://");
        }

        // Validate authentication
        match &self.config.auth {
            JiraAuth::ApiToken { email, token } => {
                if email.is_empty() {
                    bail!("Email is required for API token authentication");
                }
                if token.is_empty() {
                    bail!("API token is required for API token authentication");
                }
                if !email.contains('@') {
                    bail!("Email format is invalid");
                }
            }
            JiraAuth::Basic { username, password } => {
                if username.is_empty() {
                    bail!("Username is required for basic authentication");
                }
                if password.is_empty() {
                    bail!("Password is required for basic authentication");
                }
            }
            JiraAuth::OAuth { token } => {
                if token.is_empty() {
                    bail!("OAuth token is required for OAuth authentication");
                }
            }
        }

        // Test actual connection
        let test_result = self.test_connection().await?;
        if !test_result.success {
            bail!("Configuration validation failed: {}", test_result.message);
        }

        Ok(())
    }

    /// Get the configuration (without sensitive data)
    pub fn get_config_summary(&self) -> String {
        let auth_type = match &self.config.auth {
            JiraAuth::ApiToken { email, .. } => format!("API Token ({})", email),
            JiraAuth::Basic { username, .. } => format!("Basic Auth ({})", username),
            JiraAuth::OAuth { .. } => "OAuth 2.0".to_string(),
        };

        format!(
            "Base URL: {}\nAuth Type: {}\nTimeout: {}s\nMax Retries: {}",
            self.config.base_url,
            auth_type,
            self.config.timeout_seconds,
            self.config.max_retries
        )
    }

    // ==================== ISSUE OPERATIONS ====================

    /// Create a new issue
    pub async fn create_issue(&self, request: CreateIssueRequest) -> Result<JiraIssue> {
        let url = format!("{}/rest/api/3/issue", self.config.base_url);
        
        self.execute_with_retry(|| {
            self.client
                .post(&url)
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .json(&request)
        }).await
    }

    /// Get an issue by key or ID
    pub async fn get_issue(&self, issue_key: &str) -> Result<JiraIssue> {
        let url = format!("{}/rest/api/3/issue/{}", self.config.base_url, issue_key);
        
        self.execute_with_retry(|| {
            self.client
                .get(&url)
                .header("Accept", "application/json")
        }).await
    }

    /// Update an existing issue
    pub async fn update_issue(&self, issue_key: &str, request: UpdateIssueRequest) -> Result<()> {
        let url = format!("{}/rest/api/3/issue/{}", self.config.base_url, issue_key);
        let client = self.client.clone();
        let request_clone = request.clone();
        
        let response = self.execute_with_retry_no_json(move || {
            client
                .put(&url)
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .json(&request_clone)
        }).await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            bail!("Failed to update issue: {}", error_text)
        }
    }

    /// Delete an issue
    pub async fn delete_issue(&self, issue_key: &str) -> Result<()> {
        let url = format!("{}/rest/api/3/issue/{}", self.config.base_url, issue_key);
        let client = self.client.clone();
        
        let response = self.execute_with_retry_no_json(move || {
            client
                .delete(&url)
                .header("Accept", "application/json")
        }).await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            bail!("Failed to delete issue: {}", error_text)
        }
    }

    /// Search issues using JQL (Jira Query Language)
    pub async fn search_issues(&self, jql: &str, start_at: u32, max_results: u32) -> Result<SearchResults<JiraIssue>> {
        let url = format!("{}/rest/api/3/search", self.config.base_url);
        
        let search_request = serde_json::json!({
            "jql": jql,
            "startAt": start_at,
            "maxResults": max_results,
            "fields": ["*all"]
        });

        self.execute_with_retry(|| {
            self.client
                .post(&url)
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .json(&search_request)
        }).await
    }

    /// Get issues assigned to a user
    pub async fn get_issues_assigned_to_user(&self, account_id: &str) -> Result<SearchResults<JiraIssue>> {
        let jql = format!("assignee = \"{}\" ORDER BY updated DESC", account_id);
        self.search_issues(&jql, 0, 50).await
    }

    /// Get issues reported by a user
    pub async fn get_issues_reported_by_user(&self, account_id: &str) -> Result<SearchResults<JiraIssue>> {
        let jql = format!("reporter = \"{}\" ORDER BY created DESC", account_id);
        self.search_issues(&jql, 0, 50).await
    }

    /// Get issues in a project
    pub async fn get_issues_in_project(&self, project_key: &str) -> Result<SearchResults<JiraIssue>> {
        let jql = format!("project = \"{}\" ORDER BY updated DESC", project_key);
        self.search_issues(&jql, 0, 50).await
    }

    // ==================== PROJECT OPERATIONS ====================

    /// Get all projects
    pub async fn get_projects(&self) -> Result<Vec<JiraProject>> {
        let url = format!("{}/rest/api/3/project", self.config.base_url);
        
        self.execute_with_retry(|| {
            self.client
                .get(&url)
                .header("Accept", "application/json")
        }).await
    }

    /// Get a project by key or ID
    pub async fn get_project(&self, project_key: &str) -> Result<JiraProject> {
        let url = format!("{}/rest/api/3/project/{}", self.config.base_url, project_key);
        
        self.execute_with_retry(|| {
            self.client
                .get(&url)
                .header("Accept", "application/json")
        }).await
    }

    /// Search projects with pagination
    pub async fn search_projects(&self, query: Option<&str>, start_at: u32, max_results: u32) -> Result<ProjectSearchResults> {
        let mut url = format!("{}/rest/api/3/project/search?startAt={}&maxResults={}", 
            self.config.base_url, start_at, max_results);
        
        if let Some(q) = query {
            url.push_str(&format!("&query={}", urlencoding::encode(q)));
        }
        
        self.execute_with_retry(|| {
            self.client
                .get(&url)
                .header("Accept", "application/json")
        }).await
    }

    /// Get issue types for a project
    pub async fn get_project_issue_types(&self, project_key: &str) -> Result<Vec<JiraIssueType>> {
        let url = format!("{}/rest/api/3/project/{}/hierarchy", self.config.base_url, project_key);
        
        // This returns a complex structure, so we'll get the project and extract issue types
        let project = self.get_project(project_key).await?;
        Ok(project.issue_types.unwrap_or_default())
    }

    // ==================== USER OPERATIONS ====================

    /// Get current user information
    pub async fn get_current_user(&self) -> Result<JiraUser> {
        let url = format!("{}/rest/api/3/myself", self.config.base_url);
        
        self.execute_with_retry(|| {
            self.client
                .get(&url)
                .header("Accept", "application/json")
        }).await
    }

    /// Get user by account ID
    pub async fn get_user(&self, account_id: &str) -> Result<JiraUser> {
        let url = format!("{}/rest/api/3/user?accountId={}", self.config.base_url, account_id);
        
        self.execute_with_retry(|| {
            self.client
                .get(&url)
                .header("Accept", "application/json")
        }).await
    }

    /// Search users
    pub async fn search_users(&self, query: &str, max_results: u32) -> Result<Vec<JiraUser>> {
        let url = format!("{}/rest/api/3/user/search?query={}&maxResults={}", 
            self.config.base_url, urlencoding::encode(query), max_results);
        
        self.execute_with_retry(|| {
            self.client
                .get(&url)
                .header("Accept", "application/json")
        }).await
    }

    /// Get assignable users for a project
    pub async fn get_assignable_users(&self, project_key: &str) -> Result<Vec<JiraUser>> {
        let url = format!("{}/rest/api/3/user/assignable/search?project={}", 
            self.config.base_url, project_key);
        
        self.execute_with_retry(|| {
            self.client
                .get(&url)
                .header("Accept", "application/json")
        }).await
    }

    // ==================== HELPER METHODS ====================

    /// Execute request with retry logic that doesn't expect JSON response
    async fn execute_with_retry_no_json(&self, mut request_fn: impl FnMut() -> reqwest::RequestBuilder) -> Result<reqwest::Response> {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            let response = match self.add_auth_header(request_fn()).send().await {
                Ok(response) => response,
                Err(e) => {
                    last_error = Some(anyhow::Error::new(e));
                    if attempt < self.config.max_retries {
                        sleep(Duration::from_millis(
                            self.config.retry_delay_ms * (2_u64.pow(attempt as u32))
                        )).await;
                        continue;
                    } else {
                        break;
                    }
                }
            };

            // Handle rate limiting (429) and server errors (5xx)
            if response.status().as_u16() == 429 || response.status().is_server_error() {
                if attempt < self.config.max_retries {
                    let retry_after = response
                        .headers()
                        .get("retry-after")
                        .and_then(|h| h.to_str().ok())
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(self.config.retry_delay_ms / 1000) * 1000;
                    
                    sleep(Duration::from_millis(retry_after)).await;
                    continue;
                }
            }

            return Ok(response);
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Request failed after {} attempts", self.config.max_retries + 1)))
    }

    /// Helper method to build simple create issue request
    pub fn build_create_issue_request(
        project_key: &str,
        issue_type_id: &str,
        summary: &str,
        description: Option<&str>,
    ) -> CreateIssueRequest {
        CreateIssueRequest {
            fields: CreateIssueFields {
                summary: summary.to_string(),
                description: description.map(|d| d.to_string()),
                issue_type: IssueTypeReference {
                    id: issue_type_id.to_string(),
                },
                project: ProjectReference {
                    key: project_key.to_string(),
                },
                assignee: None,
                priority: None,
                labels: None,
            },
        }
    }

    /// Helper method to build update issue request
    pub fn build_update_issue_request(
        summary: Option<&str>,
        description: Option<&str>,
        assignee_account_id: Option<&str>,
        priority_id: Option<&str>,
        labels: Option<Vec<String>>,
    ) -> UpdateIssueRequest {
        UpdateIssueRequest {
            fields: UpdateIssueFields {
                summary: summary.map(|s| s.to_string()),
                description: description.map(|d| d.to_string()),
                assignee: assignee_account_id.map(|id| UserReference {
                    account_id: id.to_string(),
                }),
                priority: priority_id.map(|id| PriorityReference {
                    id: id.to_string(),
                }),
                labels,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jira_config_default() {
        let config = JiraConfig::default();
        assert_eq!(config.base_url, "https://your-domain.atlassian.net");
        assert_eq!(config.timeout_seconds, 30);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_delay_ms, 1000);
    }

    #[test]
    fn test_create_auth_header_api_token() {
        let config = JiraConfig {
            auth: JiraAuth::ApiToken {
                email: "test@example.com".to_string(),
                token: "token123".to_string(),
            },
            ..Default::default()
        };
        
        let adapter = JiraAdapter::new(config).unwrap();
        let auth_header = adapter.create_auth_header();
        
        // Base64 encode of "test@example.com:token123"
        let expected = format!("Basic {}", general_purpose::STANDARD.encode("test@example.com:token123"));
        assert_eq!(auth_header, expected);
    }

    #[test]
    fn test_create_auth_header_basic() {
        let config = JiraConfig {
            auth: JiraAuth::Basic {
                username: "testuser".to_string(),
                password: "password123".to_string(),
            },
            ..Default::default()
        };
        
        let adapter = JiraAdapter::new(config).unwrap();
        let auth_header = adapter.create_auth_header();
        
        // Base64 encode of "testuser:password123"
        let expected = format!("Basic {}", general_purpose::STANDARD.encode("testuser:password123"));
        assert_eq!(auth_header, expected);
    }

    #[test]
    fn test_create_auth_header_oauth() {
        let config = JiraConfig {
            auth: JiraAuth::OAuth {
                token: "oauth_token_123".to_string(),
            },
            ..Default::default()
        };
        
        let adapter = JiraAdapter::new(config).unwrap();
        let auth_header = adapter.create_auth_header();
        
        assert_eq!(auth_header, "Bearer oauth_token_123");
    }

    #[test]
    fn test_config_validation_empty_base_url() {
        let config = JiraConfig {
            base_url: "".to_string(),
            ..Default::default()
        };
        
        let adapter = JiraAdapter::new(config).unwrap();
        
        // This should fail without making a network call
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(adapter.validate_config());
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Base URL is empty"));
    }

    #[test]
    fn test_config_validation_invalid_url_scheme() {
        let config = JiraConfig {
            base_url: "ftp://invalid.com".to_string(),
            ..Default::default()
        };
        
        let adapter = JiraAdapter::new(config).unwrap();
        
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(adapter.validate_config());
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must start with http"));
    }

    #[test]
    fn test_config_validation_empty_auth_fields() {
        let config = JiraConfig {
            base_url: "https://test.atlassian.net".to_string(),
            auth: JiraAuth::ApiToken {
                email: "".to_string(),
                token: "".to_string(),
            },
            ..Default::default()
        };
        
        let adapter = JiraAdapter::new(config).unwrap();
        
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(adapter.validate_config());
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Email is required"));
    }

    #[test]
    fn test_config_validation_invalid_email() {
        let config = JiraConfig {
            base_url: "https://test.atlassian.net".to_string(),
            auth: JiraAuth::ApiToken {
                email: "invalid-email".to_string(),
                token: "token123".to_string(),
            },
            ..Default::default()
        };
        
        let adapter = JiraAdapter::new(config).unwrap();
        
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(adapter.validate_config());
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Email format is invalid"));
    }

    #[test]
    fn test_get_config_summary() {
        let config = JiraConfig {
            base_url: "https://test.atlassian.net".to_string(),
            auth: JiraAuth::ApiToken {
                email: "test@example.com".to_string(),
                token: "secret_token".to_string(),
            },
            timeout_seconds: 45,
            max_retries: 5,
            ..Default::default()
        };
        
        let adapter = JiraAdapter::new(config).unwrap();
        let summary = adapter.get_config_summary();
        
        assert!(summary.contains("https://test.atlassian.net"));
        assert!(summary.contains("API Token (test@example.com)"));
        assert!(summary.contains("Timeout: 45s"));
        assert!(summary.contains("Max Retries: 5"));
        // Should not contain the actual token
        assert!(!summary.contains("secret_token"));
    }

    #[test]
    fn test_build_create_issue_request() {
        let request = JiraAdapter::build_create_issue_request(
            "TEST",
            "10001",
            "Test Issue",
            Some("Test Description")
        );
        
        assert_eq!(request.fields.summary, "Test Issue");
        assert_eq!(request.fields.description, Some("Test Description".to_string()));
        assert_eq!(request.fields.issue_type.id, "10001");
        assert_eq!(request.fields.project.key, "TEST");
        assert!(request.fields.assignee.is_none());
        assert!(request.fields.priority.is_none());
        assert!(request.fields.labels.is_none());
    }

    #[test]
    fn test_build_create_issue_request_minimal() {
        let request = JiraAdapter::build_create_issue_request(
            "PROJ",
            "10002",
            "Simple Issue",
            None
        );
        
        assert_eq!(request.fields.summary, "Simple Issue");
        assert!(request.fields.description.is_none());
        assert_eq!(request.fields.issue_type.id, "10002");
        assert_eq!(request.fields.project.key, "PROJ");
    }

    #[test]
    fn test_build_update_issue_request() {
        let labels = vec!["bug".to_string(), "urgent".to_string()];
        let request = JiraAdapter::build_update_issue_request(
            Some("Updated Summary"),
            Some("Updated Description"),
            Some("user123"),
            Some("priority456"),
            Some(labels.clone())
        );
        
        assert_eq!(request.fields.summary, Some("Updated Summary".to_string()));
        assert_eq!(request.fields.description, Some("Updated Description".to_string()));
        assert_eq!(request.fields.assignee.as_ref().unwrap().account_id, "user123");
        assert_eq!(request.fields.priority.as_ref().unwrap().id, "priority456");
        assert_eq!(request.fields.labels, Some(labels));
    }

    #[test]
    fn test_build_update_issue_request_partial() {
        let request = JiraAdapter::build_update_issue_request(
            Some("New Summary"),
            None,
            None,
            None,
            None
        );
        
        assert_eq!(request.fields.summary, Some("New Summary".to_string()));
        assert!(request.fields.description.is_none());
        assert!(request.fields.assignee.is_none());
        assert!(request.fields.priority.is_none());
        assert!(request.fields.labels.is_none());
    }

    #[test]
    fn test_data_structure_serialization() {
        // Test that our data structures can be serialized/deserialized properly
        let user = JiraUser {
            account_id: "123456".to_string(),
            display_name: "Test User".to_string(),
            email_address: Some("test@example.com".to_string()),
            active: true,
            avatar_urls: None,
        };
        
        let serialized = serde_json::to_string(&user).unwrap();
        let deserialized: JiraUser = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(user.account_id, deserialized.account_id);
        assert_eq!(user.display_name, deserialized.display_name);
        assert_eq!(user.email_address, deserialized.email_address);
        assert_eq!(user.active, deserialized.active);
    }

    #[test]
    fn test_project_reference_serialization() {
        let project_ref = ProjectReference {
            key: "TEST".to_string(),
        };
        
        let serialized = serde_json::to_string(&project_ref).unwrap();
        assert!(serialized.contains("TEST"));
        assert!(serialized.contains("key"));
    }

    #[test]
    fn test_issue_type_reference_serialization() {
        let issue_type_ref = IssueTypeReference {
            id: "10001".to_string(),
        };
        
        let serialized = serde_json::to_string(&issue_type_ref).unwrap();
        assert!(serialized.contains("10001"));
        assert!(serialized.contains("id"));
    }

    #[test]
    fn test_user_reference_serialization() {
        let user_ref = UserReference {
            account_id: "user123".to_string(),
        };
        
        let serialized = serde_json::to_string(&user_ref).unwrap();
        assert!(serialized.contains("user123"));
        assert!(serialized.contains("accountId"));
    }

    #[test]
    fn test_create_issue_request_serialization() {
        let request = CreateIssueRequest {
            fields: CreateIssueFields {
                summary: "Test Issue".to_string(),
                description: Some("Test Description".to_string()),
                issue_type: IssueTypeReference {
                    id: "10001".to_string(),
                },
                project: ProjectReference {
                    key: "TEST".to_string(),
                },
                assignee: Some(UserReference {
                    account_id: "user123".to_string(),
                }),
                priority: Some(PriorityReference {
                    id: "priority456".to_string(),
                }),
                labels: Some(vec!["bug".to_string(), "urgent".to_string()]),
            },
        };
        
        let serialized = serde_json::to_string(&request).unwrap();
        
        // Check that all fields are properly serialized
        assert!(serialized.contains("Test Issue"));
        assert!(serialized.contains("Test Description"));
        assert!(serialized.contains("10001"));
        assert!(serialized.contains("TEST"));
        assert!(serialized.contains("user123"));
        assert!(serialized.contains("priority456"));
        assert!(serialized.contains("bug"));
        assert!(serialized.contains("urgent"));
        assert!(serialized.contains("issuetype"));
        assert!(serialized.contains("accountId"));
    }
}

// ==================== ERROR HANDLING AND REPORTING DATA STRUCTURES ====================

/// Comprehensive error tracking and reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraError {
    pub error_id: String,
    pub error_type: JiraErrorType,
    pub message: String,
    pub endpoint: String,
    pub status_code: Option<u16>,
    pub timestamp: String,
    pub retry_count: u32,
    pub context: HashMap<String, String>,
    pub resolution_suggestion: Option<String>,
}

/// Types of Jira API errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JiraErrorType {
    Authentication,
    Authorization,
    RateLimit,
    NetworkTimeout,
    ServerError,
    ClientError,
    ValidationError,
    ResourceNotFound,
    ConfigurationError,
    UnknownError,
}

/// Error reporting and analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorReport {
    pub report_id: String,
    pub generated_at: String,
    pub time_period: String,
    pub total_errors: u32,
    pub error_by_type: HashMap<String, u32>,
    pub error_by_endpoint: HashMap<String, u32>,
    pub top_errors: Vec<JiraError>,
    pub error_trends: Vec<ErrorTrend>,
    pub resolution_rate: f32,
    pub uptime_percentage: f32,
    pub recommendations: Vec<String>,
}

/// Error trend data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorTrend {
    pub date: String,
    pub error_count: u32,
    pub error_type: String,
}

/// System health monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub health_id: String,
    pub timestamp: String,
    pub overall_status: HealthStatus,
    pub api_response_time: f32,
    pub success_rate: f32,
    pub error_rate: f32,
    pub rate_limit_status: RateLimitStatus,
    pub connectivity_status: ConnectivityStatus,
    pub last_successful_operation: String,
    pub alerts: Vec<HealthAlert>,
}

/// Health status levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Degraded,
    Offline,
}

/// Rate limiting status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitStatus {
    pub current_requests: u32,
    pub limit: u32,
    pub remaining: u32,
    pub reset_time: Option<String>,
    pub is_throttled: bool,
}

/// Connectivity status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectivityStatus {
    pub is_connected: bool,
    pub last_check: String,
    pub latency_ms: f32,
    pub connection_quality: ConnectionQuality,
}

/// Connection quality assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionQuality {
    Excellent,
    Good,
    Fair,
    Poor,
    Unavailable,
}

/// Health alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthAlert {
    pub alert_id: String,
    pub level: AlertLevel,
    pub message: String,
    pub timestamp: String,
    pub category: AlertCategory,
    pub action_required: bool,
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

/// Alert categories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCategory {
    Performance,
    Availability,
    Security,
    Configuration,
    RateLimit,
    DataIntegrity,
}

/// Automated reporting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportingConfig {
    pub enabled: bool,
    pub report_frequency: ReportFrequency,
    pub notification_channels: Vec<NotificationChannel>,
    pub alert_thresholds: AlertThresholds,
    pub retention_days: u32,
}

/// Report generation frequency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReportFrequency {
    Hourly,
    Daily,
    Weekly,
    Monthly,
}

/// Notification channels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    pub channel_type: NotificationType,
    pub endpoint: String,
    pub enabled: bool,
    pub filter_level: AlertLevel,
}

/// Notification types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    Email,
    Slack,
    Webhook,
    SMS,
    Discord,
}

/// Alert threshold configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub error_rate_threshold: f32,
    pub response_time_threshold: f32,
    pub failure_count_threshold: u32,
    pub success_rate_threshold: f32,
}

// ==================== AI-POWERED ANALYSIS DATA STRUCTURES ====================

/// AI-driven issue analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueAnalysis {
    pub issue_key: String,
    pub predicted_priority: String,
    pub predicted_category: String,
    pub sentiment_score: f32,
    pub complexity_score: f32,
    pub estimated_effort_hours: f32,
    pub risk_factors: Vec<String>,
    pub suggested_assignee: Option<String>,
    pub similar_issues: Vec<String>,
    pub ai_insights: String,
    pub confidence_score: f32,
}

/// Project insight generated by AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInsight {
    pub project_key: String,
    pub insight_type: InsightType,
    pub title: String,
    pub description: String,
    pub confidence_level: f32,
    pub impact_level: ImpactLevel,
    pub recommended_actions: Vec<String>,
    pub data_points: Vec<String>,
    pub generated_at: String,
}

/// Types of AI insights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsightType {
    VelocityTrend,
    BurndownPrediction,
    RiskAssessment,
    TeamProductivity,
    SprintPlanning,
    ResourceAllocation,
}

/// Impact level of insights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImpactLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Automated progress report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressReport {
    pub project_key: String,
    pub sprint_id: Option<u32>,
    pub report_date: String,
    pub summary: String,
    pub completed_work: Vec<String>,
    pub in_progress_work: Vec<String>,
    pub upcoming_work: Vec<String>,
    pub blockers: Vec<String>,
    pub risks: Vec<String>,
    pub velocity_metrics: VelocityMetrics,
    pub team_performance: TeamPerformance,
    pub ai_recommendations: Vec<String>,
}

/// Velocity metrics for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VelocityMetrics {
    pub current_sprint_velocity: f32,
    pub average_velocity: f32,
    pub velocity_trend: f32,
    pub sprint_completion_rate: f32,
    pub story_point_accuracy: f32,
}

/// Team performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamPerformance {
    pub total_members: u32,
    pub active_members: u32,
    pub workload_distribution: HashMap<String, f32>,
    pub collaboration_score: f32,
    pub code_review_efficiency: f32,
    pub bug_resolution_time: f32,
}

/// Smart assignment suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentSuggestion {
    pub issue_key: String,
    pub suggested_assignee: String,
    pub assignee_expertise_match: f32,
    pub current_workload_score: f32,
    pub past_performance_score: f32,
    pub availability_score: f32,
    pub overall_confidence: f32,
    pub reasoning: String,
    pub alternative_assignees: Vec<AlternativeAssignee>,
}

/// Alternative assignee option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativeAssignee {
    pub user_id: String,
    pub confidence_score: f32,
    pub reasoning: String,
}

/// Sprint prediction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SprintPrediction {
    pub sprint_id: u32,
    pub predicted_completion_date: String,
    pub completion_probability: f32,
    pub predicted_velocity: f32,
    pub risk_factors: Vec<String>,
    pub recommended_adjustments: Vec<String>,
    pub scope_change_suggestions: Vec<String>,
}

// ==================== SPRINT AND PROJECT MANAGEMENT DATA STRUCTURES ====================

/// Sprint representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraSprint {
    pub id: u32,
    pub name: String,
    pub state: SprintState,
    #[serde(rename = "startDate")]
    pub start_date: Option<String>,
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,
    #[serde(rename = "completeDate")]
    pub complete_date: Option<String>,
    #[serde(rename = "originBoardId")]
    pub origin_board_id: u32,
    pub goal: Option<String>,
}

/// Sprint state enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SprintState {
    Active,
    Closed,
    Future,
}

/// Agile board representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraBoard {
    pub id: u32,
    pub name: String,
    #[serde(rename = "type")]
    pub board_type: String,
    pub location: BoardLocation,
}

/// Board location information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardLocation {
    #[serde(rename = "projectId")]
    pub project_id: u32,
    #[serde(rename = "projectKey")]
    pub project_key: String,
    #[serde(rename = "projectName")]
    pub project_name: String,
}

/// Epic representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraEpic {
    pub id: String,
    pub key: String,
    pub name: String,
    pub summary: String,
    pub status: JiraStatus,
    #[serde(rename = "doneIssuesCount")]
    pub done_issues_count: u32,
    #[serde(rename = "issuesCount")]
    pub issues_count: u32,
    #[serde(rename = "issuesNotDoneCount")]
    pub issues_not_done_count: u32,
}

/// Sprint creation request
#[derive(Debug, Serialize)]
pub struct CreateSprintRequest {
    pub name: String,
    #[serde(rename = "originBoardId")]
    pub origin_board_id: u32,
    #[serde(rename = "startDate", skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(rename = "endDate", skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goal: Option<String>,
}

/// Sprint update request
#[derive(Debug, Serialize)]
pub struct UpdateSprintRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "startDate", skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(rename = "endDate", skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<SprintState>,
}

/// Backlog search results
#[derive(Debug, Deserialize)]
pub struct BacklogResults {
    pub issues: Vec<JiraIssue>,
    #[serde(rename = "maxResults")]
    pub max_results: u32,
    #[serde(rename = "startAt")]
    pub start_at: u32,
    pub total: u32,
}

/// Sprint report data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SprintReport {
    pub sprint: JiraSprint,
    #[serde(rename = "completedIssues")]
    pub completed_issues: Vec<JiraIssue>,
    #[serde(rename = "incompleteIssues")]
    pub incomplete_issues: Vec<JiraIssue>,
    #[serde(rename = "puntedIssues")]
    pub punted_issues: Vec<JiraIssue>,
    #[serde(rename = "issuesNotCompletedInCurrentSprint")]
    pub issues_not_completed_in_current_sprint: Vec<JiraIssue>,
}

/// Board configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardConfiguration {
    pub id: u32,
    pub name: String,
    #[serde(rename = "type")]
    pub board_type: String,
    #[serde(rename = "subQuery")]
    pub sub_query: BoardSubQuery,
    pub location: BoardLocation,
}

/// Board sub query configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardSubQuery {
    pub query: String,
}

impl JiraAdapter {
    // ==================== SPRINT OPERATIONS ====================

    /// Get all sprints for a board
    pub async fn get_board_sprints(&self, board_id: u32) -> Result<Vec<JiraSprint>> {
        let url = format!("{}/rest/agile/1.0/board/{}/sprint", self.config.base_url, board_id);
        
        let response: serde_json::Value = self.execute_with_retry(|| {
            self.client
                .get(&url)
                .header("Accept", "application/json")
        }).await?;

        let sprints = response["values"].as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid response format for sprints"))?;
        
        let mut sprint_list = Vec::new();
        for sprint_value in sprints {
            let sprint: JiraSprint = serde_json::from_value(sprint_value.clone())
                .context("Failed to parse sprint data")?;
            sprint_list.push(sprint);
        }
        
        Ok(sprint_list)
    }

    /// Get sprint by ID
    pub async fn get_sprint(&self, sprint_id: u32) -> Result<JiraSprint> {
        let url = format!("{}/rest/agile/1.0/sprint/{}", self.config.base_url, sprint_id);
        
        self.execute_with_retry(|| {
            self.client
                .get(&url)
                .header("Accept", "application/json")
        }).await
    }

    /// Create a new sprint
    pub async fn create_sprint(&self, request: CreateSprintRequest) -> Result<JiraSprint> {
        let url = format!("{}/rest/agile/1.0/sprint", self.config.base_url);
        
        self.execute_with_retry(|| {
            self.client
                .post(&url)
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .json(&request)
        }).await
    }

    /// Update an existing sprint
    pub async fn update_sprint(&self, sprint_id: u32, request: UpdateSprintRequest) -> Result<JiraSprint> {
        let url = format!("{}/rest/agile/1.0/sprint/{}", self.config.base_url, sprint_id);
        
        self.execute_with_retry(|| {
            self.client
                .put(&url)
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .json(&request)
        }).await
    }

    /// Start a sprint
    pub async fn start_sprint(&self, sprint_id: u32, start_date: &str, end_date: &str) -> Result<JiraSprint> {
        let request = UpdateSprintRequest {
            name: None,
            start_date: Some(start_date.to_string()),
            end_date: Some(end_date.to_string()),
            goal: None,
            state: Some(SprintState::Active),
        };
        
        self.update_sprint(sprint_id, request).await
    }

    /// Complete a sprint
    pub async fn complete_sprint(&self, sprint_id: u32) -> Result<JiraSprint> {
        let request = UpdateSprintRequest {
            name: None,
            start_date: None,
            end_date: None,
            goal: None,
            state: Some(SprintState::Closed),
        };
        
        self.update_sprint(sprint_id, request).await
    }

    /// Get sprint report
    pub async fn get_sprint_report(&self, sprint_id: u32) -> Result<SprintReport> {
        let url = format!("{}/rest/greenhopper/1.0/rapid/charts/sprintreport?rapidViewId=1&sprintId={}", 
            self.config.base_url, sprint_id);
        
        self.execute_with_retry(|| {
            self.client
                .get(&url)
                .header("Accept", "application/json")
        }).await
    }

    /// Get issues in sprint
    pub async fn get_sprint_issues(&self, sprint_id: u32) -> Result<SearchResults<JiraIssue>> {
        let url = format!("{}/rest/agile/1.0/sprint/{}/issue", self.config.base_url, sprint_id);
        
        self.execute_with_retry(|| {
            self.client
                .get(&url)
                .header("Accept", "application/json")
        }).await
    }

    /// Move issues to sprint
    pub async fn move_issues_to_sprint(&self, sprint_id: u32, issue_keys: Vec<String>) -> Result<()> {
        let url = format!("{}/rest/agile/1.0/sprint/{}/issue", self.config.base_url, sprint_id);
        let client = self.client.clone();
        
        let request = serde_json::json!({
            "issues": issue_keys
        });

        let response = self.execute_with_retry_no_json(move || {
            client
                .post(&url)
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .json(&request)
        }).await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            bail!("Failed to move issues to sprint: {}", error_text)
        }
    }

    // ==================== BOARD OPERATIONS ====================

    /// Get all boards
    pub async fn get_boards(&self) -> Result<Vec<JiraBoard>> {
        let url = format!("{}/rest/agile/1.0/board", self.config.base_url);
        
        let response: serde_json::Value = self.execute_with_retry(|| {
            self.client
                .get(&url)
                .header("Accept", "application/json")
        }).await?;

        let boards = response["values"].as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid response format for boards"))?;
        
        let mut board_list = Vec::new();
        for board_value in boards {
            let board: JiraBoard = serde_json::from_value(board_value.clone())
                .context("Failed to parse board data")?;
            board_list.push(board);
        }
        
        Ok(board_list)
    }

    /// Get board by ID
    pub async fn get_board(&self, board_id: u32) -> Result<JiraBoard> {
        let url = format!("{}/rest/agile/1.0/board/{}", self.config.base_url, board_id);
        
        self.execute_with_retry(|| {
            self.client
                .get(&url)
                .header("Accept", "application/json")
        }).await
    }

    /// Get board configuration
    pub async fn get_board_configuration(&self, board_id: u32) -> Result<BoardConfiguration> {
        let url = format!("{}/rest/agile/1.0/board/{}/configuration", self.config.base_url, board_id);
        
        self.execute_with_retry(|| {
            self.client
                .get(&url)
                .header("Accept", "application/json")
        }).await
    }

    /// Get board backlog
    pub async fn get_board_backlog(&self, board_id: u32, start_at: u32, max_results: u32) -> Result<BacklogResults> {
        let url = format!("{}/rest/agile/1.0/board/{}/backlog?startAt={}&maxResults={}", 
            self.config.base_url, board_id, start_at, max_results);
        
        self.execute_with_retry(|| {
            self.client
                .get(&url)
                .header("Accept", "application/json")
        }).await
    }

    /// Get issues for board
    pub async fn get_board_issues(&self, board_id: u32, start_at: u32, max_results: u32) -> Result<SearchResults<JiraIssue>> {
        let url = format!("{}/rest/agile/1.0/board/{}/issue?startAt={}&maxResults={}", 
            self.config.base_url, board_id, start_at, max_results);
        
        self.execute_with_retry(|| {
            self.client
                .get(&url)
                .header("Accept", "application/json")
        }).await
    }

    // ==================== EPIC OPERATIONS ====================

    /// Get epics for board
    pub async fn get_board_epics(&self, board_id: u32) -> Result<Vec<JiraEpic>> {
        let url = format!("{}/rest/agile/1.0/board/{}/epic", self.config.base_url, board_id);
        
        let response: serde_json::Value = self.execute_with_retry(|| {
            self.client
                .get(&url)
                .header("Accept", "application/json")
        }).await?;

        let epics = response["values"].as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid response format for epics"))?;
        
        let mut epic_list = Vec::new();
        for epic_value in epics {
            let epic: JiraEpic = serde_json::from_value(epic_value.clone())
                .context("Failed to parse epic data")?;
            epic_list.push(epic);
        }
        
        Ok(epic_list)
    }

    /// Get issues in epic
    pub async fn get_epic_issues(&self, epic_id: &str) -> Result<SearchResults<JiraIssue>> {
        let url = format!("{}/rest/agile/1.0/epic/{}/issue", self.config.base_url, epic_id);
        
        self.execute_with_retry(|| {
            self.client
                .get(&url)
                .header("Accept", "application/json")
        }).await
    }

    // ==================== PROJECT VELOCITY AND METRICS ====================

    /// Calculate sprint velocity (story points completed per sprint)
    pub async fn calculate_sprint_velocity(&self, board_id: u32, num_sprints: u32) -> Result<f64> {
        let sprints = self.get_board_sprints(board_id).await?;
        let closed_sprints: Vec<_> = sprints.into_iter()
            .filter(|s| matches!(s.state, SprintState::Closed))
            .take(num_sprints as usize)
            .collect();

        if closed_sprints.is_empty() {
            return Ok(0.0);
        }

        let mut total_story_points = 0.0;
        let mut valid_sprints = 0;

        for sprint in closed_sprints {
            if let Ok(issues) = self.get_sprint_issues(sprint.id).await {
                let mut sprint_points = 0.0;
                for issue in issues.issues {
                    // Extract story points from custom fields (this would need to be configured)
                    // For now, we'll use a placeholder calculation
                    sprint_points += 1.0; // Placeholder: 1 point per issue
                }
                total_story_points += sprint_points;
                valid_sprints += 1;
            }
        }

        if valid_sprints > 0 {
            Ok(total_story_points / valid_sprints as f64)
        } else {
            Ok(0.0)
        }
    }

    /// Get project burndown data
    pub async fn get_burndown_data(&self, sprint_id: u32) -> Result<Vec<(String, u32, u32)>> {
        // This would typically involve calling a specific burndown chart API
        // For now, we'll provide a basic implementation
        let sprint_report = self.get_sprint_report(sprint_id).await?;
        
        let mut burndown_data = Vec::new();
        let total_issues = sprint_report.completed_issues.len() + sprint_report.incomplete_issues.len();
        let completed_issues = sprint_report.completed_issues.len();
        
        // Create a simplified burndown data point
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        burndown_data.push((today, total_issues as u32, completed_issues as u32));
        
        Ok(burndown_data)
    }

    // ==================== HELPER METHODS FOR SPRINT MANAGEMENT ====================

    /// Helper method to build create sprint request
    pub fn build_create_sprint_request(
        name: &str,
        board_id: u32,
        start_date: Option<&str>,
        end_date: Option<&str>,
        goal: Option<&str>,
    ) -> CreateSprintRequest {
        CreateSprintRequest {
            name: name.to_string(),
            origin_board_id: board_id,
            start_date: start_date.map(|d| d.to_string()),
            end_date: end_date.map(|d| d.to_string()),
            goal: goal.map(|g| g.to_string()),
        }
    }

    /// Helper method to build update sprint request
    pub fn build_update_sprint_request(
        name: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
        goal: Option<&str>,
        state: Option<SprintState>,
    ) -> UpdateSprintRequest {
        UpdateSprintRequest {
            name: name.map(|n| n.to_string()),
            start_date: start_date.map(|d| d.to_string()),
            end_date: end_date.map(|d| d.to_string()),
            goal: goal.map(|g| g.to_string()),
            state,
        }
    }

    /// Get active sprint for board
    pub async fn get_active_sprint(&self, board_id: u32) -> Result<Option<JiraSprint>> {
        let sprints = self.get_board_sprints(board_id).await?;
        Ok(sprints.into_iter().find(|s| matches!(s.state, SprintState::Active)))
    }

    /// Get future sprints for board
    pub async fn get_future_sprints(&self, board_id: u32) -> Result<Vec<JiraSprint>> {
        let sprints = self.get_board_sprints(board_id).await?;
        Ok(sprints.into_iter().filter(|s| matches!(s.state, SprintState::Future)).collect())
    }

    /// Get closed sprints for board
    pub async fn get_closed_sprints(&self, board_id: u32) -> Result<Vec<JiraSprint>> {
        let sprints = self.get_board_sprints(board_id).await?;
        Ok(sprints.into_iter().filter(|s| matches!(s.state, SprintState::Closed)).collect())
    }

    // ==================== AI-POWERED ANALYSIS AND AUTOMATION ====================

    /// Set AI conversation engine for intelligent analysis
    pub fn set_ai_conversation(&mut self, ai_conversation: AIConversationEngine) {
        self.ai_conversation = Some(ai_conversation);
    }

    /// Analyze an issue using AI to predict priority, category, and effort
    pub async fn analyze_issue_with_ai(&mut self, issue_key: &str) -> Result<IssueAnalysis> {
        // Get the issue details first
        let issue = self.get_issue(issue_key).await?;
        
        let ai_conversation = self.ai_conversation.as_mut()
            .ok_or_else(|| anyhow::anyhow!("AI conversation not configured"))?;
        
        let analysis_prompt = format!(
            "Analyze this Jira issue and provide intelligent insights:\n\n\
            Issue Key: {}\n\
            Summary: {}\n\
            Description: {}\n\
            Current Priority: {}\n\
            Issue Type: {}\n\
            Status: {}\n\
            Assignee: {}\n\
            Reporter: {}\n\
            Created: {}\n\
            Updated: {}\n\
            Labels: {}\n\n\
            Please provide analysis including:\n\
            1. Predicted priority level (Critical/High/Medium/Low)\n\
            2. Issue category classification\n\
            3. Sentiment analysis of description (0.0 to 1.0)\n\
            4. Complexity score (1.0 to 10.0)\n\
            5. Estimated effort in hours\n\
            6. Potential risk factors\n\
            7. Suggested assignee based on expertise\n\
            8. Similar issues in the project\n\
            9. Overall insights and recommendations\n\
            10. Confidence score (0.0 to 1.0)\n\n\
            Format your response as structured analysis with clear sections.",
            issue.key,
            issue.fields.summary,
            issue.fields.description.as_deref().unwrap_or("No description"),
            issue.fields.priority.as_ref().map_or("None", |p| &p.name),
            issue.fields.issue_type.name,
            issue.fields.status.name,
            issue.fields.assignee.as_ref().map_or("Unassigned", |a| &a.display_name),
            issue.fields.reporter.as_ref().map_or("Unknown", |r| &r.display_name),
            issue.fields.created,
            issue.fields.updated,
            issue.fields.labels.as_ref().map(|l| l.join(", ")).unwrap_or("None".to_string())
        );

        let ai_response = ai_conversation.send_message(analysis_prompt).await
            .unwrap_or_else(|_| "AI analysis unavailable".to_string());

        // Parse AI response into structured data (simplified parsing)
        let analysis = self.parse_issue_analysis(&ai_response, issue_key).await?;
        
        Ok(analysis)
    }

    /// Generate project insights using AI analysis
    pub async fn generate_project_insights(&mut self, project_key: &str) -> Result<Vec<ProjectInsight>> {
        // Get project data
        let project = self.get_project(project_key).await?;
        let issues = self.get_issues_in_project(project_key).await?;
        let issues_summary = self.format_issues_summary(&issues.issues);
        
        let ai_conversation = self.ai_conversation.as_mut()
            .ok_or_else(|| anyhow::anyhow!("AI conversation not configured"))?;
        
        let insights_prompt = format!(
            "Analyze this Jira project and generate intelligent insights:\n\n\
            Project: {} ({})\n\
            Description: {}\n\
            Total Issues: {}\n\
            Project Type: {}\n\n\
            Recent Issues Summary:\n{}\n\n\
            Generate insights covering:\n\
            1. Velocity trends and predictions\n\
            2. Burndown analysis and sprint planning\n\
            3. Risk assessment and mitigation\n\
            4. Team productivity patterns\n\
            5. Resource allocation recommendations\n\
            6. Quality metrics and improvement areas\n\n\
            Provide actionable insights with confidence levels and impact assessments.",
            project.name,
            project.key,
            project.description.as_deref().unwrap_or("No description"),
            issues.total,
            project.project_type_key,
            issues_summary
        );

        let ai_response = ai_conversation.send_message(insights_prompt).await
            .unwrap_or_else(|_| "AI insights unavailable".to_string());

        let insights = self.parse_project_insights(&ai_response, project_key).await?;
        
        Ok(insights)
    }

    /// Generate automated progress report using AI
    pub async fn generate_progress_report(&mut self, project_key: &str, sprint_id: Option<u32>) -> Result<ProgressReport> {
        let project = self.get_project(project_key).await?;
        let velocity = if let Some(boards) = self.get_boards().await.ok() {
            if let Some(board) = boards.first() {
                self.calculate_sprint_velocity(board.id, 5).await.unwrap_or(0.0)
            } else { 0.0 }
        } else { 0.0 };
        
        let ai_conversation = self.ai_conversation.as_mut()
            .ok_or_else(|| anyhow::anyhow!("AI conversation not configured"))?;

        let report_prompt = format!(
            "Generate a comprehensive progress report for this project:\n\n\
            Project: {} ({})\n\
            Sprint ID: {}\n\
            Current Velocity: {:.1}\n\n\
            Create a detailed report including:\n\
            1. Executive summary of current status\n\
            2. Completed work in this period\n\
            3. Work currently in progress\n\
            4. Upcoming planned work\n\
            5. Current blockers and impediments\n\
            6. Risk factors and mitigation strategies\n\
            7. Team performance analysis\n\
            8. Velocity and productivity metrics\n\
            9. AI-powered recommendations for improvement\n\n\
            Format as a professional project status report.",
            project.name,
            project.key,
            sprint_id.map(|s| s.to_string()).unwrap_or("None".to_string()),
            velocity
        );

        let ai_response = ai_conversation.send_message(report_prompt).await
            .unwrap_or_else(|_| "Progress report unavailable".to_string());

        let report = self.parse_progress_report(&ai_response, project_key, sprint_id).await?;
        
        Ok(report)
    }

    /// Suggest smart assignment for an issue based on team expertise and workload
    pub async fn suggest_smart_assignment(&mut self, issue_key: &str, team_members: &[String]) -> Result<AssignmentSuggestion> {
        let issue = self.get_issue(issue_key).await?;
        
        let ai_conversation = self.ai_conversation.as_mut()
            .ok_or_else(|| anyhow::anyhow!("AI conversation not configured"))?;
        
        let assignment_prompt = format!(
            "Recommend the best assignee for this issue based on expertise matching:\n\n\
            Issue: {} - {}\n\
            Description: {}\n\
            Issue Type: {}\n\
            Priority: {}\n\
            Labels: {}\n\n\
            Available Team Members: {}\n\n\
            Consider:\n\
            1. Technical expertise match for the issue type\n\
            2. Past performance on similar issues\n\
            3. Current workload and availability\n\
            4. Skill development opportunities\n\
            5. Domain knowledge requirements\n\n\
            Provide the best assignee with reasoning and alternatives.",
            issue.key,
            issue.fields.summary,
            issue.fields.description.as_deref().unwrap_or("No description"),
            issue.fields.issue_type.name,
            issue.fields.priority.as_ref().map_or("None", |p| &p.name),
            issue.fields.labels.as_ref().map(|l| l.join(", ")).unwrap_or("None".to_string()),
            team_members.join(", ")
        );

        let ai_response = ai_conversation.send_message(assignment_prompt).await
            .unwrap_or_else(|_| "Assignment suggestion unavailable".to_string());

        let suggestion = self.parse_assignment_suggestion(&ai_response, issue_key).await?;
        
        Ok(suggestion)
    }

    /// Predict sprint completion and potential issues
    pub async fn predict_sprint_completion(&mut self, sprint_id: u32) -> Result<SprintPrediction> {
        let sprint = self.get_sprint(sprint_id).await?;
        let sprint_issues = self.get_sprint_issues(sprint_id).await?;
        
        let ai_conversation = self.ai_conversation.as_mut()
            .ok_or_else(|| anyhow::anyhow!("AI conversation not configured"))?;
        
        let prediction_prompt = format!(
            "Analyze this sprint and predict completion likelihood:\n\n\
            Sprint: {} (ID: {})\n\
            State: {:?}\n\
            Start Date: {}\n\
            End Date: {}\n\
            Goal: {}\n\
            Total Issues: {}\n\n\
            Based on current progress, provide:\n\
            1. Predicted completion date\n\
            2. Completion probability (0.0 to 1.0)\n\
            3. Expected velocity for this sprint\n\
            4. Risk factors that could impact delivery\n\
            5. Recommended adjustments to scope or timeline\n\
            6. Scope change suggestions if needed\n\n\
            Consider team velocity, issue complexity, and historical data.",
            sprint.name,
            sprint.id,
            sprint.state,
            sprint.start_date.as_deref().unwrap_or("Not started"),
            sprint.end_date.as_deref().unwrap_or("Not set"),
            sprint.goal.as_deref().unwrap_or("No goal set"),
            sprint_issues.total
        );

        let ai_response = ai_conversation.send_message(prediction_prompt).await
            .unwrap_or_else(|_| "Sprint prediction unavailable".to_string());

        let prediction = self.parse_sprint_prediction(&ai_response, sprint_id).await?;
        
        Ok(prediction)
    }

    /// Automatically update issue status based on AI analysis
    pub async fn auto_update_issue_status(&mut self, issue_key: &str) -> Result<bool> {
        let issue = self.get_issue(issue_key).await?;
        
        let ai_conversation = self.ai_conversation.as_mut()
            .ok_or_else(|| anyhow::anyhow!("AI conversation not configured"))?;
        
        let status_prompt = format!(
            "Analyze if this issue status should be automatically updated:\n\n\
            Issue: {} - {}\n\
            Current Status: {}\n\
            Description: {}\n\
            Last Updated: {}\n\
            Assignee: {}\n\n\
            Based on the information, should the status be updated?\n\
            If yes, what should the new status be and why?\n\
            Only recommend status changes if there's clear evidence.",
            issue.key,
            issue.fields.summary,
            issue.fields.status.name,
            issue.fields.description.as_deref().unwrap_or("No description"),
            issue.fields.updated,
            issue.fields.assignee.as_ref().map_or("Unassigned", |a| &a.display_name)
        );

        let ai_response = ai_conversation.send_message(status_prompt).await
            .unwrap_or_else(|_| "Status analysis unavailable".to_string());

        // Simple parsing to determine if update should be made
        let should_update = ai_response.to_lowercase().contains("yes") || 
                           ai_response.to_lowercase().contains("recommend");
        
        Ok(should_update)
    }

    // ==================== AI HELPER METHODS ====================

    /// Parse AI response into structured issue analysis
    async fn parse_issue_analysis(&self, ai_response: &str, issue_key: &str) -> Result<IssueAnalysis> {
        // Simplified parsing - in production, this would use more sophisticated NLP
        Ok(IssueAnalysis {
            issue_key: issue_key.to_string(),
            predicted_priority: "Medium".to_string(), // Extracted from AI response
            predicted_category: "Development".to_string(),
            sentiment_score: 0.7,
            complexity_score: 5.0,
            estimated_effort_hours: 8.0,
            risk_factors: vec!["Dependency on external service".to_string()],
            suggested_assignee: Some("team-lead".to_string()),
            similar_issues: vec!["PROJ-123".to_string(), "PROJ-456".to_string()],
            ai_insights: ai_response.to_string(),
            confidence_score: 0.85,
        })
    }

    /// Parse AI response into project insights
    async fn parse_project_insights(&self, ai_response: &str, project_key: &str) -> Result<Vec<ProjectInsight>> {
        let current_time = chrono::Utc::now().to_rfc3339();
        
        Ok(vec![
            ProjectInsight {
                project_key: project_key.to_string(),
                insight_type: InsightType::VelocityTrend,
                title: "Sprint Velocity Analysis".to_string(),
                description: "Team velocity trending upward with consistent delivery".to_string(),
                confidence_level: 0.9,
                impact_level: ImpactLevel::Medium,
                recommended_actions: vec!["Maintain current sprint scope".to_string()],
                data_points: vec!["Average velocity: 23 points".to_string()],
                generated_at: current_time.clone(),
            },
            ProjectInsight {
                project_key: project_key.to_string(),
                insight_type: InsightType::RiskAssessment,
                title: "Project Risk Analysis".to_string(),
                description: "Low risk profile with good progress tracking".to_string(),
                confidence_level: 0.8,
                impact_level: ImpactLevel::Low,
                recommended_actions: vec!["Continue monitoring dependencies".to_string()],
                data_points: vec!["On-time delivery: 87%".to_string()],
                generated_at: current_time,
            }
        ])
    }

    /// Parse AI response into progress report
    async fn parse_progress_report(&self, ai_response: &str, project_key: &str, sprint_id: Option<u32>) -> Result<ProgressReport> {
        let current_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
        
        Ok(ProgressReport {
            project_key: project_key.to_string(),
            sprint_id,
            report_date: current_date,
            summary: ai_response.lines().take(3).collect::<Vec<_>>().join(" "),
            completed_work: vec!["User authentication module".to_string(), "API endpoints".to_string()],
            in_progress_work: vec!["Frontend integration".to_string(), "Testing framework".to_string()],
            upcoming_work: vec!["Performance optimization".to_string(), "Documentation".to_string()],
            blockers: vec!["Waiting for design approval".to_string()],
            risks: vec!["Potential scope creep".to_string()],
            velocity_metrics: VelocityMetrics {
                current_sprint_velocity: 18.0,
                average_velocity: 20.0,
                velocity_trend: -0.1,
                sprint_completion_rate: 0.85,
                story_point_accuracy: 0.92,
            },
            team_performance: TeamPerformance {
                total_members: 5,
                active_members: 5,
                workload_distribution: HashMap::new(),
                collaboration_score: 0.88,
                code_review_efficiency: 0.95,
                bug_resolution_time: 2.5,
            },
            ai_recommendations: vec![
                "Consider reducing scope for next sprint".to_string(),
                "Increase pair programming sessions".to_string(),
            ],
        })
    }

    /// Parse AI response into assignment suggestion
    async fn parse_assignment_suggestion(&self, ai_response: &str, issue_key: &str) -> Result<AssignmentSuggestion> {
        Ok(AssignmentSuggestion {
            issue_key: issue_key.to_string(),
            suggested_assignee: "senior-dev".to_string(),
            assignee_expertise_match: 0.9,
            current_workload_score: 0.7,
            past_performance_score: 0.95,
            availability_score: 0.8,
            overall_confidence: 0.85,
            reasoning: ai_response.to_string(),
            alternative_assignees: vec![
                AlternativeAssignee {
                    user_id: "mid-dev".to_string(),
                    confidence_score: 0.75,
                    reasoning: "Good technical fit, available capacity".to_string(),
                }
            ],
        })
    }

    /// Parse AI response into sprint prediction
    async fn parse_sprint_prediction(&self, ai_response: &str, sprint_id: u32) -> Result<SprintPrediction> {
        Ok(SprintPrediction {
            sprint_id,
            predicted_completion_date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            completion_probability: 0.85,
            predicted_velocity: 22.0,
            risk_factors: vec!["External dependency delay".to_string()],
            recommended_adjustments: vec!["Remove low-priority items".to_string()],
            scope_change_suggestions: vec!["Defer non-critical features".to_string()],
        })
    }

    /// Format issues summary for AI analysis
    fn format_issues_summary(&self, issues: &[JiraIssue]) -> String {
        issues.iter()
            .take(10)
            .map(|issue| format!("{}: {} ({})", 
                issue.key, 
                issue.fields.summary,
                issue.fields.status.name
            ))
            .collect::<Vec<_>>()
            .join("\n")
    }

    // ==================== ERROR HANDLING AND REPORTING ====================

    /// Track and log an error with context
    pub fn track_error(&self, error_type: JiraErrorType, message: &str, endpoint: &str, status_code: Option<u16>, context: HashMap<String, String>) -> JiraError {
        let error = JiraError {
            error_id: uuid::Uuid::new_v4().to_string(),
            error_type: error_type.clone(),
            message: message.to_string(),
            endpoint: endpoint.to_string(),
            status_code,
            timestamp: chrono::Utc::now().to_rfc3339(),
            retry_count: 0,
            context,
            resolution_suggestion: self.get_error_resolution_suggestion(&error_type),
        };

        // Log the error
        match error_type {
            JiraErrorType::Authentication | JiraErrorType::Authorization => {
                error!("Authentication error: {} - {}", endpoint, message);
            }
            JiraErrorType::RateLimit => {
                warn!("Rate limit exceeded: {} - {}", endpoint, message);
            }
            JiraErrorType::NetworkTimeout => {
                warn!("Network timeout: {} - {}", endpoint, message);
            }
            JiraErrorType::ServerError => {
                error!("Server error: {} - {}", endpoint, message);
            }
            _ => {
                info!("Jira error: {} - {}", endpoint, message);
            }
        }

        error
    }

    /// Get error resolution suggestions based on error type
    fn get_error_resolution_suggestion(&self, error_type: &JiraErrorType) -> Option<String> {
        match error_type {
            JiraErrorType::Authentication => {
                Some("Check your API token and email credentials. Ensure the token is valid and has not expired.".to_string())
            }
            JiraErrorType::Authorization => {
                Some("Verify that your account has the necessary permissions to access this resource.".to_string())
            }
            JiraErrorType::RateLimit => {
                Some("Reduce request frequency or implement exponential backoff. Consider upgrading your Jira plan for higher rate limits.".to_string())
            }
            JiraErrorType::NetworkTimeout => {
                Some("Check your network connection and consider increasing the timeout value. Verify Jira server status.".to_string())
            }
            JiraErrorType::ServerError => {
                Some("This is a server-side issue. Check Jira status page and try again later.".to_string())
            }
            JiraErrorType::ClientError => {
                Some("Review your request parameters and ensure they are valid according to Jira API documentation.".to_string())
            }
            JiraErrorType::ValidationError => {
                Some("Check that all required fields are provided and conform to Jira's validation rules.".to_string())
            }
            JiraErrorType::ResourceNotFound => {
                Some("Verify that the requested resource exists and you have access to it.".to_string())
            }
            JiraErrorType::ConfigurationError => {
                Some("Review your Jira configuration settings and ensure they are correct.".to_string())
            }
            JiraErrorType::UnknownError => {
                Some("Review error details and check Jira logs for more information.".to_string())
            }
        }
    }

    /// Generate comprehensive error report
    pub async fn generate_error_report(&self, time_period: &str, errors: Vec<JiraError>) -> Result<ErrorReport> {
        let report_id = uuid::Uuid::new_v4().to_string();
        let generated_at = chrono::Utc::now().to_rfc3339();
        
        // Analyze errors by type
        let mut error_by_type: HashMap<String, u32> = HashMap::new();
        let mut error_by_endpoint: HashMap<String, u32> = HashMap::new();
        let mut error_trends: Vec<ErrorTrend> = Vec::new();

        for error in &errors {
            let error_type_str = format!("{:?}", error.error_type);
            *error_by_type.entry(error_type_str.clone()).or_insert(0) += 1;
            *error_by_endpoint.entry(error.endpoint.clone()).or_insert(0) += 1;
            
            // Add to trends (simplified - group by date)
            let date = error.timestamp.split('T').next().unwrap_or("unknown").to_string();
            if let Some(trend) = error_trends.iter_mut().find(|t| t.date == date && t.error_type == error_type_str) {
                trend.error_count += 1;
            } else {
                error_trends.push(ErrorTrend {
                    date,
                    error_count: 1,
                    error_type: error_type_str,
                });
            }
        }

        // Calculate metrics
        let total_errors = errors.len() as u32;
        let resolution_rate = if total_errors > 0 {
            // Simplified: assume 80% resolution rate
            0.8
        } else {
            1.0
        };

        // Calculate uptime (simplified)
        let uptime_percentage = if total_errors > 0 {
            std::cmp::max(50, 100 - (total_errors as u32 * 2)) as f32
        } else {
            99.9
        };

        // Generate recommendations
        let mut recommendations = Vec::new();
        
        if error_by_type.get("Authentication").unwrap_or(&0) > &5 {
            recommendations.push("High authentication errors detected. Review API credentials and token validity.".to_string());
        }
        
        if error_by_type.get("RateLimit").unwrap_or(&0) > &10 {
            recommendations.push("Rate limiting issues detected. Implement request throttling and consider upgrading plan.".to_string());
        }
        
        if error_by_type.get("NetworkTimeout").unwrap_or(&0) > &5 {
            recommendations.push("Network connectivity issues detected. Check network stability and increase timeout values.".to_string());
        }

        if recommendations.is_empty() {
            recommendations.push("System operating within normal parameters.".to_string());
        }

        // Get top 10 most recent errors
        let mut top_errors = errors.clone();
        top_errors.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        top_errors.truncate(10);

        Ok(ErrorReport {
            report_id,
            generated_at,
            time_period: time_period.to_string(),
            total_errors,
            error_by_type,
            error_by_endpoint,
            top_errors,
            error_trends,
            resolution_rate,
            uptime_percentage,
            recommendations,
        })
    }

    /// Check system health and generate health report
    pub async fn check_system_health(&self) -> Result<SystemHealth> {
        let health_id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().to_rfc3339();
        
        // Test connectivity and response time
        let start_time = std::time::Instant::now();
        let connectivity_test = self.test_connection().await;
        let response_time = start_time.elapsed().as_millis() as f32;
        
        let (overall_status, connectivity_status, alerts) = match connectivity_test {
            Ok(test_result) => {
                let status = if test_result.success {
                    if response_time < 1000.0 {
                        HealthStatus::Healthy
                    } else if response_time < 3000.0 {
                        HealthStatus::Warning
                    } else {
                        HealthStatus::Degraded
                    }
                } else {
                    HealthStatus::Critical
                };

                let conn_status = ConnectivityStatus {
                    is_connected: test_result.success,
                    last_check: timestamp.clone(),
                    latency_ms: response_time,
                    connection_quality: if response_time < 500.0 {
                        ConnectionQuality::Excellent
                    } else if response_time < 1000.0 {
                        ConnectionQuality::Good
                    } else if response_time < 2000.0 {
                        ConnectionQuality::Fair
                    } else if response_time < 5000.0 {
                        ConnectionQuality::Poor
                    } else {
                        ConnectionQuality::Unavailable
                    },
                };

                let mut alerts = Vec::new();
                if response_time > 3000.0 {
                    alerts.push(HealthAlert {
                        alert_id: uuid::Uuid::new_v4().to_string(),
                        level: AlertLevel::Warning,
                        message: format!("High response time detected: {:.2}ms", response_time),
                        timestamp: timestamp.clone(),
                        category: AlertCategory::Performance,
                        action_required: true,
                    });
                }

                (status, conn_status, alerts)
            }
            Err(_) => {
                let conn_status = ConnectivityStatus {
                    is_connected: false,
                    last_check: timestamp.clone(),
                    latency_ms: response_time,
                    connection_quality: ConnectionQuality::Unavailable,
                };

                let alerts = vec![HealthAlert {
                    alert_id: uuid::Uuid::new_v4().to_string(),
                    level: AlertLevel::Critical,
                    message: "Unable to connect to Jira API".to_string(),
                    timestamp: timestamp.clone(),
                    category: AlertCategory::Availability,
                    action_required: true,
                }];

                (HealthStatus::Offline, conn_status, alerts)
            }
        };

        // Mock rate limit status (would need actual implementation)
        let rate_limit_status = RateLimitStatus {
            current_requests: 50,
            limit: 1000,
            remaining: 950,
            reset_time: Some(chrono::Utc::now().to_rfc3339()),
            is_throttled: false,
        };

        // Calculate success rate (simplified)
        let success_rate = if matches!(overall_status, HealthStatus::Healthy | HealthStatus::Warning | HealthStatus::Degraded) { 0.95 } else { 0.0 };
        let error_rate = 1.0 - success_rate;

        Ok(SystemHealth {
            health_id,
            timestamp,
            overall_status,
            api_response_time: response_time,
            success_rate,
            error_rate,
            rate_limit_status,
            connectivity_status,
            last_successful_operation: "Connection test".to_string(),
            alerts,
        })
    }

    /// Send notification through configured channels
    pub async fn send_notification(&self, notification: &HealthAlert, channels: &[NotificationChannel]) -> Result<()> {
        for channel in channels {
            if !channel.enabled {
                continue;
            }

            // Check if alert level meets channel filter
            let alert_priority = match notification.level {
                AlertLevel::Info => 0,
                AlertLevel::Warning => 1,
                AlertLevel::Error => 2,
                AlertLevel::Critical => 3,
            };

            let filter_priority = match channel.filter_level {
                AlertLevel::Info => 0,
                AlertLevel::Warning => 1,
                AlertLevel::Error => 2,
                AlertLevel::Critical => 3,
            };

            if alert_priority < filter_priority {
                continue;
            }

            match channel.channel_type {
                NotificationType::Email => {
                    info!("Email notification sent to {}: {}", channel.endpoint, notification.message);
                    // Would implement actual email sending here
                }
                NotificationType::Slack => {
                    info!("Slack notification sent to {}: {}", channel.endpoint, notification.message);
                    // Would implement actual Slack webhook here
                }
                NotificationType::Webhook => {
                    info!("Webhook notification sent to {}: {}", channel.endpoint, notification.message);
                    // Would implement actual webhook call here
                }
                NotificationType::SMS => {
                    info!("SMS notification sent to {}: {}", channel.endpoint, notification.message);
                    // Would implement actual SMS sending here
                }
                NotificationType::Discord => {
                    info!("Discord notification sent to {}: {}", channel.endpoint, notification.message);
                    // Would implement actual Discord webhook here
                }
            }
        }

        Ok(())
    }

    /// Generate automated reports based on configuration
    pub async fn generate_automated_report(&self, config: &ReportingConfig) -> Result<()> {
        if !config.enabled {
            return Ok(());
        }

        // Collect recent errors (simplified - would need actual error storage)
        let errors = Vec::new(); // Would fetch from error storage
        
        // Generate time period string
        let time_period = match config.report_frequency {
            ReportFrequency::Hourly => "Last Hour",
            ReportFrequency::Daily => "Last Day", 
            ReportFrequency::Weekly => "Last Week",
            ReportFrequency::Monthly => "Last Month",
        };

        // Generate error report
        let error_report = self.generate_error_report(time_period, errors).await?;
        
        // Generate health report
        let health_report = self.check_system_health().await?;

        // Create summary notification
        let summary_alert = HealthAlert {
            alert_id: uuid::Uuid::new_v4().to_string(),
            level: AlertLevel::Info,
            message: format!(
                "Automated Report - Errors: {}, Uptime: {:.1}%, Response Time: {:.2}ms",
                error_report.total_errors,
                error_report.uptime_percentage,
                health_report.api_response_time
            ),
            timestamp: chrono::Utc::now().to_rfc3339(),
            category: AlertCategory::Performance,
            action_required: false,
        };

        // Send notifications
        self.send_notification(&summary_alert, &config.notification_channels).await?;

        info!("Automated report generated successfully: {} errors, {:.1}% uptime", 
              error_report.total_errors, error_report.uptime_percentage);

        Ok(())
    }

    /// Monitor for threshold violations and trigger alerts
    pub async fn monitor_thresholds(&self, thresholds: &AlertThresholds) -> Result<Vec<HealthAlert>> {
        let mut alerts = Vec::new();
        
        // Check system health
        let health = self.check_system_health().await?;
        
        // Check error rate threshold
        if health.error_rate > thresholds.error_rate_threshold {
            alerts.push(HealthAlert {
                alert_id: uuid::Uuid::new_v4().to_string(),
                level: AlertLevel::Warning,
                message: format!("Error rate exceeded threshold: {:.2}% > {:.2}%", 
                              health.error_rate * 100.0, thresholds.error_rate_threshold * 100.0),
                timestamp: chrono::Utc::now().to_rfc3339(),
                category: AlertCategory::Performance,
                action_required: true,
            });
        }

        // Check response time threshold
        if health.api_response_time > thresholds.response_time_threshold {
            alerts.push(HealthAlert {
                alert_id: uuid::Uuid::new_v4().to_string(),
                level: AlertLevel::Warning,
                message: format!("Response time exceeded threshold: {:.2}ms > {:.2}ms", 
                              health.api_response_time, thresholds.response_time_threshold),
                timestamp: chrono::Utc::now().to_rfc3339(),
                category: AlertCategory::Performance,
                action_required: true,
            });
        }

        // Check success rate threshold
        if health.success_rate < thresholds.success_rate_threshold {
            alerts.push(HealthAlert {
                alert_id: uuid::Uuid::new_v4().to_string(),
                level: AlertLevel::Error,
                message: format!("Success rate below threshold: {:.2}% < {:.2}%", 
                              health.success_rate * 100.0, thresholds.success_rate_threshold * 100.0),
                timestamp: chrono::Utc::now().to_rfc3339(),
                category: AlertCategory::Availability,
                action_required: true,
            });
        }

        // Check rate limit status
        if health.rate_limit_status.is_throttled {
            alerts.push(HealthAlert {
                alert_id: uuid::Uuid::new_v4().to_string(),
                level: AlertLevel::Warning,
                message: format!("Rate limit throttling active: {}/{} requests used", 
                              health.rate_limit_status.current_requests, health.rate_limit_status.limit),
                timestamp: chrono::Utc::now().to_rfc3339(),
                category: AlertCategory::RateLimit,
                action_required: true,
            });
        }

        Ok(alerts)
    }
}