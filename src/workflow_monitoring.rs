use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::workflow_engine::{WorkflowExecution, WorkflowEventHandler};

/// Performance metrics for workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPerformanceMetrics {
    pub workflow_id: String,
    pub execution_count: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub average_duration_ms: f64,
    pub min_duration_ms: u64,
    pub max_duration_ms: u64,
    pub success_rate: f64,
    pub last_executed: Option<DateTime<Utc>>,
    pub last_success: Option<DateTime<Utc>>,
    pub last_failure: Option<DateTime<Utc>>,
}

/// System-wide analytics data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemAnalytics {
    pub total_workflows: u64,
    pub active_workflows: u64,
    pub total_executions: u64,
    pub total_successes: u64,
    pub total_failures: u64,
    pub overall_success_rate: f64,
    pub average_execution_time_ms: f64,
    pub executions_per_hour: f64,
    pub most_active_workflow: Option<String>,
    pub most_reliable_workflow: Option<String>,
    pub resource_usage: ResourceUsage,
}

/// Resource usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: f64,
    pub disk_usage_mb: f64,
    pub network_requests: u64,
    pub active_connections: u64,
}

/// Workflow execution event for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionEvent {
    pub id: String,
    pub execution_id: String,
    pub workflow_id: String,
    pub event_type: String, // started, completed, failed, action_executed, etc.
    pub timestamp: DateTime<Utc>,
    pub duration_ms: Option<u64>,
    pub metadata: HashMap<String, String>,
    pub error_message: Option<String>,
}

/// Alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfiguration {
    pub id: String,
    pub name: String,
    pub alert_type: String, // failure_rate, execution_time, error_count, etc.
    pub threshold: f64,
    pub time_window_minutes: u32,
    pub workflow_ids: Vec<String>, // Empty for all workflows
    pub enabled: bool,
    pub notification_channels: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Alert instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub alert_config_id: String,
    pub workflow_id: Option<String>,
    pub alert_type: String,
    pub severity: String, // low, medium, high, critical
    pub title: String,
    pub message: String,
    pub triggered_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub acknowledged_by: Option<String>,
    pub status: String, // active, resolved, acknowledged
    pub metadata: HashMap<String, String>,
}

/// Debugging trace for workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTrace {
    pub execution_id: String,
    pub workflow_id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub steps: Vec<ExecutionStep>,
    pub context_snapshot: HashMap<String, serde_json::Value>,
    pub error_details: Option<ErrorDetails>,
}

/// Individual step in workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    pub step_id: String,
    pub step_type: String, // rule_evaluation, action_execution, condition_check
    pub step_name: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub success: bool,
    pub input_data: Option<serde_json::Value>,
    pub output_data: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Error details for debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetails {
    pub error_type: String,
    pub error_message: String,
    pub stack_trace: Option<String>,
    pub context_data: HashMap<String, serde_json::Value>,
    pub suggested_actions: Vec<String>,
}

/// Workflow monitoring and analytics engine
pub struct WorkflowMonitoringEngine {
    performance_metrics: Arc<RwLock<HashMap<String, WorkflowPerformanceMetrics>>>,
    execution_events: Arc<RwLock<VecDeque<WorkflowExecutionEvent>>>,
    alerts: Arc<RwLock<Vec<Alert>>>,
    alert_configs: Arc<RwLock<Vec<AlertConfiguration>>>,
    execution_traces: Arc<RwLock<HashMap<String, ExecutionTrace>>>,
    system_analytics: Arc<RwLock<SystemAnalytics>>,
    max_events_retention: usize,
    max_traces_retention: usize,
}

impl WorkflowMonitoringEngine {
    pub fn new() -> Self {
        Self {
            performance_metrics: Arc::new(RwLock::new(HashMap::new())),
            execution_events: Arc::new(RwLock::new(VecDeque::new())),
            alerts: Arc::new(RwLock::new(Vec::new())),
            alert_configs: Arc::new(RwLock::new(Vec::new())),
            execution_traces: Arc::new(RwLock::new(HashMap::new())),
            system_analytics: Arc::new(RwLock::new(SystemAnalytics::default())),
            max_events_retention: 10000,
            max_traces_retention: 1000,
        }
    }

    /// Initialize with default alert configurations
    pub async fn initialize_default_alerts(&self) -> Result<()> {
        let default_alerts = vec![
            AlertConfiguration {
                id: "high-failure-rate".to_string(),
                name: "High Failure Rate".to_string(),
                alert_type: "failure_rate".to_string(),
                threshold: 0.3, // 30% failure rate
                time_window_minutes: 60,
                workflow_ids: Vec::new(),
                enabled: true,
                notification_channels: vec!["email".to_string(), "slack".to_string()],
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            AlertConfiguration {
                id: "slow-execution".to_string(),
                name: "Slow Execution".to_string(),
                alert_type: "execution_time".to_string(),
                threshold: 300000.0, // 5 minutes
                time_window_minutes: 30,
                workflow_ids: Vec::new(),
                enabled: true,
                notification_channels: vec!["email".to_string()],
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            AlertConfiguration {
                id: "frequent-errors".to_string(),
                name: "Frequent Errors".to_string(),
                alert_type: "error_count".to_string(),
                threshold: 10.0, // 10 errors
                time_window_minutes: 15,
                workflow_ids: Vec::new(),
                enabled: true,
                notification_channels: vec!["slack".to_string()],
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ];

        let mut alert_configs = self.alert_configs.write().await;
        alert_configs.extend(default_alerts);
        Ok(())
    }

    /// Record a workflow execution event
    pub async fn record_execution_event(&self, event: WorkflowExecutionEvent) -> Result<()> {
        // Add to events queue
        let mut events = self.execution_events.write().await;
        events.push_back(event.clone());

        // Maintain retention limit
        while events.len() > self.max_events_retention {
            events.pop_front();
        }

        // Update performance metrics
        self.update_performance_metrics(&event).await?;

        // Check for alerts
        self.check_alerts(&event).await?;

        // Update system analytics
        self.update_system_analytics().await?;

        Ok(())
    }

    /// Get performance metrics for a workflow
    pub async fn get_workflow_metrics(&self, workflow_id: &str) -> Result<Option<WorkflowPerformanceMetrics>> {
        Ok(self.performance_metrics.read().await.get(workflow_id).cloned())
    }

    /// Get system-wide analytics
    pub async fn get_system_analytics(&self) -> Result<SystemAnalytics> {
        Ok(self.system_analytics.read().await.clone())
    }

    /// Get recent execution events
    pub async fn get_recent_events(&self, limit: Option<usize>) -> Result<Vec<WorkflowExecutionEvent>> {
        let events = self.execution_events.read().await;
        let limit = limit.unwrap_or(100);
        
        Ok(events.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect())
    }

    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Result<Vec<Alert>> {
        let alerts = self.alerts.read().await;
        Ok(alerts.iter()
            .filter(|alert| alert.status == "active")
            .cloned()
            .collect())
    }

    /// Get all alerts with optional filtering
    pub async fn get_alerts(&self, workflow_id: Option<&str>, status: Option<&str>) -> Result<Vec<Alert>> {
        let alerts = self.alerts.read().await;
        Ok(alerts.iter()
            .filter(|alert| {
                let workflow_match = workflow_id.map_or(true, |id| 
                    alert.workflow_id.as_ref().map_or(true, |wid| wid == id)
                );
                let status_match = status.map_or(true, |s| alert.status == s);
                workflow_match && status_match
            })
            .cloned()
            .collect())
    }

    /// Acknowledge an alert
    pub async fn acknowledge_alert(&self, alert_id: &str, acknowledged_by: &str) -> Result<()> {
        let mut alerts = self.alerts.write().await;
        if let Some(alert) = alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.acknowledged_at = Some(Utc::now());
            alert.acknowledged_by = Some(acknowledged_by.to_string());
            alert.status = "acknowledged".to_string();
        }
        Ok(())
    }

    /// Start debugging trace for an execution
    pub async fn start_execution_trace(&self, execution_id: &str, workflow_id: &str) -> Result<()> {
        let trace = ExecutionTrace {
            execution_id: execution_id.to_string(),
            workflow_id: workflow_id.to_string(),
            started_at: Utc::now(),
            completed_at: None,
            steps: Vec::new(),
            context_snapshot: HashMap::new(),
            error_details: None,
        };

        let mut traces = self.execution_traces.write().await;
        traces.insert(execution_id.to_string(), trace);

        // Maintain retention limit
        if traces.len() > self.max_traces_retention {
            // Remove oldest traces
            let oldest_keys: Vec<_> = traces.keys().cloned().collect();
            for key in oldest_keys.iter().take(traces.len() - self.max_traces_retention) {
                traces.remove(key);
            }
        }

        Ok(())
    }

    /// Add a step to an execution trace
    pub async fn add_trace_step(&self, execution_id: &str, step: ExecutionStep) -> Result<()> {
        let mut traces = self.execution_traces.write().await;
        if let Some(trace) = traces.get_mut(execution_id) {
            trace.steps.push(step);
        }
        Ok(())
    }

    /// Complete an execution trace
    pub async fn complete_execution_trace(
        &self,
        execution_id: &str,
        error_details: Option<ErrorDetails>,
    ) -> Result<()> {
        let mut traces = self.execution_traces.write().await;
        if let Some(trace) = traces.get_mut(execution_id) {
            trace.completed_at = Some(Utc::now());
            trace.error_details = error_details;
        }
        Ok(())
    }

    /// Get execution trace
    pub async fn get_execution_trace(&self, execution_id: &str) -> Result<Option<ExecutionTrace>> {
        Ok(self.execution_traces.read().await.get(execution_id).cloned())
    }

    /// Get workflow execution statistics
    pub async fn get_workflow_statistics(
        &self,
        workflow_id: &str,
        time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    ) -> Result<HashMap<String, f64>> {
        let events = self.execution_events.read().await;
        let mut stats = HashMap::new();

        let filtered_events: Vec<_> = events.iter()
            .filter(|event| {
                let workflow_match = event.workflow_id == workflow_id;
                let time_match = time_range.map_or(true, |(start, end)| {
                    event.timestamp >= start && event.timestamp <= end
                });
                workflow_match && time_match
            })
            .collect();

        let total_executions = filtered_events.len() as f64;
        let successes = filtered_events.iter()
            .filter(|event| event.event_type == "completed" && event.error_message.is_none())
            .count() as f64;
        let failures = filtered_events.iter()
            .filter(|event| event.event_type == "failed" || event.error_message.is_some())
            .count() as f64;

        let durations: Vec<_> = filtered_events.iter()
            .filter_map(|event| event.duration_ms)
            .collect();

        stats.insert("total_executions".to_string(), total_executions);
        stats.insert("success_count".to_string(), successes);
        stats.insert("failure_count".to_string(), failures);
        stats.insert("success_rate".to_string(), if total_executions > 0.0 { successes / total_executions } else { 0.0 });

        if !durations.is_empty() {
            let avg_duration = durations.iter().sum::<u64>() as f64 / durations.len() as f64;
            let min_duration = *durations.iter().min().unwrap() as f64;
            let max_duration = *durations.iter().max().unwrap() as f64;

            stats.insert("average_duration_ms".to_string(), avg_duration);
            stats.insert("min_duration_ms".to_string(), min_duration);
            stats.insert("max_duration_ms".to_string(), max_duration);
        }

        Ok(stats)
    }

    /// Generate performance report
    pub async fn generate_performance_report(&self, time_range: Option<(DateTime<Utc>, DateTime<Utc>)>) -> Result<String> {
        let system_analytics = self.get_system_analytics().await?;
        let active_alerts = self.get_active_alerts().await?;
        let metrics = self.performance_metrics.read().await;

        let mut report = String::new();
        report.push_str("# Workflow Performance Report\n\n");
        report.push_str(&format!("**Generated:** {}\n\n", Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

        // System overview
        report.push_str("## System Overview\n\n");
        report.push_str(&format!("- Total Workflows: {}\n", system_analytics.total_workflows));
        report.push_str(&format!("- Active Workflows: {}\n", system_analytics.active_workflows));
        report.push_str(&format!("- Total Executions: {}\n", system_analytics.total_executions));
        report.push_str(&format!("- Overall Success Rate: {:.2}%\n", system_analytics.overall_success_rate * 100.0));
        report.push_str(&format!("- Average Execution Time: {:.2}ms\n", system_analytics.average_execution_time_ms));
        report.push_str(&format!("- Executions per Hour: {:.2}\n\n", system_analytics.executions_per_hour));

        // Active alerts
        report.push_str("## Active Alerts\n\n");
        if active_alerts.is_empty() {
            report.push_str("No active alerts.\n\n");
        } else {
            for alert in active_alerts {
                report.push_str(&format!("- **{}**: {} ({})\n", alert.severity.to_uppercase(), alert.title, alert.triggered_at.format("%Y-%m-%d %H:%M:%S UTC")));
            }
            report.push('\n');
        }

        // Top performing workflows
        report.push_str("## Top Performing Workflows\n\n");
        let mut sorted_metrics: Vec<_> = metrics.iter().collect();
        sorted_metrics.sort_by(|a, b| b.1.success_rate.partial_cmp(&a.1.success_rate).unwrap_or(std::cmp::Ordering::Equal));

        for (workflow_id, metric) in sorted_metrics.iter().take(10) {
            report.push_str(&format!("- **{}**: {:.2}% success rate, {} executions\n", workflow_id, metric.success_rate * 100.0, metric.execution_count));
        }

        Ok(report)
    }

    /// Update performance metrics based on execution event
    async fn update_performance_metrics(&self, event: &WorkflowExecutionEvent) -> Result<()> {
        let mut metrics = self.performance_metrics.write().await;
        let workflow_metrics = metrics.entry(event.workflow_id.clone())
            .or_insert_with(|| WorkflowPerformanceMetrics {
                workflow_id: event.workflow_id.clone(),
                execution_count: 0,
                success_count: 0,
                failure_count: 0,
                average_duration_ms: 0.0,
                min_duration_ms: u64::MAX,
                max_duration_ms: 0,
                success_rate: 0.0,
                last_executed: None,
                last_success: None,
                last_failure: None,
            });

        match event.event_type.as_str() {
            "started" => {
                workflow_metrics.execution_count += 1;
                workflow_metrics.last_executed = Some(event.timestamp);
            }
            "completed" => {
                if event.error_message.is_none() {
                    workflow_metrics.success_count += 1;
                    workflow_metrics.last_success = Some(event.timestamp);
                } else {
                    workflow_metrics.failure_count += 1;
                    workflow_metrics.last_failure = Some(event.timestamp);
                }

                if let Some(duration) = event.duration_ms {
                    // Update duration statistics
                    let total_duration = workflow_metrics.average_duration_ms * (workflow_metrics.execution_count - 1) as f64;
                    workflow_metrics.average_duration_ms = (total_duration + duration as f64) / workflow_metrics.execution_count as f64;
                    workflow_metrics.min_duration_ms = workflow_metrics.min_duration_ms.min(duration);
                    workflow_metrics.max_duration_ms = workflow_metrics.max_duration_ms.max(duration);
                }
            }
            "failed" => {
                workflow_metrics.failure_count += 1;
                workflow_metrics.last_failure = Some(event.timestamp);
            }
            _ => {}
        }

        // Update success rate
        if workflow_metrics.execution_count > 0 {
            workflow_metrics.success_rate = workflow_metrics.success_count as f64 / workflow_metrics.execution_count as f64;
        }

        Ok(())
    }

    /// Check for alert conditions
    async fn check_alerts(&self, event: &WorkflowExecutionEvent) -> Result<()> {
        let alert_configs = self.alert_configs.read().await;
        let mut alerts = self.alerts.write().await;

        for config in alert_configs.iter() {
            if !config.enabled {
                continue;
            }

            // Check if this event applies to this alert config
            if !config.workflow_ids.is_empty() && !config.workflow_ids.contains(&event.workflow_id) {
                continue;
            }

            // Check alert conditions
            let should_alert = match config.alert_type.as_str() {
                "failure_rate" => {
                    self.check_failure_rate_alert(config, &event.workflow_id).await?
                }
                "execution_time" => {
                    if let Some(duration) = event.duration_ms {
                        duration as f64 > config.threshold
                    } else {
                        false
                    }
                }
                "error_count" => {
                    self.check_error_count_alert(config, &event.workflow_id).await?
                }
                _ => false,
            };

            if should_alert {
                let alert = Alert {
                    id: Uuid::new_v4().to_string(),
                    alert_config_id: config.id.clone(),
                    workflow_id: Some(event.workflow_id.clone()),
                    alert_type: config.alert_type.clone(),
                    severity: self.determine_alert_severity(config, event).await?,
                    title: format!("{} - {}", config.name, event.workflow_id),
                    message: format!("Alert triggered for workflow {}: {}", event.workflow_id, config.name),
                    triggered_at: Utc::now(),
                    resolved_at: None,
                    acknowledged_at: None,
                    acknowledged_by: None,
                    status: "active".to_string(),
                    metadata: HashMap::new(),
                };

                alerts.push(alert);
            }
        }

        Ok(())
    }

    /// Check failure rate alert condition
    async fn check_failure_rate_alert(&self, config: &AlertConfiguration, workflow_id: &str) -> Result<bool> {
        let cutoff_time = Utc::now() - Duration::minutes(config.time_window_minutes as i64);
        let events = self.execution_events.read().await;
        
        let recent_events: Vec<_> = events.iter()
            .filter(|event| {
                event.workflow_id == workflow_id &&
                event.timestamp >= cutoff_time &&
                (event.event_type == "completed" || event.event_type == "failed")
            })
            .collect();

        if recent_events.is_empty() {
            return Ok(false);
        }

        let failure_count = recent_events.iter()
            .filter(|event| event.event_type == "failed" || event.error_message.is_some())
            .count();

        let failure_rate = failure_count as f64 / recent_events.len() as f64;
        Ok(failure_rate > config.threshold)
    }

    /// Check error count alert condition
    async fn check_error_count_alert(&self, config: &AlertConfiguration, workflow_id: &str) -> Result<bool> {
        let cutoff_time = Utc::now() - Duration::minutes(config.time_window_minutes as i64);
        let events = self.execution_events.read().await;
        
        let error_count = events.iter()
            .filter(|event| {
                event.workflow_id == workflow_id &&
                event.timestamp >= cutoff_time &&
                (event.event_type == "failed" || event.error_message.is_some())
            })
            .count();

        Ok(error_count as f64 > config.threshold)
    }

    /// Determine alert severity
    async fn determine_alert_severity(&self, _config: &AlertConfiguration, _event: &WorkflowExecutionEvent) -> Result<String> {
        // Simplified severity determination
        // In practice, this would be more sophisticated
        Ok("medium".to_string())
    }

    /// Update system-wide analytics
    async fn update_system_analytics(&self) -> Result<()> {
        let metrics = self.performance_metrics.read().await;
        let mut analytics = self.system_analytics.write().await;

        analytics.total_workflows = metrics.len() as u64;
        analytics.active_workflows = metrics.values()
            .filter(|m| m.last_executed.is_some())
            .count() as u64;

        analytics.total_executions = metrics.values()
            .map(|m| m.execution_count)
            .sum();

        analytics.total_successes = metrics.values()
            .map(|m| m.success_count)
            .sum();

        analytics.total_failures = metrics.values()
            .map(|m| m.failure_count)
            .sum();

        analytics.overall_success_rate = if analytics.total_executions > 0 {
            analytics.total_successes as f64 / analytics.total_executions as f64
        } else {
            0.0
        };

        analytics.average_execution_time_ms = if !metrics.is_empty() {
            metrics.values()
                .map(|m| m.average_duration_ms)
                .sum::<f64>() / metrics.len() as f64
        } else {
            0.0
        };

        // Find most active and reliable workflows
        analytics.most_active_workflow = metrics.iter()
            .max_by_key(|(_, m)| m.execution_count)
            .map(|(id, _)| id.clone());

        analytics.most_reliable_workflow = metrics.iter()
            .filter(|(_, m)| m.execution_count > 0)
            .max_by(|(_, a), (_, b)| a.success_rate.partial_cmp(&b.success_rate).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(id, _)| id.clone());

        Ok(())
    }
}

#[async_trait::async_trait]
impl WorkflowEventHandler for WorkflowMonitoringEngine {
    async fn handle_workflow_started(&self, execution: &WorkflowExecution) -> Result<()> {
        let event = WorkflowExecutionEvent {
            id: Uuid::new_v4().to_string(),
            execution_id: execution.id.clone(),
            workflow_id: execution.workflow_id.clone(),
            event_type: "started".to_string(),
            timestamp: execution.started_at,
            duration_ms: None,
            metadata: HashMap::new(),
            error_message: None,
        };

        self.record_execution_event(event).await?;
        self.start_execution_trace(&execution.id, &execution.workflow_id).await?;
        Ok(())
    }

    async fn handle_workflow_completed(&self, execution: &WorkflowExecution) -> Result<()> {
        let duration_ms = execution.completed_at.map(|completed| {
            (completed - execution.started_at).num_milliseconds() as u64
        });

        let event = WorkflowExecutionEvent {
            id: Uuid::new_v4().to_string(),
            execution_id: execution.id.clone(),
            workflow_id: execution.workflow_id.clone(),
            event_type: "completed".to_string(),
            timestamp: execution.completed_at.unwrap_or(Utc::now()),
            duration_ms,
            metadata: HashMap::new(),
            error_message: execution.error_message.clone(),
        };

        self.record_execution_event(event).await?;
        self.complete_execution_trace(&execution.id, None).await?;
        Ok(())
    }

    async fn handle_workflow_failed(&self, execution: &WorkflowExecution, error: &str) -> Result<()> {
        let duration_ms = execution.completed_at.map(|completed| {
            (completed - execution.started_at).num_milliseconds() as u64
        });

        let event = WorkflowExecutionEvent {
            id: Uuid::new_v4().to_string(),
            execution_id: execution.id.clone(),
            workflow_id: execution.workflow_id.clone(),
            event_type: "failed".to_string(),
            timestamp: execution.completed_at.unwrap_or(Utc::now()),
            duration_ms,
            metadata: HashMap::new(),
            error_message: Some(error.to_string()),
        };

        self.record_execution_event(event).await?;
        
        let error_details = ErrorDetails {
            error_type: "workflow_failure".to_string(),
            error_message: error.to_string(),
            stack_trace: None,
            context_data: HashMap::new(),
            suggested_actions: vec![
                "Check workflow configuration".to_string(),
                "Review execution logs".to_string(),
                "Verify service connections".to_string(),
            ],
        };

        self.complete_execution_trace(&execution.id, Some(error_details)).await?;
        Ok(())
    }

    async fn handle_action_executed(&self, execution: &WorkflowExecution, action: &crate::workflow_engine::WorkflowAction) -> Result<()> {
        let step = ExecutionStep {
            step_id: Uuid::new_v4().to_string(),
            step_type: "action_execution".to_string(),
            step_name: action.get_type().to_string(),
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            duration_ms: Some(0), // Would need to track actual duration
            success: true,
            input_data: None,
            output_data: None,
            error_message: None,
            metadata: HashMap::new(),
        };

        self.add_trace_step(&execution.id, step).await?;
        Ok(())
    }

    async fn handle_action_failed(&self, execution: &WorkflowExecution, action: &crate::workflow_engine::WorkflowAction, error: &str) -> Result<()> {
        let step = ExecutionStep {
            step_id: Uuid::new_v4().to_string(),
            step_type: "action_execution".to_string(),
            step_name: action.get_type().to_string(),
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            duration_ms: Some(0), // Would need to track actual duration
            success: false,
            input_data: None,
            output_data: None,
            error_message: Some(error.to_string()),
            metadata: HashMap::new(),
        };

        self.add_trace_step(&execution.id, step).await?;
        Ok(())
    }
}

impl Default for WorkflowMonitoringEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for SystemAnalytics {
    fn default() -> Self {
        Self {
            total_workflows: 0,
            active_workflows: 0,
            total_executions: 0,
            total_successes: 0,
            total_failures: 0,
            overall_success_rate: 0.0,
            average_execution_time_ms: 0.0,
            executions_per_hour: 0.0,
            most_active_workflow: None,
            most_reliable_workflow: None,
            resource_usage: ResourceUsage {
                cpu_usage_percent: 0.0,
                memory_usage_mb: 0.0,
                disk_usage_mb: 0.0,
                network_requests: 0,
                active_connections: 0,
            },
        }
    }
}