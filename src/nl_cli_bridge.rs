use anyhow::Result;
use std::collections::HashMap;

use crate::ai_conversation::LLMClient;
use crate::cli::Commands;
use crate::conversational_interface::{ConversationalInterface, ConversationalInterfaceConfig, ConversationResponse};
use crate::nl_command_parser::{NLCommandParser, ParsedCommand, DisambiguationQuestion};
use crate::obsidian_adapter::ObsidianAdapter;
use crate::router::route_command;
use crate::cli::Cli;
use crate::intelligent_help::{IntelligentHelpSystem, HelpContext, HelpSuggestion, determine_user_level, determine_time_context, create_mock_project_state};

/// Bridge between natural language interface and existing CLI system
pub struct NLCLIBridge {
    /// Conversational interface for managing sessions
    conversational_interface: ConversationalInterface,
    /// Active disambiguation sessions
    disambiguation_sessions: HashMap<String, DisambiguationQuestion>,
    /// Intelligent help system
    help_system: IntelligentHelpSystem,
}

/// Response from the NL CLI bridge
#[derive(Debug, Clone)]
pub struct NLBridgeResponse {
    /// The response text to show to the user
    pub response_text: String,
    /// Commands that were executed
    pub executed_commands: Vec<Commands>,
    /// Suggestions for the user
    pub suggestions: Vec<String>,
    /// Whether disambiguation is required
    pub requires_disambiguation: bool,
    /// Disambiguation options if needed
    pub disambiguation_question: Option<DisambiguationQuestion>,
    /// Whether the command was executed successfully
    pub execution_successful: bool,
    /// Any error messages from command execution
    pub error_message: Option<String>,
    /// Intelligent help suggestions
    pub help_suggestions: Vec<HelpSuggestion>,
}

/// Configuration for the NL CLI bridge
#[derive(Debug, Clone)]
pub struct NLBridgeConfig {
    /// Whether to execute commands immediately or just parse them
    pub auto_execute: bool,
    /// Whether to show detailed execution feedback
    pub verbose_feedback: bool,
    /// Whether to provide suggestions after each command
    pub provide_suggestions: bool,
}

impl Default for NLBridgeConfig {
    fn default() -> Self {
        Self {
            auto_execute: true,
            verbose_feedback: true,
            provide_suggestions: true,
        }
    }
}

impl NLCLIBridge {
    /// Create a new NL CLI bridge
    pub fn new(
        command_client: Box<dyn LLMClient>,
        conversation_client: Box<dyn LLMClient>,
        _config: NLBridgeConfig,
    ) -> Result<Self> {
        // Create natural language parser
        let nl_parser = NLCommandParser::new(command_client);
        
        // Create conversational interface with the real AI client
        let conv_config = ConversationalInterfaceConfig::default();
        let conversational_interface = ConversationalInterface::new(nl_parser, conv_config, conversation_client)?;
        
        // Create help system
        let help_system = IntelligentHelpSystem::new(None);
        
        Ok(Self {
            conversational_interface,
            disambiguation_sessions: HashMap::new(),
            help_system,
        })
    }

    /// Process natural language input and return response
    pub async fn process_input(
        &mut self,
        session_id: &str,
        user_input: &str,
        adapter: &ObsidianAdapter,
        config: &NLBridgeConfig,
    ) -> Result<NLBridgeResponse> {
        // Check if this is a disambiguation response
        if let Some(question) = self.disambiguation_sessions.get(session_id).cloned() {
            return self.handle_disambiguation_response(session_id, user_input, &question, adapter, config).await;
        }

        // Check if this is a help or discovery query
        if self.is_help_query(user_input) {
            return self.handle_help_query(session_id, user_input, config).await;
        }

        // Process the input through the conversational interface
        let conv_response = self.conversational_interface.process_input(session_id, user_input).await?;
        
        // Handle the response based on whether commands were parsed
        if !conv_response.commands.is_empty() {
            self.handle_command_execution(session_id, conv_response, adapter, config).await
        } else {
            self.handle_conversational_response(session_id, conv_response, config).await
        }
    }

    /// Handle execution of parsed commands
    async fn handle_command_execution(
        &mut self,
        session_id: &str,
        conv_response: ConversationResponse,
        adapter: &ObsidianAdapter,
        config: &NLBridgeConfig,
    ) -> Result<NLBridgeResponse> {
        let mut executed_commands = Vec::new();
        let mut execution_errors = Vec::new();
        let mut response_parts = Vec::new();

        // Add conversational response text
        response_parts.push(conv_response.text.clone());

        // Execute commands if auto-execute is enabled
        if config.auto_execute {
            for command in &conv_response.commands {
                match self.execute_command(command.clone(), adapter).await {
                    Ok(output) => {
                        executed_commands.push(command.clone());
                        if config.verbose_feedback {
                            response_parts.push(format!("✓ Command executed successfully: {}", output));
                        }
                    }
                    Err(e) => {
                        execution_errors.push(format!("Failed to execute command: {}", e));
                        if config.verbose_feedback {
                            response_parts.push(format!("✗ Command execution failed: {}", e));
                        }
                    }
                }
            }
        } else {
            // Just show what would be executed
            for command in &conv_response.commands {
                response_parts.push(format!("Would execute: {:?}", command));
            }
        }

        // Check for disambiguation needs
        let session = self.conversational_interface.get_session(session_id);
        let needs_disambiguation = self.check_disambiguation_needed(session, &conv_response).await?;

        let final_response = if needs_disambiguation.is_some() {
            let question = needs_disambiguation.unwrap();
            self.disambiguation_sessions.insert(session_id.to_string(), question.clone());
            
            response_parts.push("I need some clarification:".to_string());
            response_parts.push(question.question.clone());
            
            for (i, option) in question.options.iter().enumerate() {
                response_parts.push(format!("{}. {}", i + 1, option.display_text));
            }
            response_parts.push("Please choose an option by number.".to_string());
            
            NLBridgeResponse {
                response_text: response_parts.join("\n"),
                executed_commands,
                suggestions: Vec::new(),
                requires_disambiguation: true,
                disambiguation_question: Some(question),
                execution_successful: execution_errors.is_empty(),
                error_message: if execution_errors.is_empty() { None } else { Some(execution_errors.join("; ")) },
                help_suggestions: Vec::new(),
            }
        } else {
            // Add suggestions if enabled
            let suggestions = if config.provide_suggestions {
                conv_response.suggestions
            } else {
                Vec::new()
            };

            NLBridgeResponse {
                response_text: response_parts.join("\n"),
                executed_commands,
                suggestions,
                requires_disambiguation: false,
                disambiguation_question: None,
                execution_successful: execution_errors.is_empty(),
                error_message: if execution_errors.is_empty() { None } else { Some(execution_errors.join("; ")) },
                help_suggestions: Vec::new(),
            }
        };

        Ok(final_response)
    }

    /// Handle conversational response (no commands to execute)
    async fn handle_conversational_response(
        &mut self,
        _session_id: &str,
        conv_response: ConversationResponse,
        config: &NLBridgeConfig,
    ) -> Result<NLBridgeResponse> {
        let suggestions = if config.provide_suggestions {
            conv_response.suggestions
        } else {
            Vec::new()
        };

        Ok(NLBridgeResponse {
            response_text: conv_response.text,
            executed_commands: Vec::new(),
            suggestions,
            requires_disambiguation: conv_response.requires_clarification,
            disambiguation_question: None,
            execution_successful: true,
            error_message: None,
            help_suggestions: Vec::new(),
        })
    }

    /// Handle disambiguation response from user
    async fn handle_disambiguation_response(
        &mut self,
        session_id: &str,
        user_input: &str,
        question: &DisambiguationQuestion,
        adapter: &ObsidianAdapter,
        config: &NLBridgeConfig,
    ) -> Result<NLBridgeResponse> {
        // Parse user's choice
        let choice = self.parse_disambiguation_choice(user_input, question)?;
        
        // Remove the disambiguation session
        self.disambiguation_sessions.remove(session_id);
        
        // Get the chosen option
        let option = &question.options[choice];
        
        // Create a CLI command from the chosen option
        let cli_command = self.create_cli_command_from_option(option)?;
        
        // Execute the command
        let mut response_parts = vec![
            format!("You chose: {}", option.display_text),
            "Executing your command...".to_string(),
        ];

        let execution_result = if config.auto_execute {
            match self.execute_command(cli_command.clone(), adapter).await {
                Ok(output) => {
                    response_parts.push(format!("✓ Command executed successfully: {}", output));
                    Ok(vec![cli_command])
                }
                Err(e) => {
                    response_parts.push(format!("✗ Command execution failed: {}", e));
                    Err(e)
                }
            }
        } else {
            response_parts.push(format!("Would execute: {:?}", cli_command));
            Ok(vec![cli_command])
        };

        match execution_result {
            Ok(executed_commands) => {
                Ok(NLBridgeResponse {
                    response_text: response_parts.join("\n"),
                    executed_commands,
                    suggestions: Vec::new(),
                    requires_disambiguation: false,
                    disambiguation_question: None,
                    execution_successful: true,
                    error_message: None,
                    help_suggestions: Vec::new(),
                })
            }
            Err(e) => {
                Ok(NLBridgeResponse {
                    response_text: response_parts.join("\n"),
                    executed_commands: Vec::new(),
                    suggestions: Vec::new(),
                    requires_disambiguation: false,
                    disambiguation_question: None,
                    execution_successful: false,
                    error_message: Some(e.to_string()),
                    help_suggestions: Vec::new(),
                })
            }
        }
    }

    /// Execute a CLI command using the existing router
    async fn execute_command(&self, command: Commands, adapter: &ObsidianAdapter) -> Result<String> {
        // Create a CLI struct with the command
        let cli = Cli { command: Some(command) };
        
        // Execute the command through the existing router
        match route_command(cli, adapter).await {
            Ok(()) => Ok("Command completed successfully".to_string()),
            Err(e) => Err(e),
        }
    }

    /// Check if disambiguation is needed based on the conversation response
    async fn check_disambiguation_needed(
        &self,
        session: Option<&crate::conversational_interface::ConversationSession>,
        conv_response: &ConversationResponse,
    ) -> Result<Option<DisambiguationQuestion>> {
        // Check if the last turn had a parsed command that needs disambiguation
        if let Some(session) = session {
            if let Some(last_turn) = session.history.last() {
                if let Some(ref parsed_command) = last_turn.parsed_command {
                    if parsed_command.needs_disambiguation {
                        // Generate disambiguation question
                        let question = self.generate_disambiguation_question(parsed_command)?;
                        return Ok(Some(question));
                    }
                }
            }
        }
        
        // Check if the conversational response requires clarification
        if conv_response.requires_clarification {
            if let Some(ref clarification) = conv_response.clarification_question {
                // Convert clarification to disambiguation question
                let question = DisambiguationQuestion {
                    question: clarification.clone(),
                    options: Vec::new(),
                    context: "General clarification needed".to_string(),
                };
                return Ok(Some(question));
            }
        }

        Ok(None)
    }

    /// Generate a disambiguation question from a parsed command
    fn generate_disambiguation_question(&self, parsed_command: &ParsedCommand) -> Result<DisambiguationQuestion> {
        // This is a simplified implementation - in practice, you'd use the NL parser's
        // disambiguation generation capabilities
        let mut options = Vec::new();
        
        // Add primary interpretation
        options.push(crate::nl_command_parser::DisambiguationOption {
            display_text: format!("Execute: {}", parsed_command.intent),
            intent: parsed_command.intent.clone(),
            entities: parsed_command.entities.clone(),
            description: format!("Execute the {} command", parsed_command.intent),
        });
        
        // Add alternatives
        for (i, alt) in parsed_command.alternatives.iter().enumerate() {
            options.push(crate::nl_command_parser::DisambiguationOption {
                display_text: format!("Alternative {}: {}", i + 1, alt.intent),
                intent: alt.intent.clone(),
                entities: alt.entities.clone(),
                description: alt.reason.clone(),
            });
        }
        
        Ok(DisambiguationQuestion {
            question: format!("Your command '{}' could mean several things. Which did you intend?", parsed_command.original_input),
            options,
            context: format!("Confidence: {:.0}%", parsed_command.confidence * 100.0),
        })
    }

    /// Parse user's disambiguation choice
    fn parse_disambiguation_choice(&self, user_input: &str, question: &DisambiguationQuestion) -> Result<usize> {
        // Try to parse as number
        if let Ok(choice) = user_input.trim().parse::<usize>() {
            if choice > 0 && choice <= question.options.len() {
                return Ok(choice - 1); // Convert to 0-based index
            }
        }
        
        // Try to match against option text
        let input_lower = user_input.to_lowercase();
        for (i, option) in question.options.iter().enumerate() {
            if option.display_text.to_lowercase().contains(&input_lower) ||
               option.intent.to_lowercase().contains(&input_lower) {
                return Ok(i);
            }
        }
        
        Err(anyhow::anyhow!("Invalid choice: '{}'. Please enter a number between 1 and {}", user_input, question.options.len()))
    }

    /// Create CLI command from disambiguation option
    fn create_cli_command_from_option(&self, option: &crate::nl_command_parser::DisambiguationOption) -> Result<Commands> {
        // This is a simplified mapping - in practice, you'd use the NL parser's
        // command conversion capabilities
        match option.intent.as_str() {
            "add_todo" => {
                let description = option.entities.get("description")
                    .cloned()
                    .unwrap_or_else(|| "New task".to_string());
                Ok(Commands::Todo(crate::cli::TodoArgs {
                    action: crate::cli::TodoAction::Add {
                        description,
                        due_date: option.entities.get("due_date").cloned(),
                        tags: option.entities.get("tags")
                            .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                            .unwrap_or_default(),
                    },
                }))
            }
            "list_todos" => {
                Ok(Commands::Todo(crate::cli::TodoArgs {
                    action: crate::cli::TodoAction::List {
                        status: option.entities.get("status").cloned(),
                    },
                }))
            }
            "add_goal" => {
                let title = option.entities.get("title")
                    .cloned()
                    .unwrap_or_else(|| "New goal".to_string());
                Ok(Commands::Goal(crate::cli::GoalArgs {
                    action: crate::cli::GoalAction::Add {
                        title,
                        description: option.entities.get("description").cloned(),
                        target_date: option.entities.get("target_date").cloned(),
                        tags: option.entities.get("tags")
                            .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                            .unwrap_or_default(),
                    },
                }))
            }
            "create_note" => {
                let title = option.entities.get("title")
                    .cloned()
                    .unwrap_or_else(|| "New note".to_string());
                Ok(Commands::Note(crate::cli::NoteArgs {
                    action: crate::cli::NoteAction::Create {
                        title,
                        content: option.entities.get("content").cloned(),
                        tags: option.entities.get("tags")
                            .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                            .unwrap_or_default(),
                    },
                }))
            }
            _ => Err(anyhow::anyhow!("Unknown intent: {}", option.intent)),
        }
    }

    /// Start a new conversation session
    pub async fn start_session(&mut self, user_id: Option<String>) -> Result<String> {
        self.conversational_interface.start_session(user_id).await
    }

    /// End a conversation session
    pub async fn end_session(&mut self, session_id: &str) -> Result<()> {
        self.disambiguation_sessions.remove(session_id);
        self.conversational_interface.end_session(session_id).await
    }

    /// Get conversation statistics
    pub fn get_stats(&self) -> crate::conversational_interface::ConversationStats {
        self.conversational_interface.get_stats()
    }

    /// Check if the input is a help query
    fn is_help_query(&self, input: &str) -> bool {
        let input_lower = input.to_lowercase();
        let help_keywords = [
            "help", "what can i do", "what should i do", "how do i", "how to",
            "show me", "guide me", "assist me", "what's possible", "what are my options",
            "examples", "tutorial", "getting started", "what's next"
        ];
        
        help_keywords.iter().any(|keyword| input_lower.contains(keyword))
    }

    /// Handle help queries
    async fn handle_help_query(
        &mut self,
        session_id: &str,
        user_input: &str,
        _config: &NLBridgeConfig,
    ) -> Result<NLBridgeResponse> {
        // Create help context
        let session = self.conversational_interface.get_session(session_id);
        let help_context = HelpContext {
            session: session.cloned(),
            recent_commands: Vec::new(), // Could be populated from session history
            project_state: create_mock_project_state(),
            user_level: determine_user_level(session),
            time_context: determine_time_context(),
        };

        // Generate help suggestions
        let help_suggestions = if user_input.to_lowercase().contains("what can i do") || 
                                 user_input.to_lowercase().contains("what should i do") {
            self.help_system.handle_discovery_query(user_input, &help_context).await?
        } else {
            self.help_system.generate_help_suggestions(&help_context).await?
        };

        // Create response text
        let mut response_parts = Vec::new();
        
        if user_input.to_lowercase().contains("what can i do") || 
           user_input.to_lowercase().contains("what should i do") {
            response_parts.push("Here are some suggestions for what you can do:".to_string());
        } else {
            response_parts.push("Here's how I can help you:".to_string());
        }

        // Add help suggestions to response
        for (i, suggestion) in help_suggestions.iter().enumerate().take(5) {
            response_parts.push(format!("{}. {}", i + 1, suggestion.text));
            if let Some(ref example) = suggestion.example {
                response_parts.push(format!("   Example: \"{}\"", example));
            }
        }

        // Add general tips
        response_parts.push("\nTips:".to_string());
        response_parts.push("• Use natural language - no need for complex commands".to_string());
        response_parts.push("• Be specific about what you want to do".to_string());
        response_parts.push("• I can help with todos, goals, and notes".to_string());
        response_parts.push("• Ask 'what can I do next?' for personalized suggestions".to_string());

        Ok(NLBridgeResponse {
            response_text: response_parts.join("\n"),
            executed_commands: Vec::new(),
            suggestions: help_suggestions.iter().filter_map(|s| s.example.clone()).collect(),
            requires_disambiguation: false,
            disambiguation_question: None,
            execution_successful: true,
            error_message: None,
            help_suggestions,
        })
    }

    /// Get contextual help suggestions for a session
    pub async fn get_contextual_help(&self, session_id: &str) -> Result<Vec<HelpSuggestion>> {
        let session = self.conversational_interface.get_session(session_id);
        let help_context = HelpContext {
            session: session.cloned(),
            recent_commands: Vec::new(),
            project_state: create_mock_project_state(),
            user_level: determine_user_level(session),
            time_context: determine_time_context(),
        };

        self.help_system.generate_help_suggestions(&help_context).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_conversation::{Message, MessageRole};
    
    use async_trait::async_trait;
    use chrono::Utc;
    use std::collections::HashMap;

    // Mock LLM client for testing
    struct MockLLMClient {
        responses: HashMap<String, String>,
    }

    impl MockLLMClient {
        fn new() -> Self {
            let mut responses = HashMap::new();
            responses.insert(
                "add_todo".to_string(),
                r#"{"intent": "add_todo", "entities": {"description": "test task"}, "confidence": 0.9, "alternatives": [], "needs_disambiguation": false}"#.to_string(),
            );
            responses.insert(
                "ambiguous".to_string(),
                r#"{"intent": "add_todo", "entities": {"description": "task"}, "confidence": 0.6, "alternatives": [{"intent": "list_todos", "entities": {}, "confidence": 0.5, "reason": "Could be asking to list todos"}], "needs_disambiguation": true}"#.to_string(),
            );
            Self { responses }
        }
    }

    #[async_trait]
    impl LLMClient for MockLLMClient {
        async fn send_message(&self, messages: Vec<Message>) -> Result<Message> {
            let content = messages.last().unwrap().content.to_lowercase();
            
            let response_content = if content.contains("add") && content.contains("todo") {
                self.responses.get("add_todo").unwrap().clone()
            } else if content.contains("ambiguous") {
                self.responses.get("ambiguous").unwrap().clone()
            } else {
                "I understand. How can I help you?".to_string()
            };
            
            Ok(Message {
                id: "test".to_string(),
                role: MessageRole::Assistant,
                content: response_content,
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
    async fn test_bridge_creation() {
        let llm_client = Box::new(MockLLMClient::new());
        let config = NLBridgeConfig::default();
        let bridge = NLCLIBridge::new(llm_client, config);
        assert!(bridge.is_ok());
    }

    #[tokio::test]
    async fn test_simple_command_processing() {
        let llm_client = Box::new(MockLLMClient::new());
        let config = NLBridgeConfig {
            auto_execute: false, // Don't actually execute for testing
            verbose_feedback: true,
            provide_suggestions: true,
        };
        let mut bridge = NLCLIBridge::new(llm_client, config.clone()).unwrap();
        
        let session_id = bridge.start_session(None).await.unwrap();
        
        // Mock adapter
        let adapter = crate::obsidian_adapter::ObsidianAdapter::new(None, None);
        
        let response = bridge.process_input(&session_id, "Add a todo to test the system", &adapter, &config).await.unwrap();
        
        assert!(response.response_text.contains("add"));
        assert!(!response.requires_disambiguation);
    }

    #[tokio::test]
    async fn test_disambiguation_choice_parsing() {
        let llm_client = Box::new(MockLLMClient::new());
        let config = NLBridgeConfig::default();
        let bridge = NLCLIBridge::new(llm_client, config).unwrap();
        
        let options = vec![
            crate::nl_command_parser::DisambiguationOption {
                display_text: "Add a todo".to_string(),
                intent: "add_todo".to_string(),
                entities: HashMap::new(),
                description: "Create a new task".to_string(),
            },
            crate::nl_command_parser::DisambiguationOption {
                display_text: "List todos".to_string(),
                intent: "list_todos".to_string(),
                entities: HashMap::new(),
                description: "Show all tasks".to_string(),
            },
        ];
        
        let question = DisambiguationQuestion {
            question: "What did you mean?".to_string(),
            options,
            context: "Test".to_string(),
        };
        
        // Test number parsing
        assert_eq!(bridge.parse_disambiguation_choice("1", &question).unwrap(), 0);
        assert_eq!(bridge.parse_disambiguation_choice("2", &question).unwrap(), 1);
        
        // Test text matching
        assert_eq!(bridge.parse_disambiguation_choice("add", &question).unwrap(), 0);
        assert_eq!(bridge.parse_disambiguation_choice("list", &question).unwrap(), 1);
    }
}