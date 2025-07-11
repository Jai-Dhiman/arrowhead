use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::obsidian_adapter::ObsidianAdapter;
use crate::jira_adapter::JiraAdapter;
use crate::calendar_adapter::{CalendarAdapter, CalendarEvent};
use crate::workflow_engine::{WorkflowAction, WorkflowActionExecutor, WorkflowContext};

/// Unified data model for notes across different systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedNote {
    pub id: String,
    pub title: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub source: String, // obsidian, jira, etc.
    pub source_id: String,
    pub metadata: HashMap<String, String>,
}

/// Unified data model for tasks across different systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedTask {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub due_date: Option<DateTime<Utc>>,
    pub source: String, // jira, obsidian, etc.
    pub source_id: String,
    pub metadata: HashMap<String, String>,
}

/// Unified data model for calendar events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedEvent {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub attendees: Vec<String>,
    pub location: Option<String>,
    pub source: String, // google, outlook, etc.
    pub source_id: String,
    pub metadata: HashMap<String, String>,
}

/// Authentication credentials for different services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceCredentials {
    pub service_type: String,
    pub credentials: HashMap<String, String>,
}

/// Configuration for service connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub service_type: String,
    pub base_url: Option<String>,
    pub enabled: bool,
    pub rate_limit: Option<u32>,
    pub timeout_seconds: Option<u32>,
    pub retry_attempts: Option<u32>,
    pub metadata: HashMap<String, String>,
}

/// Trait for service adapters
#[async_trait]
pub trait ServiceAdapter: Send + Sync {
    async fn health_check(&self) -> Result<bool>;
    async fn authenticate(&self, credentials: &ServiceCredentials) -> Result<()>;
    fn get_service_type(&self) -> &str;
    fn is_authenticated(&self) -> bool;
}

/// Trait for note-based services
#[async_trait]
pub trait NoteService: ServiceAdapter {
    async fn create_note(&self, note: &UnifiedNote) -> Result<String>;
    async fn update_note(&self, id: &str, note: &UnifiedNote) -> Result<()>;
    async fn get_note(&self, id: &str) -> Result<Option<UnifiedNote>>;
    async fn list_notes(&self, filter: Option<&str>) -> Result<Vec<UnifiedNote>>;
    async fn delete_note(&self, id: &str) -> Result<()>;
}

/// Trait for task-based services
#[async_trait]
pub trait TaskService: ServiceAdapter {
    async fn create_task(&self, task: &UnifiedTask) -> Result<String>;
    async fn update_task(&self, id: &str, task: &UnifiedTask) -> Result<()>;
    async fn get_task(&self, id: &str) -> Result<Option<UnifiedTask>>;
    async fn list_tasks(&self, filter: Option<&str>) -> Result<Vec<UnifiedTask>>;
    async fn delete_task(&self, id: &str) -> Result<()>;
}

/// Trait for calendar-based services
#[async_trait]
pub trait CalendarService: ServiceAdapter {
    async fn create_event(&self, event: &UnifiedEvent) -> Result<String>;
    async fn update_event(&self, id: &str, event: &UnifiedEvent) -> Result<()>;
    async fn get_event(&self, id: &str) -> Result<Option<UnifiedEvent>>;
    async fn list_events(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<UnifiedEvent>>;
    async fn delete_event(&self, id: &str) -> Result<()>;
}

/// Obsidian service adapter
pub struct ObsidianService {
    adapter: Arc<ObsidianAdapter>,
    authenticated: bool,
}

impl ObsidianService {
    pub fn new(adapter: Arc<ObsidianAdapter>) -> Self {
        Self {
            adapter,
            authenticated: false,
        }
    }
}

#[async_trait]
impl ServiceAdapter for ObsidianService {
    async fn health_check(&self) -> Result<bool> {
        // Simple health check by trying to list vaults
        match self.adapter.list_vaults().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    async fn authenticate(&self, _credentials: &ServiceCredentials) -> Result<()> {
        // Obsidian authentication is typically handled by the adapter
        Ok(())
    }

    fn get_service_type(&self) -> &str {
        "obsidian"
    }

    fn is_authenticated(&self) -> bool {
        self.authenticated
    }
}

#[async_trait]
impl NoteService for ObsidianService {
    async fn create_note(&self, note: &UnifiedNote) -> Result<String> {
        // Extract vault from metadata or use default
        let vault = note.metadata.get("vault").unwrap_or(&"default".to_string()).clone();
        let path = format!("{}.md", note.title.replace(" ", "_"));
        
        self.adapter.create_note(&vault, &path, &note.content).await
            .context("Failed to create note in Obsidian")?;
        
        Ok(format!("{}:{}", vault, path))
    }

    async fn update_note(&self, id: &str, note: &UnifiedNote) -> Result<()> {
        let parts: Vec<&str> = id.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid note ID format"));
        }
        
        let vault = parts[0];
        let path = parts[1];
        
        self.adapter.update_note(vault, path, &note.content).await
            .context("Failed to update note in Obsidian")
    }

    async fn get_note(&self, id: &str) -> Result<Option<UnifiedNote>> {
        let parts: Vec<&str> = id.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid note ID format"));
        }
        
        let vault = parts[0];
        let path = parts[1];
        
        match self.adapter.get_note(vault, path).await {
            Ok(obsidian_note) => {
                let unified_note = UnifiedNote {
                    id: id.to_string(),
                    title: path.to_string(), // Use path as title for now
                    content: obsidian_note.content.clone(),
                    created_at: Utc::now(), // Use current time as default
                    updated_at: Utc::now(), // Use current time as default
                    tags: obsidian_note.frontmatter.tags.clone().unwrap_or_default(),
                    source: "obsidian".to_string(),
                    source_id: format!("{}:{}", vault, path),
                    metadata: {
                        let mut metadata = HashMap::new();
                        metadata.insert("vault".to_string(), vault.to_string());
                        metadata.insert("path".to_string(), path.to_string());
                        metadata
                    },
                };
                Ok(Some(unified_note))
            }
            Err(_) => Ok(None),
        }
    }

    async fn list_notes(&self, filter: Option<&str>) -> Result<Vec<UnifiedNote>> {
        let vaults = self.adapter.list_vaults().await?;
        let mut unified_notes = Vec::new();
        
        for vault in vaults {
            let note_paths = self.adapter.list_notes(&vault).await?;
            
            for note_path in note_paths {
                if let Some(filter_str) = filter {
                    if !note_path.contains(filter_str) {
                        continue;
                    }
                }
                
                let unified_note = UnifiedNote {
                    id: format!("{}:{}", vault, note_path),
                    title: note_path.clone(),
                    content: String::new(), // Empty content for list
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    tags: Vec::new(),
                    source: "obsidian".to_string(),
                    source_id: format!("{}:{}", vault, note_path),
                    metadata: {
                        let mut metadata = HashMap::new();
                        metadata.insert("vault".to_string(), vault.clone());
                        metadata.insert("path".to_string(), note_path.clone());
                        metadata
                    },
                };
                unified_notes.push(unified_note);
            }
        }
        
        Ok(unified_notes)
    }

    async fn delete_note(&self, id: &str) -> Result<()> {
        let parts: Vec<&str> = id.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid note ID format"));
        }
        
        let vault = parts[0];
        let path = parts[1];
        
        self.adapter.delete_note(vault, path).await
            .context("Failed to delete note in Obsidian")
    }
}

/// Jira service adapter
pub struct JiraService {
    adapter: Arc<JiraAdapter>,
    authenticated: bool,
}

impl JiraService {
    pub fn new(adapter: Arc<JiraAdapter>) -> Self {
        Self {
            adapter,
            authenticated: false,
        }
    }
}

#[async_trait]
impl ServiceAdapter for JiraService {
    async fn health_check(&self) -> Result<bool> {
        // Simple health check by trying to get server info
        match self.adapter.get_server_info().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    async fn authenticate(&self, credentials: &ServiceCredentials) -> Result<()> {
        // Jira authentication is typically handled by the adapter
        // Here we could validate the credentials if needed
        Ok(())
    }

    fn get_service_type(&self) -> &str {
        "jira"
    }

    fn is_authenticated(&self) -> bool {
        self.authenticated
    }
}

#[async_trait]
impl TaskService for JiraService {
    async fn create_task(&self, task: &UnifiedTask) -> Result<String> {
        let project = task.metadata.get("project")
            .ok_or_else(|| anyhow::anyhow!("Project not specified for Jira task"))?;
        
        let default_issue_type = "Task".to_string();
        let issue_type = task.metadata.get("issue_type").unwrap_or(&default_issue_type);
        
        let request = crate::jira_adapter::CreateIssueRequest {
            fields: crate::jira_adapter::CreateIssueFields {
                summary: task.title.clone(),
                description: task.description.clone(),
                issue_type: crate::jira_adapter::IssueTypeReference {
                    id: issue_type.clone(),
                },
                project: crate::jira_adapter::ProjectReference {
                    key: project.clone(),
                },
                assignee: task.assignee.as_ref().map(|a| crate::jira_adapter::UserReference {
                    account_id: a.clone(),
                }),
                priority: None,
                labels: None,
            },
        };
        
        let issue = self.adapter.create_issue(request).await?;
        
        Ok(issue.key)
    }

    async fn update_task(&self, id: &str, task: &UnifiedTask) -> Result<()> {
        let request = crate::jira_adapter::UpdateIssueRequest {
            fields: crate::jira_adapter::UpdateIssueFields {
                summary: Some(task.title.clone()),
                description: task.description.clone(),
                assignee: task.assignee.as_ref().map(|a| crate::jira_adapter::UserReference {
                    account_id: a.clone(),
                }),
                priority: None,
                labels: None,
            },
        };
        
        self.adapter.update_issue(id, request).await
            .context("Failed to update task in Jira")
    }

    async fn get_task(&self, id: &str) -> Result<Option<UnifiedTask>> {
        match self.adapter.get_issue(id).await {
            Ok(jira_issue) => {
                let unified_task = UnifiedTask {
                    id: id.to_string(),
                    title: jira_issue.fields.summary,
                    description: jira_issue.fields.description,
                    status: jira_issue.fields.status.name,
                    priority: jira_issue.fields.priority.map(|p| p.name),
                    assignee: jira_issue.fields.assignee.map(|a| a.display_name),
                    created_at: DateTime::parse_from_rfc3339(&jira_issue.fields.created)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: DateTime::parse_from_rfc3339(&jira_issue.fields.updated)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    due_date: None, // JiraIssue doesn't have due_date field
                    source: "jira".to_string(),
                    source_id: jira_issue.key,
                    metadata: {
                        let mut metadata = HashMap::new();
                        metadata.insert("project".to_string(), jira_issue.fields.project.key);
                        metadata.insert("issue_type".to_string(), jira_issue.fields.issue_type.name);
                        metadata
                    },
                };
                Ok(Some(unified_task))
            }
            Err(_) => Ok(None),
        }
    }

    async fn list_tasks(&self, filter: Option<&str>) -> Result<Vec<UnifiedTask>> {
        let jql = filter.unwrap_or("order by created DESC");
        let search_results = self.adapter.search_issues(jql, 0, 50).await?;
        
        let mut unified_tasks = Vec::new();
        for issue in search_results.issues {
            let unified_task = UnifiedTask {
                id: issue.key.clone(),
                title: issue.fields.summary,
                description: issue.fields.description,
                status: issue.fields.status.name,
                priority: issue.fields.priority.map(|p| p.name),
                assignee: issue.fields.assignee.map(|a| a.display_name),
                created_at: DateTime::parse_from_rfc3339(&issue.fields.created)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: DateTime::parse_from_rfc3339(&issue.fields.updated)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                due_date: None, // JiraIssue doesn't have due_date field
                source: "jira".to_string(),
                source_id: issue.key.clone(),
                metadata: {
                    let mut metadata = HashMap::new();
                    metadata.insert("project".to_string(), issue.fields.project.key);
                    metadata.insert("issue_type".to_string(), issue.fields.issue_type.name);
                    metadata
                },
            };
            unified_tasks.push(unified_task);
        }
        
        Ok(unified_tasks)
    }

    async fn delete_task(&self, id: &str) -> Result<()> {
        self.adapter.delete_issue(id).await
            .context("Failed to delete task in Jira")
    }
}

/// Calendar service adapter
pub struct CalendarServiceAdapter {
    adapter: Arc<CalendarAdapter>,
    authenticated: bool,
}

impl CalendarServiceAdapter {
    pub fn new(adapter: Arc<CalendarAdapter>) -> Self {
        Self {
            adapter,
            authenticated: false,
        }
    }
}

#[async_trait]
impl ServiceAdapter for CalendarServiceAdapter {
    async fn health_check(&self) -> Result<bool> {
        // Simple health check by trying to list calendars
        match self.adapter.get_calendar_list().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    async fn authenticate(&self, credentials: &ServiceCredentials) -> Result<()> {
        // Calendar authentication is typically handled by the adapter
        Ok(())
    }

    fn get_service_type(&self) -> &str {
        "calendar"
    }

    fn is_authenticated(&self) -> bool {
        self.authenticated
    }
}

#[async_trait]
impl CalendarService for CalendarServiceAdapter {
    async fn create_event(&self, event: &UnifiedEvent) -> Result<String> {
        let calendar_id = event.metadata.get("calendar_id")
            .ok_or_else(|| anyhow::anyhow!("Calendar ID not specified"))?;
        
        let calendar_event = CalendarEvent {
            id: String::new(),
            title: event.title.clone(),
            description: event.description.clone(),
            start_time: event.start_time,
            end_time: event.end_time,
            location: event.location.clone(),
            attendees: event.attendees.clone(),
            all_day: false,
            recurring: false,
            calendar_id: calendar_id.clone(),
        };
        
        let created_event = self.adapter.create_event(calendar_id, &calendar_event).await?;
        let event_id = created_event.id;
        
        Ok(event_id)
    }

    async fn update_event(&self, id: &str, event: &UnifiedEvent) -> Result<()> {
        let calendar_id = event.metadata.get("calendar_id")
            .ok_or_else(|| anyhow::anyhow!("Calendar ID not specified"))?;
        
        let calendar_event = CalendarEvent {
            id: id.to_string(),
            title: event.title.clone(),
            description: event.description.clone(),
            start_time: event.start_time,
            end_time: event.end_time,
            location: event.location.clone(),
            attendees: event.attendees.clone(),
            all_day: false,
            recurring: false,
            calendar_id: calendar_id.clone(),
        };
        
        self.adapter.update_event(calendar_id, id, &calendar_event).await
            .context("Failed to update event in calendar")?;
        
        Ok(())
    }

    async fn get_event(&self, id: &str) -> Result<Option<UnifiedEvent>> {
        // For this example, we'll need to search through calendars
        // In a real implementation, you'd store calendar_id with the event_id
        let calendars = self.adapter.get_calendar_list().await?;
        
        for calendar in calendars {
            if let Ok(cal_event) = self.adapter.get_event(&calendar.id, id).await {
                let unified_event = UnifiedEvent {
                    id: id.to_string(),
                    title: cal_event.title,
                    description: cal_event.description,
                    start_time: cal_event.start_time,
                    end_time: cal_event.end_time,
                    attendees: cal_event.attendees,
                    location: cal_event.location,
                    source: "calendar".to_string(),
                    source_id: cal_event.id,
                    metadata: {
                        let mut metadata = HashMap::new();
                        metadata.insert("calendar_id".to_string(), calendar.id);
                        metadata
                    },
                };
                return Ok(Some(unified_event));
            }
        }
        
        Ok(None)
    }

    async fn list_events(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<UnifiedEvent>> {
        let calendars = self.adapter.get_calendar_list().await?;
        let mut unified_events = Vec::new();
        
        for calendar in calendars {
            let events = self.adapter.list_events(&calendar.id, Some(start), Some(end)).await?;
            
            for event in events {
                let unified_event = UnifiedEvent {
                    id: event.id.clone(),
                    title: event.title,
                    description: event.description,
                    start_time: event.start_time,
                    end_time: event.end_time,
                    attendees: event.attendees,
                    location: event.location,
                    source: "calendar".to_string(),
                    source_id: event.id,
                    metadata: {
                        let mut metadata = HashMap::new();
                        metadata.insert("calendar_id".to_string(), calendar.id.clone());
                        metadata
                    },
                };
                unified_events.push(unified_event);
            }
        }
        
        Ok(unified_events)
    }

    async fn delete_event(&self, id: &str) -> Result<()> {
        // For this example, we'll need to search through calendars
        let calendars = self.adapter.get_calendar_list().await?;
        
        for calendar in calendars {
            if let Ok(_) = self.adapter.get_event(&calendar.id, id).await {
                return self.adapter.delete_event(&calendar.id, id).await
                    .context("Failed to delete event from calendar");
            }
        }
        
        Err(anyhow::anyhow!("Event not found"))
    }
}

/// Workflow action executor that handles cross-tool integrations
pub struct CrossToolActionExecutor {
    obsidian_service: Arc<ObsidianService>,
    jira_service: Arc<JiraService>,
    calendar_service: Arc<CalendarServiceAdapter>,
}

impl CrossToolActionExecutor {
    pub fn new(
        obsidian_adapter: Arc<ObsidianAdapter>,
        jira_adapter: Arc<JiraAdapter>,
        calendar_adapter: Arc<CalendarAdapter>,
    ) -> Self {
        Self {
            obsidian_service: Arc::new(ObsidianService::new(obsidian_adapter)),
            jira_service: Arc::new(JiraService::new(jira_adapter)),
            calendar_service: Arc::new(CalendarServiceAdapter::new(calendar_adapter)),
        }
    }
}

#[async_trait]
impl WorkflowActionExecutor for CrossToolActionExecutor {
    async fn execute_action(&self, action: &WorkflowAction, context: &WorkflowContext) -> Result<serde_json::Value> {
        match action {
            WorkflowAction::CreateObsidianNote { vault, path, content, template } => {
                let note = UnifiedNote {
                    id: format!("{}:{}", vault, path),
                    title: path.replace(".md", "").replace("_", " "),
                    content: content.clone(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    tags: Vec::new(),
                    source: "obsidian".to_string(),
                    source_id: format!("{}:{}", vault, path),
                    metadata: {
                        let mut metadata = HashMap::new();
                        metadata.insert("vault".to_string(), vault.clone());
                        metadata.insert("path".to_string(), path.clone());
                        if let Some(tmpl) = template {
                            metadata.insert("template".to_string(), tmpl.clone());
                        }
                        metadata
                    },
                };
                
                let note_id = self.obsidian_service.create_note(&note).await?;
                Ok(serde_json::json!({"note_id": note_id}))
            }
            
            WorkflowAction::CreateJiraIssue { project, issue_type, summary, description, assignee } => {
                let task = UnifiedTask {
                    id: String::new(), // Will be set by Jira
                    title: summary.clone(),
                    description: Some(description.clone()),
                    status: "To Do".to_string(),
                    priority: None,
                    assignee: assignee.clone(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    due_date: None,
                    source: "jira".to_string(),
                    source_id: String::new(),
                    metadata: {
                        let mut metadata = HashMap::new();
                        metadata.insert("project".to_string(), project.clone());
                        metadata.insert("issue_type".to_string(), issue_type.clone());
                        metadata
                    },
                };
                
                let task_id = self.jira_service.create_task(&task).await?;
                Ok(serde_json::json!({"task_id": task_id}))
            }
            
            WorkflowAction::CreateCalendarEvent { calendar_id, title, description, start_time, end_time, attendees } => {
                let event = UnifiedEvent {
                    id: String::new(), // Will be set by calendar service
                    title: title.clone(),
                    description: description.clone(),
                    start_time: *start_time,
                    end_time: *end_time,
                    attendees: attendees.clone(),
                    location: None,
                    source: "calendar".to_string(),
                    source_id: String::new(),
                    metadata: {
                        let mut metadata = HashMap::new();
                        metadata.insert("calendar_id".to_string(), calendar_id.clone());
                        metadata
                    },
                };
                
                let event_id = self.calendar_service.create_event(&event).await?;
                Ok(serde_json::json!({"event_id": event_id}))
            }
            
            _ => Err(anyhow::anyhow!("Action not supported by CrossToolActionExecutor")),
        }
    }

    fn supports_action(&self, action: &WorkflowAction) -> bool {
        matches!(action, 
            WorkflowAction::CreateObsidianNote { .. } |
            WorkflowAction::UpdateObsidianNote { .. } |
            WorkflowAction::CreateJiraIssue { .. } |
            WorkflowAction::UpdateJiraIssue { .. } |
            WorkflowAction::CreateCalendarEvent { .. }
        )
    }
}

/// Service registry for managing all available services
pub struct ServiceRegistry {
    services: Arc<RwLock<HashMap<String, Arc<dyn ServiceAdapter>>>>,
    note_services: Arc<RwLock<HashMap<String, Arc<dyn NoteService>>>>,
    task_services: Arc<RwLock<HashMap<String, Arc<dyn TaskService>>>>,
    calendar_services: Arc<RwLock<HashMap<String, Arc<dyn CalendarService>>>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            note_services: Arc::new(RwLock::new(HashMap::new())),
            task_services: Arc::new(RwLock::new(HashMap::new())),
            calendar_services: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_note_service(&self, service: Arc<dyn NoteService>) {
        let service_type = service.get_service_type().to_string();
        self.note_services.write().await.insert(service_type.clone(), service.clone());
        self.services.write().await.insert(service_type, service);
    }

    pub async fn register_task_service(&self, service: Arc<dyn TaskService>) {
        let service_type = service.get_service_type().to_string();
        self.task_services.write().await.insert(service_type.clone(), service.clone());
        self.services.write().await.insert(service_type, service);
    }

    pub async fn register_calendar_service(&self, service: Arc<dyn CalendarService>) {
        let service_type = service.get_service_type().to_string();
        self.calendar_services.write().await.insert(service_type.clone(), service.clone());
        self.services.write().await.insert(service_type, service);
    }

    pub async fn get_note_service(&self, service_type: &str) -> Option<Arc<dyn NoteService>> {
        self.note_services.read().await.get(service_type).cloned()
    }

    pub async fn get_task_service(&self, service_type: &str) -> Option<Arc<dyn TaskService>> {
        self.task_services.read().await.get(service_type).cloned()
    }

    pub async fn get_calendar_service(&self, service_type: &str) -> Option<Arc<dyn CalendarService>> {
        self.calendar_services.read().await.get(service_type).cloned()
    }

    pub async fn health_check_all(&self) -> Result<HashMap<String, bool>> {
        let services = self.services.read().await;
        let mut results = HashMap::new();
        
        for (service_type, service) in services.iter() {
            let health = service.health_check().await.unwrap_or(false);
            results.insert(service_type.clone(), health);
        }
        
        Ok(results)
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}