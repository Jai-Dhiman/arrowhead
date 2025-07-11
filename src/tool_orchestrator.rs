use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;
use uuid::Uuid;

/// Represents the execution result of a tool operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

impl ToolResult {
    pub fn success(data: Option<serde_json::Value>, execution_time_ms: u64) -> Self {
        Self {
            success: true,
            data,
            error: None,
            execution_time_ms,
        }
    }

    pub fn error(error: String, execution_time_ms: u64) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            execution_time_ms,
        }
    }
}

/// Represents the health status of a tool
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
            HealthStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// Health check result containing status and optional details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub status: HealthStatus,
    pub message: Option<String>,
    pub last_checked: chrono::DateTime<chrono::Utc>,
    pub response_time_ms: Option<u64>,
}

impl HealthCheck {
    pub fn healthy(message: Option<String>, response_time_ms: Option<u64>) -> Self {
        Self {
            status: HealthStatus::Healthy,
            message,
            last_checked: chrono::Utc::now(),
            response_time_ms,
        }
    }

    pub fn degraded(message: String, response_time_ms: Option<u64>) -> Self {
        Self {
            status: HealthStatus::Degraded,
            message: Some(message),
            last_checked: chrono::Utc::now(),
            response_time_ms,
        }
    }

    pub fn unhealthy(message: String) -> Self {
        Self {
            status: HealthStatus::Unhealthy,
            message: Some(message),
            last_checked: chrono::Utc::now(),
            response_time_ms: None,
        }
    }
}

/// Metadata about a tool's capabilities and requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub dependencies: Vec<String>,
    pub timeout_ms: u64,
    pub retry_attempts: u32,
}

impl ToolMetadata {
    pub fn new(
        name: String,
        version: String,
        description: String,
        capabilities: Vec<String>,
    ) -> Self {
        Self {
            name,
            version,
            description,
            capabilities,
            dependencies: Vec::new(),
            timeout_ms: 5000, // 5 second default timeout
            retry_attempts: 3,
        }
    }

    pub fn with_dependencies(mut self, dependencies: Vec<String>) -> Self {
        self.dependencies = dependencies;
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn with_retry_attempts(mut self, retry_attempts: u32) -> Self {
        self.retry_attempts = retry_attempts;
        self
    }
}

/// Core trait that all tools must implement for orchestration
#[async_trait]
pub trait Tool: Send + Sync + fmt::Debug {
    /// Execute a tool operation with the given parameters
    async fn execute(&self, operation: &str, params: serde_json::Value) -> Result<ToolResult>;

    /// Get the current status/state of the tool
    async fn status(&self) -> Result<serde_json::Value>;

    /// Perform a health check on the tool
    async fn health_check(&self) -> HealthCheck;

    /// Get metadata about the tool's capabilities
    fn metadata(&self) -> &ToolMetadata;

    /// Get the unique identifier for this tool instance
    fn id(&self) -> &str;

    /// Initialize the tool (called when registering)
    async fn initialize(&mut self) -> Result<()> {
        Ok(())
    }

    /// Cleanup resources (called when unregistering)
    async fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Registry for managing and discovering tools
#[derive(Debug)]
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
    tool_metadata: HashMap<String, ToolMetadata>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            tool_metadata: HashMap::new(),
        }
    }

    /// Register a new tool in the registry
    pub async fn register_tool(&mut self, mut tool: Box<dyn Tool>) -> Result<String> {
        let tool_id = tool.id().to_string();

        // Initialize the tool
        tool.initialize()
            .await
            .context(format!("Failed to initialize tool {}", tool_id))?;

        // Store metadata
        let metadata = tool.metadata().clone();
        self.tool_metadata.insert(tool_id.clone(), metadata);

        // Store the tool
        self.tools.insert(tool_id.clone(), tool);

        Ok(tool_id)
    }

    /// Unregister a tool from the registry
    pub async fn unregister_tool(&mut self, tool_id: &str) -> Result<()> {
        if let Some(mut tool) = self.tools.remove(tool_id) {
            tool.shutdown()
                .await
                .context(format!("Failed to shutdown tool {}", tool_id))?;
            self.tool_metadata.remove(tool_id);
            Ok(())
        } else {
            anyhow::bail!("Tool {} not found in registry", tool_id)
        }
    }

    /// Get a reference to a registered tool
    pub fn get_tool(&self, tool_id: &str) -> Option<&dyn Tool> {
        self.tools.get(tool_id).map(|tool| tool.as_ref())
    }

    /// Get a mutable reference to a registered tool
    pub fn get_tool_mut(&mut self, tool_id: &str) -> Option<&mut Box<dyn Tool>> {
        self.tools.get_mut(tool_id)
    }

    /// List all registered tools with their metadata
    pub fn list_tools(&self) -> Vec<(&str, &ToolMetadata)> {
        self.tool_metadata
            .iter()
            .map(|(id, metadata)| (id.as_str(), metadata))
            .collect()
    }

    /// Discover tools by capability
    pub fn discover_by_capability(&self, capability: &str) -> Vec<&str> {
        self.tool_metadata
            .iter()
            .filter(|(_, metadata)| metadata.capabilities.contains(&capability.to_string()))
            .map(|(id, _)| id.as_str())
            .collect()
    }

    /// Check if a tool is registered
    pub fn is_registered(&self, tool_id: &str) -> bool {
        self.tools.contains_key(tool_id)
    }

    /// Get the number of registered tools
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    /// Execute an operation on a tool with timeout
    pub async fn execute_with_timeout(
        &self,
        tool_id: &str,
        operation: &str,
        params: serde_json::Value,
    ) -> Result<ToolResult> {
        let tool = self
            .get_tool(tool_id)
            .ok_or_else(|| anyhow::anyhow!("Tool {} not found", tool_id))?;

        let timeout_duration = Duration::from_millis(tool.metadata().timeout_ms);

        timeout(timeout_duration, tool.execute(operation, params))
            .await
            .context(format!(
                "Tool {} operation timed out after {}ms",
                tool_id,
                tool.metadata().timeout_ms
            ))?
    }

    /// Perform health checks on all registered tools
    pub async fn health_check_all(&self) -> HashMap<String, HealthCheck> {
        let mut results = HashMap::new();

        for (tool_id, tool) in &self.tools {
            let health = tool.health_check().await;
            results.insert(tool_id.clone(), health);
        }

        results
    }

    /// Get status from all registered tools
    pub async fn status_all(&self) -> HashMap<String, Result<serde_json::Value>> {
        let mut results = HashMap::new();

        for (tool_id, tool) in &self.tools {
            let status = tool.status().await;
            results.insert(tool_id.clone(), status);
        }

        results
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Core orchestrator that manages multiple tools and coordinates their operations
#[derive(Debug)]
pub struct ToolOrchestrator {
    registry: ToolRegistry,
    active_tools: HashMap<String, String>, // tool_type -> tool_id mapping
    coordination_state: CoordinationState,
    monitoring_enabled: bool,
    data_flow_system: DataFlowSystem,
}

/// State information for coordinating multi-tool operations
#[derive(Debug)]
pub struct CoordinationState {
    pub active_workflows: HashMap<String, WorkflowState>,
    pub pending_operations: Vec<PendingOperation>,
    pub last_health_check: Option<chrono::DateTime<chrono::Utc>>,
}

/// State of a workflow execution
#[derive(Debug, Clone)]
pub struct WorkflowState {
    pub id: String,
    pub status: WorkflowStatus,
    pub steps: Vec<WorkflowStep>,
    pub current_step: usize,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Status of a workflow
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorkflowStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Individual step in a workflow
#[derive(Debug, Clone)]
pub struct WorkflowStep {
    pub id: String,
    pub tool_id: String,
    pub operation: String,
    pub params: serde_json::Value,
    pub status: WorkflowStatus,
    pub result: Option<ToolResult>,
    pub dependencies: Vec<String>, // step_ids that must complete first
}

/// Operation waiting to be executed
#[derive(Debug, Clone)]
pub struct PendingOperation {
    pub id: String,
    pub tool_id: String,
    pub operation: String,
    pub params: serde_json::Value,
    pub priority: u8, // 0-255, higher is more important
    pub scheduled_at: chrono::DateTime<chrono::Utc>,
    pub timeout_ms: u64,
}

impl ToolOrchestrator {
    /// Create a new ToolOrchestrator instance
    pub fn new() -> Self {
        Self {
            registry: ToolRegistry::new(),
            active_tools: HashMap::new(),
            coordination_state: CoordinationState::new(),
            monitoring_enabled: true,
            data_flow_system: DataFlowSystem::new(),
        }
    }

    /// Register a tool with the orchestrator
    pub async fn register_tool(
        &mut self,
        tool: Box<dyn Tool>,
        tool_type: String,
    ) -> Result<String> {
        let tool_id = self.registry.register_tool(tool).await?;
        self.active_tools.insert(tool_type, tool_id.clone());
        Ok(tool_id)
    }

    /// Unregister a tool from the orchestrator
    pub async fn unregister_tool(&mut self, tool_id: &str) -> Result<()> {
        self.registry.unregister_tool(tool_id).await?;

        // Remove from active tools mapping
        self.active_tools.retain(|_, id| id != tool_id);

        Ok(())
    }

    /// Get a tool by its type (e.g., "obsidian", "calendar", "jira", "mcp")
    pub fn get_tool_by_type(&self, tool_type: &str) -> Option<&dyn Tool> {
        self.active_tools
            .get(tool_type)
            .and_then(|tool_id| self.registry.get_tool(tool_id))
    }

    /// Execute an operation on a specific tool with coordination
    pub async fn execute_coordinated(
        &mut self,
        tool_type: &str,
        operation: &str,
        params: serde_json::Value,
    ) -> Result<ToolResult> {
        let tool_id = self
            .active_tools
            .get(tool_type)
            .ok_or_else(|| anyhow::anyhow!("Tool type '{}' not found", tool_type))?;

        // Execute with timeout and coordination
        let start_time = std::time::Instant::now();
        let result = self
            .registry
            .execute_with_timeout(tool_id, operation, params)
            .await?;

        // Update coordination state
        self.coordination_state
            .record_operation(tool_id, operation, &result);

        Ok(result)
    }

    /// Execute operations on multiple tools in parallel
    pub async fn execute_parallel(
        &mut self,
        operations: Vec<(String, String, serde_json::Value)>, // (tool_type, operation, params)
    ) -> Result<Vec<ToolResult>> {
        let mut results = Vec::new();

        for (tool_type, operation, params) in operations {
            let tool_id = self
                .active_tools
                .get(&tool_type)
                .ok_or_else(|| anyhow::anyhow!("Tool type '{}' not found", tool_type))?;

            let result = self
                .registry
                .execute_with_timeout(tool_id, &operation, params)
                .await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Perform health checks on all active tools
    pub async fn health_check_all(&self) -> HashMap<String, HealthCheck> {
        let mut type_health = HashMap::new();

        for (tool_type, tool_id) in &self.active_tools {
            if let Some(tool) = self.registry.get_tool(tool_id) {
                let health = tool.health_check().await;
                type_health.insert(tool_type.clone(), health);
            }
        }

        type_health
    }

    /// Get status from all active tools
    pub async fn status_all(&self) -> HashMap<String, Result<serde_json::Value>> {
        let mut type_status = HashMap::new();

        for (tool_type, tool_id) in &self.active_tools {
            if let Some(tool) = self.registry.get_tool(tool_id) {
                let status = tool.status().await;
                type_status.insert(tool_type.clone(), status);
            }
        }

        type_status
    }

    /// Initialize all registered tools
    pub async fn initialize_all(&mut self) -> Result<()> {
        for (tool_type, tool_id) in &self.active_tools.clone() {
            if let Some(tool) = self.registry.get_tool_mut(tool_id) {
                tool.initialize()
                    .await
                    .context(format!("Failed to initialize {} tool", tool_type))?;
            }
        }
        Ok(())
    }

    /// Shutdown all registered tools gracefully
    pub async fn shutdown_all(&mut self) -> Result<()> {
        for (tool_type, tool_id) in &self.active_tools.clone() {
            if let Some(tool) = self.registry.get_tool_mut(tool_id) {
                tool.shutdown()
                    .await
                    .context(format!("Failed to shutdown {} tool", tool_type))?;
            }
        }

        self.active_tools.clear();
        Ok(())
    }

    /// Get orchestrator statistics
    pub fn get_stats(&self) -> OrchestratorStats {
        let data_flow_stats = self.data_flow_system.get_stats();
        OrchestratorStats {
            total_tools: self.registry.tool_count(),
            active_tools: self.active_tools.len(),
            active_workflows: self.coordination_state.active_workflows.len(),
            pending_operations: self.coordination_state.pending_operations.len(),
            monitoring_enabled: self.monitoring_enabled,
            data_flow_stats,
        }
    }

    /// Enable or disable monitoring
    pub fn set_monitoring(&mut self, enabled: bool) {
        self.monitoring_enabled = enabled;
    }

    /// Get the tool registry for direct access
    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }

    /// Get mutable access to the tool registry
    pub fn registry_mut(&mut self) -> &mut ToolRegistry {
        &mut self.registry
    }

    /// Send data between tools using the data flow system
    pub async fn send_data(&self, tool_id: &str, packet: DataPacket) -> Result<()> {
        self.data_flow_system.send_to_tool(tool_id, packet).await
    }

    /// Broadcast data to all registered tools
    pub async fn broadcast_data(&self, packet: DataPacket) -> Result<()> {
        self.data_flow_system.broadcast(packet).await
    }

    /// Send data with automatic transformation
    pub async fn send_data_with_transformation(
        &self,
        tool_id: &str,
        packet: DataPacket,
        target_type: DataType,
    ) -> Result<()> {
        self.data_flow_system
            .send_with_transformation(tool_id, packet, target_type)
            .await
    }

    /// Register a tool in the data flow system
    pub fn register_data_flow(&mut self, tool_id: String) -> mpsc::UnboundedReceiver<DataPacket> {
        self.data_flow_system.register_tool(tool_id)
    }

    /// Unregister a tool from the data flow system
    pub fn unregister_data_flow(&mut self, tool_id: &str) {
        self.data_flow_system.unregister_tool(tool_id)
    }

    /// Get access to the data flow system
    pub fn data_flow_system(&self) -> &DataFlowSystem {
        &self.data_flow_system
    }

    /// Get mutable access to the data flow system
    pub fn data_flow_system_mut(&mut self) -> &mut DataFlowSystem {
        &mut self.data_flow_system
    }
}

/// Statistics about the orchestrator state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorStats {
    pub total_tools: usize,
    pub active_tools: usize,
    pub active_workflows: usize,
    pub pending_operations: usize,
    pub monitoring_enabled: bool,
    pub data_flow_stats: DataFlowStats,
}

impl CoordinationState {
    pub fn new() -> Self {
        Self {
            active_workflows: HashMap::new(),
            pending_operations: Vec::new(),
            last_health_check: None,
        }
    }

    /// Record the completion of an operation
    pub fn record_operation(&mut self, tool_id: &str, operation: &str, result: &ToolResult) {
        // This would typically update workflow states, log operations, etc.
        // For now, we'll just update the last health check time if this was a health check
        if operation == "health_check" {
            self.last_health_check = Some(chrono::Utc::now());
        }
    }

    /// Add a pending operation
    pub fn add_pending_operation(&mut self, operation: PendingOperation) {
        self.pending_operations.push(operation);

        // Sort by priority (higher priority first)
        self.pending_operations
            .sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Get the next pending operation
    pub fn next_pending_operation(&mut self) -> Option<PendingOperation> {
        self.pending_operations.pop()
    }
}

impl Default for ToolOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

/// Workflow execution engine for orchestrating multi-tool operations
pub struct WorkflowEngine {
    orchestrator: ToolOrchestrator,
    workflow_timeout_ms: u64,
    max_parallel_workflows: usize,
    retry_config: RetryConfig,
}

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub exponential_base: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            exponential_base: 2.0,
        }
    }
}

/// Definition of a workflow with steps and execution logic
#[derive(Debug, Clone)]
pub struct WorkflowDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStepDefinition>,
    pub timeout_ms: u64,
    pub retry_config: RetryConfig,
    pub metadata: HashMap<String, String>,
}

/// Definition of a single step in a workflow
#[derive(Debug, Clone)]
pub struct WorkflowStepDefinition {
    pub id: String,
    pub name: String,
    pub tool_type: String,
    pub operation: String,
    pub params: serde_json::Value,
    pub dependencies: Vec<String>, // step IDs that must complete first
    pub timeout_ms: Option<u64>,
    pub retry_attempts: Option<u32>,
    pub condition: Option<StepCondition>,
    pub rollback_operation: Option<String>,
}

/// Condition for executing a step
#[derive(Debug, Clone)]
pub struct StepCondition {
    pub condition_type: ConditionType,
    pub target_step_id: String,
    pub expected_value: serde_json::Value,
}

/// Types of conditions for step execution
#[derive(Debug, Clone, PartialEq)]
pub enum ConditionType {
    StepSucceeded,
    StepFailed,
    StepResultEquals,
    StepResultContains,
    Always,
}

/// Execution context for a workflow
#[derive(Debug, Clone)]
pub struct WorkflowContext {
    pub workflow_id: String,
    pub variables: HashMap<String, serde_json::Value>,
    pub step_results: HashMap<String, ToolResult>,
    pub current_step_index: usize,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub timeout_at: chrono::DateTime<chrono::Utc>,
}

impl WorkflowEngine {
    pub fn new(orchestrator: ToolOrchestrator) -> Self {
        Self {
            orchestrator,
            workflow_timeout_ms: 30000, // 30 seconds default
            max_parallel_workflows: 10,
            retry_config: RetryConfig::default(),
        }
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.workflow_timeout_ms = timeout_ms;
        self
    }

    pub fn with_max_parallel(mut self, max_parallel: usize) -> Self {
        self.max_parallel_workflows = max_parallel;
        self
    }

    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    /// Execute a workflow with proper sequencing and error handling
    pub async fn execute_workflow(
        &mut self,
        definition: WorkflowDefinition,
        initial_variables: HashMap<String, serde_json::Value>,
    ) -> Result<WorkflowState> {
        let workflow_id = Uuid::new_v4().to_string();
        let timeout_at =
            chrono::Utc::now() + chrono::Duration::milliseconds(definition.timeout_ms as i64);

        let mut context = WorkflowContext {
            workflow_id: workflow_id.clone(),
            variables: initial_variables,
            step_results: HashMap::new(),
            current_step_index: 0,
            started_at: chrono::Utc::now(),
            timeout_at,
        };

        let mut workflow_state = WorkflowState {
            id: workflow_id,
            status: WorkflowStatus::Running,
            steps: definition
                .steps
                .iter()
                .map(|step_def| WorkflowStep {
                    id: step_def.id.clone(),
                    tool_id: step_def.tool_type.clone(),
                    operation: step_def.operation.clone(),
                    params: step_def.params.clone(),
                    status: WorkflowStatus::Pending,
                    result: None,
                    dependencies: step_def.dependencies.clone(),
                })
                .collect(),
            current_step: 0,
            started_at: context.started_at,
            updated_at: chrono::Utc::now(),
        };

        // Execute workflow steps with proper sequencing
        let result = self
            .execute_workflow_steps(&definition, &mut context, &mut workflow_state)
            .await;

        match result {
            Ok(()) => {
                workflow_state.status = WorkflowStatus::Completed;
                workflow_state.updated_at = chrono::Utc::now();
            }
            Err(e) => {
                workflow_state.status = WorkflowStatus::Failed;
                workflow_state.updated_at = chrono::Utc::now();

                // Attempt rollback
                if let Err(rollback_error) = self.rollback_workflow(&definition, &context).await {
                    eprintln!("Rollback failed: {}", rollback_error);
                }

                return Err(e);
            }
        }

        Ok(workflow_state)
    }

    /// Execute workflow steps in proper sequence
    async fn execute_workflow_steps(
        &mut self,
        definition: &WorkflowDefinition,
        context: &mut WorkflowContext,
        workflow_state: &mut WorkflowState,
    ) -> Result<()> {
        let mut pending_steps: Vec<&WorkflowStepDefinition> = definition.steps.iter().collect();
        let mut completed_steps: HashSet<String> = HashSet::new();

        while !pending_steps.is_empty() {
            // Check for timeout
            if chrono::Utc::now() > context.timeout_at {
                return Err(anyhow::anyhow!(
                    "Workflow timed out after {}ms",
                    definition.timeout_ms
                ));
            }

            // Find steps that can be executed (dependencies satisfied)
            let mut ready_steps = Vec::new();
            let mut remaining_steps = Vec::new();

            for step in pending_steps {
                if self.can_execute_step(step, &completed_steps, context)? {
                    ready_steps.push(step);
                } else {
                    remaining_steps.push(step);
                }
            }

            if ready_steps.is_empty() {
                return Err(anyhow::anyhow!(
                    "Workflow deadlock: no steps can be executed"
                ));
            }

            // Execute ready steps
            for step in ready_steps {
                let result = self.execute_step(step, context, workflow_state).await?;

                // Update workflow state
                if let Some(workflow_step) =
                    workflow_state.steps.iter_mut().find(|s| s.id == step.id)
                {
                    workflow_step.result = Some(result.clone());
                    workflow_step.status = if result.success {
                        WorkflowStatus::Completed
                    } else {
                        WorkflowStatus::Failed
                    };
                }

                // Store result in context
                context.step_results.insert(step.id.clone(), result);
                completed_steps.insert(step.id.clone());
            }

            pending_steps = remaining_steps;
        }

        Ok(())
    }

    /// Check if a step can be executed based on its dependencies and conditions
    fn can_execute_step(
        &self,
        step: &WorkflowStepDefinition,
        completed_steps: &HashSet<String>,
        context: &WorkflowContext,
    ) -> Result<bool> {
        // Check dependencies
        for dep_id in &step.dependencies {
            if !completed_steps.contains(dep_id) {
                return Ok(false);
            }
        }

        // Check condition if present
        if let Some(condition) = &step.condition {
            return self.evaluate_condition(condition, context);
        }

        Ok(true)
    }

    /// Evaluate a step condition
    fn evaluate_condition(
        &self,
        condition: &StepCondition,
        context: &WorkflowContext,
    ) -> Result<bool> {
        match condition.condition_type {
            ConditionType::Always => Ok(true),
            ConditionType::StepSucceeded => {
                if let Some(result) = context.step_results.get(&condition.target_step_id) {
                    Ok(result.success)
                } else {
                    Ok(false)
                }
            }
            ConditionType::StepFailed => {
                if let Some(result) = context.step_results.get(&condition.target_step_id) {
                    Ok(!result.success)
                } else {
                    Ok(false)
                }
            }
            ConditionType::StepResultEquals => {
                if let Some(result) = context.step_results.get(&condition.target_step_id) {
                    Ok(result.data.as_ref() == Some(&condition.expected_value))
                } else {
                    Ok(false)
                }
            }
            ConditionType::StepResultContains => {
                if let Some(result) = context.step_results.get(&condition.target_step_id) {
                    if let Some(data) = &result.data {
                        // Simple string contains check
                        let data_str = data.to_string();
                        let expected_str = condition.expected_value.to_string();
                        Ok(data_str.contains(&expected_str))
                    } else {
                        Ok(false)
                    }
                } else {
                    Ok(false)
                }
            }
        }
    }

    /// Execute a single workflow step
    async fn execute_step(
        &mut self,
        step: &WorkflowStepDefinition,
        context: &mut WorkflowContext,
        workflow_state: &mut WorkflowState,
    ) -> Result<ToolResult> {
        let mut params = step.params.clone();

        // Substitute variables in parameters
        self.substitute_variables(&mut params, &context.variables)?;

        // Execute with retry logic
        let retry_attempts = step
            .retry_attempts
            .unwrap_or(self.retry_config.max_attempts);
        let mut last_error = None;

        for attempt in 0..retry_attempts {
            let result = self
                .orchestrator
                .execute_coordinated(&step.tool_type, &step.operation, params.clone())
                .await;

            match result {
                Ok(tool_result) => {
                    if tool_result.success {
                        // Update variables with step result
                        if let Some(data) = &tool_result.data {
                            context
                                .variables
                                .insert(format!("step_{}_result", step.id), data.clone());
                        }
                        return Ok(tool_result);
                    } else {
                        last_error = Some(anyhow::anyhow!(
                            "Step failed: {}",
                            tool_result
                                .error
                                .unwrap_or_else(|| "Unknown error".to_string())
                        ));
                    }
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }

            // Wait before retry (exponential backoff)
            if attempt < retry_attempts - 1 {
                let delay = self.calculate_retry_delay(attempt);
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }
        }

        Err(last_error
            .unwrap_or_else(|| anyhow::anyhow!("Step failed after {} attempts", retry_attempts)))
    }

    /// Calculate retry delay with exponential backoff
    fn calculate_retry_delay(&self, attempt: u32) -> u64 {
        let delay = self.retry_config.initial_delay_ms as f64
            * self.retry_config.exponential_base.powf(attempt as f64);
        delay.min(self.retry_config.max_delay_ms as f64) as u64
    }

    /// Substitute variables in JSON parameters
    fn substitute_variables(
        &self,
        params: &mut serde_json::Value,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        match params {
            serde_json::Value::String(s) => {
                if s.starts_with("${") && s.ends_with("}") {
                    let var_name = &s[2..s.len() - 1];
                    if let Some(value) = variables.get(var_name) {
                        *params = value.clone();
                    }
                }
            }
            serde_json::Value::Object(obj) => {
                for (_, value) in obj.iter_mut() {
                    self.substitute_variables(value, variables)?;
                }
            }
            serde_json::Value::Array(arr) => {
                for value in arr.iter_mut() {
                    self.substitute_variables(value, variables)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Rollback a workflow by executing rollback operations
    async fn rollback_workflow(
        &mut self,
        definition: &WorkflowDefinition,
        context: &WorkflowContext,
    ) -> Result<()> {
        // Execute rollback operations in reverse order
        for step in definition.steps.iter().rev() {
            if let Some(rollback_op) = &step.rollback_operation {
                if context.step_results.contains_key(&step.id) {
                    let _ = self
                        .orchestrator
                        .execute_coordinated(&step.tool_type, rollback_op, step.params.clone())
                        .await;
                }
            }
        }
        Ok(())
    }

    /// Get access to the underlying orchestrator
    pub fn orchestrator(&self) -> &ToolOrchestrator {
        &self.orchestrator
    }

    /// Get mutable access to the underlying orchestrator
    pub fn orchestrator_mut(&mut self) -> &mut ToolOrchestrator {
        &mut self.orchestrator
    }
}

/// Comprehensive monitoring and health management system
#[derive(Debug)]
pub struct MonitoringSystem {
    metrics: HashMap<String, ToolMetrics>,
    health_history: HashMap<String, Vec<HealthCheckRecord>>,
    alerts: Vec<Alert>,
    monitoring_enabled: bool,
    health_check_interval: Duration,
    last_health_check: Option<chrono::DateTime<chrono::Utc>>,
    performance_thresholds: PerformanceThresholds,
}

/// Metrics collected for each tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetrics {
    pub tool_id: String,
    pub operations_count: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub total_execution_time_ms: u64,
    pub average_response_time_ms: f64,
    pub last_operation_time: Option<chrono::DateTime<chrono::Utc>>,
    pub peak_response_time_ms: u64,
    pub error_rate: f64,
    pub uptime_percentage: f64,
}

/// Historical health check record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckRecord {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub status: HealthStatus,
    pub response_time_ms: Option<u64>,
    pub message: Option<String>,
    pub error_details: Option<String>,
}

/// Alert generated by the monitoring system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub tool_id: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub resolved: bool,
    pub resolved_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Types of alerts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertType {
    ToolDown,
    HighLatency,
    HighErrorRate,
    HealthCheckFailed,
    PerformanceDegraded,
    ToolUnresponsive,
    ThresholdExceeded,
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Performance thresholds for alerting
#[derive(Debug, Clone)]
pub struct PerformanceThresholds {
    pub max_response_time_ms: u64,
    pub max_error_rate: f64,
    pub min_uptime_percentage: f64,
    pub max_consecutive_failures: u32,
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            max_response_time_ms: 5000,
            max_error_rate: 0.1, // 10%
            min_uptime_percentage: 95.0,
            max_consecutive_failures: 3,
        }
    }
}

impl MonitoringSystem {
    pub fn new() -> Self {
        Self {
            metrics: HashMap::new(),
            health_history: HashMap::new(),
            alerts: Vec::new(),
            monitoring_enabled: true,
            health_check_interval: Duration::from_secs(30),
            last_health_check: None,
            performance_thresholds: PerformanceThresholds::default(),
        }
    }

    pub fn with_health_check_interval(mut self, interval: Duration) -> Self {
        self.health_check_interval = interval;
        self
    }

    pub fn with_performance_thresholds(mut self, thresholds: PerformanceThresholds) -> Self {
        self.performance_thresholds = thresholds;
        self
    }

    /// Record a tool operation for metrics collection
    pub fn record_operation(&mut self, tool_id: &str, _operation: &str, result: &ToolResult) {
        if !self.monitoring_enabled {
            return;
        }

        let metrics = self
            .metrics
            .entry(tool_id.to_string())
            .or_insert_with(|| ToolMetrics {
                tool_id: tool_id.to_string(),
                operations_count: 0,
                success_count: 0,
                failure_count: 0,
                total_execution_time_ms: 0,
                average_response_time_ms: 0.0,
                last_operation_time: None,
                peak_response_time_ms: 0,
                error_rate: 0.0,
                uptime_percentage: 100.0,
            });

        // Update metrics
        metrics.operations_count += 1;
        metrics.total_execution_time_ms += result.execution_time_ms;
        metrics.last_operation_time = Some(chrono::Utc::now());

        if result.success {
            metrics.success_count += 1;
        } else {
            metrics.failure_count += 1;
        }

        // Update derived metrics
        metrics.average_response_time_ms =
            metrics.total_execution_time_ms as f64 / metrics.operations_count as f64;
        metrics.peak_response_time_ms = metrics.peak_response_time_ms.max(result.execution_time_ms);
        metrics.error_rate = metrics.failure_count as f64 / metrics.operations_count as f64;
        metrics.uptime_percentage =
            (metrics.success_count as f64 / metrics.operations_count as f64) * 100.0;

        // Check for alerts - need to clone metrics to avoid borrow checker issues
        let metrics_clone = metrics.clone();
        self.check_performance_alerts(tool_id, &metrics_clone);
    }

    /// Record a health check result
    pub fn record_health_check(&mut self, tool_id: &str, health_check: &HealthCheck) {
        if !self.monitoring_enabled {
            return;
        }

        let record = HealthCheckRecord {
            timestamp: health_check.last_checked,
            status: health_check.status.clone(),
            response_time_ms: health_check.response_time_ms,
            message: health_check.message.clone(),
            error_details: None,
        };

        self.health_history
            .entry(tool_id.to_string())
            .or_insert_with(Vec::new)
            .push(record);

        // Keep only last 100 records per tool
        if let Some(history) = self.health_history.get_mut(tool_id) {
            if history.len() > 100 {
                history.drain(0..history.len() - 100);
            }
        }

        // Check for health alerts
        self.check_health_alerts(tool_id, health_check);
    }

    /// Check for performance-related alerts
    fn check_performance_alerts(&mut self, tool_id: &str, metrics: &ToolMetrics) {
        // High latency alert
        if metrics.average_response_time_ms
            > self.performance_thresholds.max_response_time_ms as f64
        {
            self.create_alert(
                AlertType::HighLatency,
                AlertSeverity::High,
                tool_id,
                format!(
                    "Average response time {:.2}ms exceeds threshold {}ms",
                    metrics.average_response_time_ms,
                    self.performance_thresholds.max_response_time_ms
                ),
            );
        }

        // High error rate alert
        if metrics.error_rate > self.performance_thresholds.max_error_rate {
            self.create_alert(
                AlertType::HighErrorRate,
                AlertSeverity::High,
                tool_id,
                format!(
                    "Error rate {:.2}% exceeds threshold {:.2}%",
                    metrics.error_rate * 100.0,
                    self.performance_thresholds.max_error_rate * 100.0
                ),
            );
        }

        // Low uptime alert
        if metrics.uptime_percentage < self.performance_thresholds.min_uptime_percentage {
            self.create_alert(
                AlertType::PerformanceDegraded,
                AlertSeverity::Medium,
                tool_id,
                format!(
                    "Uptime {:.2}% below threshold {:.2}%",
                    metrics.uptime_percentage, self.performance_thresholds.min_uptime_percentage
                ),
            );
        }
    }

    /// Check for health-related alerts
    fn check_health_alerts(&mut self, tool_id: &str, health_check: &HealthCheck) {
        match health_check.status {
            HealthStatus::Unhealthy => {
                self.create_alert(
                    AlertType::ToolDown,
                    AlertSeverity::Critical,
                    tool_id,
                    format!(
                        "Tool is unhealthy: {}",
                        health_check.message.as_deref().unwrap_or("Unknown error")
                    ),
                );
            }
            HealthStatus::Degraded => {
                self.create_alert(
                    AlertType::PerformanceDegraded,
                    AlertSeverity::Medium,
                    tool_id,
                    format!(
                        "Tool performance degraded: {}",
                        health_check
                            .message
                            .as_deref()
                            .unwrap_or("Performance issues")
                    ),
                );
            }
            _ => {
                // Resolve any existing alerts for this tool
                self.resolve_alerts_for_tool(tool_id);
            }
        }
    }

    /// Create a new alert
    fn create_alert(
        &mut self,
        alert_type: AlertType,
        severity: AlertSeverity,
        tool_id: &str,
        message: String,
    ) {
        // Check if similar alert already exists
        if self
            .alerts
            .iter()
            .any(|a| a.tool_id == tool_id && a.alert_type == alert_type && !a.resolved)
        {
            return;
        }

        let alert = Alert {
            id: Uuid::new_v4().to_string(),
            alert_type,
            severity,
            tool_id: tool_id.to_string(),
            message,
            timestamp: chrono::Utc::now(),
            resolved: false,
            resolved_at: None,
        };

        self.alerts.push(alert);
    }

    /// Resolve alerts for a tool
    fn resolve_alerts_for_tool(&mut self, tool_id: &str) {
        let now = chrono::Utc::now();
        for alert in self.alerts.iter_mut() {
            if alert.tool_id == tool_id && !alert.resolved {
                alert.resolved = true;
                alert.resolved_at = Some(now);
            }
        }
    }

    /// Get metrics for a specific tool
    pub fn get_tool_metrics(&self, tool_id: &str) -> Option<&ToolMetrics> {
        self.metrics.get(tool_id)
    }

    /// Get health history for a specific tool
    pub fn get_health_history(&self, tool_id: &str) -> Option<&Vec<HealthCheckRecord>> {
        self.health_history.get(tool_id)
    }

    /// Get all active alerts
    pub fn get_active_alerts(&self) -> Vec<&Alert> {
        self.alerts.iter().filter(|a| !a.resolved).collect()
    }

    /// Get alerts by severity
    pub fn get_alerts_by_severity(&self, severity: AlertSeverity) -> Vec<&Alert> {
        self.alerts
            .iter()
            .filter(|a| a.severity == severity && !a.resolved)
            .collect()
    }

    /// Get system-wide monitoring statistics
    pub fn get_monitoring_stats(&self) -> MonitoringStats {
        let total_operations: u64 = self.metrics.values().map(|m| m.operations_count).sum();
        let total_failures: u64 = self.metrics.values().map(|m| m.failure_count).sum();
        let average_response_time = if !self.metrics.is_empty() {
            self.metrics
                .values()
                .map(|m| m.average_response_time_ms)
                .sum::<f64>()
                / self.metrics.len() as f64
        } else {
            0.0
        };

        MonitoringStats {
            monitored_tools: self.metrics.len(),
            total_operations,
            total_failures,
            overall_error_rate: if total_operations > 0 {
                total_failures as f64 / total_operations as f64
            } else {
                0.0
            },
            average_response_time_ms: average_response_time,
            active_alerts: self.get_active_alerts().len(),
            critical_alerts: self.get_alerts_by_severity(AlertSeverity::Critical).len(),
            monitoring_enabled: self.monitoring_enabled,
            last_health_check: self.last_health_check,
        }
    }

    /// Enable or disable monitoring
    pub fn set_monitoring_enabled(&mut self, enabled: bool) {
        self.monitoring_enabled = enabled;
    }

    /// Clear old alerts
    pub fn clear_old_alerts(&mut self, older_than: chrono::DateTime<chrono::Utc>) {
        self.alerts.retain(|alert| alert.timestamp > older_than);
    }

    /// Reset metrics for a tool
    pub fn reset_tool_metrics(&mut self, tool_id: &str) {
        self.metrics.remove(tool_id);
    }

    /// Update health check timestamp
    pub fn update_health_check_timestamp(&mut self) {
        self.last_health_check = Some(chrono::Utc::now());
    }
}

/// Overall monitoring statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringStats {
    pub monitored_tools: usize,
    pub total_operations: u64,
    pub total_failures: u64,
    pub overall_error_rate: f64,
    pub average_response_time_ms: f64,
    pub active_alerts: usize,
    pub critical_alerts: usize,
    pub monitoring_enabled: bool,
    pub last_health_check: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for MonitoringSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Enhanced Tool Orchestrator with integrated monitoring
pub struct MonitoredToolOrchestrator {
    orchestrator: ToolOrchestrator,
    monitoring: MonitoringSystem,
}

impl MonitoredToolOrchestrator {
    pub fn new() -> Self {
        Self {
            orchestrator: ToolOrchestrator::new(),
            monitoring: MonitoringSystem::new(),
        }
    }

    pub fn with_monitoring_config(mut self, monitoring: MonitoringSystem) -> Self {
        self.monitoring = monitoring;
        self
    }

    /// Execute a coordinated operation with monitoring
    pub async fn execute_coordinated_monitored(
        &mut self,
        tool_type: &str,
        operation: &str,
        params: serde_json::Value,
    ) -> Result<ToolResult> {
        let result = self
            .orchestrator
            .execute_coordinated(tool_type, operation, params)
            .await?;

        // Record operation for monitoring
        if let Some(tool_id) = self.orchestrator.active_tools.get(tool_type) {
            self.monitoring
                .record_operation(tool_id, operation, &result);
        }

        Ok(result)
    }

    /// Perform health checks on all tools with monitoring
    pub async fn health_check_all_monitored(&mut self) -> HashMap<String, HealthCheck> {
        let health_results = self.orchestrator.health_check_all().await;

        // Record health checks for monitoring
        for (tool_type, health_check) in &health_results {
            if let Some(tool_id) = self.orchestrator.active_tools.get(tool_type) {
                self.monitoring.record_health_check(tool_id, health_check);
            }
        }

        self.monitoring.update_health_check_timestamp();
        health_results
    }

    /// Get comprehensive system status
    pub fn get_system_status(&self) -> SystemStatus {
        let orchestrator_stats = self.orchestrator.get_stats();
        let monitoring_stats = self.monitoring.get_monitoring_stats();
        let active_alerts = self.monitoring.get_active_alerts();
        let active_alerts_cloned: Vec<Alert> = active_alerts.iter().cloned().cloned().collect();
        let system_health = self.determine_system_health(&monitoring_stats, &active_alerts);

        SystemStatus {
            orchestrator_stats,
            monitoring_stats,
            active_alerts: active_alerts_cloned,
            system_health,
        }
    }

    /// Determine overall system health
    fn determine_system_health(
        &self,
        monitoring_stats: &MonitoringStats,
        active_alerts: &[&Alert],
    ) -> SystemHealth {
        if active_alerts
            .iter()
            .any(|a| a.severity == AlertSeverity::Critical)
        {
            SystemHealth::Critical
        } else if active_alerts
            .iter()
            .any(|a| a.severity == AlertSeverity::High)
        {
            SystemHealth::Degraded
        } else if monitoring_stats.overall_error_rate > 0.05 {
            SystemHealth::Warning
        } else {
            SystemHealth::Healthy
        }
    }

    /// Get access to the orchestrator
    pub fn orchestrator(&self) -> &ToolOrchestrator {
        &self.orchestrator
    }

    /// Get mutable access to the orchestrator
    pub fn orchestrator_mut(&mut self) -> &mut ToolOrchestrator {
        &mut self.orchestrator
    }

    /// Get access to the monitoring system
    pub fn monitoring(&self) -> &MonitoringSystem {
        &self.monitoring
    }

    /// Get mutable access to the monitoring system
    pub fn monitoring_mut(&mut self) -> &mut MonitoringSystem {
        &mut self.monitoring
    }
}

/// Comprehensive system status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub orchestrator_stats: OrchestratorStats,
    pub monitoring_stats: MonitoringStats,
    pub active_alerts: Vec<Alert>,
    pub system_health: SystemHealth,
}

/// Overall system health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SystemHealth {
    Healthy,
    Warning,
    Degraded,
    Critical,
}

impl Default for MonitoredToolOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

/// Common data structures for inter-tool communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPacket {
    pub id: String,
    pub source_tool: String,
    pub target_tool: Option<String>, // None for broadcast
    pub data_type: DataType,
    pub payload: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, String>,
}

/// Types of data that can flow between tools
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DataType {
    // Obsidian-specific
    MarkdownNote {
        path: String,
    },
    VaultIndex,
    SearchResults,

    // Calendar-specific
    CalendarEvent {
        event_id: String,
    },
    TimeSlot {
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    },

    // Jira-specific
    JiraIssue {
        issue_key: String,
    },
    ProjectInfo {
        project_key: String,
    },

    // MCP-specific
    MCPRequest {
        method: String,
    },
    MCPResponse {
        request_id: String,
    },

    // Generic types
    TextContent,
    JsonData,
    ErrorMessage,
    StatusUpdate,
}

impl DataPacket {
    pub fn new(source_tool: String, data_type: DataType, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            source_tool,
            target_tool: None,
            data_type,
            payload,
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_target(mut self, target_tool: String) -> Self {
        self.target_tool = Some(target_tool);
        self
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Data transformation pipeline for converting between tool-specific formats
pub struct DataTransformer {
    transformers: HashMap<
        (DataType, DataType),
        Box<dyn Fn(serde_json::Value) -> Result<serde_json::Value> + Send + Sync>,
    >,
}

impl std::fmt::Debug for DataTransformer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataTransformer")
            .field(
                "transformers",
                &format!("{} transformers", self.transformers.len()),
            )
            .finish()
    }
}

impl DataTransformer {
    pub fn new() -> Self {
        let mut transformer = Self {
            transformers: HashMap::new(),
        };

        // Register built-in transformations
        transformer.register_builtin_transformers();
        transformer
    }

    pub fn register_transformer<F>(&mut self, from: DataType, to: DataType, transform_fn: F)
    where
        F: Fn(serde_json::Value) -> Result<serde_json::Value> + Send + Sync + 'static,
    {
        self.transformers.insert((from, to), Box::new(transform_fn));
    }

    pub fn transform(&self, data: &DataPacket, target_type: DataType) -> Result<DataPacket> {
        if data.data_type == target_type {
            return Ok(data.clone());
        }

        let transformer = self
            .transformers
            .get(&(data.data_type.clone(), target_type.clone()))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No transformer found for {:?} -> {:?}",
                    data.data_type,
                    target_type
                )
            })?;

        let transformed_payload = transformer(data.payload.clone())?;

        Ok(DataPacket {
            id: Uuid::new_v4().to_string(),
            source_tool: data.source_tool.clone(),
            target_tool: data.target_tool.clone(),
            data_type: target_type,
            payload: transformed_payload,
            timestamp: chrono::Utc::now(),
            metadata: data.metadata.clone(),
        })
    }

    fn register_builtin_transformers(&mut self) {
        // Text content to JSON data transformation
        self.register_transformer(DataType::TextContent, DataType::JsonData, |payload| {
            if let Some(text) = payload.as_str() {
                Ok(serde_json::json!({ "text": text, "type": "text_content" }))
            } else {
                Ok(serde_json::json!({ "data": payload, "type": "generic" }))
            }
        });

        // JSON data to text content transformation
        self.register_transformer(DataType::JsonData, DataType::TextContent, |payload| {
            if let Some(text) = payload.get("text").and_then(|v| v.as_str()) {
                Ok(serde_json::Value::String(text.to_string()))
            } else {
                Ok(serde_json::Value::String(payload.to_string()))
            }
        });

        // Markdown note to text content transformation
        self.register_transformer(
            DataType::MarkdownNote {
                path: String::new(),
            },
            DataType::TextContent,
            |payload| {
                if let Some(content) = payload.get("content").and_then(|v| v.as_str()) {
                    Ok(serde_json::Value::String(content.to_string()))
                } else {
                    Ok(serde_json::Value::String("".to_string()))
                }
            },
        );
    }
}

impl Default for DataTransformer {
    fn default() -> Self {
        Self::new()
    }
}

/// Data flow system for managing inter-tool communication
#[derive(Debug)]
pub struct DataFlowSystem {
    channels: HashMap<String, mpsc::UnboundedSender<DataPacket>>,
    transformer: DataTransformer,
    buffer_size: usize,
    validation_enabled: bool,
}

impl DataFlowSystem {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            transformer: DataTransformer::new(),
            buffer_size: 1000,
            validation_enabled: true,
        }
    }

    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    pub fn with_validation(mut self, enabled: bool) -> Self {
        self.validation_enabled = enabled;
        self
    }

    /// Register a tool to receive data packets
    pub fn register_tool(&mut self, tool_id: String) -> mpsc::UnboundedReceiver<DataPacket> {
        let (sender, receiver) = mpsc::unbounded_channel();
        self.channels.insert(tool_id, sender);
        receiver
    }

    /// Unregister a tool from the data flow system
    pub fn unregister_tool(&mut self, tool_id: &str) {
        self.channels.remove(tool_id);
    }

    /// Send a data packet to a specific tool
    pub async fn send_to_tool(&self, tool_id: &str, packet: DataPacket) -> Result<()> {
        if self.validation_enabled {
            self.validate_packet(&packet)?;
        }

        let sender = self
            .channels
            .get(tool_id)
            .ok_or_else(|| anyhow::anyhow!("Tool {} not registered", tool_id))?;

        sender
            .send(packet)
            .map_err(|e| anyhow::anyhow!("Failed to send packet to {}: {}", tool_id, e))?;

        Ok(())
    }

    /// Broadcast a data packet to all registered tools
    pub async fn broadcast(&self, packet: DataPacket) -> Result<()> {
        if self.validation_enabled {
            self.validate_packet(&packet)?;
        }

        for (tool_id, sender) in &self.channels {
            if tool_id != &packet.source_tool {
                if let Err(e) = sender.send(packet.clone()) {
                    eprintln!("Failed to send packet to {}: {}", tool_id, e);
                }
            }
        }

        Ok(())
    }

    /// Send a data packet with automatic transformation
    pub async fn send_with_transformation(
        &self,
        tool_id: &str,
        packet: DataPacket,
        target_type: DataType,
    ) -> Result<()> {
        let transformed_packet = self.transformer.transform(&packet, target_type)?;
        self.send_to_tool(tool_id, transformed_packet).await
    }

    /// Get a reference to the data transformer
    pub fn transformer(&self) -> &DataTransformer {
        &self.transformer
    }

    /// Get a mutable reference to the data transformer
    pub fn transformer_mut(&mut self) -> &mut DataTransformer {
        &mut self.transformer
    }

    /// Validate a data packet
    fn validate_packet(&self, packet: &DataPacket) -> Result<()> {
        if packet.id.is_empty() {
            return Err(anyhow::anyhow!("Packet ID cannot be empty"));
        }

        if packet.source_tool.is_empty() {
            return Err(anyhow::anyhow!("Source tool cannot be empty"));
        }

        // Additional validation based on data type
        match &packet.data_type {
            DataType::MarkdownNote { path } => {
                if path.is_empty() {
                    return Err(anyhow::anyhow!("Markdown note path cannot be empty"));
                }
            }
            DataType::CalendarEvent { event_id } => {
                if event_id.is_empty() {
                    return Err(anyhow::anyhow!("Calendar event ID cannot be empty"));
                }
            }
            DataType::JiraIssue { issue_key } => {
                if issue_key.is_empty() {
                    return Err(anyhow::anyhow!("Jira issue key cannot be empty"));
                }
            }
            _ => {} // Other types don't require specific validation
        }

        Ok(())
    }

    /// Get statistics about the data flow system
    pub fn get_stats(&self) -> DataFlowStats {
        DataFlowStats {
            registered_tools: self.channels.len(),
            buffer_size: self.buffer_size,
            validation_enabled: self.validation_enabled,
            transformer_count: self.transformer.transformers.len(),
        }
    }
}

impl Default for DataFlowSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the data flow system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFlowStats {
    pub registered_tools: usize,
    pub buffer_size: usize,
    pub validation_enabled: bool,
    pub transformer_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Debug)]
    struct MockTool {
        id: String,
        metadata: ToolMetadata,
        should_fail: bool,
    }

    impl MockTool {
        fn new(id: String, name: String, should_fail: bool) -> Self {
            let metadata = ToolMetadata::new(
                name,
                "1.0.0".to_string(),
                "Mock tool for testing".to_string(),
                vec!["test".to_string(), "mock".to_string()],
            );

            Self {
                id,
                metadata,
                should_fail,
            }
        }
    }

    #[async_trait]
    impl Tool for MockTool {
        async fn execute(&self, operation: &str, _params: serde_json::Value) -> Result<ToolResult> {
            let start = std::time::Instant::now();

            if self.should_fail {
                return Ok(ToolResult::error(
                    format!("Mock tool {} failed operation {}", self.id, operation),
                    start.elapsed().as_millis() as u64,
                ));
            }

            Ok(ToolResult::success(
                Some(json!({"operation": operation, "tool_id": self.id})),
                start.elapsed().as_millis() as u64,
            ))
        }

        async fn status(&self) -> Result<serde_json::Value> {
            Ok(json!({"status": "active", "tool_id": self.id}))
        }

        async fn health_check(&self) -> HealthCheck {
            if self.should_fail {
                HealthCheck::unhealthy("Mock tool configured to fail".to_string())
            } else {
                HealthCheck::healthy(Some("Mock tool is healthy".to_string()), Some(10))
            }
        }

        fn metadata(&self) -> &ToolMetadata {
            &self.metadata
        }

        fn id(&self) -> &str {
            &self.id
        }
    }

    #[tokio::test]
    async fn test_tool_registry_register_and_get() {
        let mut registry = ToolRegistry::new();
        let tool = Box::new(MockTool::new(
            "test-1".to_string(),
            "Test Tool".to_string(),
            false,
        ));

        let tool_id = registry.register_tool(tool).await.unwrap();
        assert_eq!(tool_id, "test-1");
        assert!(registry.is_registered("test-1"));
        assert_eq!(registry.tool_count(), 1);

        let retrieved_tool = registry.get_tool("test-1").unwrap();
        assert_eq!(retrieved_tool.id(), "test-1");
    }

    #[tokio::test]
    async fn test_tool_registry_execute_with_timeout() {
        let mut registry = ToolRegistry::new();
        let tool = Box::new(MockTool::new(
            "test-1".to_string(),
            "Test Tool".to_string(),
            false,
        ));

        registry.register_tool(tool).await.unwrap();

        let result = registry
            .execute_with_timeout("test-1", "test_operation", json!({"param": "value"}))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.data.is_some());
    }

    #[tokio::test]
    async fn test_tool_registry_discover_by_capability() {
        let mut registry = ToolRegistry::new();
        let tool1 = Box::new(MockTool::new(
            "test-1".to_string(),
            "Test Tool 1".to_string(),
            false,
        ));
        let tool2 = Box::new(MockTool::new(
            "test-2".to_string(),
            "Test Tool 2".to_string(),
            false,
        ));

        registry.register_tool(tool1).await.unwrap();
        registry.register_tool(tool2).await.unwrap();

        let tools_with_test = registry.discover_by_capability("test");
        assert_eq!(tools_with_test.len(), 2);
        assert!(tools_with_test.contains(&"test-1"));
        assert!(tools_with_test.contains(&"test-2"));

        let tools_with_nonexistent = registry.discover_by_capability("nonexistent");
        assert_eq!(tools_with_nonexistent.len(), 0);
    }

    #[tokio::test]
    async fn test_tool_registry_health_check_all() {
        let mut registry = ToolRegistry::new();
        let healthy_tool = Box::new(MockTool::new(
            "healthy".to_string(),
            "Healthy Tool".to_string(),
            false,
        ));
        let unhealthy_tool = Box::new(MockTool::new(
            "unhealthy".to_string(),
            "Unhealthy Tool".to_string(),
            true,
        ));

        registry.register_tool(healthy_tool).await.unwrap();
        registry.register_tool(unhealthy_tool).await.unwrap();

        let health_results = registry.health_check_all().await;
        assert_eq!(health_results.len(), 2);

        assert_eq!(
            health_results.get("healthy").unwrap().status,
            HealthStatus::Healthy
        );
        assert_eq!(
            health_results.get("unhealthy").unwrap().status,
            HealthStatus::Unhealthy
        );
    }

    #[tokio::test]
    async fn test_tool_registry_unregister() {
        let mut registry = ToolRegistry::new();
        let tool = Box::new(MockTool::new(
            "test-1".to_string(),
            "Test Tool".to_string(),
            false,
        ));

        registry.register_tool(tool).await.unwrap();
        assert!(registry.is_registered("test-1"));

        registry.unregister_tool("test-1").await.unwrap();
        assert!(!registry.is_registered("test-1"));
        assert_eq!(registry.tool_count(), 0);
    }

    #[tokio::test]
    async fn test_tool_orchestrator_register_and_get() {
        let mut orchestrator = ToolOrchestrator::new();
        let tool = Box::new(MockTool::new(
            "test-1".to_string(),
            "Test Tool".to_string(),
            false,
        ));

        let tool_id = orchestrator
            .register_tool(tool, "test_type".to_string())
            .await
            .unwrap();
        assert_eq!(tool_id, "test-1");

        let retrieved_tool = orchestrator.get_tool_by_type("test_type").unwrap();
        assert_eq!(retrieved_tool.id(), "test-1");
    }

    #[tokio::test]
    async fn test_tool_orchestrator_execute_coordinated() {
        let mut orchestrator = ToolOrchestrator::new();
        let tool = Box::new(MockTool::new(
            "test-1".to_string(),
            "Test Tool".to_string(),
            false,
        ));

        orchestrator
            .register_tool(tool, "test_type".to_string())
            .await
            .unwrap();

        let result = orchestrator
            .execute_coordinated(
                "test_type",
                "test_operation",
                serde_json::json!({"param": "value"}),
            )
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.data.is_some());
    }

    #[tokio::test]
    async fn test_tool_orchestrator_execute_parallel() {
        let mut orchestrator = ToolOrchestrator::new();
        let tool1 = Box::new(MockTool::new(
            "test-1".to_string(),
            "Test Tool 1".to_string(),
            false,
        ));
        let tool2 = Box::new(MockTool::new(
            "test-2".to_string(),
            "Test Tool 2".to_string(),
            false,
        ));

        orchestrator
            .register_tool(tool1, "type1".to_string())
            .await
            .unwrap();
        orchestrator
            .register_tool(tool2, "type2".to_string())
            .await
            .unwrap();

        let operations = vec![
            (
                "type1".to_string(),
                "operation1".to_string(),
                serde_json::json!({"param": "value1"}),
            ),
            (
                "type2".to_string(),
                "operation2".to_string(),
                serde_json::json!({"param": "value2"}),
            ),
        ];

        let results = orchestrator.execute_parallel(operations).await.unwrap();

        assert_eq!(results.len(), 2);
        assert!(results[0].success);
        assert!(results[1].success);
    }

    #[tokio::test]
    async fn test_tool_orchestrator_health_check_all() {
        let mut orchestrator = ToolOrchestrator::new();
        let healthy_tool = Box::new(MockTool::new(
            "healthy".to_string(),
            "Healthy Tool".to_string(),
            false,
        ));
        let unhealthy_tool = Box::new(MockTool::new(
            "unhealthy".to_string(),
            "Unhealthy Tool".to_string(),
            true,
        ));

        orchestrator
            .register_tool(healthy_tool, "healthy_type".to_string())
            .await
            .unwrap();
        orchestrator
            .register_tool(unhealthy_tool, "unhealthy_type".to_string())
            .await
            .unwrap();

        let health_results = orchestrator.health_check_all().await;
        assert_eq!(health_results.len(), 2);

        assert_eq!(
            health_results.get("healthy_type").unwrap().status,
            HealthStatus::Healthy
        );
        assert_eq!(
            health_results.get("unhealthy_type").unwrap().status,
            HealthStatus::Unhealthy
        );
    }

    #[tokio::test]
    async fn test_tool_orchestrator_lifecycle() {
        let mut orchestrator = ToolOrchestrator::new();
        let tool = Box::new(MockTool::new(
            "test-1".to_string(),
            "Test Tool".to_string(),
            false,
        ));

        orchestrator
            .register_tool(tool, "test_type".to_string())
            .await
            .unwrap();

        let stats = orchestrator.get_stats();
        assert_eq!(stats.total_tools, 1);
        assert_eq!(stats.active_tools, 1);

        orchestrator.initialize_all().await.unwrap();
        orchestrator.shutdown_all().await.unwrap();

        let stats_after_shutdown = orchestrator.get_stats();
        assert_eq!(stats_after_shutdown.active_tools, 0);
    }

    #[tokio::test]
    async fn test_coordination_state() {
        let mut state = CoordinationState::new();

        let operation = PendingOperation {
            id: "op1".to_string(),
            tool_id: "tool1".to_string(),
            operation: "test".to_string(),
            params: serde_json::json!({}),
            priority: 100,
            scheduled_at: chrono::Utc::now(),
            timeout_ms: 5000,
        };

        state.add_pending_operation(operation);
        assert_eq!(state.pending_operations.len(), 1);

        let next_op = state.next_pending_operation().unwrap();
        assert_eq!(next_op.id, "op1");
        assert_eq!(state.pending_operations.len(), 0);
    }

    #[tokio::test]
    async fn test_data_packet_creation() {
        let packet = DataPacket::new(
            "obsidian".to_string(),
            DataType::TextContent,
            serde_json::json!("Hello, world!"),
        );

        assert_eq!(packet.source_tool, "obsidian");
        assert_eq!(packet.data_type, DataType::TextContent);
        assert_eq!(packet.payload, serde_json::json!("Hello, world!"));
        assert!(packet.target_tool.is_none());
        assert!(packet.metadata.is_empty());
    }

    #[tokio::test]
    async fn test_data_packet_with_target_and_metadata() {
        let packet = DataPacket::new(
            "obsidian".to_string(),
            DataType::TextContent,
            serde_json::json!("Hello, world!"),
        )
        .with_target("jira".to_string())
        .with_metadata("priority".to_string(), "high".to_string());

        assert_eq!(packet.target_tool, Some("jira".to_string()));
        assert_eq!(packet.metadata.get("priority"), Some(&"high".to_string()));
    }

    #[tokio::test]
    async fn test_data_transformer_builtin() {
        let transformer = DataTransformer::new();

        let packet = DataPacket::new(
            "test".to_string(),
            DataType::TextContent,
            serde_json::json!("Hello, world!"),
        );

        let transformed = transformer.transform(&packet, DataType::JsonData).unwrap();
        assert_eq!(transformed.data_type, DataType::JsonData);
        assert_eq!(transformed.payload["text"], "Hello, world!");
        assert_eq!(transformed.payload["type"], "text_content");
    }

    #[tokio::test]
    async fn test_data_transformer_custom() {
        let mut transformer = DataTransformer::new();

        transformer.register_transformer(
            DataType::TextContent,
            DataType::ErrorMessage,
            |payload| {
                if let Some(text) = payload.as_str() {
                    Ok(serde_json::json!({ "error": text, "code": 400 }))
                } else {
                    Ok(serde_json::json!({ "error": "Invalid payload", "code": 500 }))
                }
            },
        );

        let packet = DataPacket::new(
            "test".to_string(),
            DataType::TextContent,
            serde_json::json!("Something went wrong"),
        );

        let transformed = transformer
            .transform(&packet, DataType::ErrorMessage)
            .unwrap();
        assert_eq!(transformed.data_type, DataType::ErrorMessage);
        assert_eq!(transformed.payload["error"], "Something went wrong");
        assert_eq!(transformed.payload["code"], 400);
    }

    #[tokio::test]
    async fn test_data_flow_system_registration() {
        let mut flow_system = DataFlowSystem::new();

        let _receiver1 = flow_system.register_tool("tool1".to_string());
        let _receiver2 = flow_system.register_tool("tool2".to_string());

        let stats = flow_system.get_stats();
        assert_eq!(stats.registered_tools, 2);
        assert_eq!(stats.buffer_size, 1000);
        assert!(stats.validation_enabled);
        assert!(stats.transformer_count > 0);
    }

    #[tokio::test]
    async fn test_data_flow_system_send_to_tool() {
        let mut flow_system = DataFlowSystem::new();
        let mut receiver = flow_system.register_tool("tool1".to_string());

        let packet = DataPacket::new(
            "source".to_string(),
            DataType::TextContent,
            serde_json::json!("Test message"),
        );

        flow_system
            .send_to_tool("tool1", packet.clone())
            .await
            .unwrap();

        let received = receiver.recv().await.unwrap();
        assert_eq!(received.source_tool, "source");
        assert_eq!(received.data_type, DataType::TextContent);
        assert_eq!(received.payload, serde_json::json!("Test message"));
    }

    #[tokio::test]
    async fn test_data_flow_system_broadcast() {
        let mut flow_system = DataFlowSystem::new();
        let mut receiver1 = flow_system.register_tool("tool1".to_string());
        let mut receiver2 = flow_system.register_tool("tool2".to_string());

        let packet = DataPacket::new(
            "source".to_string(),
            DataType::TextContent,
            serde_json::json!("Broadcast message"),
        );

        flow_system.broadcast(packet.clone()).await.unwrap();

        let received1 = receiver1.recv().await.unwrap();
        let received2 = receiver2.recv().await.unwrap();

        assert_eq!(received1.payload, serde_json::json!("Broadcast message"));
        assert_eq!(received2.payload, serde_json::json!("Broadcast message"));
    }

    #[tokio::test]
    async fn test_data_flow_system_with_transformation() {
        let mut flow_system = DataFlowSystem::new();
        let mut receiver = flow_system.register_tool("tool1".to_string());

        let packet = DataPacket::new(
            "source".to_string(),
            DataType::TextContent,
            serde_json::json!("Transform me"),
        );

        flow_system
            .send_with_transformation("tool1", packet, DataType::JsonData)
            .await
            .unwrap();

        let received = receiver.recv().await.unwrap();
        assert_eq!(received.data_type, DataType::JsonData);
        assert_eq!(received.payload["text"], "Transform me");
        assert_eq!(received.payload["type"], "text_content");
    }

    #[tokio::test]
    async fn test_data_flow_system_validation() {
        let flow_system = DataFlowSystem::new();

        let invalid_packet = DataPacket {
            id: "".to_string(), // Empty ID should fail validation
            source_tool: "source".to_string(),
            target_tool: None,
            data_type: DataType::TextContent,
            payload: serde_json::json!("Test"),
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        let result = flow_system.validate_packet(&invalid_packet);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Packet ID cannot be empty"));
    }

    #[tokio::test]
    async fn test_orchestrator_data_flow_integration() {
        let mut orchestrator = ToolOrchestrator::new();

        // Register a tool in the data flow system
        let mut receiver = orchestrator.register_data_flow("tool1".to_string());

        // Create a data packet
        let packet = DataPacket::new(
            "source".to_string(),
            DataType::TextContent,
            serde_json::json!("Integration test"),
        );

        // Send data through the orchestrator
        orchestrator
            .send_data("tool1", packet.clone())
            .await
            .unwrap();

        // Verify the packet was received
        let received = receiver.recv().await.unwrap();
        assert_eq!(received.source_tool, "source");
        assert_eq!(received.payload, serde_json::json!("Integration test"));

        // Check updated stats
        let stats = orchestrator.get_stats();
        assert_eq!(stats.data_flow_stats.registered_tools, 1);
    }

    #[tokio::test]
    async fn test_orchestrator_broadcast_data() {
        let mut orchestrator = ToolOrchestrator::new();

        // Register multiple tools
        let mut receiver1 = orchestrator.register_data_flow("tool1".to_string());
        let mut receiver2 = orchestrator.register_data_flow("tool2".to_string());

        // Create and broadcast a packet
        let packet = DataPacket::new(
            "broadcaster".to_string(),
            DataType::StatusUpdate,
            serde_json::json!("System update"),
        );

        orchestrator.broadcast_data(packet.clone()).await.unwrap();

        // Verify both tools received the broadcast
        let received1 = receiver1.recv().await.unwrap();
        let received2 = receiver2.recv().await.unwrap();

        assert_eq!(received1.payload, serde_json::json!("System update"));
        assert_eq!(received2.payload, serde_json::json!("System update"));
    }

    #[tokio::test]
    async fn test_orchestrator_data_transformation() {
        let mut orchestrator = ToolOrchestrator::new();

        // Register a tool
        let mut receiver = orchestrator.register_data_flow("tool1".to_string());

        // Create a text content packet
        let packet = DataPacket::new(
            "source".to_string(),
            DataType::TextContent,
            serde_json::json!("Transform this text"),
        );

        // Send with transformation to JSON
        orchestrator
            .send_data_with_transformation("tool1", packet, DataType::JsonData)
            .await
            .unwrap();

        // Verify the packet was transformed
        let received = receiver.recv().await.unwrap();
        assert_eq!(received.data_type, DataType::JsonData);
        assert_eq!(received.payload["text"], "Transform this text");
        assert_eq!(received.payload["type"], "text_content");
    }

    #[tokio::test]
    async fn test_workflow_engine_simple_workflow() {
        let mut orchestrator = ToolOrchestrator::new();
        let tool = Box::new(MockTool::new(
            "test-tool".to_string(),
            "Test Tool".to_string(),
            false,
        ));

        orchestrator
            .register_tool(tool, "test_type".to_string())
            .await
            .unwrap();

        let mut engine = WorkflowEngine::new(orchestrator);

        // Create a simple workflow
        let workflow_def = WorkflowDefinition {
            id: "test-workflow".to_string(),
            name: "Test Workflow".to_string(),
            description: "A simple test workflow".to_string(),
            steps: vec![WorkflowStepDefinition {
                id: "step1".to_string(),
                name: "First Step".to_string(),
                tool_type: "test_type".to_string(),
                operation: "test_operation".to_string(),
                params: serde_json::json!({"param": "value"}),
                dependencies: vec![],
                timeout_ms: None,
                retry_attempts: None,
                condition: None,
                rollback_operation: None,
            }],
            timeout_ms: 10000,
            retry_config: RetryConfig::default(),
            metadata: HashMap::new(),
        };

        let variables = HashMap::new();
        let result = engine
            .execute_workflow(workflow_def, variables)
            .await
            .unwrap();

        assert_eq!(result.status, WorkflowStatus::Completed);
        assert_eq!(result.steps.len(), 1);
        assert_eq!(result.steps[0].status, WorkflowStatus::Completed);
    }

    #[tokio::test]
    async fn test_workflow_engine_with_dependencies() {
        let mut orchestrator = ToolOrchestrator::new();
        let tool = Box::new(MockTool::new(
            "test-tool".to_string(),
            "Test Tool".to_string(),
            false,
        ));

        orchestrator
            .register_tool(tool, "test_type".to_string())
            .await
            .unwrap();

        let mut engine = WorkflowEngine::new(orchestrator);

        // Create a workflow with dependencies
        let workflow_def = WorkflowDefinition {
            id: "test-workflow".to_string(),
            name: "Test Workflow".to_string(),
            description: "A workflow with dependencies".to_string(),
            steps: vec![
                WorkflowStepDefinition {
                    id: "step1".to_string(),
                    name: "First Step".to_string(),
                    tool_type: "test_type".to_string(),
                    operation: "operation1".to_string(),
                    params: serde_json::json!({"param": "value1"}),
                    dependencies: vec![],
                    timeout_ms: None,
                    retry_attempts: None,
                    condition: None,
                    rollback_operation: None,
                },
                WorkflowStepDefinition {
                    id: "step2".to_string(),
                    name: "Second Step".to_string(),
                    tool_type: "test_type".to_string(),
                    operation: "operation2".to_string(),
                    params: serde_json::json!({"param": "value2"}),
                    dependencies: vec!["step1".to_string()],
                    timeout_ms: None,
                    retry_attempts: None,
                    condition: None,
                    rollback_operation: None,
                },
            ],
            timeout_ms: 10000,
            retry_config: RetryConfig::default(),
            metadata: HashMap::new(),
        };

        let variables = HashMap::new();
        let result = engine
            .execute_workflow(workflow_def, variables)
            .await
            .unwrap();

        assert_eq!(result.status, WorkflowStatus::Completed);
        assert_eq!(result.steps.len(), 2);
        assert_eq!(result.steps[0].status, WorkflowStatus::Completed);
        assert_eq!(result.steps[1].status, WorkflowStatus::Completed);
    }

    #[tokio::test]
    async fn test_workflow_engine_with_conditions() {
        let mut orchestrator = ToolOrchestrator::new();
        let tool = Box::new(MockTool::new(
            "test-tool".to_string(),
            "Test Tool".to_string(),
            false,
        ));

        orchestrator
            .register_tool(tool, "test_type".to_string())
            .await
            .unwrap();

        let mut engine = WorkflowEngine::new(orchestrator);

        // Create a workflow with conditions
        let workflow_def = WorkflowDefinition {
            id: "test-workflow".to_string(),
            name: "Test Workflow".to_string(),
            description: "A workflow with conditions".to_string(),
            steps: vec![
                WorkflowStepDefinition {
                    id: "step1".to_string(),
                    name: "First Step".to_string(),
                    tool_type: "test_type".to_string(),
                    operation: "operation1".to_string(),
                    params: serde_json::json!({"param": "value1"}),
                    dependencies: vec![],
                    timeout_ms: None,
                    retry_attempts: None,
                    condition: None,
                    rollback_operation: None,
                },
                WorkflowStepDefinition {
                    id: "step2".to_string(),
                    name: "Conditional Step".to_string(),
                    tool_type: "test_type".to_string(),
                    operation: "operation2".to_string(),
                    params: serde_json::json!({"param": "value2"}),
                    dependencies: vec!["step1".to_string()],
                    timeout_ms: None,
                    retry_attempts: None,
                    condition: Some(StepCondition {
                        condition_type: ConditionType::StepSucceeded,
                        target_step_id: "step1".to_string(),
                        expected_value: serde_json::json!(true),
                    }),
                    rollback_operation: None,
                },
            ],
            timeout_ms: 10000,
            retry_config: RetryConfig::default(),
            metadata: HashMap::new(),
        };

        let variables = HashMap::new();
        let result = engine
            .execute_workflow(workflow_def, variables)
            .await
            .unwrap();

        assert_eq!(result.status, WorkflowStatus::Completed);
        assert_eq!(result.steps.len(), 2);
        assert_eq!(result.steps[0].status, WorkflowStatus::Completed);
        assert_eq!(result.steps[1].status, WorkflowStatus::Completed);
    }

    #[tokio::test]
    async fn test_workflow_engine_variable_substitution() {
        let mut orchestrator = ToolOrchestrator::new();
        let tool = Box::new(MockTool::new(
            "test-tool".to_string(),
            "Test Tool".to_string(),
            false,
        ));

        orchestrator
            .register_tool(tool, "test_type".to_string())
            .await
            .unwrap();

        let mut engine = WorkflowEngine::new(orchestrator);

        // Create a workflow with variable substitution
        let workflow_def = WorkflowDefinition {
            id: "test-workflow".to_string(),
            name: "Test Workflow".to_string(),
            description: "A workflow with variable substitution".to_string(),
            steps: vec![WorkflowStepDefinition {
                id: "step1".to_string(),
                name: "First Step".to_string(),
                tool_type: "test_type".to_string(),
                operation: "operation1".to_string(),
                params: serde_json::json!({"param": "${input_value}"}),
                dependencies: vec![],
                timeout_ms: None,
                retry_attempts: None,
                condition: None,
                rollback_operation: None,
            }],
            timeout_ms: 10000,
            retry_config: RetryConfig::default(),
            metadata: HashMap::new(),
        };

        let mut variables = HashMap::new();
        variables.insert("input_value".to_string(), serde_json::json!("test_value"));

        let result = engine
            .execute_workflow(workflow_def, variables)
            .await
            .unwrap();

        assert_eq!(result.status, WorkflowStatus::Completed);
        assert_eq!(result.steps.len(), 1);
        assert_eq!(result.steps[0].status, WorkflowStatus::Completed);
    }

    #[tokio::test]
    async fn test_workflow_engine_retry_config() {
        let mut orchestrator = ToolOrchestrator::new();
        let tool = Box::new(MockTool::new(
            "test-tool".to_string(),
            "Test Tool".to_string(),
            true,
        )); // Failing tool

        orchestrator
            .register_tool(tool, "test_type".to_string())
            .await
            .unwrap();

        let retry_config = RetryConfig {
            max_attempts: 2,
            initial_delay_ms: 10,
            max_delay_ms: 100,
            exponential_base: 2.0,
        };

        let mut engine = WorkflowEngine::new(orchestrator).with_retry_config(retry_config);

        // Create a workflow that will fail
        let workflow_def = WorkflowDefinition {
            id: "test-workflow".to_string(),
            name: "Test Workflow".to_string(),
            description: "A failing workflow".to_string(),
            steps: vec![WorkflowStepDefinition {
                id: "step1".to_string(),
                name: "Failing Step".to_string(),
                tool_type: "test_type".to_string(),
                operation: "operation1".to_string(),
                params: serde_json::json!({"param": "value"}),
                dependencies: vec![],
                timeout_ms: None,
                retry_attempts: Some(2),
                condition: None,
                rollback_operation: None,
            }],
            timeout_ms: 10000,
            retry_config: RetryConfig::default(),
            metadata: HashMap::new(),
        };

        let variables = HashMap::new();
        let result = engine.execute_workflow(workflow_def, variables).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_monitoring_system_record_operation() {
        let mut monitoring = MonitoringSystem::new();

        let result = ToolResult::success(Some(serde_json::json!({"test": "data"})), 150);
        monitoring.record_operation("test-tool", "test_operation", &result);

        let metrics = monitoring.get_tool_metrics("test-tool").unwrap();
        assert_eq!(metrics.operations_count, 1);
        assert_eq!(metrics.success_count, 1);
        assert_eq!(metrics.failure_count, 0);
        assert_eq!(metrics.average_response_time_ms, 150.0);
        assert_eq!(metrics.error_rate, 0.0);
        assert_eq!(metrics.uptime_percentage, 100.0);
    }

    #[tokio::test]
    async fn test_monitoring_system_alert_generation() {
        let mut monitoring = MonitoringSystem::new();

        // Set low threshold for testing
        let thresholds = PerformanceThresholds {
            max_response_time_ms: 100,
            max_error_rate: 0.1,
            min_uptime_percentage: 95.0,
            max_consecutive_failures: 3,
        };
        monitoring = monitoring.with_performance_thresholds(thresholds);

        // Record a slow operation
        let slow_result = ToolResult::success(Some(serde_json::json!({})), 200);
        monitoring.record_operation("test-tool", "slow_operation", &slow_result);

        let alerts = monitoring.get_active_alerts();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].alert_type, AlertType::HighLatency);
        assert_eq!(alerts[0].severity, AlertSeverity::High);
    }

    #[tokio::test]
    async fn test_monitoring_system_health_check_recording() {
        let mut monitoring = MonitoringSystem::new();

        let health_check =
            HealthCheck::healthy(Some("All systems operational".to_string()), Some(50));
        monitoring.record_health_check("test-tool", &health_check);

        let history = monitoring.get_health_history("test-tool").unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].status, HealthStatus::Healthy);
        assert_eq!(history[0].response_time_ms, Some(50));
    }

    #[tokio::test]
    async fn test_monitoring_system_unhealthy_tool_alert() {
        let mut monitoring = MonitoringSystem::new();

        let unhealthy_check = HealthCheck::unhealthy("Service unavailable".to_string());
        monitoring.record_health_check("test-tool", &unhealthy_check);

        let alerts = monitoring.get_active_alerts();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].alert_type, AlertType::ToolDown);
        assert_eq!(alerts[0].severity, AlertSeverity::Critical);
    }

    #[tokio::test]
    async fn test_monitoring_system_alert_resolution() {
        let mut monitoring = MonitoringSystem::new();

        // Generate an alert
        let unhealthy_check = HealthCheck::unhealthy("Service down".to_string());
        monitoring.record_health_check("test-tool", &unhealthy_check);

        assert_eq!(monitoring.get_active_alerts().len(), 1);

        // Resolve the alert
        let healthy_check = HealthCheck::healthy(Some("Service restored".to_string()), Some(100));
        monitoring.record_health_check("test-tool", &healthy_check);

        assert_eq!(monitoring.get_active_alerts().len(), 0);
    }

    #[tokio::test]
    async fn test_monitoring_system_stats() {
        let mut monitoring = MonitoringSystem::new();

        // Record some operations
        let success_result = ToolResult::success(Some(serde_json::json!({})), 100);
        let failure_result = ToolResult::error("Test error".to_string(), 150);

        monitoring.record_operation("tool1", "op1", &success_result);
        monitoring.record_operation("tool1", "op2", &failure_result);
        monitoring.record_operation("tool2", "op3", &success_result);

        let stats = monitoring.get_monitoring_stats();
        assert_eq!(stats.monitored_tools, 2);
        assert_eq!(stats.total_operations, 3);
        assert_eq!(stats.total_failures, 1);
        assert_eq!(stats.overall_error_rate, 1.0 / 3.0);
        assert!(stats.monitoring_enabled);
    }

    #[tokio::test]
    async fn test_monitored_tool_orchestrator() {
        let mut monitored_orchestrator = MonitoredToolOrchestrator::new();
        let tool = Box::new(MockTool::new(
            "test-tool".to_string(),
            "Test Tool".to_string(),
            false,
        ));

        monitored_orchestrator
            .orchestrator_mut()
            .register_tool(tool, "test_type".to_string())
            .await
            .unwrap();

        // Execute operation with monitoring
        let result = monitored_orchestrator
            .execute_coordinated_monitored(
                "test_type",
                "test_operation",
                serde_json::json!({"param": "value"}),
            )
            .await
            .unwrap();

        assert!(result.success);

        // Check that metrics were recorded
        let system_status = monitored_orchestrator.get_system_status();
        assert_eq!(system_status.monitoring_stats.total_operations, 1);
        assert_eq!(system_status.system_health, SystemHealth::Healthy);
    }

    #[tokio::test]
    async fn test_monitored_orchestrator_health_checks() {
        let mut monitored_orchestrator = MonitoredToolOrchestrator::new();
        let tool = Box::new(MockTool::new(
            "test-tool".to_string(),
            "Test Tool".to_string(),
            false,
        ));

        monitored_orchestrator
            .orchestrator_mut()
            .register_tool(tool, "test_type".to_string())
            .await
            .unwrap();

        // Perform health checks with monitoring
        let health_results = monitored_orchestrator.health_check_all_monitored().await;
        assert_eq!(health_results.len(), 1);
        assert_eq!(
            health_results.get("test_type").unwrap().status,
            HealthStatus::Healthy
        );

        // Check that health check was recorded
        let system_status = monitored_orchestrator.get_system_status();
        assert!(system_status.monitoring_stats.last_health_check.is_some());
    }

    #[tokio::test]
    async fn test_system_health_determination() {
        let mut monitored_orchestrator = MonitoredToolOrchestrator::new();
        let failing_tool = Box::new(MockTool::new(
            "failing-tool".to_string(),
            "Failing Tool".to_string(),
            true,
        ));

        monitored_orchestrator
            .orchestrator_mut()
            .register_tool(failing_tool, "failing_type".to_string())
            .await
            .unwrap();

        // Perform health check on failing tool
        let _health_results = monitored_orchestrator.health_check_all_monitored().await;

        // Check system health
        let system_status = monitored_orchestrator.get_system_status();
        assert_eq!(system_status.system_health, SystemHealth::Critical);
        assert!(system_status.active_alerts.len() > 0);
    }

    #[tokio::test]
    async fn test_monitoring_system_clear_old_alerts() {
        let mut monitoring = MonitoringSystem::new();

        // Generate an alert
        let unhealthy_check = HealthCheck::unhealthy("Service down".to_string());
        monitoring.record_health_check("test-tool", &unhealthy_check);

        assert_eq!(monitoring.get_active_alerts().len(), 1);

        // Clear old alerts (older than now + 1 hour)
        let future_time = chrono::Utc::now() + chrono::Duration::hours(1);
        monitoring.clear_old_alerts(future_time);

        assert_eq!(monitoring.alerts.len(), 0);
    }

    #[tokio::test]
    async fn test_monitoring_system_reset_metrics() {
        let mut monitoring = MonitoringSystem::new();

        // Record operation
        let result = ToolResult::success(Some(serde_json::json!({})), 100);
        monitoring.record_operation("test-tool", "operation", &result);

        assert!(monitoring.get_tool_metrics("test-tool").is_some());

        // Reset metrics
        monitoring.reset_tool_metrics("test-tool");

        assert!(monitoring.get_tool_metrics("test-tool").is_none());
    }

    #[tokio::test]
    async fn test_performance_thresholds_configuration() {
        let custom_thresholds = PerformanceThresholds {
            max_response_time_ms: 200,
            max_error_rate: 0.05,
            min_uptime_percentage: 99.0,
            max_consecutive_failures: 5,
        };

        let monitoring = MonitoringSystem::new()
            .with_performance_thresholds(custom_thresholds.clone())
            .with_health_check_interval(Duration::from_secs(60));

        assert_eq!(monitoring.performance_thresholds.max_response_time_ms, 200);
        assert_eq!(monitoring.performance_thresholds.max_error_rate, 0.05);
        assert_eq!(monitoring.health_check_interval, Duration::from_secs(60));
    }
}
