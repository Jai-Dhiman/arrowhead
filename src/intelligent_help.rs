use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{Utc, Timelike};

use crate::ai_conversation::LLMClient;
use crate::cli::Commands;
use crate::conversational_interface::ConversationSession;

/// Represents a help suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelpSuggestion {
    /// The suggestion text
    pub text: String,
    /// Example command or phrase
    pub example: Option<String>,
    /// Category of the suggestion
    pub category: HelpCategory,
    /// Priority/relevance score (0.0 to 1.0)
    pub relevance: f32,
    /// Whether this suggestion is contextual
    pub contextual: bool,
}

/// Categories of help suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HelpCategory {
    /// Getting started with basic commands
    GettingStarted,
    /// Todo management tasks
    TodoManagement,
    /// Goal setting and tracking
    GoalTracking,
    /// Note-taking and organization
    NoteOrganization,
    /// Project management workflows
    ProjectWorkflows,
    /// Advanced features and integrations
    AdvancedFeatures,
    /// Troubleshooting and common issues
    Troubleshooting,
}

/// Context information for generating help suggestions
#[derive(Debug, Clone)]
pub struct HelpContext {
    /// Current user session
    pub session: Option<ConversationSession>,
    /// Recent commands executed
    pub recent_commands: Vec<Commands>,
    /// Current project state (mocked for now)
    pub project_state: ProjectState,
    /// User's experience level
    pub user_level: UserLevel,
    /// Time of day for contextual suggestions
    pub time_context: TimeContext,
}

/// Represents the current state of user's project
#[derive(Debug, Clone)]
pub struct ProjectState {
    /// Number of open todos
    pub open_todos: usize,
    /// Number of active goals
    pub active_goals: usize,
    /// Number of notes
    pub note_count: usize,
    /// Recent activity level
    pub activity_level: ActivityLevel,
    /// Common workflow patterns
    pub workflow_patterns: Vec<String>,
}

/// User experience level
#[derive(Debug, Clone)]
pub enum UserLevel {
    /// New user, needs basic guidance
    Beginner,
    /// Familiar with basic commands
    Intermediate,
    /// Experienced user, needs advanced tips
    Advanced,
}

/// Activity level indicator
#[derive(Debug, Clone)]
pub enum ActivityLevel {
    /// Low activity, may need motivation
    Low,
    /// Normal activity level
    Normal,
    /// High activity, may need efficiency tips
    High,
}

/// Time-based context for suggestions
#[derive(Debug, Clone)]
pub enum TimeContext {
    /// Morning - focus on planning
    Morning,
    /// Afternoon - focus on execution
    Afternoon,
    /// Evening - focus on review and wrap-up
    Evening,
}

/// Intelligent help system
pub struct IntelligentHelpSystem {
    /// LLM client for generating contextual help
    llm_client: Option<Box<dyn LLMClient>>,
    /// Pre-defined help suggestions
    base_suggestions: Vec<HelpSuggestion>,
    /// Command examples database
    command_examples: HashMap<String, Vec<CommandExample>>,
    /// Workflow patterns
    workflow_patterns: Vec<WorkflowPattern>,
}

/// A command example with context
#[derive(Debug, Clone)]
pub struct CommandExample {
    /// Natural language description
    pub description: String,
    /// Example command
    pub command: String,
    /// Use case or scenario
    pub use_case: String,
    /// Difficulty level
    pub difficulty: UserLevel,
}

/// A workflow pattern
#[derive(Debug, Clone)]
pub struct WorkflowPattern {
    /// Pattern name
    pub name: String,
    /// Pattern description
    pub description: String,
    /// Sequence of commands
    pub commands: Vec<String>,
    /// When to suggest this pattern
    pub trigger_conditions: Vec<String>,
}

impl IntelligentHelpSystem {
    /// Create a new intelligent help system
    pub fn new(llm_client: Option<Box<dyn LLMClient>>) -> Self {
        Self {
            llm_client,
            base_suggestions: Self::initialize_base_suggestions(),
            command_examples: Self::initialize_command_examples(),
            workflow_patterns: Self::initialize_workflow_patterns(),
        }
    }

    /// Generate contextual help suggestions
    pub async fn generate_help_suggestions(
        &self,
        context: &HelpContext,
    ) -> Result<Vec<HelpSuggestion>> {
        let mut suggestions = Vec::new();
        
        // Add base suggestions filtered by context
        suggestions.extend(self.get_contextual_base_suggestions(context));
        
        // Add project-specific suggestions
        suggestions.extend(self.get_project_specific_suggestions(context));
        
        // Add time-based suggestions
        suggestions.extend(self.get_time_based_suggestions(context));
        
        // Add workflow suggestions
        suggestions.extend(self.get_workflow_suggestions(context));
        
        // Generate AI-powered suggestions if available
        if let Some(ref llm_client) = self.llm_client {
            if let Ok(ai_suggestions) = self.generate_ai_suggestions(context, llm_client).await {
                suggestions.extend(ai_suggestions);
            }
        }
        
        // Sort by relevance and limit results
        suggestions.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap_or(std::cmp::Ordering::Equal));
        suggestions.truncate(10);
        
        Ok(suggestions)
    }

    /// Handle discovery queries like "what can I do next?"
    pub async fn handle_discovery_query(
        &self,
        query: &str,
        context: &HelpContext,
    ) -> Result<Vec<HelpSuggestion>> {
        let query_lower = query.to_lowercase();
        
        // Determine query type
        let query_type = if query_lower.contains("what can i do") || query_lower.contains("what should i do") {
            DiscoveryQueryType::NextActions
        } else if query_lower.contains("how do i") || query_lower.contains("how to") {
            DiscoveryQueryType::HowTo
        } else if query_lower.contains("help") || query_lower.contains("assist") {
            DiscoveryQueryType::GeneralHelp
        } else if query_lower.contains("example") || query_lower.contains("show me") {
            DiscoveryQueryType::Examples
        } else {
            DiscoveryQueryType::GeneralHelp
        };
        
        // Generate suggestions based on query type
        match query_type {
            DiscoveryQueryType::NextActions => self.suggest_next_actions(context).await,
            DiscoveryQueryType::HowTo => self.suggest_how_to_guides(context).await,
            DiscoveryQueryType::Examples => self.suggest_examples(context).await,
            DiscoveryQueryType::GeneralHelp => self.generate_help_suggestions(context).await,
        }
    }

    /// Get command examples for a specific intent
    pub fn get_command_examples(&self, intent: &str) -> Vec<CommandExample> {
        self.command_examples.get(intent).cloned().unwrap_or_default()
    }

    /// Get workflow patterns that match current context
    pub fn get_relevant_workflows(&self, context: &HelpContext) -> Vec<WorkflowPattern> {
        self.workflow_patterns.iter()
            .filter(|pattern| self.workflow_matches_context(pattern, context))
            .cloned()
            .collect()
    }

    /// Initialize base help suggestions
    fn initialize_base_suggestions() -> Vec<HelpSuggestion> {
        vec![
            HelpSuggestion {
                text: "Add a new todo item to track your tasks".to_string(),
                example: Some("Add a todo to finish the project report".to_string()),
                category: HelpCategory::TodoManagement,
                relevance: 0.9,
                contextual: false,
            },
            HelpSuggestion {
                text: "Create a goal to define your objectives".to_string(),
                example: Some("Create a goal to learn Rust programming".to_string()),
                category: HelpCategory::GoalTracking,
                relevance: 0.8,
                contextual: false,
            },
            HelpSuggestion {
                text: "Make a note to capture important information".to_string(),
                example: Some("Create a note about today's meeting".to_string()),
                category: HelpCategory::NoteOrganization,
                relevance: 0.7,
                contextual: false,
            },
            HelpSuggestion {
                text: "View your current todos to see what's pending".to_string(),
                example: Some("Show me my todos".to_string()),
                category: HelpCategory::TodoManagement,
                relevance: 0.8,
                contextual: true,
            },
            HelpSuggestion {
                text: "Check your goals to track progress".to_string(),
                example: Some("List my goals".to_string()),
                category: HelpCategory::GoalTracking,
                relevance: 0.7,
                contextual: true,
            },
            HelpSuggestion {
                text: "Try asking 'what can I do next?' for personalized suggestions".to_string(),
                example: Some("What can I do next?".to_string()),
                category: HelpCategory::GettingStarted,
                relevance: 0.6,
                contextual: false,
            },
            HelpSuggestion {
                text: "Use natural language - no need for complex commands".to_string(),
                example: Some("Mark my first todo as done".to_string()),
                category: HelpCategory::GettingStarted,
                relevance: 0.5,
                contextual: false,
            },
        ]
    }

    /// Initialize command examples database
    fn initialize_command_examples() -> HashMap<String, Vec<CommandExample>> {
        let mut examples = HashMap::new();
        
        // Todo examples
        examples.insert("add_todo".to_string(), vec![
            CommandExample {
                description: "Add a simple todo".to_string(),
                command: "Add a todo to call the dentist".to_string(),
                use_case: "Quick task reminder".to_string(),
                difficulty: UserLevel::Beginner,
            },
            CommandExample {
                description: "Add a todo with due date".to_string(),
                command: "Add a todo to submit report by Friday".to_string(),
                use_case: "Time-sensitive task".to_string(),
                difficulty: UserLevel::Intermediate,
            },
            CommandExample {
                description: "Add a todo with tags".to_string(),
                command: "Add a todo to review code tagged with work and urgent".to_string(),
                use_case: "Categorized task management".to_string(),
                difficulty: UserLevel::Advanced,
            },
        ]);
        
        // Goal examples
        examples.insert("add_goal".to_string(), vec![
            CommandExample {
                description: "Set a learning goal".to_string(),
                command: "Create a goal to learn Spanish".to_string(),
                use_case: "Personal development".to_string(),
                difficulty: UserLevel::Beginner,
            },
            CommandExample {
                description: "Set a goal with target date".to_string(),
                command: "Create a goal to run a marathon by December".to_string(),
                use_case: "Time-bound objective".to_string(),
                difficulty: UserLevel::Intermediate,
            },
        ]);
        
        // Note examples
        examples.insert("create_note".to_string(), vec![
            CommandExample {
                description: "Create a simple note".to_string(),
                command: "Create a note about today's meeting".to_string(),
                use_case: "Information capture".to_string(),
                difficulty: UserLevel::Beginner,
            },
            CommandExample {
                description: "Create a note with content".to_string(),
                command: "Create a note called 'Ideas' with content about new features".to_string(),
                use_case: "Structured information".to_string(),
                difficulty: UserLevel::Intermediate,
            },
        ]);
        
        examples
    }

    /// Initialize workflow patterns
    fn initialize_workflow_patterns() -> Vec<WorkflowPattern> {
        vec![
            WorkflowPattern {
                name: "Morning Planning".to_string(),
                description: "Start your day by reviewing goals and planning tasks".to_string(),
                commands: vec![
                    "List my goals".to_string(),
                    "Show me my todos".to_string(),
                    "Add a todo for today's priority".to_string(),
                ],
                trigger_conditions: vec!["morning".to_string(), "empty_todos".to_string()],
            },
            WorkflowPattern {
                name: "Project Setup".to_string(),
                description: "Set up a new project with goals and initial tasks".to_string(),
                commands: vec![
                    "Create a goal for the new project".to_string(),
                    "Add todos for project milestones".to_string(),
                    "Create a note for project requirements".to_string(),
                ],
                trigger_conditions: vec!["new_user".to_string(), "empty_goals".to_string()],
            },
            WorkflowPattern {
                name: "Task Completion".to_string(),
                description: "Complete tasks and track progress".to_string(),
                commands: vec![
                    "Mark todo as done".to_string(),
                    "Update goal progress".to_string(),
                    "Add note about completion".to_string(),
                ],
                trigger_conditions: vec!["high_activity".to_string(), "many_todos".to_string()],
            },
        ]
    }

    /// Get contextual base suggestions
    fn get_contextual_base_suggestions(&self, context: &HelpContext) -> Vec<HelpSuggestion> {
        self.base_suggestions.iter()
            .filter(|suggestion| {
                if !suggestion.contextual {
                    return true;
                }
                
                // Filter based on user level
                match context.user_level {
                    UserLevel::Beginner => matches!(suggestion.category, HelpCategory::GettingStarted | HelpCategory::TodoManagement),
                    UserLevel::Intermediate => !matches!(suggestion.category, HelpCategory::GettingStarted),
                    UserLevel::Advanced => matches!(suggestion.category, HelpCategory::AdvancedFeatures | HelpCategory::ProjectWorkflows),
                }
            })
            .cloned()
            .collect()
    }

    /// Get project-specific suggestions
    fn get_project_specific_suggestions(&self, context: &HelpContext) -> Vec<HelpSuggestion> {
        let mut suggestions = Vec::new();
        
        // Suggest based on project state
        if context.project_state.open_todos == 0 {
            suggestions.push(HelpSuggestion {
                text: "You have no open todos. Consider adding some tasks to stay organized".to_string(),
                example: Some("Add a todo for your next priority".to_string()),
                category: HelpCategory::TodoManagement,
                relevance: 0.9,
                contextual: true,
            });
        } else if context.project_state.open_todos > 10 {
            suggestions.push(HelpSuggestion {
                text: "You have many open todos. Consider completing some or organizing them".to_string(),
                example: Some("Show me my todos to review what's pending".to_string()),
                category: HelpCategory::TodoManagement,
                relevance: 0.8,
                contextual: true,
            });
        }
        
        if context.project_state.active_goals == 0 {
            suggestions.push(HelpSuggestion {
                text: "Setting goals can help you stay focused. Consider creating your first goal".to_string(),
                example: Some("Create a goal for this month".to_string()),
                category: HelpCategory::GoalTracking,
                relevance: 0.7,
                contextual: true,
            });
        }
        
        suggestions
    }

    /// Get time-based suggestions
    fn get_time_based_suggestions(&self, context: &HelpContext) -> Vec<HelpSuggestion> {
        let mut suggestions = Vec::new();
        
        match context.time_context {
            TimeContext::Morning => {
                suggestions.push(HelpSuggestion {
                    text: "Good morning! Start your day by reviewing your goals and planning tasks".to_string(),
                    example: Some("What should I work on today?".to_string()),
                    category: HelpCategory::ProjectWorkflows,
                    relevance: 0.8,
                    contextual: true,
                });
            }
            TimeContext::Afternoon => {
                suggestions.push(HelpSuggestion {
                    text: "How's your progress? Consider marking completed tasks as done".to_string(),
                    example: Some("Mark my first todo as done".to_string()),
                    category: HelpCategory::TodoManagement,
                    relevance: 0.7,
                    contextual: true,
                });
            }
            TimeContext::Evening => {
                suggestions.push(HelpSuggestion {
                    text: "End of day review: check what you've accomplished and plan for tomorrow".to_string(),
                    example: Some("Show me my completed tasks".to_string()),
                    category: HelpCategory::ProjectWorkflows,
                    relevance: 0.6,
                    contextual: true,
                });
            }
        }
        
        suggestions
    }

    /// Get workflow suggestions
    fn get_workflow_suggestions(&self, context: &HelpContext) -> Vec<HelpSuggestion> {
        let relevant_workflows = self.get_relevant_workflows(context);
        
        relevant_workflows.into_iter()
            .map(|workflow| HelpSuggestion {
                text: format!("Try the '{}' workflow: {}", workflow.name, workflow.description),
                example: Some(workflow.commands.first().cloned().unwrap_or_default()),
                category: HelpCategory::ProjectWorkflows,
                relevance: 0.6,
                contextual: true,
            })
            .collect()
    }

    /// Generate AI-powered suggestions
    async fn generate_ai_suggestions(
        &self,
        _context: &HelpContext,
        _llm_client: &Box<dyn LLMClient>,
    ) -> Result<Vec<HelpSuggestion>> {
        // This would use the LLM client to generate contextual suggestions
        // For now, return empty vector
        Ok(Vec::new())
    }

    /// Suggest next actions based on context
    async fn suggest_next_actions(&self, context: &HelpContext) -> Result<Vec<HelpSuggestion>> {
        let mut suggestions = Vec::new();
        
        // Analyze current state and suggest actions
        if context.project_state.open_todos > 0 {
            suggestions.push(HelpSuggestion {
                text: "You have open todos. Consider working on one of them".to_string(),
                example: Some("Show me my todos to pick one to work on".to_string()),
                category: HelpCategory::TodoManagement,
                relevance: 0.9,
                contextual: true,
            });
        }
        
        if context.project_state.active_goals > 0 {
            suggestions.push(HelpSuggestion {
                text: "Check your goals and see if you can make progress".to_string(),
                example: Some("List my goals to see what I'm working towards".to_string()),
                category: HelpCategory::GoalTracking,
                relevance: 0.8,
                contextual: true,
            });
        }
        
        // Default suggestions for new users
        if context.project_state.open_todos == 0 && context.project_state.active_goals == 0 {
            suggestions.push(HelpSuggestion {
                text: "Start by creating your first goal or adding a todo".to_string(),
                example: Some("Add a todo for something I need to do".to_string()),
                category: HelpCategory::GettingStarted,
                relevance: 0.9,
                contextual: true,
            });
        }
        
        Ok(suggestions)
    }

    /// Suggest how-to guides
    async fn suggest_how_to_guides(&self, context: &HelpContext) -> Result<Vec<HelpSuggestion>> {
        let mut suggestions = Vec::new();
        
        // Common how-to suggestions based on user level
        match context.user_level {
            UserLevel::Beginner => {
                suggestions.extend(vec![
                    HelpSuggestion {
                        text: "How to add your first todo".to_string(),
                        example: Some("Add a todo to learn the basics".to_string()),
                        category: HelpCategory::GettingStarted,
                        relevance: 0.9,
                        contextual: true,
                    },
                    HelpSuggestion {
                        text: "How to create a goal".to_string(),
                        example: Some("Create a goal to be more productive".to_string()),
                        category: HelpCategory::GettingStarted,
                        relevance: 0.8,
                        contextual: true,
                    },
                ]);
            }
            UserLevel::Intermediate => {
                suggestions.extend(vec![
                    HelpSuggestion {
                        text: "How to organize tasks with tags".to_string(),
                        example: Some("Add a todo tagged with work and urgent".to_string()),
                        category: HelpCategory::TodoManagement,
                        relevance: 0.7,
                        contextual: true,
                    },
                    HelpSuggestion {
                        text: "How to track goal progress".to_string(),
                        example: Some("Update my fitness goal progress".to_string()),
                        category: HelpCategory::GoalTracking,
                        relevance: 0.6,
                        contextual: true,
                    },
                ]);
            }
            UserLevel::Advanced => {
                suggestions.extend(vec![
                    HelpSuggestion {
                        text: "How to set up automated workflows".to_string(),
                        example: Some("What workflows are available?".to_string()),
                        category: HelpCategory::AdvancedFeatures,
                        relevance: 0.6,
                        contextual: true,
                    },
                ]);
            }
        }
        
        Ok(suggestions)
    }

    /// Suggest examples based on context
    async fn suggest_examples(&self, context: &HelpContext) -> Result<Vec<HelpSuggestion>> {
        let mut suggestions = Vec::new();
        
        // Get examples for common commands
        for (_intent, examples) in &self.command_examples {
            let filtered_examples: Vec<_> = examples.iter()
                .filter(|example| self.example_matches_user_level(&example.difficulty, &context.user_level))
                .collect();
            
            if !filtered_examples.is_empty() {
                let example = filtered_examples.first().unwrap();
                suggestions.push(HelpSuggestion {
                    text: format!("Example: {}", example.description),
                    example: Some(example.command.clone()),
                    category: HelpCategory::GettingStarted,
                    relevance: 0.7,
                    contextual: true,
                });
            }
        }
        
        Ok(suggestions)
    }

    /// Check if workflow matches context
    fn workflow_matches_context(&self, pattern: &WorkflowPattern, context: &HelpContext) -> bool {
        pattern.trigger_conditions.iter().any(|condition| {
            match condition.as_str() {
                "morning" => matches!(context.time_context, TimeContext::Morning),
                "empty_todos" => context.project_state.open_todos == 0,
                "empty_goals" => context.project_state.active_goals == 0,
                "new_user" => matches!(context.user_level, UserLevel::Beginner),
                "high_activity" => matches!(context.project_state.activity_level, ActivityLevel::High),
                "many_todos" => context.project_state.open_todos > 5,
                _ => false,
            }
        })
    }

    /// Check if example matches user level
    fn example_matches_user_level(&self, example_level: &UserLevel, user_level: &UserLevel) -> bool {
        match (user_level, example_level) {
            (UserLevel::Beginner, UserLevel::Beginner) => true,
            (UserLevel::Intermediate, UserLevel::Beginner | UserLevel::Intermediate) => true,
            (UserLevel::Advanced, _) => true,
            _ => false,
        }
    }
}

/// Types of discovery queries
#[derive(Debug, Clone)]
enum DiscoveryQueryType {
    /// "What can I do next?"
    NextActions,
    /// "How do I...?"
    HowTo,
    /// "Help me with..."
    GeneralHelp,
    /// "Show me examples"
    Examples,
}

/// Helper function to determine user level based on session history
pub fn determine_user_level(session: Option<&ConversationSession>) -> UserLevel {
    if let Some(session) = session {
        let command_count = session.history.len();
        match command_count {
            0..=5 => UserLevel::Beginner,
            6..=20 => UserLevel::Intermediate,
            _ => UserLevel::Advanced,
        }
    } else {
        UserLevel::Beginner
    }
}

/// Helper function to determine time context
pub fn determine_time_context() -> TimeContext {
    let now = Utc::now();
    let hour = now.hour();
    
    match hour {
        5..=11 => TimeContext::Morning,
        12..=17 => TimeContext::Afternoon,
        _ => TimeContext::Evening,
    }
}

/// Helper function to create mock project state
pub fn create_mock_project_state() -> ProjectState {
    ProjectState {
        open_todos: 3,
        active_goals: 2,
        note_count: 5,
        activity_level: ActivityLevel::Normal,
        workflow_patterns: vec![
            "morning_planning".to_string(),
            "task_completion".to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_conversation::{Message, MessageRole};
    use async_trait::async_trait;
    use chrono::Utc;

    // Mock LLM client for testing
    struct MockLLMClient;

    #[async_trait]
    impl LLMClient for MockLLMClient {
        async fn send_message(&self, _messages: Vec<Message>) -> Result<Message> {
            Ok(Message {
                id: "test".to_string(),
                role: MessageRole::Assistant,
                content: "Test response".to_string(),
                timestamp: Utc::now(),
                function_call: None,
            })
        }

        async fn stream_response(&self, messages: Vec<Message>) -> Result<tokio::sync::mpsc::Receiver<String>> {
            let (tx, rx) = tokio::sync::mpsc::channel(100);
            let response = self.send_message(messages).await?;
            let _ = tx.send(response.content).await;
            Ok(rx)
        }

        async fn function_calling(&self, messages: Vec<Message>, _functions: Vec<crate::ai_conversation::FunctionSchema>) -> Result<Message> {
            self.send_message(messages).await
        }

        fn get_model_name(&self) -> String {
            "mock-model".to_string()
        }
    }

    #[tokio::test]
    async fn test_help_system_creation() {
        let help_system = IntelligentHelpSystem::new(None);
        assert!(!help_system.base_suggestions.is_empty());
        assert!(!help_system.command_examples.is_empty());
        assert!(!help_system.workflow_patterns.is_empty());
    }

    #[tokio::test]
    async fn test_generate_help_suggestions() {
        let help_system = IntelligentHelpSystem::new(None);
        
        let context = HelpContext {
            session: None,
            recent_commands: Vec::new(),
            project_state: create_mock_project_state(),
            user_level: UserLevel::Beginner,
            time_context: TimeContext::Morning,
        };
        
        let suggestions = help_system.generate_help_suggestions(&context).await.unwrap();
        assert!(!suggestions.is_empty());
        
        // Check that suggestions are sorted by relevance
        for i in 1..suggestions.len() {
            assert!(suggestions[i-1].relevance >= suggestions[i].relevance);
        }
    }

    #[tokio::test]
    async fn test_discovery_queries() {
        let help_system = IntelligentHelpSystem::new(None);
        
        let context = HelpContext {
            session: None,
            recent_commands: Vec::new(),
            project_state: create_mock_project_state(),
            user_level: UserLevel::Beginner,
            time_context: TimeContext::Morning,
        };
        
        // Test different types of discovery queries
        let queries = vec![
            "What can I do next?",
            "How do I add a todo?",
            "Help me get started",
            "Show me examples",
        ];
        
        for query in queries {
            let suggestions = help_system.handle_discovery_query(query, &context).await.unwrap();
            assert!(!suggestions.is_empty(), "No suggestions for query: {}", query);
        }
    }

    #[tokio::test]
    async fn test_command_examples() {
        let help_system = IntelligentHelpSystem::new(None);
        
        let examples = help_system.get_command_examples("add_todo");
        assert!(!examples.is_empty());
        
        let examples = help_system.get_command_examples("nonexistent_command");
        assert!(examples.is_empty());
    }

    #[tokio::test]
    async fn test_workflow_patterns() {
        let help_system = IntelligentHelpSystem::new(None);
        
        let context = HelpContext {
            session: None,
            recent_commands: Vec::new(),
            project_state: ProjectState {
                open_todos: 0,
                active_goals: 0,
                note_count: 0,
                activity_level: ActivityLevel::Normal,
                workflow_patterns: Vec::new(),
            },
            user_level: UserLevel::Beginner,
            time_context: TimeContext::Morning,
        };
        
        let workflows = help_system.get_relevant_workflows(&context);
        assert!(!workflows.is_empty());
        
        // Should suggest morning planning workflow
        let has_morning_workflow = workflows.iter().any(|w| w.name == "Morning Planning");
        assert!(has_morning_workflow);
    }

    #[test]
    fn test_user_level_determination() {
        // Test with no session
        let level = determine_user_level(None);
        assert!(matches!(level, UserLevel::Beginner));
        
        // Test with session history
        let session = ConversationSession {
            id: "test".to_string(),
            user_id: None,
            created_at: Utc::now(),
            last_active: Utc::now(),
            history: vec![],
            context: crate::conversational_interface::ConversationContext::default(),
            config: crate::conversational_interface::SessionConfig::default(),
        };
        
        let level = determine_user_level(Some(&session));
        assert!(matches!(level, UserLevel::Beginner));
    }

    #[test]
    fn test_time_context_determination() {
        let time_context = determine_time_context();
        // Just verify it returns a valid enum variant
        match time_context {
            TimeContext::Morning | TimeContext::Afternoon | TimeContext::Evening => {
                // Success
            }
        }
    }
}