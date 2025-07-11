use anyhow::{Context, Result};
use chrono::{DateTime, Utc, Duration, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::user_learning::{UserLearningEngine, UserInteractionPattern, UserPreference};

/// Types of proactive assistance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssistanceType {
    DeadlineReminder,
    PriorityAdjustment,
    WorkflowOptimization,
    ContextualHelp,
    ResourceAlert,
    PerformanceInsight,
}

/// Proactive assistance suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProactiveAssistance {
    pub id: String,
    pub user_id: String,
    pub assistance_type: AssistanceType,
    pub priority: String, // low, medium, high, urgent
    pub title: String,
    pub message: String,
    pub suggested_action: Option<String>,
    pub context: HashMap<String, String>,
    pub confidence: f64,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub shown_to_user: bool,
    pub user_response: Option<String>, // accepted, dismissed, ignored
    pub response_time: Option<DateTime<Utc>>,
}

/// Notification preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    pub user_id: String,
    pub deadline_reminders: bool,
    pub priority_suggestions: bool,
    pub workflow_tips: bool,
    pub performance_insights: bool,
    pub quiet_hours_start: Option<u8>,
    pub quiet_hours_end: Option<u8>,
    pub max_notifications_per_day: u32,
    pub min_confidence_threshold: f64,
}

/// ML prediction model for user behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBehaviorModel {
    pub user_id: String,
    pub model_type: String,
    pub accuracy: f64,
    pub last_trained: DateTime<Utc>,
    pub predictions: HashMap<String, f64>,
    pub feature_weights: HashMap<String, f64>,
}

/// Workflow optimization recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowOptimization {
    pub optimization_id: String,
    pub user_id: String,
    pub workflow_type: String,
    pub current_efficiency: f64,
    pub predicted_efficiency: f64,
    pub optimization_type: String, // sequence, timing, automation
    pub description: String,
    pub steps: Vec<String>,
    pub estimated_time_savings: u64, // minutes
    pub confidence: f64,
    pub created_at: DateTime<Utc>,
}

/// Proactive assistance engine
pub struct ProactiveAssistanceEngine {
    learning_engine: Arc<UserLearningEngine>,
    assistances: Arc<RwLock<HashMap<String, Vec<ProactiveAssistance>>>>,
    notification_prefs: Arc<RwLock<HashMap<String, NotificationPreferences>>>,
    behavior_models: Arc<RwLock<HashMap<String, UserBehaviorModel>>>,
    workflow_optimizations: Arc<RwLock<HashMap<String, Vec<WorkflowOptimization>>>>,
}

impl ProactiveAssistanceEngine {
    pub fn new(learning_engine: Arc<UserLearningEngine>) -> Self {
        Self {
            learning_engine,
            assistances: Arc::new(RwLock::new(HashMap::new())),
            notification_prefs: Arc::new(RwLock::new(HashMap::new())),
            behavior_models: Arc::new(RwLock::new(HashMap::new())),
            workflow_optimizations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize notification preferences for user
    pub async fn initialize_notification_preferences(&self, user_id: &str) -> Result<()> {
        let prefs = NotificationPreferences {
            user_id: user_id.to_string(),
            deadline_reminders: true,
            priority_suggestions: true,
            workflow_tips: true,
            performance_insights: false,
            quiet_hours_start: Some(22), // 10 PM
            quiet_hours_end: Some(8),   // 8 AM
            max_notifications_per_day: 5,
            min_confidence_threshold: 0.7,
        };

        self.notification_prefs.write().await.insert(user_id.to_string(), prefs);
        Ok(())
    }

    /// Generate proactive assistance for user
    pub async fn generate_assistance(&self, user_id: &str) -> Result<Vec<ProactiveAssistance>> {
        let mut assistances = Vec::new();

        // Get user patterns and preferences
        let patterns = self.learning_engine.get_user_patterns(user_id).await?;
        let preferences = self.learning_engine.get_user_preferences(user_id).await?;
        let notification_prefs = self.notification_prefs.read().await
            .get(user_id)
            .cloned()
            .unwrap_or_else(|| NotificationPreferences {
                user_id: user_id.to_string(),
                deadline_reminders: true,
                priority_suggestions: true,
                workflow_tips: true,
                performance_insights: false,
                quiet_hours_start: None,
                quiet_hours_end: None,
                max_notifications_per_day: 5,
                min_confidence_threshold: 0.7,
            });

        // Generate different types of assistance
        if notification_prefs.deadline_reminders {
            assistances.extend(self.generate_deadline_reminders(user_id, &patterns).await?);
        }

        if notification_prefs.priority_suggestions {
            assistances.extend(self.generate_priority_suggestions(user_id, &patterns).await?);
        }

        if notification_prefs.workflow_tips {
            assistances.extend(self.generate_workflow_tips(user_id, &patterns, &preferences).await?);
        }

        if notification_prefs.performance_insights {
            assistances.extend(self.generate_performance_insights(user_id).await?);
        }

        // Filter by confidence threshold
        assistances.retain(|a| a.confidence >= notification_prefs.min_confidence_threshold);

        // Respect quiet hours
        if let (Some(start), Some(end)) = (notification_prefs.quiet_hours_start, notification_prefs.quiet_hours_end) {
            let current_hour = Utc::now().hour() as u8;
            let in_quiet_hours = if start < end {
                current_hour >= start && current_hour < end
            } else {
                current_hour >= start || current_hour < end
            };

            if in_quiet_hours {
                // Only urgent notifications during quiet hours
                assistances.retain(|a| a.priority == "urgent");
            }
        }

        // Limit daily notifications
        let today_start = Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
        let existing_assistances = self.assistances.read().await
            .get(user_id)
            .cloned()
            .unwrap_or_default();
        
        let today_count = existing_assistances.iter()
            .filter(|a| a.created_at >= today_start && a.shown_to_user)
            .count();

        if today_count >= notification_prefs.max_notifications_per_day as usize {
            // Only high priority notifications if limit reached
            assistances.retain(|a| a.priority == "high" || a.priority == "urgent");
        }

        // Sort by priority and confidence
        assistances.sort_by(|a, b| {
            let priority_order = ["urgent", "high", "medium", "low"];
            let a_priority = priority_order.iter().position(|&p| p == a.priority).unwrap_or(3);
            let b_priority = priority_order.iter().position(|&p| p == b.priority).unwrap_or(3);
            
            a_priority.cmp(&b_priority)
                .then_with(|| b.confidence.partial_cmp(&a.confidence).unwrap())
        });

        // Store generated assistances
        self.assistances.write().await.insert(user_id.to_string(), assistances.clone());

        Ok(assistances)
    }

    /// Get pending assistances for user
    pub async fn get_pending_assistances(&self, user_id: &str) -> Result<Vec<ProactiveAssistance>> {
        let assistances = self.assistances.read().await
            .get(user_id)
            .cloned()
            .unwrap_or_default();

        // Filter out expired and already shown assistances
        let now = Utc::now();
        let pending = assistances.into_iter()
            .filter(|a| {
                !a.shown_to_user && 
                a.expires_at.map_or(true, |exp| exp > now)
            })
            .collect();

        Ok(pending)
    }

    /// Mark assistance as shown to user
    pub async fn mark_assistance_shown(&self, user_id: &str, assistance_id: &str) -> Result<()> {
        let mut assistances = self.assistances.write().await;
        if let Some(user_assistances) = assistances.get_mut(user_id) {
            if let Some(assistance) = user_assistances.iter_mut().find(|a| a.id == assistance_id) {
                assistance.shown_to_user = true;
            }
        }
        Ok(())
    }

    /// Record user response to assistance
    pub async fn record_user_response(&self, user_id: &str, assistance_id: &str, response: &str) -> Result<()> {
        let mut assistances = self.assistances.write().await;
        if let Some(user_assistances) = assistances.get_mut(user_id) {
            if let Some(assistance) = user_assistances.iter_mut().find(|a| a.id == assistance_id) {
                assistance.user_response = Some(response.to_string());
                assistance.response_time = Some(Utc::now());
            }
        }
        Ok(())
    }

    /// Train behavior prediction model
    pub async fn train_behavior_model(&self, user_id: &str) -> Result<()> {
        let patterns = self.learning_engine.get_user_patterns(user_id).await?;
        let analytics = self.learning_engine.get_user_analytics(user_id).await?;

        if patterns.is_empty() || analytics.is_none() {
            return Ok(());
        }

        let analytics = analytics.unwrap();
        let mut predictions = HashMap::new();
        let mut feature_weights = HashMap::new();

        // Simple prediction model based on patterns
        for pattern in patterns {
            match pattern.pattern_type.as_str() {
                "command_sequence" => {
                    predictions.insert("next_command".to_string(), pattern.confidence);
                    feature_weights.insert("command_frequency".to_string(), pattern.frequency / 100.0);
                }
                "time_preference" => {
                    predictions.insert("peak_usage_time".to_string(), pattern.confidence);
                    feature_weights.insert("time_consistency".to_string(), pattern.confidence);
                }
                "workflow_pattern" => {
                    predictions.insert("workflow_completion".to_string(), pattern.confidence);
                    feature_weights.insert("workflow_efficiency".to_string(), analytics.workflow_efficiency);
                }
                _ => {}
            }
        }

        let model = UserBehaviorModel {
            user_id: user_id.to_string(),
            model_type: "pattern_based".to_string(),
            accuracy: analytics.learning_confidence,
            last_trained: Utc::now(),
            predictions,
            feature_weights,
        };

        self.behavior_models.write().await.insert(user_id.to_string(), model);
        Ok(())
    }

    /// Get behavior predictions for user
    pub async fn get_behavior_predictions(&self, user_id: &str) -> Result<Option<UserBehaviorModel>> {
        Ok(self.behavior_models.read().await.get(user_id).cloned())
    }

    /// Generate workflow optimizations
    pub async fn generate_workflow_optimizations(&self, user_id: &str) -> Result<Vec<WorkflowOptimization>> {
        let patterns = self.learning_engine.get_user_patterns(user_id).await?;
        let analytics = self.learning_engine.get_user_analytics(user_id).await?;

        if analytics.is_none() {
            return Ok(Vec::new());
        }

        let analytics = analytics.unwrap();
        let mut optimizations = Vec::new();

        // Analyze workflow patterns for optimization opportunities
        for pattern in patterns {
            if pattern.pattern_type == "workflow_pattern" {
                let workflow_type = pattern.pattern_data["workflow_type"].as_str().unwrap_or("unknown");
                
                // Suggest optimization based on efficiency
                if analytics.workflow_efficiency < 0.8 {
                    let optimization = WorkflowOptimization {
                        optimization_id: uuid::Uuid::new_v4().to_string(),
                        user_id: user_id.to_string(),
                        workflow_type: workflow_type.to_string(),
                        current_efficiency: analytics.workflow_efficiency,
                        predicted_efficiency: (analytics.workflow_efficiency + 0.2).min(1.0),
                        optimization_type: "sequence".to_string(),
                        description: format!("Optimize {} workflow by reordering common commands", workflow_type),
                        steps: vec![
                            format!("Analyze your {} command patterns", workflow_type),
                            "Group related commands together".to_string(),
                            "Create workflow templates for common sequences".to_string(),
                        ],
                        estimated_time_savings: 15,
                        confidence: pattern.confidence,
                        created_at: Utc::now(),
                    };
                    optimizations.push(optimization);
                }
            }
        }

        // Store optimizations
        self.workflow_optimizations.write().await.insert(user_id.to_string(), optimizations.clone());

        Ok(optimizations)
    }

    /// Get workflow optimizations for user
    pub async fn get_workflow_optimizations(&self, user_id: &str) -> Result<Vec<WorkflowOptimization>> {
        Ok(self.workflow_optimizations.read().await
            .get(user_id)
            .cloned()
            .unwrap_or_default())
    }

    /// Generate deadline reminders
    async fn generate_deadline_reminders(&self, user_id: &str, patterns: &[UserInteractionPattern]) -> Result<Vec<ProactiveAssistance>> {
        let mut assistances = Vec::new();

        // Look for time-based patterns to predict deadline issues
        for pattern in patterns {
            if pattern.pattern_type == "time_preference" {
                let peak_hours = pattern.pattern_data["peak_hours"].as_array();
                if let Some(hours) = peak_hours {
                    let current_hour = Utc::now().hour();
                    let is_peak_time = hours.iter().any(|h| {
                        h.as_array().map_or(false, |arr| {
                            arr.get(0).and_then(|v| v.as_u64()).map_or(false, |hour| hour == current_hour as u64)
                        })
                    });

                    if is_peak_time {
                        let assistance = ProactiveAssistance {
                            id: uuid::Uuid::new_v4().to_string(),
                            user_id: user_id.to_string(),
                            assistance_type: AssistanceType::DeadlineReminder,
                            priority: "medium".to_string(),
                            title: "Peak Productivity Time".to_string(),
                            message: "This is one of your most productive hours. Consider working on high-priority tasks.".to_string(),
                            suggested_action: Some("list high-priority tasks".to_string()),
                            context: HashMap::from([
                                ("peak_hour".to_string(), current_hour.to_string()),
                                ("confidence".to_string(), pattern.confidence.to_string()),
                            ]),
                            confidence: pattern.confidence,
                            created_at: Utc::now(),
                            expires_at: Some(Utc::now() + Duration::hours(1)),
                            shown_to_user: false,
                            user_response: None,
                            response_time: None,
                        };
                        assistances.push(assistance);
                    }
                }
            }
        }

        Ok(assistances)
    }

    /// Generate priority suggestions
    async fn generate_priority_suggestions(&self, user_id: &str, patterns: &[UserInteractionPattern]) -> Result<Vec<ProactiveAssistance>> {
        let mut assistances = Vec::new();

        // Look for command sequence patterns that might indicate priority issues
        for pattern in patterns {
            if pattern.pattern_type == "command_sequence" {
                let sequence = pattern.pattern_data["sequence"].as_str().unwrap_or("");
                
                // Check for patterns that indicate task switching
                if sequence.contains("list") && sequence.contains("show") {
                    let assistance = ProactiveAssistance {
                        id: uuid::Uuid::new_v4().to_string(),
                        user_id: user_id.to_string(),
                        assistance_type: AssistanceType::PriorityAdjustment,
                        priority: "low".to_string(),
                        title: "Task Priority Suggestion".to_string(),
                        message: "I notice you frequently switch between viewing tasks. Consider prioritizing your task list.".to_string(),
                        suggested_action: Some("prioritize tasks by importance".to_string()),
                        context: HashMap::from([
                            ("pattern_sequence".to_string(), sequence.to_string()),
                            ("frequency".to_string(), pattern.frequency.to_string()),
                        ]),
                        confidence: pattern.confidence * 0.8, // Lower confidence for inference
                        created_at: Utc::now(),
                        expires_at: Some(Utc::now() + Duration::days(1)),
                        shown_to_user: false,
                        user_response: None,
                        response_time: None,
                    };
                    assistances.push(assistance);
                }
            }
        }

        Ok(assistances)
    }

    /// Generate workflow tips
    async fn generate_workflow_tips(&self, user_id: &str, patterns: &[UserInteractionPattern], preferences: &[UserPreference]) -> Result<Vec<ProactiveAssistance>> {
        let mut assistances = Vec::new();

        // Look for workflow improvement opportunities
        for preference in preferences {
            if preference.category == "workflow" && preference.preference_key.starts_with("command_help_") {
                let command = preference.preference_value["command"].as_str().unwrap_or("");
                let error_count = preference.preference_value["error_count"].as_u64().unwrap_or(0);

                if error_count > 2 {
                    let assistance = ProactiveAssistance {
                        id: uuid::Uuid::new_v4().to_string(),
                        user_id: user_id.to_string(),
                        assistance_type: AssistanceType::ContextualHelp,
                        priority: "medium".to_string(),
                        title: "Command Help Available".to_string(),
                        message: format!("I've noticed you've had some issues with the '{}' command. Would you like some tips?", command),
                        suggested_action: Some(format!("help {}", command)),
                        context: HashMap::from([
                            ("command".to_string(), command.to_string()),
                            ("error_count".to_string(), error_count.to_string()),
                        ]),
                        confidence: preference.confidence,
                        created_at: Utc::now(),
                        expires_at: Some(Utc::now() + Duration::days(3)),
                        shown_to_user: false,
                        user_response: None,
                        response_time: None,
                    };
                    assistances.push(assistance);
                }
            }
        }

        Ok(assistances)
    }

    /// Generate performance insights
    async fn generate_performance_insights(&self, user_id: &str) -> Result<Vec<ProactiveAssistance>> {
        let mut assistances = Vec::new();

        if let Some(analytics) = self.learning_engine.get_user_analytics(user_id).await? {
            // Generate insights based on analytics
            if analytics.workflow_efficiency < 0.6 {
                let assistance = ProactiveAssistance {
                    id: uuid::Uuid::new_v4().to_string(),
                    user_id: user_id.to_string(),
                    assistance_type: AssistanceType::PerformanceInsight,
                    priority: "low".to_string(),
                    title: "Workflow Efficiency Insight".to_string(),
                    message: format!("Your workflow efficiency is {:.1}%. Consider reviewing your most common task patterns.", analytics.workflow_efficiency * 100.0),
                    suggested_action: Some("review workflow patterns".to_string()),
                    context: HashMap::from([
                        ("efficiency".to_string(), analytics.workflow_efficiency.to_string()),
                        ("total_interactions".to_string(), analytics.total_interactions.to_string()),
                    ]),
                    confidence: analytics.learning_confidence,
                    created_at: Utc::now(),
                    expires_at: Some(Utc::now() + Duration::days(7)),
                    shown_to_user: false,
                    user_response: None,
                    response_time: None,
                };
                assistances.push(assistance);
            }
        }

        Ok(assistances)
    }
}

impl Default for ProactiveAssistanceEngine {
    fn default() -> Self {
        Self::new(Arc::new(UserLearningEngine::default()))
    }
}