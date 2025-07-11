use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

/// Represents the state of a workflow execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkflowState {
    Pending,
    Running,
    Completed,
    Failed,
    Paused,
    Cancelled,
}

/// Represents different types of workflow triggers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowTrigger {
    Manual,
    CalendarEvent {
        event_type: String,
        pattern: String,
    },
    ObsidianNote {
        vault: String,
        path_pattern: String,
        operation: String, // created, updated, deleted
    },
    JiraIssue {
        project: String,
        issue_type: String,
        status_change: Option<String>,
    },
    Scheduled {
        cron_expression: String,
    },
    Webhook {
        url: String,
        secret: Option<String>,
    },
}

/// Represents a workflow action that can be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowAction {
    CreateObsidianNote {
        vault: String,
        path: String,
        content: String,
        template: Option<String>,
    },
    UpdateObsidianNote {
        vault: String,
        path: String,
        content: String,
        append: bool,
    },
    CreateJiraIssue {
        project: String,
        issue_type: String,
        summary: String,
        description: String,
        assignee: Option<String>,
    },
    UpdateJiraIssue {
        issue_key: String,
        fields: HashMap<String, serde_json::Value>,
    },
    CreateCalendarEvent {
        calendar_id: String,
        title: String,
        description: Option<String>,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        attendees: Vec<String>,
    },
    SendNotification {
        channel: String,
        message: String,
        priority: String,
    },
    ExecuteShellCommand {
        command: String,
        args: Vec<String>,
        working_directory: Option<String>,
    },
    CallWebhook {
        url: String,
        method: String,
        headers: HashMap<String, String>,
        body: Option<String>,
    },
}

/// Represents a workflow rule condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCondition {
    pub field: String,
    pub operator: String, // eq, ne, gt, lt, contains, regex
    pub value: serde_json::Value,
}

/// Represents a workflow rule that determines when actions should be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRule {
    pub id: String,
    pub name: String,
    pub conditions: Vec<WorkflowCondition>,
    pub actions: Vec<WorkflowAction>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Represents a complete workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub trigger: WorkflowTrigger,
    pub rules: Vec<WorkflowRule>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
}

/// Represents the context data available during workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowContext {
    pub workflow_id: String,
    pub execution_id: String,
    pub trigger_data: serde_json::Value,
    pub variables: HashMap<String, serde_json::Value>,
    pub metadata: HashMap<String, String>,
}

/// Represents the result of a workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    pub id: String,
    pub workflow_id: String,
    pub state: WorkflowState,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub context: WorkflowContext,
    pub executed_actions: Vec<String>,
    pub failed_actions: Vec<String>,
}

/// Trait for workflow event handlers
#[async_trait]
pub trait WorkflowEventHandler: Send + Sync {
    async fn handle_workflow_started(&self, execution: &WorkflowExecution) -> Result<()>;
    async fn handle_workflow_completed(&self, execution: &WorkflowExecution) -> Result<()>;
    async fn handle_workflow_failed(&self, execution: &WorkflowExecution, error: &str) -> Result<()>;
    async fn handle_action_executed(&self, execution: &WorkflowExecution, action: &WorkflowAction) -> Result<()>;
    async fn handle_action_failed(&self, execution: &WorkflowExecution, action: &WorkflowAction, error: &str) -> Result<()>;
}

/// Trait for workflow action executors
#[async_trait]
pub trait WorkflowActionExecutor: Send + Sync {
    async fn execute_action(&self, action: &WorkflowAction, context: &WorkflowContext) -> Result<serde_json::Value>;
    fn supports_action(&self, action: &WorkflowAction) -> bool;
}

/// Main workflow engine that manages workflow execution
pub struct WorkflowEngine {
    workflows: Arc<RwLock<HashMap<String, WorkflowDefinition>>>,
    executions: Arc<RwLock<HashMap<String, WorkflowExecution>>>,
    action_executors: Arc<RwLock<Vec<Arc<dyn WorkflowActionExecutor>>>>,
    event_handlers: Arc<RwLock<Vec<Arc<dyn WorkflowEventHandler>>>>,
    trigger_rx: Arc<RwLock<Option<mpsc::Receiver<WorkflowTriggerEvent>>>>,
    trigger_tx: mpsc::Sender<WorkflowTriggerEvent>,
}

/// Represents a workflow trigger event
#[derive(Debug, Clone)]
pub struct WorkflowTriggerEvent {
    pub trigger_type: String,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

impl WorkflowEngine {
    /// Create a new workflow engine
    pub fn new() -> Self {
        let (trigger_tx, trigger_rx) = mpsc::channel(1000);
        
        Self {
            workflows: Arc::new(RwLock::new(HashMap::new())),
            executions: Arc::new(RwLock::new(HashMap::new())),
            action_executors: Arc::new(RwLock::new(Vec::new())),
            event_handlers: Arc::new(RwLock::new(Vec::new())),
            trigger_rx: Arc::new(RwLock::new(Some(trigger_rx))),
            trigger_tx,
        }
    }

    /// Start the workflow engine
    pub async fn start(&self) -> Result<()> {
        let mut trigger_rx = self.trigger_rx.write().await.take()
            .ok_or_else(|| anyhow::anyhow!("Workflow engine already started"))?;

        let workflows = self.workflows.clone();
        let executions = self.executions.clone();
        let action_executors = self.action_executors.clone();
        let event_handlers = self.event_handlers.clone();

        // Start the trigger processing loop
        tokio::spawn(async move {
            while let Some(trigger_event) = trigger_rx.recv().await {
                // Process the trigger event
                let workflows_read = workflows.read().await;
                
                for workflow in workflows_read.values() {
                    if workflow.enabled && Self::trigger_matches(&workflow.trigger, &trigger_event) {
                        // Execute the workflow
                        let execution_id = Uuid::new_v4().to_string();
                        let context = WorkflowContext {
                            workflow_id: workflow.id.clone(),
                            execution_id: execution_id.clone(),
                            trigger_data: trigger_event.data.clone(),
                            variables: HashMap::new(),
                            metadata: HashMap::new(),
                        };

                        let execution = WorkflowExecution {
                            id: execution_id,
                            workflow_id: workflow.id.clone(),
                            state: WorkflowState::Running,
                            started_at: Utc::now(),
                            completed_at: None,
                            error_message: None,
                            context,
                            executed_actions: Vec::new(),
                            failed_actions: Vec::new(),
                        };

                        // Store the execution
                        executions.write().await.insert(execution.id.clone(), execution.clone());

                        // Notify event handlers
                        let handlers = event_handlers.read().await;
                        for handler in handlers.iter() {
                            if let Err(e) = handler.handle_workflow_started(&execution).await {
                                eprintln!("Error handling workflow started event: {}", e);
                            }
                        }

                        // Execute the workflow in a separate task
                        let workflow_clone = workflow.clone();
                        let execution_clone = execution.clone();
                        let executors = action_executors.clone();
                        let handlers = event_handlers.clone();
                        let executions_clone = executions.clone();

                        tokio::spawn(async move {
                            let result = Self::execute_workflow_internal(
                                &workflow_clone,
                                execution_clone,
                                executors,
                                handlers,
                                executions_clone,
                            ).await;

                            if let Err(e) = result {
                                eprintln!("Error executing workflow {}: {}", workflow_clone.id, e);
                            }
                        });
                    }
                }
            }
        });

        Ok(())
    }

    /// Register a new workflow definition
    pub async fn register_workflow(&self, workflow: WorkflowDefinition) -> Result<()> {
        self.workflows.write().await.insert(workflow.id.clone(), workflow);
        Ok(())
    }

    /// Remove a workflow definition
    pub async fn remove_workflow(&self, workflow_id: &str) -> Result<()> {
        self.workflows.write().await.remove(workflow_id);
        Ok(())
    }

    /// Get all registered workflows
    pub async fn get_workflows(&self) -> Result<Vec<WorkflowDefinition>> {
        Ok(self.workflows.read().await.values().cloned().collect())
    }

    /// Get a specific workflow by ID
    pub async fn get_workflow(&self, workflow_id: &str) -> Result<Option<WorkflowDefinition>> {
        Ok(self.workflows.read().await.get(workflow_id).cloned())
    }

    /// Trigger a workflow manually
    pub async fn trigger_workflow(&self, workflow_id: &str, trigger_data: serde_json::Value) -> Result<String> {
        let trigger_event = WorkflowTriggerEvent {
            trigger_type: "manual".to_string(),
            data: trigger_data,
            timestamp: Utc::now(),
        };

        self.trigger_tx.send(trigger_event).await
            .context("Failed to send trigger event")?;

        Ok("Workflow triggered successfully".to_string())
    }

    /// Send a trigger event to the workflow engine
    pub async fn send_trigger_event(&self, trigger_event: WorkflowTriggerEvent) -> Result<()> {
        self.trigger_tx.send(trigger_event).await
            .context("Failed to send trigger event")?;
        Ok(())
    }

    /// Register an action executor
    pub async fn register_action_executor(&self, executor: Arc<dyn WorkflowActionExecutor>) {
        self.action_executors.write().await.push(executor);
    }

    /// Register an event handler
    pub async fn register_event_handler(&self, handler: Arc<dyn WorkflowEventHandler>) {
        self.event_handlers.write().await.push(handler);
    }

    /// Get all workflow executions
    pub async fn get_executions(&self) -> Result<Vec<WorkflowExecution>> {
        Ok(self.executions.read().await.values().cloned().collect())
    }

    /// Get a specific workflow execution by ID
    pub async fn get_execution(&self, execution_id: &str) -> Result<Option<WorkflowExecution>> {
        Ok(self.executions.read().await.get(execution_id).cloned())
    }

    /// Check if a trigger matches a trigger event
    fn trigger_matches(trigger: &WorkflowTrigger, event: &WorkflowTriggerEvent) -> bool {
        match trigger {
            WorkflowTrigger::Manual => event.trigger_type == "manual",
            WorkflowTrigger::CalendarEvent { event_type, .. } => {
                event.trigger_type == "calendar" && 
                event.data.get("event_type").and_then(|v| v.as_str()) == Some(event_type)
            }
            WorkflowTrigger::ObsidianNote { operation, .. } => {
                event.trigger_type == "obsidian" && 
                event.data.get("operation").and_then(|v| v.as_str()) == Some(operation)
            }
            WorkflowTrigger::JiraIssue { project, .. } => {
                event.trigger_type == "jira" && 
                event.data.get("project").and_then(|v| v.as_str()) == Some(project)
            }
            WorkflowTrigger::Scheduled { .. } => {
                event.trigger_type == "scheduled"
            }
            WorkflowTrigger::Webhook { .. } => {
                event.trigger_type == "webhook"
            }
        }
    }

    /// Execute a workflow internally
    async fn execute_workflow_internal(
        workflow: &WorkflowDefinition,
        mut execution: WorkflowExecution,
        action_executors: Arc<RwLock<Vec<Arc<dyn WorkflowActionExecutor>>>>,
        event_handlers: Arc<RwLock<Vec<Arc<dyn WorkflowEventHandler>>>>,
        executions: Arc<RwLock<HashMap<String, WorkflowExecution>>>,
    ) -> Result<()> {
        // Execute rules in order
        for rule in &workflow.rules {
            if !rule.enabled {
                continue;
            }

            // Check if rule conditions are met
            if Self::evaluate_rule_conditions(&rule.conditions, &execution.context)? {
                // Execute actions for this rule
                for action in &rule.actions {
                    let result = Self::execute_action(
                        action,
                        &execution.context,
                        &action_executors,
                        &event_handlers,
                        &execution,
                    ).await;

                    match result {
                        Ok(_) => {
                            execution.executed_actions.push(format!("{}:{}", rule.id, action.get_type()));
                        }
                        Err(e) => {
                            execution.failed_actions.push(format!("{}:{}", rule.id, action.get_type()));
                            
                            // Notify event handlers of action failure
                            let handlers = event_handlers.read().await;
                            for handler in handlers.iter() {
                                if let Err(err) = handler.handle_action_failed(&execution, action, &e.to_string()).await {
                                    eprintln!("Error handling action failed event: {}", err);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Update execution state
        execution.state = if execution.failed_actions.is_empty() {
            WorkflowState::Completed
        } else {
            WorkflowState::Failed
        };
        execution.completed_at = Some(Utc::now());

        // Store updated execution
        executions.write().await.insert(execution.id.clone(), execution.clone());

        // Notify event handlers
        let handlers = event_handlers.read().await;
        for handler in handlers.iter() {
            let result = if execution.state == WorkflowState::Completed {
                handler.handle_workflow_completed(&execution).await
            } else {
                handler.handle_workflow_failed(&execution, "Some actions failed").await
            };

            if let Err(e) = result {
                eprintln!("Error handling workflow completion event: {}", e);
            }
        }

        Ok(())
    }

    /// Evaluate rule conditions
    fn evaluate_rule_conditions(conditions: &[WorkflowCondition], context: &WorkflowContext) -> Result<bool> {
        if conditions.is_empty() {
            return Ok(true);
        }

        for condition in conditions {
            if !Self::evaluate_condition(condition, context)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Evaluate a single condition
    fn evaluate_condition(condition: &WorkflowCondition, context: &WorkflowContext) -> Result<bool> {
        // Get the field value from context
        let field_value = context.trigger_data.get(&condition.field)
            .or_else(|| context.variables.get(&condition.field))
            .ok_or_else(|| anyhow::anyhow!("Field '{}' not found in context", condition.field))?;

        // Evaluate based on operator
        match condition.operator.as_str() {
            "eq" => Ok(field_value == &condition.value),
            "ne" => Ok(field_value != &condition.value),
            "gt" => {
                if let (Some(field_num), Some(cond_num)) = (field_value.as_f64(), condition.value.as_f64()) {
                    Ok(field_num > cond_num)
                } else {
                    Ok(false)
                }
            }
            "lt" => {
                if let (Some(field_num), Some(cond_num)) = (field_value.as_f64(), condition.value.as_f64()) {
                    Ok(field_num < cond_num)
                } else {
                    Ok(false)
                }
            }
            "contains" => {
                if let (Some(field_str), Some(cond_str)) = (field_value.as_str(), condition.value.as_str()) {
                    Ok(field_str.contains(cond_str))
                } else {
                    Ok(false)
                }
            }
            "regex" => {
                if let (Some(field_str), Some(pattern)) = (field_value.as_str(), condition.value.as_str()) {
                    // Note: For production use, consider using a proper regex library
                    Ok(field_str.contains(pattern)) // Simplified for now
                } else {
                    Ok(false)
                }
            }
            _ => Err(anyhow::anyhow!("Unknown operator: {}", condition.operator)),
        }
    }

    /// Execute a single action
    async fn execute_action(
        action: &WorkflowAction,
        context: &WorkflowContext,
        action_executors: &Arc<RwLock<Vec<Arc<dyn WorkflowActionExecutor>>>>,
        event_handlers: &Arc<RwLock<Vec<Arc<dyn WorkflowEventHandler>>>>,
        execution: &WorkflowExecution,
    ) -> Result<serde_json::Value> {
        let executors = action_executors.read().await;
        
        for executor in executors.iter() {
            if executor.supports_action(action) {
                let result = executor.execute_action(action, context).await;
                
                match &result {
                    Ok(_) => {
                        // Notify event handlers of successful action execution
                        let handlers = event_handlers.read().await;
                        for handler in handlers.iter() {
                            if let Err(e) = handler.handle_action_executed(execution, action).await {
                                eprintln!("Error handling action executed event: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        // Notify event handlers of action failure
                        let handlers = event_handlers.read().await;
                        for handler in handlers.iter() {
                            if let Err(err) = handler.handle_action_failed(execution, action, &e.to_string()).await {
                                eprintln!("Error handling action failed event: {}", err);
                            }
                        }
                    }
                }
                
                return result;
            }
        }

        Err(anyhow::anyhow!("No executor found for action type: {}", action.get_type()))
    }
}

impl WorkflowAction {
    /// Get the type string for this action
    pub fn get_type(&self) -> &'static str {
        match self {
            WorkflowAction::CreateObsidianNote { .. } => "create_obsidian_note",
            WorkflowAction::UpdateObsidianNote { .. } => "update_obsidian_note",
            WorkflowAction::CreateJiraIssue { .. } => "create_jira_issue",
            WorkflowAction::UpdateJiraIssue { .. } => "update_jira_issue",
            WorkflowAction::CreateCalendarEvent { .. } => "create_calendar_event",
            WorkflowAction::SendNotification { .. } => "send_notification",
            WorkflowAction::ExecuteShellCommand { .. } => "execute_shell_command",
            WorkflowAction::CallWebhook { .. } => "call_webhook",
        }
    }
}

impl Default for WorkflowEngine {
    fn default() -> Self {
        Self::new()
    }
}