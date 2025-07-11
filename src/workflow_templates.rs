use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::workflow_engine::{WorkflowAction, WorkflowCondition, WorkflowDefinition, WorkflowRule, WorkflowTrigger};

/// Represents a workflow template parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateParameter {
    pub name: String,
    pub parameter_type: String, // string, number, boolean, date, choice
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
    pub description: Option<String>,
    pub choices: Option<Vec<String>>, // For choice type
    pub validation_regex: Option<String>,
}

/// Represents a workflow template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub version: String,
    pub parameters: Vec<TemplateParameter>,
    pub template_definition: WorkflowDefinition,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub author: Option<String>,
    pub usage_count: u64,
}

/// Represents the result of template instantiation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInstantiationResult {
    pub workflow_id: String,
    pub template_id: String,
    pub parameters_used: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Template validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Workflow template engine
pub struct WorkflowTemplateEngine {
    templates: Arc<RwLock<HashMap<String, WorkflowTemplate>>>,
}

impl WorkflowTemplateEngine {
    pub fn new() -> Self {
        Self {
            templates: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize the template engine with built-in templates
    pub async fn initialize_with_builtin_templates(&self) -> Result<()> {
        // Meeting to Note to Task template
        let meeting_template = self.create_meeting_to_note_to_task_template();
        self.register_template(meeting_template).await?;

        // Project Retrospective template
        let retrospective_template = self.create_project_retrospective_template();
        self.register_template(retrospective_template).await?;

        // Goal Synchronization template
        let goal_sync_template = self.create_goal_synchronization_template();
        self.register_template(goal_sync_template).await?;

        // Deadline Management template
        let deadline_template = self.create_deadline_management_template();
        self.register_template(deadline_template).await?;

        // Daily Standup template
        let standup_template = self.create_daily_standup_template();
        self.register_template(standup_template).await?;

        Ok(())
    }

    /// Register a new workflow template
    pub async fn register_template(&self, template: WorkflowTemplate) -> Result<()> {
        let validation_result = self.validate_template(&template)?;
        if !validation_result.valid {
            return Err(anyhow::anyhow!("Template validation failed: {:?}", validation_result.errors));
        }

        self.templates.write().await.insert(template.id.clone(), template);
        Ok(())
    }

    /// Get all available templates
    pub async fn get_templates(&self) -> Result<Vec<WorkflowTemplate>> {
        Ok(self.templates.read().await.values().cloned().collect())
    }

    /// Get templates by category
    pub async fn get_templates_by_category(&self, category: &str) -> Result<Vec<WorkflowTemplate>> {
        let templates = self.templates.read().await;
        Ok(templates.values()
            .filter(|t| t.category == category)
            .cloned()
            .collect())
    }

    /// Get a specific template by ID
    pub async fn get_template(&self, template_id: &str) -> Result<Option<WorkflowTemplate>> {
        Ok(self.templates.read().await.get(template_id).cloned())
    }

    /// Search templates by name or description
    pub async fn search_templates(&self, query: &str) -> Result<Vec<WorkflowTemplate>> {
        let templates = self.templates.read().await;
        let query_lower = query.to_lowercase();
        
        Ok(templates.values()
            .filter(|t| {
                t.name.to_lowercase().contains(&query_lower) ||
                t.description.to_lowercase().contains(&query_lower) ||
                t.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect())
    }

    /// Instantiate a template with parameters
    pub async fn instantiate_template(
        &self,
        template_id: &str,
        parameters: HashMap<String, serde_json::Value>,
    ) -> Result<TemplateInstantiationResult> {
        let template = self.get_template(template_id).await?
            .ok_or_else(|| anyhow::anyhow!("Template not found: {}", template_id))?;

        // Validate parameters
        self.validate_parameters(&template, &parameters)?;

        // Apply parameters to template
        let workflow_definition = self.apply_parameters_to_template(&template, &parameters)?;

        // Generate unique workflow ID
        let workflow_id = uuid::Uuid::new_v4().to_string();

        // Update usage count
        if let Some(tmpl) = self.templates.write().await.get_mut(template_id) {
            tmpl.usage_count += 1;
        }

        Ok(TemplateInstantiationResult {
            workflow_id,
            template_id: template_id.to_string(),
            parameters_used: parameters,
            created_at: Utc::now(),
        })
    }

    /// Validate a template
    pub fn validate_template(&self, template: &WorkflowTemplate) -> Result<TemplateValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check required fields
        if template.name.is_empty() {
            errors.push("Template name is required".to_string());
        }

        if template.description.is_empty() {
            errors.push("Template description is required".to_string());
        }

        // Validate parameters
        for param in &template.parameters {
            if param.name.is_empty() {
                errors.push("Parameter name is required".to_string());
            }

            if param.parameter_type.is_empty() {
                errors.push("Parameter type is required".to_string());
            }

            if param.required && param.default_value.is_some() {
                warnings.push(format!("Parameter '{}' is required but has a default value", param.name));
            }
        }

        // Validate template definition
        if template.template_definition.rules.is_empty() {
            warnings.push("Template has no rules defined".to_string());
        }

        Ok(TemplateValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    /// Validate parameters against template requirements
    fn validate_parameters(
        &self,
        template: &WorkflowTemplate,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        for param in &template.parameters {
            if param.required && !parameters.contains_key(&param.name) {
                return Err(anyhow::anyhow!("Required parameter '{}' is missing", param.name));
            }

            if let Some(value) = parameters.get(&param.name) {
                // Type validation
                match param.parameter_type.as_str() {
                    "string" => {
                        if !value.is_string() {
                            return Err(anyhow::anyhow!("Parameter '{}' must be a string", param.name));
                        }
                    }
                    "number" => {
                        if !value.is_number() {
                            return Err(anyhow::anyhow!("Parameter '{}' must be a number", param.name));
                        }
                    }
                    "boolean" => {
                        if !value.is_boolean() {
                            return Err(anyhow::anyhow!("Parameter '{}' must be a boolean", param.name));
                        }
                    }
                    "choice" => {
                        if let Some(choices) = &param.choices {
                            if let Some(str_value) = value.as_str() {
                                if !choices.contains(&str_value.to_string()) {
                                    return Err(anyhow::anyhow!(
                                        "Parameter '{}' must be one of: {:?}",
                                        param.name, choices
                                    ));
                                }
                            }
                        }
                    }
                    _ => {}
                }

                // Regex validation
                if let Some(regex) = &param.validation_regex {
                    if let Some(str_value) = value.as_str() {
                        // Note: For production, use a proper regex library
                        if !str_value.contains(regex) {
                            return Err(anyhow::anyhow!(
                                "Parameter '{}' does not match validation pattern",
                                param.name
                            ));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Apply parameters to template to create workflow definition
    fn apply_parameters_to_template(
        &self,
        template: &WorkflowTemplate,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<WorkflowDefinition> {
        let workflow_definition = template.template_definition.clone();
        
        // Replace placeholders in workflow definition
        let serialized = serde_json::to_string(&workflow_definition)?;
        let mut processed = serialized;

        // Replace template parameters
        for (param_name, param_value) in parameters {
            let placeholder = format!("{{{{{}}}}}", param_name);
            let replacement = match param_value {
                serde_json::Value::String(s) => s.clone(),
                _ => param_value.to_string(),
            };
            processed = processed.replace(&placeholder, &replacement);
        }

        // Replace with default values for missing parameters
        for param in &template.parameters {
            if !parameters.contains_key(&param.name) {
                if let Some(default_value) = &param.default_value {
                    let placeholder = format!("{{{{{}}}}}", param.name);
                    let replacement = match default_value {
                        serde_json::Value::String(s) => s.clone(),
                        _ => default_value.to_string(),
                    };
                    processed = processed.replace(&placeholder, &replacement);
                }
            }
        }

        // Deserialize back to workflow definition
        let mut final_workflow: WorkflowDefinition = serde_json::from_str(&processed)?;
        final_workflow.id = uuid::Uuid::new_v4().to_string();
        final_workflow.created_at = Utc::now();
        final_workflow.updated_at = Utc::now();

        Ok(final_workflow)
    }

    /// Create meeting to note to task template
    fn create_meeting_to_note_to_task_template(&self) -> WorkflowTemplate {
        WorkflowTemplate {
            id: "meeting-to-note-to-task".to_string(),
            name: "Meeting to Note to Task".to_string(),
            description: "Automatically create notes from calendar meetings and extract action items as Jira tasks".to_string(),
            category: "productivity".to_string(),
            version: "1.0".to_string(),
            parameters: vec![
                TemplateParameter {
                    name: "vault_name".to_string(),
                    parameter_type: "string".to_string(),
                    required: true,
                    default_value: Some(serde_json::Value::String("Meeting Notes".to_string())),
                    description: Some("Obsidian vault for storing meeting notes".to_string()),
                    choices: None,
                    validation_regex: None,
                },
                TemplateParameter {
                    name: "jira_project".to_string(),
                    parameter_type: "string".to_string(),
                    required: true,
                    default_value: None,
                    description: Some("Jira project key for creating tasks".to_string()),
                    choices: None,
                    validation_regex: Some("^[A-Z]+$".to_string()),
                },
                TemplateParameter {
                    name: "assignee".to_string(),
                    parameter_type: "string".to_string(),
                    required: false,
                    default_value: None,
                    description: Some("Default assignee for created tasks".to_string()),
                    choices: None,
                    validation_regex: None,
                },
            ],
            template_definition: WorkflowDefinition {
                id: "template".to_string(),
                name: "Meeting to Note to Task".to_string(),
                description: Some("Template workflow".to_string()),
                version: "1.0".to_string(),
                trigger: WorkflowTrigger::CalendarEvent {
                    event_type: "meeting_ended".to_string(),
                    pattern: "*".to_string(),
                },
                rules: vec![
                    WorkflowRule {
                        id: "create_note".to_string(),
                        name: "Create Meeting Note".to_string(),
                        conditions: vec![],
                        actions: vec![
                            WorkflowAction::CreateObsidianNote {
                                vault: "{{vault_name}}".to_string(),
                                path: "{{meeting_title}}_{{date}}.md".to_string(),
                                content: "# {{meeting_title}}\n\n**Date:** {{date}}\n**Duration:** {{duration}}\n**Attendees:** {{attendees}}\n\n## Notes\n\n{{meeting_notes}}\n\n## Action Items\n\n{{action_items}}".to_string(),
                                template: Some("meeting_note".to_string()),
                            },
                        ],
                        enabled: true,
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    },
                    WorkflowRule {
                        id: "extract_tasks".to_string(),
                        name: "Extract Action Items".to_string(),
                        conditions: vec![
                            WorkflowCondition {
                                field: "action_items".to_string(),
                                operator: "ne".to_string(),
                                value: serde_json::Value::String("".to_string()),
                            },
                        ],
                        actions: vec![
                            WorkflowAction::CreateJiraIssue {
                                project: "{{jira_project}}".to_string(),
                                issue_type: "Task".to_string(),
                                summary: "{{action_item_title}}".to_string(),
                                description: "Action item from meeting: {{meeting_title}}\n\n{{action_item_description}}".to_string(),
                                assignee: Some("{{assignee}}".to_string()),
                            },
                        ],
                        enabled: true,
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    },
                ],
                enabled: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                tags: vec!["meeting".to_string(), "productivity".to_string()],
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: vec!["meeting".to_string(), "productivity".to_string(), "automation".to_string()],
            author: Some("Arrowhead System".to_string()),
            usage_count: 0,
        }
    }

    /// Create project retrospective template
    fn create_project_retrospective_template(&self) -> WorkflowTemplate {
        WorkflowTemplate {
            id: "project-retrospective".to_string(),
            name: "Project Retrospective".to_string(),
            description: "Automatically create retrospective notes and action items at project milestones".to_string(),
            category: "project-management".to_string(),
            version: "1.0".to_string(),
            parameters: vec![
                TemplateParameter {
                    name: "project_name".to_string(),
                    parameter_type: "string".to_string(),
                    required: true,
                    default_value: None,
                    description: Some("Name of the project".to_string()),
                    choices: None,
                    validation_regex: None,
                },
                TemplateParameter {
                    name: "vault_name".to_string(),
                    parameter_type: "string".to_string(),
                    required: true,
                    default_value: Some(serde_json::Value::String("Project Notes".to_string())),
                    description: Some("Obsidian vault for storing retrospective notes".to_string()),
                    choices: None,
                    validation_regex: None,
                },
                TemplateParameter {
                    name: "jira_project".to_string(),
                    parameter_type: "string".to_string(),
                    required: true,
                    default_value: None,
                    description: Some("Jira project key".to_string()),
                    choices: None,
                    validation_regex: Some("^[A-Z]+$".to_string()),
                },
            ],
            template_definition: WorkflowDefinition {
                id: "template".to_string(),
                name: "Project Retrospective".to_string(),
                description: Some("Template workflow".to_string()),
                version: "1.0".to_string(),
                trigger: WorkflowTrigger::JiraIssue {
                    project: "{{jira_project}}".to_string(),
                    issue_type: "Epic".to_string(),
                    status_change: Some("Done".to_string()),
                },
                rules: vec![
                    WorkflowRule {
                        id: "create_retrospective".to_string(),
                        name: "Create Retrospective Note".to_string(),
                        conditions: vec![],
                        actions: vec![
                            WorkflowAction::CreateObsidianNote {
                                vault: "{{vault_name}}".to_string(),
                                path: "{{project_name}}_Retrospective_{{date}}.md".to_string(),
                                content: "# {{project_name}} Retrospective\n\n**Date:** {{date}}\n**Epic:** {{epic_key}}\n\n## What Went Well\n\n- \n\n## What Could Be Improved\n\n- \n\n## Action Items\n\n- \n\n## Metrics\n\n- **Duration:** {{duration}}\n- **Story Points:** {{story_points}}\n- **Team Size:** {{team_size}}".to_string(),
                                template: Some("retrospective".to_string()),
                            },
                        ],
                        enabled: true,
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    },
                ],
                enabled: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                tags: vec!["retrospective".to_string(), "project-management".to_string()],
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: vec!["retrospective".to_string(), "project-management".to_string(), "automation".to_string()],
            author: Some("Arrowhead System".to_string()),
            usage_count: 0,
        }
    }

    /// Create goal synchronization template
    fn create_goal_synchronization_template(&self) -> WorkflowTemplate {
        WorkflowTemplate {
            id: "goal-synchronization".to_string(),
            name: "Goal Synchronization".to_string(),
            description: "Automatically sync goals between Obsidian and Jira, creating tracking issues".to_string(),
            category: "goal-management".to_string(),
            version: "1.0".to_string(),
            parameters: vec![
                TemplateParameter {
                    name: "goals_vault".to_string(),
                    parameter_type: "string".to_string(),
                    required: true,
                    default_value: Some(serde_json::Value::String("Goals".to_string())),
                    description: Some("Obsidian vault for goal tracking".to_string()),
                    choices: None,
                    validation_regex: None,
                },
                TemplateParameter {
                    name: "jira_project".to_string(),
                    parameter_type: "string".to_string(),
                    required: true,
                    default_value: None,
                    description: Some("Jira project for goal tracking".to_string()),
                    choices: None,
                    validation_regex: Some("^[A-Z]+$".to_string()),
                },
                TemplateParameter {
                    name: "sync_frequency".to_string(),
                    parameter_type: "choice".to_string(),
                    required: false,
                    default_value: Some(serde_json::Value::String("weekly".to_string())),
                    description: Some("How often to sync goals".to_string()),
                    choices: Some(vec!["daily".to_string(), "weekly".to_string(), "monthly".to_string()]),
                    validation_regex: None,
                },
            ],
            template_definition: WorkflowDefinition {
                id: "template".to_string(),
                name: "Goal Synchronization".to_string(),
                description: Some("Template workflow".to_string()),
                version: "1.0".to_string(),
                trigger: WorkflowTrigger::Scheduled {
                    cron_expression: "0 9 * * 1".to_string(), // Weekly on Monday at 9 AM
                },
                rules: vec![
                    WorkflowRule {
                        id: "sync_goals".to_string(),
                        name: "Sync Goals to Jira".to_string(),
                        conditions: vec![],
                        actions: vec![
                            WorkflowAction::CreateJiraIssue {
                                project: "{{jira_project}}".to_string(),
                                issue_type: "Epic".to_string(),
                                summary: "{{goal_title}}".to_string(),
                                description: "Goal tracking epic\n\n{{goal_description}}\n\n**Success Criteria:**\n{{success_criteria}}".to_string(),
                                assignee: Some("{{goal_owner}}".to_string()),
                            },
                        ],
                        enabled: true,
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    },
                ],
                enabled: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                tags: vec!["goals".to_string(), "synchronization".to_string()],
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: vec!["goals".to_string(), "synchronization".to_string(), "automation".to_string()],
            author: Some("Arrowhead System".to_string()),
            usage_count: 0,
        }
    }

    /// Create deadline management template
    fn create_deadline_management_template(&self) -> WorkflowTemplate {
        WorkflowTemplate {
            id: "deadline-management".to_string(),
            name: "Deadline Management".to_string(),
            description: "Automatically create reminders and calendar events for approaching deadlines".to_string(),
            category: "time-management".to_string(),
            version: "1.0".to_string(),
            parameters: vec![
                TemplateParameter {
                    name: "reminder_days".to_string(),
                    parameter_type: "number".to_string(),
                    required: false,
                    default_value: Some(serde_json::Value::Number(serde_json::Number::from(7))),
                    description: Some("Days before deadline to send reminder".to_string()),
                    choices: None,
                    validation_regex: None,
                },
                TemplateParameter {
                    name: "calendar_id".to_string(),
                    parameter_type: "string".to_string(),
                    required: true,
                    default_value: None,
                    description: Some("Calendar ID for deadline reminders".to_string()),
                    choices: None,
                    validation_regex: None,
                },
            ],
            template_definition: WorkflowDefinition {
                id: "template".to_string(),
                name: "Deadline Management".to_string(),
                description: Some("Template workflow".to_string()),
                version: "1.0".to_string(),
                trigger: WorkflowTrigger::Scheduled {
                    cron_expression: "0 8 * * *".to_string(), // Daily at 8 AM
                },
                rules: vec![
                    WorkflowRule {
                        id: "check_deadlines".to_string(),
                        name: "Check Approaching Deadlines".to_string(),
                        conditions: vec![
                            WorkflowCondition {
                                field: "days_until_deadline".to_string(),
                                operator: "eq".to_string(),
                                value: serde_json::Value::Number(serde_json::Number::from(7)),
                            },
                        ],
                        actions: vec![
                            WorkflowAction::CreateCalendarEvent {
                                calendar_id: "{{calendar_id}}".to_string(),
                                title: "DEADLINE: {{task_title}}".to_string(),
                                description: Some("Deadline reminder for: {{task_title}}\n\n{{task_description}}".to_string()),
                                start_time: chrono::Utc::now(),
                                end_time: chrono::Utc::now(),
                                attendees: vec!["{{assignee}}".to_string()],
                            },
                            WorkflowAction::SendNotification {
                                channel: "email".to_string(),
                                message: "Deadline approaching for: {{task_title}}".to_string(),
                                priority: "high".to_string(),
                            },
                        ],
                        enabled: true,
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    },
                ],
                enabled: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                tags: vec!["deadlines".to_string(), "reminders".to_string()],
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: vec!["deadlines".to_string(), "reminders".to_string(), "automation".to_string()],
            author: Some("Arrowhead System".to_string()),
            usage_count: 0,
        }
    }

    /// Create daily standup template
    fn create_daily_standup_template(&self) -> WorkflowTemplate {
        WorkflowTemplate {
            id: "daily-standup".to_string(),
            name: "Daily Standup".to_string(),
            description: "Automatically prepare daily standup notes from recent activity".to_string(),
            category: "team-collaboration".to_string(),
            version: "1.0".to_string(),
            parameters: vec![
                TemplateParameter {
                    name: "team_name".to_string(),
                    parameter_type: "string".to_string(),
                    required: true,
                    default_value: None,
                    description: Some("Name of the team".to_string()),
                    choices: None,
                    validation_regex: None,
                },
                TemplateParameter {
                    name: "standup_vault".to_string(),
                    parameter_type: "string".to_string(),
                    required: true,
                    default_value: Some(serde_json::Value::String("Team Notes".to_string())),
                    description: Some("Obsidian vault for standup notes".to_string()),
                    choices: None,
                    validation_regex: None,
                },
                TemplateParameter {
                    name: "jira_project".to_string(),
                    parameter_type: "string".to_string(),
                    required: true,
                    default_value: None,
                    description: Some("Jira project to track progress".to_string()),
                    choices: None,
                    validation_regex: Some("^[A-Z]+$".to_string()),
                },
            ],
            template_definition: WorkflowDefinition {
                id: "template".to_string(),
                name: "Daily Standup".to_string(),
                description: Some("Template workflow".to_string()),
                version: "1.0".to_string(),
                trigger: WorkflowTrigger::Scheduled {
                    cron_expression: "0 9 * * 1-5".to_string(), // Weekdays at 9 AM
                },
                rules: vec![
                    WorkflowRule {
                        id: "prepare_standup".to_string(),
                        name: "Prepare Standup Notes".to_string(),
                        conditions: vec![],
                        actions: vec![
                            WorkflowAction::CreateObsidianNote {
                                vault: "{{standup_vault}}".to_string(),
                                path: "{{team_name}}_Standup_{{date}}.md".to_string(),
                                content: "# {{team_name}} Daily Standup - {{date}}\n\n## What I Did Yesterday\n\n{{yesterday_work}}\n\n## What I'm Doing Today\n\n{{today_plan}}\n\n## Blockers\n\n{{blockers}}\n\n## Sprint Progress\n\n{{sprint_progress}}".to_string(),
                                template: Some("standup".to_string()),
                            },
                        ],
                        enabled: true,
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    },
                ],
                enabled: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                tags: vec!["standup".to_string(), "team".to_string()],
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: vec!["standup".to_string(), "team".to_string(), "automation".to_string()],
            author: Some("Arrowhead System".to_string()),
            usage_count: 0,
        }
    }

    /// Export template to JSON
    pub async fn export_template(&self, template_id: &str) -> Result<String> {
        let template = self.get_template(template_id).await?
            .ok_or_else(|| anyhow::anyhow!("Template not found"))?;
        
        serde_json::to_string_pretty(&template)
            .context("Failed to serialize template")
    }

    /// Import template from JSON
    pub async fn import_template(&self, template_json: &str) -> Result<String> {
        let template: WorkflowTemplate = serde_json::from_str(template_json)
            .context("Failed to parse template JSON")?;
        
        let template_id = template.id.clone();
        self.register_template(template).await?;
        
        Ok(template_id)
    }

    /// Remove a template
    pub async fn remove_template(&self, template_id: &str) -> Result<()> {
        self.templates.write().await.remove(template_id);
        Ok(())
    }

    /// Get template usage statistics
    pub async fn get_template_stats(&self) -> Result<HashMap<String, u64>> {
        let templates = self.templates.read().await;
        let mut stats = HashMap::new();
        
        for (id, template) in templates.iter() {
            stats.insert(id.clone(), template.usage_count);
        }
        
        Ok(stats)
    }
}

impl Default for WorkflowTemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}