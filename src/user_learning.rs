use anyhow::{Context, Result};
use chrono::{DateTime, Utc, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Privacy levels for data collection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PrivacyLevel {
    Minimal,    // Only essential metrics
    Standard,   // Basic usage patterns
    Enhanced,   // Detailed behavior analysis
    Research,   // Full analytics for optimization
}

/// User behavior data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBehaviorData {
    pub id: String,
    pub user_id: String,
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub context: HashMap<String, String>,
    pub duration_ms: Option<u64>,
    pub success: bool,
    pub error_message: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// User interaction patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInteractionPattern {
    pub pattern_id: String,
    pub pattern_type: String, // command_sequence, time_preference, workflow_pattern
    pub frequency: f64,
    pub confidence: f64,
    pub first_observed: DateTime<Utc>,
    pub last_observed: DateTime<Utc>,
    pub pattern_data: serde_json::Value,
    pub user_id: String,
}

/// User preference data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreference {
    pub preference_id: String,
    pub user_id: String,
    pub category: String, // ui, workflow, notifications, etc.
    pub preference_key: String,
    pub preference_value: serde_json::Value,
    pub confidence: f64,
    pub source: String, // explicit, inferred, default
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Privacy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    pub user_id: String,
    pub privacy_level: PrivacyLevel,
    pub data_retention_days: u32,
    pub consent_given: bool,
    pub consent_date: Option<DateTime<Utc>>,
    pub collect_commands: bool,
    pub collect_timings: bool,
    pub collect_errors: bool,
    pub collect_workflow_data: bool,
    pub anonymize_data: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// User learning analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAnalytics {
    pub user_id: String,
    pub total_interactions: u64,
    pub most_used_commands: Vec<(String, u64)>,
    pub peak_usage_hours: Vec<u8>,
    pub average_session_duration: f64,
    pub workflow_efficiency: f64,
    pub learning_confidence: f64,
    pub last_updated: DateTime<Utc>,
}

/// User behavior collection and analysis engine
pub struct UserLearningEngine {
    behavior_data: Arc<RwLock<Vec<UserBehaviorData>>>,
    interaction_patterns: Arc<RwLock<HashMap<String, Vec<UserInteractionPattern>>>>,
    user_preferences: Arc<RwLock<HashMap<String, Vec<UserPreference>>>>,
    privacy_configs: Arc<RwLock<HashMap<String, PrivacyConfig>>>,
    analytics: Arc<RwLock<HashMap<String, UserAnalytics>>>,
    data_store_path: PathBuf,
}

impl UserLearningEngine {
    pub fn new(data_store_path: PathBuf) -> Self {
        Self {
            behavior_data: Arc::new(RwLock::new(Vec::new())),
            interaction_patterns: Arc::new(RwLock::new(HashMap::new())),
            user_preferences: Arc::new(RwLock::new(HashMap::new())),
            privacy_configs: Arc::new(RwLock::new(HashMap::new())),
            analytics: Arc::new(RwLock::new(HashMap::new())),
            data_store_path,
        }
    }

    /// Initialize user privacy configuration
    pub async fn initialize_user_privacy(&self, user_id: &str, privacy_level: PrivacyLevel) -> Result<()> {
        let config = PrivacyConfig {
            user_id: user_id.to_string(),
            privacy_level,
            data_retention_days: 90,
            consent_given: false,
            consent_date: None,
            collect_commands: true,
            collect_timings: true,
            collect_errors: true,
            collect_workflow_data: true,
            anonymize_data: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.privacy_configs.write().await.insert(user_id.to_string(), config);
        Ok(())
    }

    /// Request user consent for data collection
    pub async fn request_consent(&self, user_id: &str) -> Result<bool> {
        // In a real implementation, this would show a consent dialog
        // For now, we'll simulate consent
        if let Some(config) = self.privacy_configs.write().await.get_mut(user_id) {
            config.consent_given = true;
            config.consent_date = Some(Utc::now());
            config.updated_at = Utc::now();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Record user behavior data
    pub async fn record_behavior(&self, behavior: UserBehaviorData) -> Result<()> {
        let privacy_config = self.privacy_configs.read().await
            .get(&behavior.user_id)
            .cloned()
            .unwrap_or_else(|| PrivacyConfig {
                user_id: behavior.user_id.clone(),
                privacy_level: PrivacyLevel::Minimal,
                data_retention_days: 30,
                consent_given: false,
                consent_date: None,
                collect_commands: false,
                collect_timings: false,
                collect_errors: false,
                collect_workflow_data: false,
                anonymize_data: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            });

        // Check if we have consent and if collection is enabled
        if !privacy_config.consent_given {
            return Ok(()); // Don't collect data without consent
        }

        // Filter data based on privacy settings
        let filtered_behavior = self.filter_behavior_data(behavior, &privacy_config)?;

        // Store the behavior data
        self.behavior_data.write().await.push(filtered_behavior);

        // Clean up old data based on retention policy
        self.cleanup_old_data(&privacy_config.user_id, privacy_config.data_retention_days).await?;

        Ok(())
    }

    /// Analyze user behavior patterns
    pub async fn analyze_patterns(&self, user_id: &str) -> Result<Vec<UserInteractionPattern>> {
        let behavior_data = self.behavior_data.read().await;
        let user_behaviors: Vec<_> = behavior_data
            .iter()
            .filter(|b| b.user_id == user_id)
            .collect();

        let mut patterns = Vec::new();

        // Analyze command sequences
        patterns.extend(self.analyze_command_sequences(&user_behaviors)?);

        // Analyze time preferences
        patterns.extend(self.analyze_time_preferences(&user_behaviors)?);

        // Analyze workflow patterns
        patterns.extend(self.analyze_workflow_patterns(&user_behaviors)?);

        // Store patterns
        self.interaction_patterns.write().await.insert(user_id.to_string(), patterns.clone());

        Ok(patterns)
    }

    /// Learn user preferences from behavior
    pub async fn learn_preferences(&self, user_id: &str) -> Result<Vec<UserPreference>> {
        let behavior_data = self.behavior_data.read().await;
        let user_behaviors: Vec<_> = behavior_data
            .iter()
            .filter(|b| b.user_id == user_id)
            .collect();

        let mut preferences = Vec::new();

        // Learn command preferences
        preferences.extend(self.learn_command_preferences(&user_behaviors)?);

        // Learn UI preferences
        preferences.extend(self.learn_ui_preferences(&user_behaviors)?);

        // Learn workflow preferences
        preferences.extend(self.learn_workflow_preferences(&user_behaviors)?);

        // Store preferences
        self.user_preferences.write().await.insert(user_id.to_string(), preferences.clone());

        Ok(preferences)
    }

    /// Get user analytics
    pub async fn get_user_analytics(&self, user_id: &str) -> Result<Option<UserAnalytics>> {
        // Update analytics first
        self.update_user_analytics(user_id).await?;
        
        Ok(self.analytics.read().await.get(user_id).cloned())
    }

    /// Get user patterns
    pub async fn get_user_patterns(&self, user_id: &str) -> Result<Vec<UserInteractionPattern>> {
        Ok(self.interaction_patterns.read().await
            .get(user_id)
            .cloned()
            .unwrap_or_default())
    }

    /// Get user preferences
    pub async fn get_user_preferences(&self, user_id: &str) -> Result<Vec<UserPreference>> {
        Ok(self.user_preferences.read().await
            .get(user_id)
            .cloned()
            .unwrap_or_default())
    }

    /// Update user preference
    pub async fn update_preference(&self, user_id: &str, preference: UserPreference) -> Result<()> {
        let mut preferences = self.user_preferences.write().await;
        let user_prefs = preferences.entry(user_id.to_string()).or_insert_with(Vec::new);
        
        // Update existing or add new preference
        if let Some(existing) = user_prefs.iter_mut().find(|p| p.preference_key == preference.preference_key) {
            existing.preference_value = preference.preference_value;
            existing.confidence = preference.confidence;
            existing.updated_at = Utc::now();
        } else {
            user_prefs.push(preference);
        }

        Ok(())
    }

    /// Get privacy configuration
    pub async fn get_privacy_config(&self, user_id: &str) -> Result<Option<PrivacyConfig>> {
        Ok(self.privacy_configs.read().await.get(user_id).cloned())
    }

    /// Update privacy configuration
    pub async fn update_privacy_config(&self, user_id: &str, config: PrivacyConfig) -> Result<()> {
        let mut updated_config = config;
        updated_config.updated_at = Utc::now();
        
        self.privacy_configs.write().await.insert(user_id.to_string(), updated_config);
        Ok(())
    }

    /// Filter behavior data based on privacy settings
    fn filter_behavior_data(&self, mut behavior: UserBehaviorData, config: &PrivacyConfig) -> Result<UserBehaviorData> {
        // Apply privacy filtering based on level
        match config.privacy_level {
            PrivacyLevel::Minimal => {
                behavior.command = None;
                behavior.args = None;
                behavior.context.clear();
                behavior.metadata.clear();
            }
            PrivacyLevel::Standard => {
                if !config.collect_commands {
                    behavior.command = None;
                    behavior.args = None;
                }
            }
            PrivacyLevel::Enhanced => {
                // Keep most data but anonymize sensitive information
                if config.anonymize_data {
                    behavior = self.anonymize_behavior_data(behavior)?;
                }
            }
            PrivacyLevel::Research => {
                // Keep all data for research purposes
                if config.anonymize_data {
                    behavior = self.anonymize_behavior_data(behavior)?;
                }
            }
        }

        // Apply specific collection settings
        if !config.collect_commands {
            behavior.command = None;
            behavior.args = None;
        }

        if !config.collect_timings {
            behavior.duration_ms = None;
        }

        if !config.collect_errors {
            behavior.error_message = None;
        }

        Ok(behavior)
    }

    /// Anonymize behavior data
    fn anonymize_behavior_data(&self, mut behavior: UserBehaviorData) -> Result<UserBehaviorData> {
        // Replace user ID with hash
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        behavior.user_id.hash(&mut hasher);
        behavior.user_id = format!("user_{}", hasher.finish());

        // Remove or hash sensitive data in context and metadata
        behavior.context.retain(|k, _| !k.contains("path") && !k.contains("file"));
        behavior.metadata.retain(|k, _| !k.contains("path") && !k.contains("file"));

        Ok(behavior)
    }

    /// Clean up old data based on retention policy
    async fn cleanup_old_data(&self, user_id: &str, retention_days: u32) -> Result<()> {
        let cutoff_date = Utc::now() - chrono::Duration::days(retention_days as i64);
        
        let mut behavior_data = self.behavior_data.write().await;
        behavior_data.retain(|b| b.user_id != user_id || b.timestamp > cutoff_date);

        Ok(())
    }

    /// Analyze command sequences
    fn analyze_command_sequences(&self, behaviors: &[&UserBehaviorData]) -> Result<Vec<UserInteractionPattern>> {
        let mut patterns = Vec::new();
        let mut sequences = HashMap::new();

        // Group behaviors by session
        let mut sessions: HashMap<String, Vec<&UserBehaviorData>> = HashMap::new();
        for behavior in behaviors {
            sessions.entry(behavior.session_id.clone())
                .or_insert_with(Vec::new)
                .push(behavior);
        }

        // Analyze sequences within sessions
        for (session_id, session_behaviors) in sessions {
            let mut sorted_behaviors = session_behaviors;
            sorted_behaviors.sort_by_key(|b| b.timestamp);

            // Look for command sequences
            for window in sorted_behaviors.windows(3) {
                if let (Some(cmd1), Some(cmd2), Some(cmd3)) = (
                    &window[0].command,
                    &window[1].command,
                    &window[2].command,
                ) {
                    let sequence = format!("{} -> {} -> {}", cmd1, cmd2, cmd3);
                    *sequences.entry(sequence).or_insert(0) += 1;
                }
            }
        }

        // Convert frequent sequences to patterns
        for (sequence, count) in sequences {
            if count >= 3 { // Minimum frequency threshold
                let pattern = UserInteractionPattern {
                    pattern_id: uuid::Uuid::new_v4().to_string(),
                    pattern_type: "command_sequence".to_string(),
                    frequency: count as f64,
                    confidence: (count as f64 / behaviors.len() as f64).min(1.0),
                    first_observed: Utc::now(), // Would track actual first observation
                    last_observed: Utc::now(),
                    pattern_data: serde_json::json!({
                        "sequence": sequence,
                        "count": count
                    }),
                    user_id: behaviors[0].user_id.clone(),
                };
                patterns.push(pattern);
            }
        }

        Ok(patterns)
    }

    /// Analyze time preferences
    fn analyze_time_preferences(&self, behaviors: &[&UserBehaviorData]) -> Result<Vec<UserInteractionPattern>> {
        let mut patterns = Vec::new();
        let mut hour_counts = HashMap::new();

        // Count interactions by hour
        for behavior in behaviors {
            let hour = behavior.timestamp.hour();
            *hour_counts.entry(hour).or_insert(0) += 1;
        }

        // Find peak hours
        let max_count = hour_counts.values().max().unwrap_or(&0);
        let peak_hours: Vec<_> = hour_counts
            .iter()
            .filter(|(_, &count)| count as f64 > *max_count as f64 * 0.7)
            .map(|(&hour, &count)| (hour, count))
            .collect();

        if !peak_hours.is_empty() {
            let pattern = UserInteractionPattern {
                pattern_id: uuid::Uuid::new_v4().to_string(),
                pattern_type: "time_preference".to_string(),
                frequency: peak_hours.len() as f64,
                confidence: 0.8,
                first_observed: Utc::now(),
                last_observed: Utc::now(),
                pattern_data: serde_json::json!({
                    "peak_hours": peak_hours,
                    "total_interactions": behaviors.len()
                }),
                user_id: behaviors[0].user_id.clone(),
            };
            patterns.push(pattern);
        }

        Ok(patterns)
    }

    /// Analyze workflow patterns
    fn analyze_workflow_patterns(&self, behaviors: &[&UserBehaviorData]) -> Result<Vec<UserInteractionPattern>> {
        let mut patterns = Vec::new();
        let mut workflow_sequences = HashMap::new();

        // Look for workflow-related command patterns
        for behavior in behaviors {
            if let Some(command) = &behavior.command {
                if command.starts_with("task-") || command.starts_with("goal-") || command.starts_with("workflow-") {
                    let workflow_type = command.split('-').next().unwrap_or("unknown");
                    *workflow_sequences.entry(workflow_type.to_string()).or_insert(0) += 1;
                }
            }
        }

        // Create patterns for frequent workflow types
        for (workflow_type, count) in workflow_sequences {
            if count >= 5 { // Minimum frequency threshold
                let pattern = UserInteractionPattern {
                    pattern_id: uuid::Uuid::new_v4().to_string(),
                    pattern_type: "workflow_pattern".to_string(),
                    frequency: count as f64,
                    confidence: (count as f64 / behaviors.len() as f64).min(1.0),
                    first_observed: Utc::now(),
                    last_observed: Utc::now(),
                    pattern_data: serde_json::json!({
                        "workflow_type": workflow_type,
                        "count": count
                    }),
                    user_id: behaviors[0].user_id.clone(),
                };
                patterns.push(pattern);
            }
        }

        Ok(patterns)
    }

    /// Learn command preferences
    fn learn_command_preferences(&self, behaviors: &[&UserBehaviorData]) -> Result<Vec<UserPreference>> {
        let mut preferences = Vec::new();
        let mut command_counts = HashMap::new();

        // Count command usage
        for behavior in behaviors {
            if let Some(command) = &behavior.command {
                *command_counts.entry(command.clone()).or_insert(0) += 1;
            }
        }

        // Create preferences for frequently used commands
        let total_commands = command_counts.values().sum::<u32>();
        for (command, count) in command_counts {
            if count >= 3 {
                let preference = UserPreference {
                    preference_id: uuid::Uuid::new_v4().to_string(),
                    user_id: behaviors[0].user_id.clone(),
                    category: "commands".to_string(),
                    preference_key: format!("favorite_command_{}", command),
                    preference_value: serde_json::json!({
                        "command": command,
                        "usage_count": count,
                        "usage_frequency": count as f64 / total_commands as f64
                    }),
                    confidence: (count as f64 / total_commands as f64).min(1.0),
                    source: "inferred".to_string(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                };
                preferences.push(preference);
            }
        }

        Ok(preferences)
    }

    /// Learn UI preferences
    fn learn_ui_preferences(&self, behaviors: &[&UserBehaviorData]) -> Result<Vec<UserPreference>> {
        let mut preferences = Vec::new();

        // Analyze session durations for UI timeout preferences
        let mut session_durations = HashMap::new();
        for behavior in behaviors {
            if let Some(duration) = behavior.duration_ms {
                let session_total = session_durations.entry(behavior.session_id.clone()).or_insert(0);
                *session_total += duration;
            }
        }

        if !session_durations.is_empty() {
            let avg_session_duration = session_durations.values().sum::<u64>() as f64 / session_durations.len() as f64;
            
            let preference = UserPreference {
                preference_id: uuid::Uuid::new_v4().to_string(),
                user_id: behaviors[0].user_id.clone(),
                category: "ui".to_string(),
                preference_key: "session_timeout".to_string(),
                preference_value: serde_json::json!({
                    "average_session_duration_ms": avg_session_duration,
                    "suggested_timeout_ms": avg_session_duration * 1.5
                }),
                confidence: 0.7,
                source: "inferred".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            preferences.push(preference);
        }

        Ok(preferences)
    }

    /// Learn workflow preferences
    fn learn_workflow_preferences(&self, behaviors: &[&UserBehaviorData]) -> Result<Vec<UserPreference>> {
        let mut preferences = Vec::new();

        // Analyze error patterns to infer workflow preferences
        let mut error_commands = HashMap::new();
        for behavior in behaviors {
            if !behavior.success {
                if let Some(command) = &behavior.command {
                    *error_commands.entry(command.clone()).or_insert(0) += 1;
                }
            }
        }

        // Create preferences for error-prone commands (suggest alternatives)
        for (command, error_count) in error_commands {
            if error_count >= 2 {
                let preference = UserPreference {
                    preference_id: uuid::Uuid::new_v4().to_string(),
                    user_id: behaviors[0].user_id.clone(),
                    category: "workflow".to_string(),
                    preference_key: format!("command_help_{}", command),
                    preference_value: serde_json::json!({
                        "command": command,
                        "error_count": error_count,
                        "suggest_help": true
                    }),
                    confidence: 0.6,
                    source: "inferred".to_string(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                };
                preferences.push(preference);
            }
        }

        Ok(preferences)
    }

    /// Update user analytics
    async fn update_user_analytics(&self, user_id: &str) -> Result<()> {
        let behavior_data = self.behavior_data.read().await;
        let user_behaviors: Vec<_> = behavior_data
            .iter()
            .filter(|b| b.user_id == user_id)
            .collect();

        if user_behaviors.is_empty() {
            return Ok(());
        }

        // Calculate command usage
        let mut command_counts = HashMap::new();
        for behavior in &user_behaviors {
            if let Some(command) = &behavior.command {
                *command_counts.entry(command.clone()).or_insert(0) += 1;
            }
        }

        let mut most_used_commands: Vec<_> = command_counts.into_iter().collect();
        most_used_commands.sort_by(|a, b| b.1.cmp(&a.1));
        most_used_commands.truncate(10);

        // Calculate peak usage hours
        let mut hour_counts = HashMap::new();
        for behavior in &user_behaviors {
            let hour = behavior.timestamp.hour() as u8;
            *hour_counts.entry(hour).or_insert(0) += 1;
        }

        let mut peak_hours: Vec<_> = hour_counts.into_iter().collect();
        peak_hours.sort_by(|a, b| b.1.cmp(&a.1));
        let peak_usage_hours = peak_hours.into_iter().take(5).map(|(hour, _)| hour).collect();

        // Calculate session duration
        let mut session_durations = HashMap::new();
        for behavior in &user_behaviors {
            if let Some(duration) = behavior.duration_ms {
                let session_total = session_durations.entry(behavior.session_id.clone()).or_insert(0);
                *session_total += duration;
            }
        }

        let average_session_duration = if !session_durations.is_empty() {
            session_durations.values().sum::<u64>() as f64 / session_durations.len() as f64
        } else {
            0.0
        };

        // Calculate workflow efficiency (success rate)
        let successful_commands = user_behaviors.iter().filter(|b| b.success).count();
        let workflow_efficiency = if !user_behaviors.is_empty() {
            successful_commands as f64 / user_behaviors.len() as f64
        } else {
            0.0
        };

        let analytics = UserAnalytics {
            user_id: user_id.to_string(),
            total_interactions: user_behaviors.len() as u64,
            most_used_commands,
            peak_usage_hours,
            average_session_duration,
            workflow_efficiency,
            learning_confidence: 0.7, // Would be calculated based on data quality
            last_updated: Utc::now(),
        };

        self.analytics.write().await.insert(user_id.to_string(), analytics);
        Ok(())
    }
}

impl Default for UserLearningEngine {
    fn default() -> Self {
        Self::new(PathBuf::from("./data/user_learning"))
    }
}