use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::ai_conversation::{AIConversationEngine, LLMClient, Message, MessageRole};
use crate::cli::Commands;
use crate::nl_command_parser::{NLCommandParser, ParsedCommand};
use async_trait::async_trait;

/// Represents the state of a conversation session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSession {
    /// Unique session identifier
    pub id: String,
    /// User identifier (if applicable)
    pub user_id: Option<String>,
    /// Session creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last interaction timestamp
    pub last_active: DateTime<Utc>,
    /// Conversation history
    pub history: Vec<ConversationTurn>,
    /// Session context and state
    pub context: ConversationContext,
    /// Session configuration
    pub config: SessionConfig,
}

/// Represents a single turn in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    /// Unique turn identifier
    pub id: String,
    /// Timestamp of this turn
    pub timestamp: DateTime<Utc>,
    /// User's input
    pub user_input: String,
    /// Parsed command (if successfully parsed)
    pub parsed_command: Option<ParsedCommand>,
    /// System's response
    pub system_response: String,
    /// Any executed commands
    pub executed_commands: Vec<Commands>,
    /// Turn metadata
    pub metadata: HashMap<String, String>,
}

/// Conversation context for maintaining state across turns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    /// Current task or project being discussed
    pub current_task: Option<String>,
    /// Recent entities mentioned in conversation
    pub entities: HashMap<String, String>,
    /// Conversation topic or theme
    pub topic: Option<String>,
    /// User preferences learned during this session
    pub preferences: HashMap<String, String>,
    /// Pending actions or questions
    pub pending_actions: Vec<String>,
    /// Context from previous commands
    pub command_context: HashMap<String, String>,
}

/// Configuration for conversation sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Maximum number of turns to retain in history
    pub max_history_turns: usize,
    /// Session timeout in minutes
    pub session_timeout_minutes: u64,
    /// Whether to enable proactive suggestions
    pub enable_proactive_suggestions: bool,
    /// Response style preference
    pub response_style: ResponseStyle,
    /// Whether to remember context across sessions
    pub persistent_context: bool,
}

/// Response style options for the conversational interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseStyle {
    /// Brief, concise responses
    Concise,
    /// Detailed explanations
    Detailed,
    /// Friendly, conversational tone
    Friendly,
    /// Professional, formal tone
    Professional,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_history_turns: 50,
            session_timeout_minutes: 30,
            enable_proactive_suggestions: true,
            response_style: ResponseStyle::Friendly,
            persistent_context: true,
        }
    }
}

impl Default for ConversationContext {
    fn default() -> Self {
        Self {
            current_task: None,
            entities: HashMap::new(),
            topic: None,
            preferences: HashMap::new(),
            pending_actions: Vec::new(),
            command_context: HashMap::new(),
        }
    }
}

/// Simple mock LLM client for basic conversational responses
struct MockLLMClient {
    default_response: String,
}

impl MockLLMClient {
    fn new() -> Self {
        Self {
            default_response: "I understand. How can I help you further?".to_string(),
        }
    }
}

#[async_trait]
impl LLMClient for MockLLMClient {
    async fn send_message(&self, _messages: Vec<Message>) -> Result<Message> {
        Ok(Message {
            id: "mock".to_string(),
            role: MessageRole::Assistant,
            content: self.default_response.clone(),
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
        "mock-conversational-model".to_string()
    }
}

/// Main conversational interface manager
pub struct ConversationalInterface {
    /// AI conversation engine
    ai_engine: AIConversationEngine,
    /// Natural language command parser
    nl_parser: NLCommandParser,
    /// Active conversation sessions
    sessions: HashMap<String, ConversationSession>,
    /// Interface configuration
    config: ConversationalInterfaceConfig,
}

/// Configuration for the conversational interface
#[derive(Debug, Clone)]
pub struct ConversationalInterfaceConfig {
    /// Maximum number of active sessions
    pub max_active_sessions: usize,
    /// Session cleanup interval in minutes
    pub cleanup_interval_minutes: u64,
    /// Whether to enable session persistence
    pub session_persistence: bool,
    /// Default session configuration
    pub default_session_config: SessionConfig,
}

impl Default for ConversationalInterfaceConfig {
    fn default() -> Self {
        Self {
            max_active_sessions: 100,
            cleanup_interval_minutes: 60,
            session_persistence: true,
            default_session_config: SessionConfig::default(),
        }
    }
}

impl ConversationalInterface {
    /// Create a new conversational interface
    pub fn new(
        nl_parser: NLCommandParser,
        config: ConversationalInterfaceConfig,
    ) -> Result<Self> {
        // Create a simple AI engine without LLM client for now
        // We'll handle AI responses directly in the conversational interface
        let ai_engine = AIConversationEngine::new(Box::new(MockLLMClient::new()));
        
        Ok(Self {
            ai_engine,
            nl_parser,
            sessions: HashMap::new(),
            config,
        })
    }

    /// Start a new conversation session
    pub async fn start_session(&mut self, user_id: Option<String>) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        
        let session = ConversationSession {
            id: session_id.clone(),
            user_id,
            created_at: now,
            last_active: now,
            history: Vec::new(),
            context: ConversationContext::default(),
            config: self.config.default_session_config.clone(),
        };
        
        self.sessions.insert(session_id.clone(), session);
        
        // Cleanup old sessions if we're at the limit
        if self.sessions.len() > self.config.max_active_sessions {
            self.cleanup_old_sessions().await?;
        }
        
        Ok(session_id)
    }

    /// Process a user input and generate a response
    pub async fn process_input(
        &mut self,
        session_id: &str,
        user_input: &str,
    ) -> Result<ConversationResponse> {
        // Try to parse the input as a command first
        let parsed_command = match self.nl_parser.parse_command(user_input).await {
            Ok(cmd) => Some(cmd),
            Err(_) => None, // Continue as conversational input if parsing fails
        };
        
        // Create conversation turn
        let turn_id = Uuid::new_v4().to_string();
        let mut turn = ConversationTurn {
            id: turn_id.clone(),
            timestamp: Utc::now(),
            user_input: user_input.to_string(),
            parsed_command: parsed_command.clone(),
            system_response: String::new(),
            executed_commands: Vec::new(),
            metadata: HashMap::new(),
        };
        
        // Generate response based on whether we have a parsed command
        let response = if let Some(ref cmd) = parsed_command {
            self.handle_command_response(session_id, cmd, &mut turn).await?
        } else {
            self.handle_conversational_response(session_id, user_input, &mut turn).await?
        };
        
        // Now update the session with the turn
        {
            let session = self.sessions.get_mut(session_id)
                .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
            
            // Update last active time
            session.last_active = Utc::now();
            
            // Update context with entities from parsed command
            if let Some(ref cmd) = parsed_command {
                for (key, value) in &cmd.entities {
                    session.context.entities.insert(key.clone(), value.clone());
                }
            }
            
            // Add turn to session history
            session.history.push(turn);
            
            // Trim history if it exceeds the limit
            if session.history.len() > session.config.max_history_turns {
                session.history.remove(0);
            }
        }
        
        Ok(response)
    }

    /// Handle a command-based response
    async fn handle_command_response(
        &mut self,
        session_id: &str,
        command: &ParsedCommand,
        turn: &mut ConversationTurn,
    ) -> Result<ConversationResponse> {
        // Convert parsed command to CLI command
        let cli_command = self.nl_parser.to_cli_command(command)?;
        
        // Store the command for execution
        turn.executed_commands.push(cli_command.clone());
        
        // Generate appropriate response based on command type
        let response_text = {
            let session = self.sessions.get(session_id)
                .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
            self.generate_command_response(command, &cli_command, session).await?
        };
        
        // Update turn response
        turn.system_response = response_text.clone();
        
        // Update context based on command
        {
            let session = self.sessions.get_mut(session_id)
                .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
            Self::update_context_from_command_static(session, command, &cli_command).await?;
        }
        
        let suggestions = {
            let session = self.sessions.get(session_id)
                .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
            self.generate_suggestions(session).await?
        };
        
        Ok(ConversationResponse {
            text: response_text,
            commands: vec![cli_command],
            suggestions,
            requires_clarification: false,
            clarification_question: None,
            context_updated: true,
        })
    }

    /// Handle a conversational response (no command parsed)
    async fn handle_conversational_response(
        &mut self,
        session_id: &str,
        user_input: &str,
        turn: &mut ConversationTurn,
    ) -> Result<ConversationResponse> {
        // Create context-aware prompt for the AI
        let context_prompt = {
            let session = self.sessions.get(session_id)
                .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
            self.create_context_prompt(session, user_input)
        };
        
        // Generate response using AI conversation engine
        let ai_response = self.ai_engine.send_message(context_prompt).await?;
        
        // Update turn response
        turn.system_response = ai_response.clone();
        
        // Check if we need clarification
        let (requires_clarification, clarification_question) = 
            self.check_clarification_needed(user_input, &ai_response);
        
        let suggestions = {
            let session = self.sessions.get(session_id)
                .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
            self.generate_suggestions(session).await?
        };
        
        Ok(ConversationResponse {
            text: ai_response,
            commands: Vec::new(),
            suggestions,
            requires_clarification,
            clarification_question,
            context_updated: false,
        })
    }

    /// Generate a response for a successfully parsed command
    async fn generate_command_response(
        &self,
        command: &ParsedCommand,
        _cli_command: &Commands,
        session: &ConversationSession,
    ) -> Result<String> {
        let response = match command.intent.as_str() {
            "add_todo" => {
                let description = command.entities.get("description").map(|s| s.as_str()).unwrap_or("task");
                format!("I'll add a new todo: '{}'. This will help you track this task.", description)
            }
            "list_todos" => {
                "I'll show you all your current todos. Let me retrieve them for you.".to_string()
            }
            "complete_todo" => {
                let id = command.entities.get("todo_id").map(|s| s.as_str()).unwrap_or("task");
                format!("Great! I'll mark todo '{}' as complete. Well done!", id)
            }
            "add_goal" => {
                let title = command.entities.get("title").map(|s| s.as_str()).unwrap_or("goal");
                format!("I'll create a new goal: '{}'. Setting goals is a great way to stay focused!", title)
            }
            "create_note" => {
                let title = command.entities.get("title").map(|s| s.as_str()).unwrap_or("note");
                format!("I'll create a new note titled '{}'. This will help you capture your thoughts.", title)
            }
            "start_chat" => {
                "I'm ready to chat! How can I help you today?".to_string()
            }
            _ => format!("I'll execute the '{}' command for you.", command.intent),
        };
        
        // Add personalized touch based on session history
        let personalized_response = self.personalize_response(response, session).await?;
        
        Ok(personalized_response)
    }

    /// Personalize response based on session history and context
    async fn personalize_response(
        &self,
        response: String,
        session: &ConversationSession,
    ) -> Result<String> {
        // Add context-aware personalization
        let mut personalized = response;
        
        // Add user's name if available
        if let Some(user_id) = &session.user_id {
            if !personalized.contains(&user_id.to_lowercase()) {
                personalized = format!("{} {}", personalized, 
                    self.get_appropriate_closing(session));
            }
        }
        
        // Add encouraging words based on session history
        if session.history.len() > 5 {
            personalized = format!("{} You're making great progress!", personalized);
        }
        
        Ok(personalized)
    }

    /// Get appropriate closing based on response style
    fn get_appropriate_closing(&self, session: &ConversationSession) -> String {
        match session.config.response_style {
            ResponseStyle::Friendly => "😊".to_string(),
            ResponseStyle::Professional => "".to_string(),
            ResponseStyle::Detailed => "Let me know if you need any clarification!".to_string(),
            ResponseStyle::Concise => "".to_string(),
        }
    }

    /// Create context-aware prompt for AI responses
    fn create_context_prompt(&self, session: &ConversationSession, user_input: &str) -> String {
        let mut prompt = String::new();
        
        // Add context from current session
        if let Some(ref topic) = session.context.topic {
            prompt.push_str(&format!("Current topic: {}\n", topic));
        }
        
        if let Some(ref task) = session.context.current_task {
            prompt.push_str(&format!("Current task: {}\n", task));
        }
        
        // Add recent conversation history for context
        let recent_turns = session.history.iter().rev().take(3).collect::<Vec<_>>();
        if !recent_turns.is_empty() {
            prompt.push_str("Recent conversation:\n");
            for turn in recent_turns.iter().rev() {
                prompt.push_str(&format!("User: {}\n", turn.user_input));
                prompt.push_str(&format!("Assistant: {}\n", turn.system_response));
            }
            prompt.push_str("\n");
        }
        
        // Add current user input
        prompt.push_str(&format!("User: {}\n", user_input));
        prompt.push_str("Assistant: ");
        
        prompt
    }

    /// Update conversation context based on executed command
    async fn update_context_from_command(
        &self,
        session: &mut ConversationSession,
        command: &ParsedCommand,
        _cli_command: &Commands,
    ) -> Result<()> {
        Self::update_context_from_command_static(session, command, _cli_command).await
    }

    /// Static version of update_context_from_command to avoid borrowing issues
    async fn update_context_from_command_static(
        session: &mut ConversationSession,
        command: &ParsedCommand,
        _cli_command: &Commands,
    ) -> Result<()> {
        // Update current task context
        match command.intent.as_str() {
            "add_todo" | "view_todo" | "complete_todo" => {
                session.context.current_task = Some("task_management".to_string());
            }
            "add_goal" | "view_goal" | "update_goal" => {
                session.context.current_task = Some("goal_setting".to_string());
            }
            "create_note" | "view_note" | "edit_note" => {
                session.context.current_task = Some("note_taking".to_string());
            }
            _ => {}
        }
        
        // Store command context for future reference
        session.context.command_context.insert(
            command.intent.clone(),
            serde_json::to_string(command)?,
        );
        
        Ok(())
    }

    /// Generate contextual suggestions for the user
    async fn generate_suggestions(&self, session: &ConversationSession) -> Result<Vec<String>> {
        let mut suggestions = Vec::new();
        
        // Generate suggestions based on current context
        if let Some(ref task) = session.context.current_task {
            match task.as_str() {
                "task_management" => {
                    suggestions.push("Would you like to see your task list?".to_string());
                    suggestions.push("Need help setting a deadline?".to_string());
                }
                "goal_setting" => {
                    suggestions.push("Want to break this goal into smaller tasks?".to_string());
                    suggestions.push("Should we set a target date?".to_string());
                }
                "note_taking" => {
                    suggestions.push("Would you like to organize your notes?".to_string());
                    suggestions.push("Need help finding related notes?".to_string());
                }
                _ => {}
            }
        }
        
        // Add general suggestions if no specific context
        if suggestions.is_empty() {
            suggestions.push("What would you like to work on today?".to_string());
            suggestions.push("Need help with tasks, goals, or notes?".to_string());
        }
        
        Ok(suggestions)
    }

    /// Check if clarification is needed based on user input
    fn check_clarification_needed(&self, user_input: &str, _ai_response: &str) -> (bool, Option<String>) {
        // Simple heuristics to determine if clarification is needed
        let unclear_indicators = [
            "not sure", "maybe", "unclear", "don't know", "help me decide"
        ];
        
        let input_lower = user_input.to_lowercase();
        let needs_clarification = unclear_indicators.iter()
            .any(|indicator| input_lower.contains(indicator));
        
        if needs_clarification {
            let clarification = Some(
                "I'd like to help you better. Could you provide more specific details about what you'd like to do?".to_string()
            );
            (true, clarification)
        } else {
            (false, None)
        }
    }

    /// Get a conversation session by ID
    pub fn get_session(&self, session_id: &str) -> Option<&ConversationSession> {
        self.sessions.get(session_id)
    }

    /// Get mutable reference to a conversation session
    pub fn get_session_mut(&mut self, session_id: &str) -> Option<&mut ConversationSession> {
        self.sessions.get_mut(session_id)
    }

    /// End a conversation session
    pub async fn end_session(&mut self, session_id: &str) -> Result<()> {
        if let Some(session) = self.sessions.remove(session_id) {
            // Save session if persistence is enabled
            if self.config.session_persistence {
                self.save_session(&session).await?;
            }
        }
        Ok(())
    }

    /// Save a session to persistent storage
    async fn save_session(&self, session: &ConversationSession) -> Result<()> {
        // In a real implementation, this would save to a database or file
        // For now, we'll just log the session
        println!("Saving session: {} with {} turns", session.id, session.history.len());
        Ok(())
    }

    /// Cleanup old and inactive sessions
    async fn cleanup_old_sessions(&mut self) -> Result<()> {
        let timeout_duration = chrono::Duration::minutes(self.config.cleanup_interval_minutes as i64);
        let cutoff_time = Utc::now() - timeout_duration;
        
        let mut sessions_to_remove = Vec::new();
        
        // First, remove sessions that are older than the timeout
        for (session_id, session) in &self.sessions {
            if session.last_active < cutoff_time {
                sessions_to_remove.push(session_id.clone());
            }
        }
        
        // If we still have too many sessions after removing old ones,
        // remove the oldest remaining sessions
        if self.sessions.len() > self.config.max_active_sessions {
            let mut sessions_by_age: Vec<_> = self.sessions.iter()
                .filter(|(id, _)| !sessions_to_remove.contains(id))
                .collect();
            
            sessions_by_age.sort_by(|a, b| a.1.last_active.cmp(&b.1.last_active));
            
            let excess_count = self.sessions.len() - self.config.max_active_sessions;
            for (session_id, _) in sessions_by_age.iter().take(excess_count) {
                sessions_to_remove.push(session_id.to_string());
            }
        }
        
        for session_id in sessions_to_remove {
            self.end_session(&session_id).await?;
        }
        
        Ok(())
    }

    /// Get conversation statistics
    pub fn get_stats(&self) -> ConversationStats {
        let total_sessions = self.sessions.len();
        let total_turns = self.sessions.values()
            .map(|s| s.history.len())
            .sum::<usize>();
        
        let avg_turns_per_session = if total_sessions > 0 {
            total_turns as f64 / total_sessions as f64
        } else {
            0.0
        };
        
        ConversationStats {
            total_sessions,
            total_turns,
            avg_turns_per_session,
            active_sessions: self.sessions.len(),
        }
    }
}

/// Response from the conversational interface
#[derive(Debug, Clone)]
pub struct ConversationResponse {
    /// The text response to display to the user
    pub text: String,
    /// Any commands that should be executed
    pub commands: Vec<Commands>,
    /// Suggestions for the user
    pub suggestions: Vec<String>,
    /// Whether clarification is needed
    pub requires_clarification: bool,
    /// Clarification question if needed
    pub clarification_question: Option<String>,
    /// Whether the context was updated
    pub context_updated: bool,
}

/// Statistics about the conversational interface
#[derive(Debug, Clone)]
pub struct ConversationStats {
    /// Total number of sessions created
    pub total_sessions: usize,
    /// Total number of conversation turns
    pub total_turns: usize,
    /// Average turns per session
    pub avg_turns_per_session: f64,
    /// Number of currently active sessions
    pub active_sessions: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_conversation::{Message, MessageRole};
    use async_trait::async_trait;

    // Mock LLM client for testing
    struct MockLLMClient {
        responses: Vec<String>,
        current_response: std::sync::Mutex<usize>,
    }

    impl MockLLMClient {
        fn new(responses: Vec<String>) -> Self {
            Self {
                responses,
                current_response: std::sync::Mutex::new(0),
            }
        }
    }

    #[async_trait]
    impl LLMClient for MockLLMClient {
        async fn send_message(&self, _messages: Vec<Message>) -> Result<Message> {
            let mut current = self.current_response.lock().unwrap();
            let response_text = if *current < self.responses.len() {
                self.responses[*current].clone()
            } else {
                "I understand. How can I help you further?".to_string()
            };
            *current += 1;
            
            Ok(Message {
                id: "test".to_string(),
                role: MessageRole::Assistant,
                content: response_text,
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
    async fn test_conversation_session_creation() {
        let mock_client = Box::new(MockLLMClient::new(vec![
            "Hello! How can I help you today?".to_string(),
        ]));
        
        let config = ConversationalInterfaceConfig::default();
        let nl_parser = NLCommandParser::new(mock_client);
        let mut interface = ConversationalInterface::new(nl_parser, config).unwrap();
        
        let session_id = interface.start_session(Some("test_user".to_string())).await.unwrap();
        
        let session = interface.get_session(&session_id).unwrap();
        assert_eq!(session.user_id, Some("test_user".to_string()));
        assert_eq!(session.history.len(), 0);
    }

    #[tokio::test]
    async fn test_command_parsing_and_execution() {
        let mock_client = Box::new(MockLLMClient::new(vec![
            r#"{"intent": "add_todo", "entities": {"description": "test task"}, "confidence": 0.9}"#.to_string(),
            "I'll add that task for you!".to_string(),
        ]));
        
        let config = ConversationalInterfaceConfig::default();
        let nl_parser = NLCommandParser::new(mock_client);
        let mut interface = ConversationalInterface::new(nl_parser, config).unwrap();
        
        let session_id = interface.start_session(None).await.unwrap();
        
        let response = interface.process_input(&session_id, "Add a todo to test the system").await.unwrap();
        
        assert!(!response.commands.is_empty());
        assert!(response.text.contains("add"));
        assert!(!response.suggestions.is_empty());
    }

    #[tokio::test]
    async fn test_conversational_response() {
        let mock_client = Box::new(MockLLMClient::new(vec![
            "I'm here to help you manage your tasks and goals!".to_string(),
        ]));
        
        let config = ConversationalInterfaceConfig::default();
        let nl_parser = NLCommandParser::new(mock_client);
        let mut interface = ConversationalInterface::new(nl_parser, config).unwrap();
        
        let session_id = interface.start_session(None).await.unwrap();
        
        let response = interface.process_input(&session_id, "Hello, how are you?").await.unwrap();
        
        assert!(response.commands.is_empty());
        assert!(response.text.contains("help"));
        assert!(!response.suggestions.is_empty());
    }

    #[tokio::test]
    async fn test_context_retention() {
        let mock_client = Box::new(MockLLMClient::new(vec![
            r#"{"intent": "add_todo", "entities": {"description": "first task"}, "confidence": 0.9}"#.to_string(),
            "Task added successfully!".to_string(),
            "Your current focus is on task management.".to_string(),
        ]));
        
        let config = ConversationalInterfaceConfig::default();
        let nl_parser = NLCommandParser::new(mock_client);
        let mut interface = ConversationalInterface::new(nl_parser, config).unwrap();
        
        let session_id = interface.start_session(None).await.unwrap();
        
        // First interaction - add a todo
        let _response1 = interface.process_input(&session_id, "Add a todo for first task").await.unwrap();
        
        // Second interaction - should have context about task management
        let _response2 = interface.process_input(&session_id, "What am I working on?").await.unwrap();
        
        let session = interface.get_session(&session_id).unwrap();
        assert_eq!(session.history.len(), 2);
        assert_eq!(session.context.current_task, Some("task_management".to_string()));
    }

    #[tokio::test]
    async fn test_session_cleanup() {
        let mock_client = Box::new(MockLLMClient::new(vec![]));
        
        let mut config = ConversationalInterfaceConfig::default();
        config.max_active_sessions = 2;
        
        let nl_parser = NLCommandParser::new(mock_client);
        let mut interface = ConversationalInterface::new(nl_parser, config).unwrap();
        
        // Create 3 sessions (exceeds max)
        let _session1 = interface.start_session(None).await.unwrap();
        let _session2 = interface.start_session(None).await.unwrap();
        let _session3 = interface.start_session(None).await.unwrap();
        
        // Should have cleaned up to max sessions
        assert!(interface.sessions.len() <= 2);
    }

    #[tokio::test]
    async fn test_response_personalization() {
        let mock_client = Box::new(MockLLMClient::new(vec![
            r#"{"intent": "add_todo", "entities": {"description": "personal task"}, "confidence": 0.9}"#.to_string(),
        ]));
        
        let mut config = ConversationalInterfaceConfig::default();
        config.default_session_config.response_style = ResponseStyle::Friendly;
        
        let nl_parser = NLCommandParser::new(mock_client);
        let mut interface = ConversationalInterface::new(nl_parser, config).unwrap();
        
        let session_id = interface.start_session(Some("Alice".to_string())).await.unwrap();
        
        let response = interface.process_input(&session_id, "Add a todo for personal task").await.unwrap();
        
        // Should contain personalized response
        assert!(response.text.len() > 0);
    }

    #[tokio::test]
    async fn test_conversation_stats() {
        let mock_client = Box::new(MockLLMClient::new(vec![
            "Hello!".to_string(),
            "How can I help?".to_string(),
        ]));
        
        let config = ConversationalInterfaceConfig::default();
        let nl_parser = NLCommandParser::new(mock_client);
        let mut interface = ConversationalInterface::new(nl_parser, config).unwrap();
        
        let session_id = interface.start_session(None).await.unwrap();
        let _response1 = interface.process_input(&session_id, "Hello").await.unwrap();
        let _response2 = interface.process_input(&session_id, "Help me").await.unwrap();
        
        let stats = interface.get_stats();
        assert_eq!(stats.total_sessions, 1);
        assert_eq!(stats.total_turns, 2);
        assert_eq!(stats.avg_turns_per_session, 2.0);
        assert_eq!(stats.active_sessions, 1);
    }
}