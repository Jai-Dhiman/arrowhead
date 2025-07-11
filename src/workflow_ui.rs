use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::workflow_engine::{WorkflowDefinition, WorkflowEngine, WorkflowExecution};
use crate::workflow_templates::{WorkflowTemplate, WorkflowTemplateEngine};

/// Represents a user's workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWorkflowConfig {
    pub user_id: String,
    pub enabled_workflows: Vec<String>,
    pub workflow_settings: HashMap<String, serde_json::Value>,
    pub notification_preferences: NotificationPreferences,
    pub ui_preferences: UiPreferences,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// User notification preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    pub email_notifications: bool,
    pub in_app_notifications: bool,
    pub slack_notifications: bool,
    pub webhook_notifications: Option<String>,
    pub notification_types: Vec<String>, // workflow_started, workflow_completed, etc.
}

/// User interface preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiPreferences {
    pub theme: String, // dark, light, auto
    pub dashboard_layout: String, // grid, list, compact
    pub default_view: String, // workflows, executions, templates
    pub items_per_page: u32,
    pub auto_refresh: bool,
    pub refresh_interval: u32, // seconds
}

/// Workflow dashboard data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDashboard {
    pub active_workflows: Vec<WorkflowSummary>,
    pub recent_executions: Vec<ExecutionSummary>,
    pub available_templates: Vec<TemplateSummary>,
    pub system_health: SystemHealthSummary,
    pub user_stats: UserStatsSummary,
}

/// Summary of a workflow for dashboard display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub trigger_type: String,
    pub last_executed: Option<DateTime<Utc>>,
    pub execution_count: u32,
    pub success_rate: f64,
    pub tags: Vec<String>,
}

/// Summary of a workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub id: String,
    pub workflow_id: String,
    pub workflow_name: String,
    pub state: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Summary of a workflow template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub usage_count: u64,
    pub rating: Option<f64>,
    pub tags: Vec<String>,
}

/// System health summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealthSummary {
    pub overall_health: String, // healthy, degraded, unhealthy
    pub service_status: HashMap<String, String>,
    pub active_executions: u32,
    pub failed_executions_24h: u32,
    pub system_uptime: u64, // seconds
}

/// User statistics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStatsSummary {
    pub total_workflows: u32,
    pub active_workflows: u32,
    pub total_executions: u32,
    pub successful_executions: u32,
    pub templates_created: u32,
    pub automations_saved_time: u64, // estimated time saved in minutes
}

/// Workflow builder configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowBuilderConfig {
    pub workflow_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub trigger_config: TriggerConfig,
    pub rules: Vec<RuleConfig>,
    pub enabled: bool,
    pub tags: Vec<String>,
}

/// Trigger configuration for workflow builder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerConfig {
    pub trigger_type: String,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Rule configuration for workflow builder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConfig {
    pub id: String,
    pub name: String,
    pub conditions: Vec<ConditionConfig>,
    pub actions: Vec<ActionConfig>,
    pub enabled: bool,
}

/// Condition configuration for workflow builder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionConfig {
    pub field: String,
    pub operator: String,
    pub value: serde_json::Value,
}

/// Action configuration for workflow builder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionConfig {
    pub action_type: String,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Workflow import/export data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExportData {
    pub workflows: Vec<WorkflowDefinition>,
    pub templates: Vec<WorkflowTemplate>,
    pub metadata: WorkflowExportMetadata,
}

/// Metadata for workflow export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExportMetadata {
    pub export_date: DateTime<Utc>,
    pub version: String,
    pub user_id: String,
    pub export_type: String, // full, workflows_only, templates_only
}

/// Workflow configuration interface
pub struct WorkflowConfigurationInterface {
    workflow_engine: Arc<WorkflowEngine>,
    template_engine: Arc<WorkflowTemplateEngine>,
    user_configs: Arc<RwLock<HashMap<String, UserWorkflowConfig>>>,
    execution_stats: Arc<RwLock<HashMap<String, Vec<ExecutionSummary>>>>,
}

impl WorkflowConfigurationInterface {
    pub fn new(
        workflow_engine: Arc<WorkflowEngine>,
        template_engine: Arc<WorkflowTemplateEngine>,
    ) -> Self {
        Self {
            workflow_engine,
            template_engine,
            user_configs: Arc::new(RwLock::new(HashMap::new())),
            execution_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize default user configuration
    pub async fn initialize_user_config(&self, user_id: &str) -> Result<()> {
        let config = UserWorkflowConfig {
            user_id: user_id.to_string(),
            enabled_workflows: Vec::new(),
            workflow_settings: HashMap::new(),
            notification_preferences: NotificationPreferences {
                email_notifications: true,
                in_app_notifications: true,
                slack_notifications: false,
                webhook_notifications: None,
                notification_types: vec![
                    "workflow_completed".to_string(),
                    "workflow_failed".to_string(),
                ],
            },
            ui_preferences: UiPreferences {
                theme: "auto".to_string(),
                dashboard_layout: "grid".to_string(),
                default_view: "workflows".to_string(),
                items_per_page: 20,
                auto_refresh: true,
                refresh_interval: 30,
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.user_configs.write().await.insert(user_id.to_string(), config);
        Ok(())
    }

    /// Get user configuration
    pub async fn get_user_config(&self, user_id: &str) -> Result<Option<UserWorkflowConfig>> {
        Ok(self.user_configs.read().await.get(user_id).cloned())
    }

    /// Update user configuration
    pub async fn update_user_config(&self, user_id: &str, config: UserWorkflowConfig) -> Result<()> {
        let mut updated_config = config;
        updated_config.updated_at = Utc::now();
        
        self.user_configs.write().await.insert(user_id.to_string(), updated_config);
        Ok(())
    }

    /// Get workflow dashboard data
    pub async fn get_dashboard(&self, user_id: &str) -> Result<WorkflowDashboard> {
        let workflows = self.workflow_engine.get_workflows().await?;
        let executions = self.workflow_engine.get_executions().await?;
        let templates = self.template_engine.get_templates().await?;

        // Convert workflows to summaries
        let mut active_workflows = Vec::new();
        for workflow in workflows {
            let execution_stats = self.calculate_workflow_stats(&workflow.id, &executions);
            
            active_workflows.push(WorkflowSummary {
                id: workflow.id.clone(),
                name: workflow.name.clone(),
                description: workflow.description.clone(),
                enabled: workflow.enabled,
                trigger_type: self.get_trigger_type_string(&workflow.trigger),
                last_executed: execution_stats.last_executed,
                execution_count: execution_stats.execution_count,
                success_rate: execution_stats.success_rate,
                tags: workflow.tags.clone(),
            });
        }

        // Convert recent executions to summaries
        let mut recent_executions = Vec::new();
        for execution in executions.into_iter().take(10) {
            let workflow_name = active_workflows.iter()
                .find(|w| w.id == execution.workflow_id)
                .map(|w| w.name.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            let duration_ms = execution.completed_at.map(|completed| {
                (completed - execution.started_at).num_milliseconds() as u64
            });

            recent_executions.push(ExecutionSummary {
                id: execution.id.clone(),
                workflow_id: execution.workflow_id.clone(),
                workflow_name,
                state: format!("{:?}", execution.state),
                started_at: execution.started_at,
                completed_at: execution.completed_at,
                duration_ms,
                success: execution.error_message.is_none(),
                error_message: execution.error_message.clone(),
            });
        }

        // Convert templates to summaries
        let available_templates = templates.into_iter().map(|template| {
            TemplateSummary {
                id: template.id.clone(),
                name: template.name.clone(),
                description: template.description.clone(),
                category: template.category.clone(),
                usage_count: template.usage_count,
                rating: None, // Could be implemented later
                tags: template.tags.clone(),
            }
        }).collect();

        // Calculate system health
        let system_health = SystemHealthSummary {
            overall_health: "healthy".to_string(), // Simplified for now
            service_status: HashMap::new(),
            active_executions: recent_executions.iter().filter(|e| e.state == "Running").count() as u32,
            failed_executions_24h: recent_executions.iter().filter(|e| !e.success).count() as u32,
            system_uptime: 86400, // 24 hours in seconds (placeholder)
        };

        // Calculate user stats
        let user_stats = UserStatsSummary {
            total_workflows: active_workflows.len() as u32,
            active_workflows: active_workflows.iter().filter(|w| w.enabled).count() as u32,
            total_executions: recent_executions.len() as u32,
            successful_executions: recent_executions.iter().filter(|e| e.success).count() as u32,
            templates_created: 0, // Would track user-created templates
            automations_saved_time: 0, // Would calculate estimated time savings
        };

        Ok(WorkflowDashboard {
            active_workflows,
            recent_executions,
            available_templates,
            system_health,
            user_stats,
        })
    }

    /// Create workflow from builder configuration
    pub async fn create_workflow_from_builder(&self, config: WorkflowBuilderConfig) -> Result<String> {
        // Convert builder config to workflow definition
        let workflow_definition = self.convert_builder_config_to_workflow(config)?;
        
        // Register the workflow
        self.workflow_engine.register_workflow(workflow_definition.clone()).await?;
        
        Ok(workflow_definition.id)
    }

    /// Get workflow builder configuration for editing
    pub async fn get_workflow_builder_config(&self, workflow_id: &str) -> Result<Option<WorkflowBuilderConfig>> {
        if let Some(workflow) = self.workflow_engine.get_workflow(workflow_id).await? {
            Ok(Some(self.convert_workflow_to_builder_config(workflow)?))
        } else {
            Ok(None)
        }
    }

    /// Export workflows and templates
    pub async fn export_workflows(
        &self,
        user_id: &str,
        export_type: &str,
        workflow_ids: Option<Vec<String>>,
    ) -> Result<WorkflowExportData> {
        let workflows = if let Some(ids) = workflow_ids {
            let mut selected_workflows = Vec::new();
            for id in ids {
                if let Some(workflow) = self.workflow_engine.get_workflow(&id).await? {
                    selected_workflows.push(workflow);
                }
            }
            selected_workflows
        } else {
            self.workflow_engine.get_workflows().await?
        };

        let templates = if export_type == "full" || export_type == "templates_only" {
            self.template_engine.get_templates().await?
        } else {
            Vec::new()
        };

        let metadata = WorkflowExportMetadata {
            export_date: Utc::now(),
            version: "1.0".to_string(),
            user_id: user_id.to_string(),
            export_type: export_type.to_string(),
        };

        Ok(WorkflowExportData {
            workflows,
            templates,
            metadata,
        })
    }

    /// Import workflows and templates
    pub async fn import_workflows(&self, import_data: WorkflowExportData) -> Result<Vec<String>> {
        let mut imported_ids = Vec::new();

        // Import workflows
        for workflow in import_data.workflows {
            self.workflow_engine.register_workflow(workflow.clone()).await?;
            imported_ids.push(workflow.id);
        }

        // Import templates
        for template in import_data.templates {
            self.template_engine.register_template(template.clone()).await?;
            imported_ids.push(template.id);
        }

        Ok(imported_ids)
    }

    /// Get workflow execution logs
    pub async fn get_execution_logs(
        &self,
        workflow_id: Option<&str>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<ExecutionSummary>> {
        let executions = self.workflow_engine.get_executions().await?;
        
        let filtered_executions: Vec<_> = executions
            .into_iter()
            .filter(|e| workflow_id.map_or(true, |id| e.workflow_id == id))
            .collect();

        let offset = offset.unwrap_or(0);
        let limit = limit.unwrap_or(100);
        
        let paginated_executions = filtered_executions
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect::<Vec<_>>();

        let mut execution_summaries = Vec::new();
        for execution in paginated_executions {
            let duration_ms = execution.completed_at.map(|completed| {
                (completed - execution.started_at).num_milliseconds() as u64
            });

            execution_summaries.push(ExecutionSummary {
                id: execution.id.clone(),
                workflow_id: execution.workflow_id.clone(),
                workflow_name: "Unknown".to_string(), // Would need to look up
                state: format!("{:?}", execution.state),
                started_at: execution.started_at,
                completed_at: execution.completed_at,
                duration_ms,
                success: execution.error_message.is_none(),
                error_message: execution.error_message.clone(),
            });
        }

        Ok(execution_summaries)
    }

    /// Validate workflow configuration
    pub async fn validate_workflow_config(&self, config: &WorkflowBuilderConfig) -> Result<Vec<String>> {
        let mut errors = Vec::new();

        // Basic validation
        if config.name.is_empty() {
            errors.push("Workflow name is required".to_string());
        }

        if config.rules.is_empty() {
            errors.push("At least one rule is required".to_string());
        }

        // Validate rules
        for rule in &config.rules {
            if rule.name.is_empty() {
                errors.push(format!("Rule '{}' must have a name", rule.id));
            }

            if rule.actions.is_empty() {
                errors.push(format!("Rule '{}' must have at least one action", rule.name));
            }

            // Validate conditions
            for condition in &rule.conditions {
                if condition.field.is_empty() {
                    errors.push(format!("Condition in rule '{}' must have a field", rule.name));
                }

                if condition.operator.is_empty() {
                    errors.push(format!("Condition in rule '{}' must have an operator", rule.name));
                }
            }

            // Validate actions
            for action in &rule.actions {
                if action.action_type.is_empty() {
                    errors.push(format!("Action in rule '{}' must have a type", rule.name));
                }
            }
        }

        Ok(errors)
    }

    /// Helper method to calculate workflow statistics
    fn calculate_workflow_stats(&self, workflow_id: &str, executions: &[WorkflowExecution]) -> WorkflowStats {
        let workflow_executions: Vec<_> = executions
            .iter()
            .filter(|e| e.workflow_id == workflow_id)
            .collect();

        let execution_count = workflow_executions.len() as u32;
        let successful_executions = workflow_executions
            .iter()
            .filter(|e| e.error_message.is_none())
            .count() as u32;

        let success_rate = if execution_count > 0 {
            successful_executions as f64 / execution_count as f64
        } else {
            0.0
        };

        let last_executed = workflow_executions
            .iter()
            .map(|e| e.started_at)
            .max();

        WorkflowStats {
            execution_count,
            success_rate,
            last_executed,
        }
    }

    /// Helper method to get trigger type string
    fn get_trigger_type_string(&self, trigger: &crate::workflow_engine::WorkflowTrigger) -> String {
        match trigger {
            crate::workflow_engine::WorkflowTrigger::Manual => "manual".to_string(),
            crate::workflow_engine::WorkflowTrigger::CalendarEvent { .. } => "calendar".to_string(),
            crate::workflow_engine::WorkflowTrigger::ObsidianNote { .. } => "obsidian".to_string(),
            crate::workflow_engine::WorkflowTrigger::JiraIssue { .. } => "jira".to_string(),
            crate::workflow_engine::WorkflowTrigger::Scheduled { .. } => "scheduled".to_string(),
            crate::workflow_engine::WorkflowTrigger::Webhook { .. } => "webhook".to_string(),
        }
    }

    /// Convert builder config to workflow definition
    fn convert_builder_config_to_workflow(&self, config: WorkflowBuilderConfig) -> Result<WorkflowDefinition> {
        // This is a simplified conversion - in practice, you'd need more sophisticated logic
        let workflow_id = config.workflow_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        
        // Convert trigger config
        let trigger = match config.trigger_config.trigger_type.as_str() {
            "manual" => crate::workflow_engine::WorkflowTrigger::Manual,
            "calendar" => crate::workflow_engine::WorkflowTrigger::CalendarEvent {
                event_type: config.trigger_config.parameters
                    .get("event_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("meeting")
                    .to_string(),
                pattern: config.trigger_config.parameters
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .unwrap_or("*")
                    .to_string(),
            },
            _ => crate::workflow_engine::WorkflowTrigger::Manual,
        };

        // Convert rules (simplified)
        let rules = config.rules.into_iter().map(|rule_config| {
            crate::workflow_engine::WorkflowRule {
                id: rule_config.id,
                name: rule_config.name,
                conditions: rule_config.conditions.into_iter().map(|cond| {
                    crate::workflow_engine::WorkflowCondition {
                        field: cond.field,
                        operator: cond.operator,
                        value: cond.value,
                    }
                }).collect(),
                actions: Vec::new(), // Would need to convert action configs
                enabled: rule_config.enabled,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
        }).collect();

        Ok(WorkflowDefinition {
            id: workflow_id,
            name: config.name,
            description: config.description,
            version: "1.0".to_string(),
            trigger,
            rules,
            enabled: config.enabled,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: config.tags,
        })
    }

    /// Convert workflow definition to builder config
    fn convert_workflow_to_builder_config(&self, workflow: WorkflowDefinition) -> Result<WorkflowBuilderConfig> {
        // This is a simplified conversion - in practice, you'd need more sophisticated logic
        let trigger_config = TriggerConfig {
            trigger_type: self.get_trigger_type_string(&workflow.trigger),
            parameters: HashMap::new(), // Would need to extract trigger parameters
        };

        let rules = workflow.rules.into_iter().map(|rule| {
            RuleConfig {
                id: rule.id,
                name: rule.name,
                conditions: rule.conditions.into_iter().map(|cond| {
                    ConditionConfig {
                        field: cond.field,
                        operator: cond.operator,
                        value: cond.value,
                    }
                }).collect(),
                actions: Vec::new(), // Would need to convert workflow actions
                enabled: rule.enabled,
            }
        }).collect();

        Ok(WorkflowBuilderConfig {
            workflow_id: Some(workflow.id),
            name: workflow.name,
            description: workflow.description,
            trigger_config,
            rules,
            enabled: workflow.enabled,
            tags: workflow.tags,
        })
    }
}

/// Helper struct for workflow statistics
struct WorkflowStats {
    execution_count: u32,
    success_rate: f64,
    last_executed: Option<DateTime<Utc>>,
}

/// Web API handlers for workflow configuration
impl WorkflowConfigurationInterface {
    /// Handle dashboard request
    pub async fn handle_dashboard_request(&self, user_id: &str) -> Result<String> {
        let dashboard = self.get_dashboard(user_id).await?;
        serde_json::to_string_pretty(&dashboard)
            .context("Failed to serialize dashboard")
    }

    /// Handle workflow creation request
    pub async fn handle_create_workflow_request(&self, config_json: &str) -> Result<String> {
        let config: WorkflowBuilderConfig = serde_json::from_str(config_json)
            .context("Failed to parse workflow configuration")?;
        
        let validation_errors = self.validate_workflow_config(&config).await?;
        if !validation_errors.is_empty() {
            return Err(anyhow::anyhow!("Validation errors: {:?}", validation_errors));
        }

        let workflow_id = self.create_workflow_from_builder(config).await?;
        Ok(workflow_id)
    }

    /// Handle workflow export request
    pub async fn handle_export_request(
        &self,
        user_id: &str,
        export_type: &str,
        workflow_ids: Option<Vec<String>>,
    ) -> Result<String> {
        let export_data = self.export_workflows(user_id, export_type, workflow_ids).await?;
        serde_json::to_string_pretty(&export_data)
            .context("Failed to serialize export data")
    }

    /// Handle workflow import request
    pub async fn handle_import_request(&self, import_json: &str) -> Result<Vec<String>> {
        let import_data: WorkflowExportData = serde_json::from_str(import_json)
            .context("Failed to parse import data")?;
        
        self.import_workflows(import_data).await
    }
}

impl Default for WorkflowConfigurationInterface {
    fn default() -> Self {
        Self::new(
            Arc::new(WorkflowEngine::new()),
            Arc::new(WorkflowTemplateEngine::new()),
        )
    }
}