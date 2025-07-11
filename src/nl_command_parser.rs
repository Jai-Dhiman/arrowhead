use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::ai_conversation::{LLMClient, Message, MessageRole};
use crate::cli::{Commands, GoalAction, GoalArgs, NoteAction, NoteArgs, TodoAction, TodoArgs};

/// Represents a parsed natural language command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedCommand {
    /// The intent of the command (e.g., "add_todo", "list_goals")
    pub intent: String,
    /// Extracted entities and their values
    pub entities: HashMap<String, String>,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Original natural language input
    pub original_input: String,
    /// Parsed timestamp
    pub parsed_at: DateTime<Utc>,
    /// Unique identifier for this parsed command
    pub id: String,
    /// Alternative interpretations with lower confidence
    pub alternatives: Vec<AlternativeInterpretation>,
    /// Whether this command requires disambiguation
    pub needs_disambiguation: bool,
}

/// Represents a command intent with its variations
#[derive(Debug, Clone)]
pub struct CommandIntent {
    /// Primary intent name
    pub name: String,
    /// Description of what this intent does
    pub description: String,
    /// Example phrases that match this intent
    pub examples: Vec<String>,
    /// Required entities for this intent
    pub required_entities: Vec<String>,
    /// Optional entities for this intent
    pub optional_entities: Vec<String>,
}

/// Entity extraction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    /// Entity type (e.g., "todo_description", "due_date", "goal_title")
    pub entity_type: String,
    /// Extracted value
    pub value: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Position in original text
    pub start_pos: Option<usize>,
    pub end_pos: Option<usize>,
}

/// Alternative interpretation for ambiguous commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativeInterpretation {
    /// The intent of this alternative
    pub intent: String,
    /// Extracted entities for this alternative
    pub entities: HashMap<String, String>,
    /// Confidence score for this alternative
    pub confidence: f32,
    /// Reason why this interpretation is plausible
    pub reason: String,
}

/// Disambiguation question to ask the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisambiguationQuestion {
    /// The question to ask the user
    pub question: String,
    /// Possible answers mapped to their corresponding intents
    pub options: Vec<DisambiguationOption>,
    /// Context that led to this disambiguation
    pub context: String,
}

/// A disambiguation option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisambiguationOption {
    /// Display text for this option
    pub display_text: String,
    /// The intent this option maps to
    pub intent: String,
    /// Entities for this option
    pub entities: HashMap<String, String>,
    /// Brief description of what this option does
    pub description: String,
}

/// Configuration for the natural language parser
#[derive(Debug, Clone)]
pub struct NLParserConfig {
    /// Minimum confidence threshold for accepting a parsed command
    pub min_confidence_threshold: f32,
    /// Maximum number of alternative interpretations to consider
    pub max_alternatives: usize,
    /// Whether to use context from previous commands
    pub use_context: bool,
    /// Timeout for AI parsing requests (in seconds)
    pub parsing_timeout_secs: u64,
}

impl Default for NLParserConfig {
    fn default() -> Self {
        Self {
            min_confidence_threshold: 0.7,
            max_alternatives: 3,
            use_context: true,
            parsing_timeout_secs: 10,
        }
    }
}

/// Natural Language Command Parser
pub struct NLCommandParser {
    /// LLM client for AI-powered parsing
    llm_client: Box<dyn LLMClient>,
    /// Configuration settings
    config: NLParserConfig,
    /// Cache of command intents
    intents: Vec<CommandIntent>,
    /// Parsing history for context
    parsing_history: Vec<ParsedCommand>,
}

impl NLCommandParser {
    /// Create a new natural language command parser
    pub fn new(llm_client: Box<dyn LLMClient>) -> Self {
        Self {
            llm_client,
            config: NLParserConfig::default(),
            intents: Self::initialize_intents(),
            parsing_history: Vec::new(),
        }
    }

    /// Create parser with custom configuration
    pub fn with_config(llm_client: Box<dyn LLMClient>, config: NLParserConfig) -> Self {
        Self {
            llm_client,
            config,
            intents: Self::initialize_intents(),
            parsing_history: Vec::new(),
        }
    }

    /// Initialize the command intents database
    fn initialize_intents() -> Vec<CommandIntent> {
        vec![
            // Todo intents
            CommandIntent {
                name: "add_todo".to_string(),
                description: "Add a new todo item".to_string(),
                examples: vec![
                    "Add a todo to finish the project".to_string(),
                    "Create a task to call the client".to_string(),
                    "I need to remember to buy groceries".to_string(),
                    "Add reminder to submit report by Friday".to_string(),
                ],
                required_entities: vec!["description".to_string()],
                optional_entities: vec!["due_date".to_string(), "tags".to_string()],
            },
            CommandIntent {
                name: "list_todos".to_string(),
                description: "List all todos or filter by status".to_string(),
                examples: vec![
                    "Show me all my todos".to_string(),
                    "List all tasks".to_string(),
                    "What are my open todos?".to_string(),
                    "Show completed tasks".to_string(),
                ],
                required_entities: vec![],
                optional_entities: vec!["status".to_string()],
            },
            CommandIntent {
                name: "complete_todo".to_string(),
                description: "Mark a todo as done".to_string(),
                examples: vec![
                    "Mark todo 123 as done".to_string(),
                    "Complete the task about calling client".to_string(),
                    "I finished the project task".to_string(),
                    "Check off item 5".to_string(),
                ],
                required_entities: vec!["todo_id".to_string()],
                optional_entities: vec![],
            },
            CommandIntent {
                name: "view_todo".to_string(),
                description: "View details of a specific todo".to_string(),
                examples: vec![
                    "Show me todo 123".to_string(),
                    "View details of my project task".to_string(),
                    "What's in todo item 5?".to_string(),
                ],
                required_entities: vec!["todo_id".to_string()],
                optional_entities: vec![],
            },
            // Goal intents
            CommandIntent {
                name: "add_goal".to_string(),
                description: "Add a new goal".to_string(),
                examples: vec![
                    "Add a goal to learn Spanish".to_string(),
                    "Create a goal to run a marathon by next year".to_string(),
                    "I want to set a goal to save money".to_string(),
                ],
                required_entities: vec!["title".to_string()],
                optional_entities: vec!["description".to_string(), "target_date".to_string(), "tags".to_string()],
            },
            CommandIntent {
                name: "list_goals".to_string(),
                description: "List all goals".to_string(),
                examples: vec![
                    "Show me all my goals".to_string(),
                    "List my objectives".to_string(),
                    "What are my current goals?".to_string(),
                ],
                required_entities: vec![],
                optional_entities: vec!["status".to_string()],
            },
            CommandIntent {
                name: "update_goal".to_string(),
                description: "Update an existing goal".to_string(),
                examples: vec![
                    "Update goal 456 with new deadline".to_string(),
                    "Change the description of my fitness goal".to_string(),
                    "Mark goal as achieved".to_string(),
                ],
                required_entities: vec!["goal_id".to_string()],
                optional_entities: vec!["title".to_string(), "description".to_string(), "status".to_string(), "target_date".to_string()],
            },
            CommandIntent {
                name: "view_goal".to_string(),
                description: "View details of a specific goal".to_string(),
                examples: vec![
                    "Show me goal 456".to_string(),
                    "View my fitness goal details".to_string(),
                ],
                required_entities: vec!["goal_id".to_string()],
                optional_entities: vec![],
            },
            // Note intents
            CommandIntent {
                name: "create_note".to_string(),
                description: "Create a new note".to_string(),
                examples: vec![
                    "Create a note about the meeting".to_string(),
                    "Add a note with my thoughts on the project".to_string(),
                    "Make a note titled 'Ideas'".to_string(),
                ],
                required_entities: vec!["title".to_string()],
                optional_entities: vec!["content".to_string(), "tags".to_string()],
            },
            CommandIntent {
                name: "list_notes".to_string(),
                description: "List all notes".to_string(),
                examples: vec![
                    "Show me all my notes".to_string(),
                    "List notes tagged with 'work'".to_string(),
                    "What notes do I have?".to_string(),
                ],
                required_entities: vec![],
                optional_entities: vec!["tags".to_string()],
            },
            CommandIntent {
                name: "view_note".to_string(),
                description: "View a specific note".to_string(),
                examples: vec![
                    "Show me the meeting note".to_string(),
                    "View note 'project-ideas'".to_string(),
                ],
                required_entities: vec!["note_id".to_string()],
                optional_entities: vec![],
            },
            CommandIntent {
                name: "append_note".to_string(),
                description: "Append content to an existing note".to_string(),
                examples: vec![
                    "Add 'remember to follow up' to my meeting notes".to_string(),
                    "Append my thoughts to the project note".to_string(),
                ],
                required_entities: vec!["note_id".to_string(), "content".to_string()],
                optional_entities: vec![],
            },
            CommandIntent {
                name: "edit_note".to_string(),
                description: "Edit an existing note".to_string(),
                examples: vec![
                    "Edit my meeting notes".to_string(),
                    "Open the project note for editing".to_string(),
                ],
                required_entities: vec!["note_id".to_string()],
                optional_entities: vec![],
            },
            // Chat intent
            CommandIntent {
                name: "start_chat".to_string(),
                description: "Start a chat session".to_string(),
                examples: vec![
                    "Start a chat".to_string(),
                    "Let's have a conversation".to_string(),
                    "I want to chat about my project".to_string(),
                ],
                required_entities: vec![],
                optional_entities: vec!["initial_message".to_string()],
            },
        ]
    }

    /// Parse a natural language command
    pub async fn parse_command(&mut self, input: &str) -> Result<ParsedCommand> {
        // Create the parsing prompt
        let prompt = self.create_parsing_prompt(input);
        
        // Send to AI for parsing
        let messages = vec![
            Message {
                id: Uuid::new_v4().to_string(),
                role: MessageRole::System,
                content: "You are a natural language command parser. Parse the given command and return a structured JSON response with intent, entities, confidence score, and alternatives for disambiguation.".to_string(),
                timestamp: Utc::now(),
                function_call: None,
            },
            Message {
                id: Uuid::new_v4().to_string(),
                role: MessageRole::User,
                content: prompt,
                timestamp: Utc::now(),
                function_call: None,
            },
        ];

        let response = self.llm_client.send_message(messages).await
            .context("Failed to get AI response for command parsing")?;

        // Parse the AI response
        let parsed_command = self.parse_ai_response(&response.content, input)?;
        
        // Add to parsing history
        self.parsing_history.push(parsed_command.clone());
        
        // Keep only recent history (last 10 commands)
        if self.parsing_history.len() > 10 {
            self.parsing_history.remove(0);
        }

        Ok(parsed_command)
    }

    /// Generate disambiguation question for ambiguous commands
    pub fn generate_disambiguation_question(&self, parsed_command: &ParsedCommand) -> Option<DisambiguationQuestion> {
        if !parsed_command.needs_disambiguation || parsed_command.alternatives.is_empty() {
            return None;
        }

        let mut options = Vec::new();
        
        // Add primary interpretation as first option
        options.push(DisambiguationOption {
            display_text: format!("Option 1: {}", self.format_intent_display(&parsed_command.intent)),
            intent: parsed_command.intent.clone(),
            entities: parsed_command.entities.clone(),
            description: self.get_intent_description(&parsed_command.intent),
        });

        // Add alternatives as additional options
        for (i, alt) in parsed_command.alternatives.iter().enumerate() {
            options.push(DisambiguationOption {
                display_text: format!("Option {}: {}", i + 2, self.format_intent_display(&alt.intent)),
                intent: alt.intent.clone(),
                entities: alt.entities.clone(),
                description: format!("{} ({})", self.get_intent_description(&alt.intent), alt.reason),
            });
        }

        Some(DisambiguationQuestion {
            question: format!("Your command \"{}\" could mean several things. Which did you intend?", parsed_command.original_input),
            options,
            context: format!("Command parsed with {}% confidence", (parsed_command.confidence * 100.0) as i32),
        })
    }

    /// Get suggestions for commands that didn't match well
    pub fn get_did_you_mean_suggestions(&self, parsed_command: &ParsedCommand) -> Vec<String> {
        if parsed_command.confidence >= self.config.min_confidence_threshold {
            return Vec::new();
        }

        let input_lower = parsed_command.original_input.to_lowercase();
        let mut suggestions = Vec::new();

        // Find intents with similar keywords
        for intent in &self.intents {
            for example in &intent.examples {
                if self.calculate_similarity(&input_lower, &example.to_lowercase()) > 0.6 {
                    suggestions.push(format!("Did you mean: \"{}\"?", example));
                    if suggestions.len() >= 3 {
                        break;
                    }
                }
            }
            if suggestions.len() >= 3 {
                break;
            }
        }

        // If no good suggestions, provide common command patterns
        if suggestions.is_empty() {
            suggestions.extend_from_slice(&[
                "Try: \"Add a todo to [description]\"".to_string(),
                "Try: \"List my todos\"".to_string(),
                "Try: \"Show me my goals\"".to_string(),
            ]);
        }

        suggestions
    }

    /// Resolve command with user's disambiguation choice
    pub fn resolve_disambiguation(&self, question: &DisambiguationQuestion, choice_index: usize) -> Result<ParsedCommand> {
        let option = question.options.get(choice_index)
            .ok_or_else(|| anyhow::anyhow!("Invalid disambiguation choice: {}", choice_index))?;

        Ok(ParsedCommand {
            intent: option.intent.clone(),
            entities: option.entities.clone(),
            confidence: 1.0, // User explicitly chose this
            original_input: format!("Disambiguated: {}", question.context),
            parsed_at: Utc::now(),
            id: Uuid::new_v4().to_string(),
            alternatives: Vec::new(),
            needs_disambiguation: false,
        })
    }

    /// Convert parsed command to CLI command
    pub fn to_cli_command(&self, parsed: &ParsedCommand) -> Result<Commands> {
        match parsed.intent.as_str() {
            "add_todo" => {
                let description = parsed.entities.get("description")
                    .ok_or_else(|| anyhow::anyhow!("Missing required entity: description"))?
                    .clone();
                
                let due_date = parsed.entities.get("due_date").cloned();
                let tags = parsed.entities.get("tags")
                    .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default();

                Ok(Commands::Todo(TodoArgs {
                    action: TodoAction::Add {
                        description,
                        due_date,
                        tags,
                    },
                }))
            }
            "list_todos" => {
                let status = parsed.entities.get("status").cloned();
                Ok(Commands::Todo(TodoArgs {
                    action: TodoAction::List { status },
                }))
            }
            "complete_todo" => {
                let id = parsed.entities.get("todo_id")
                    .ok_or_else(|| anyhow::anyhow!("Missing required entity: todo_id"))?
                    .clone();
                
                Ok(Commands::Todo(TodoArgs {
                    action: TodoAction::Done { id },
                }))
            }
            "view_todo" => {
                let id = parsed.entities.get("todo_id")
                    .ok_or_else(|| anyhow::anyhow!("Missing required entity: todo_id"))?
                    .clone();
                
                Ok(Commands::Todo(TodoArgs {
                    action: TodoAction::View { id },
                }))
            }
            "add_goal" => {
                let title = parsed.entities.get("title")
                    .ok_or_else(|| anyhow::anyhow!("Missing required entity: title"))?
                    .clone();
                
                let description = parsed.entities.get("description").cloned();
                let target_date = parsed.entities.get("target_date").cloned();
                let tags = parsed.entities.get("tags")
                    .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default();

                Ok(Commands::Goal(GoalArgs {
                    action: GoalAction::Add {
                        title,
                        description,
                        target_date,
                        tags,
                    },
                }))
            }
            "list_goals" => {
                let status = parsed.entities.get("status").cloned();
                Ok(Commands::Goal(GoalArgs {
                    action: GoalAction::List { status },
                }))
            }
            "update_goal" => {
                let id = parsed.entities.get("goal_id")
                    .ok_or_else(|| anyhow::anyhow!("Missing required entity: goal_id"))?
                    .clone();
                
                let title = parsed.entities.get("title").cloned();
                let description = parsed.entities.get("description").cloned();
                let status = parsed.entities.get("status").cloned();
                let target_date = parsed.entities.get("target_date").cloned();

                Ok(Commands::Goal(GoalArgs {
                    action: GoalAction::Update {
                        id,
                        title,
                        description,
                        status,
                        target_date,
                    },
                }))
            }
            "view_goal" => {
                let id = parsed.entities.get("goal_id")
                    .ok_or_else(|| anyhow::anyhow!("Missing required entity: goal_id"))?
                    .clone();
                
                Ok(Commands::Goal(GoalArgs {
                    action: GoalAction::View { id },
                }))
            }
            "create_note" => {
                let title = parsed.entities.get("title")
                    .ok_or_else(|| anyhow::anyhow!("Missing required entity: title"))?
                    .clone();
                
                let content = parsed.entities.get("content").cloned();
                let tags = parsed.entities.get("tags")
                    .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default();

                Ok(Commands::Note(NoteArgs {
                    action: NoteAction::Create {
                        title,
                        content,
                        tags,
                    },
                }))
            }
            "list_notes" => {
                let tags = parsed.entities.get("tags")
                    .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default();

                Ok(Commands::Note(NoteArgs {
                    action: NoteAction::List { tags },
                }))
            }
            "view_note" => {
                let name_or_id = parsed.entities.get("note_id")
                    .ok_or_else(|| anyhow::anyhow!("Missing required entity: note_id"))?
                    .clone();
                
                Ok(Commands::Note(NoteArgs {
                    action: NoteAction::View { name_or_id },
                }))
            }
            "append_note" => {
                let name_or_id = parsed.entities.get("note_id")
                    .ok_or_else(|| anyhow::anyhow!("Missing required entity: note_id"))?
                    .clone();
                
                let content = parsed.entities.get("content")
                    .ok_or_else(|| anyhow::anyhow!("Missing required entity: content"))?
                    .clone();

                Ok(Commands::Note(NoteArgs {
                    action: NoteAction::Append { name_or_id, content },
                }))
            }
            "edit_note" => {
                let name_or_id = parsed.entities.get("note_id")
                    .ok_or_else(|| anyhow::anyhow!("Missing required entity: note_id"))?
                    .clone();
                
                Ok(Commands::Note(NoteArgs {
                    action: NoteAction::Edit { name_or_id },
                }))
            }
            "start_chat" => {
                // Chat functionality is now handled by the main interactive mode
                // Return a general help response instead
                Err(anyhow::anyhow!("Chat mode is active by default. How can I help you?"))
            }
            _ => Err(anyhow::anyhow!("Unknown intent: {}", parsed.intent)),
        }
    }

    /// Create the parsing prompt for AI
    fn create_parsing_prompt(&self, input: &str) -> String {
        let mut prompt = String::new();
        
        prompt.push_str("Parse this natural language command and return a JSON response with the following structure:\n\n");
        prompt.push_str("{\n");
        prompt.push_str("  \"intent\": \"intent_name\",\n");
        prompt.push_str("  \"entities\": {\n");
        prompt.push_str("    \"entity_name\": \"entity_value\"\n");
        prompt.push_str("  },\n");
        prompt.push_str("  \"confidence\": 0.95,\n");
        prompt.push_str("  \"alternatives\": [\n");
        prompt.push_str("    {\n");
        prompt.push_str("      \"intent\": \"alternative_intent\",\n");
        prompt.push_str("      \"entities\": {\"entity\": \"value\"},\n");
        prompt.push_str("      \"confidence\": 0.7,\n");
        prompt.push_str("      \"reason\": \"why this interpretation is plausible\"\n");
        prompt.push_str("    }\n");
        prompt.push_str("  ],\n");
        prompt.push_str("  \"needs_disambiguation\": false\n");
        prompt.push_str("}\n\n");
        
        prompt.push_str("Rules for disambiguation:\n");
        prompt.push_str("- Set needs_disambiguation to true if confidence < 0.8 OR if multiple valid interpretations exist\n");
        prompt.push_str("- Include up to 3 alternatives when command is ambiguous\n");
        prompt.push_str("- Only include alternatives with confidence > 0.5\n");
        prompt.push_str("- Provide clear reasons for each alternative\n\n");
        
        prompt.push_str("Available intents and their entities:\n");
        for intent in &self.intents {
            prompt.push_str(&format!("- {}: {}\n", intent.name, intent.description));
            prompt.push_str(&format!("  Required entities: {:?}\n", intent.required_entities));
            prompt.push_str(&format!("  Optional entities: {:?}\n", intent.optional_entities));
            prompt.push_str(&format!("  Examples: {:?}\n", intent.examples));
            prompt.push_str("\n");
        }
        
        if self.config.use_context && !self.parsing_history.is_empty() {
            prompt.push_str("Recent command history for context:\n");
            for cmd in self.parsing_history.iter().rev().take(3) {
                prompt.push_str(&format!("- {} -> {}\n", cmd.original_input, cmd.intent));
            }
            prompt.push_str("\n");
        }
        
        prompt.push_str(&format!("Command to parse: \"{}\"\n", input));
        prompt.push_str("Return only the JSON response.");
        
        prompt
    }

    /// Parse AI response into ParsedCommand
    fn parse_ai_response(&self, response: &str, original_input: &str) -> Result<ParsedCommand> {
        // Extract JSON from response
        let json_start = response.find('{').unwrap_or(0);
        let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
        let json_str = &response[json_start..json_end];
        
        let parsed: serde_json::Value = serde_json::from_str(json_str)
            .context("Failed to parse AI response as JSON")?;
        
        let intent = parsed.get("intent")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing intent in AI response"))?
            .to_string();
        
        let entities = parsed.get("entities")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        
        let confidence = parsed.get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5) as f32;
        
        let alternatives = parsed.get("alternatives")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|alt| self.parse_alternative(alt).ok())
                    .collect()
            })
            .unwrap_or_default();
        
        let needs_disambiguation = parsed.get("needs_disambiguation")
            .and_then(|v| v.as_bool())
            .unwrap_or(confidence < self.config.min_confidence_threshold);
        
        Ok(ParsedCommand {
            intent,
            entities,
            confidence,
            original_input: original_input.to_string(),
            parsed_at: Utc::now(),
            id: Uuid::new_v4().to_string(),
            alternatives,
            needs_disambiguation,
        })
    }

    /// Parse alternative interpretation from JSON
    fn parse_alternative(&self, alt_json: &serde_json::Value) -> Result<AlternativeInterpretation> {
        let intent = alt_json.get("intent")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing intent in alternative"))?
            .to_string();
        
        let entities = alt_json.get("entities")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        
        let confidence = alt_json.get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5) as f32;
        
        let reason = alt_json.get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("Alternative interpretation")
            .to_string();
        
        Ok(AlternativeInterpretation {
            intent,
            entities,
            confidence,
            reason,
        })
    }

    /// Format intent name for display
    fn format_intent_display(&self, intent: &str) -> String {
        match intent {
            "add_todo" => "Add a todo item",
            "list_todos" => "List todo items",
            "complete_todo" => "Mark todo as complete",
            "view_todo" => "View todo details",
            "add_goal" => "Add a goal",
            "list_goals" => "List goals",
            "update_goal" => "Update a goal",
            "view_goal" => "View goal details",
            "create_note" => "Create a note",
            "list_notes" => "List notes",
            "view_note" => "View note details",
            "append_note" => "Add to existing note",
            "edit_note" => "Edit a note",
            "start_chat" => "Start a chat session",
            _ => intent,
        }.to_string()
    }

    /// Get description for an intent
    fn get_intent_description(&self, intent: &str) -> String {
        self.intents.iter()
            .find(|i| i.name == intent)
            .map(|i| i.description.clone())
            .unwrap_or_else(|| format!("Unknown intent: {}", intent))
    }

    /// Calculate similarity between two strings (simple implementation)
    fn calculate_similarity(&self, a: &str, b: &str) -> f32 {
        let a_words: Vec<&str> = a.split_whitespace().collect();
        let b_words: Vec<&str> = b.split_whitespace().collect();
        
        if a_words.is_empty() || b_words.is_empty() {
            return 0.0;
        }
        
        let mut common_words = 0;
        for word_a in &a_words {
            for word_b in &b_words {
                if word_a.len() > 2 && word_b.len() > 2 && 
                   (word_a.contains(word_b) || word_b.contains(word_a)) {
                    common_words += 1;
                    break;
                }
            }
        }
        
        (common_words as f32) / (a_words.len().max(b_words.len()) as f32)
    }

    /// Get parsing statistics
    pub fn get_parsing_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();
        
        stats.insert("total_commands_parsed".to_string(), 
                    serde_json::Value::Number(self.parsing_history.len().into()));
        
        if !self.parsing_history.is_empty() {
            let avg_confidence = self.parsing_history.iter()
                .map(|cmd| cmd.confidence)
                .sum::<f32>() / self.parsing_history.len() as f32;
            
            stats.insert("average_confidence".to_string(), 
                        serde_json::Value::Number(serde_json::Number::from_f64(avg_confidence as f64).unwrap()));
            
            let most_common_intent = self.parsing_history.iter()
                .map(|cmd| &cmd.intent)
                .fold(HashMap::new(), |mut acc, intent| {
                    *acc.entry(intent).or_insert(0) += 1;
                    acc
                })
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .map(|(intent, _)| intent.clone())
                .unwrap_or_default();
            
            stats.insert("most_common_intent".to_string(), 
                        serde_json::Value::String(most_common_intent));
        }
        
        stats
    }

    /// Clear parsing history
    pub fn clear_history(&mut self) {
        self.parsing_history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_conversation::{Message, MessageRole};
    use async_trait::async_trait;
    use std::collections::HashMap;

    // Mock LLM client for testing
    struct MockLLMClient {
        mock_responses: HashMap<String, String>,
    }

    impl MockLLMClient {
        fn new() -> Self {
            let mut mock_responses = HashMap::new();
            
            // Mock response for todo addition
            mock_responses.insert(
                "add_todo".to_string(),
                r#"{"intent": "add_todo", "entities": {"description": "finish the project"}, "confidence": 0.95}"#.to_string()
            );
            
            // Mock response for listing todos
            mock_responses.insert(
                "list_todos".to_string(),
                r#"{"intent": "list_todos", "entities": {}, "confidence": 0.90}"#.to_string()
            );
            
            Self { mock_responses }
        }
    }

    #[async_trait]
    impl LLMClient for MockLLMClient {
        async fn send_message(&self, messages: Vec<Message>) -> Result<Message> {
            // Simple mock logic based on content
            let content = messages.last().unwrap().content.to_lowercase();
            
            let response_content = if content.contains("add") && content.contains("todo") {
                self.mock_responses.get("add_todo").unwrap().clone()
            } else if content.contains("list") && content.contains("todos") {
                self.mock_responses.get("list_todos").unwrap().clone()
            } else {
                r#"{"intent": "unknown", "entities": {}, "confidence": 0.1}"#.to_string()
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
            
            // Send the same response as send_message but as a stream
            let response = self.send_message(messages).await?;
            let _ = tx.send(response.content).await;
            
            Ok(rx)
        }

        async fn function_calling(&self, messages: Vec<Message>, _functions: Vec<crate::ai_conversation::FunctionSchema>) -> Result<Message> {
            // For mock purposes, just return a regular message
            self.send_message(messages).await
        }

        fn get_model_name(&self) -> String {
            "mock-model".to_string()
        }
    }

    #[test]
    fn test_command_intent_creation() {
        let intent = CommandIntent {
            name: "add_todo".to_string(),
            description: "Add a new todo".to_string(),
            examples: vec!["Add a todo".to_string()],
            required_entities: vec!["description".to_string()],
            optional_entities: vec!["due_date".to_string()],
        };
        
        assert_eq!(intent.name, "add_todo");
        assert_eq!(intent.required_entities.len(), 1);
        assert_eq!(intent.optional_entities.len(), 1);
    }

    #[test]
    fn test_parsed_command_creation() {
        let mut entities = HashMap::new();
        entities.insert("description".to_string(), "test task".to_string());
        
        let parsed = ParsedCommand {
            intent: "add_todo".to_string(),
            entities,
            confidence: 0.95,
            original_input: "Add a todo to test".to_string(),
            parsed_at: Utc::now(),
            id: "test-id".to_string(),
            alternatives: Vec::new(),
            needs_disambiguation: false,
        };
        
        assert_eq!(parsed.intent, "add_todo");
        assert_eq!(parsed.confidence, 0.95);
        assert_eq!(parsed.entities.len(), 1);
        assert!(!parsed.needs_disambiguation);
    }

    #[test]
    fn test_nl_parser_config_default() {
        let config = NLParserConfig::default();
        
        assert_eq!(config.min_confidence_threshold, 0.7);
        assert_eq!(config.max_alternatives, 3);
        assert!(config.use_context);
        assert_eq!(config.parsing_timeout_secs, 10);
    }

    #[test]
    fn test_initialize_intents() {
        let intents = NLCommandParser::initialize_intents();
        
        assert!(!intents.is_empty());
        
        // Check that we have the main intent categories
        let intent_names: Vec<&str> = intents.iter().map(|i| i.name.as_str()).collect();
        assert!(intent_names.contains(&"add_todo"));
        assert!(intent_names.contains(&"list_todos"));
        assert!(intent_names.contains(&"add_goal"));
        assert!(intent_names.contains(&"create_note"));
        assert!(intent_names.contains(&"start_chat"));
    }

    #[tokio::test]
    async fn test_parse_command_basic() {
        let mock_client = Box::new(MockLLMClient::new());
        let mut parser = NLCommandParser::new(mock_client);
        
        let result = parser.parse_command("Add a todo to finish the project").await;
        assert!(result.is_ok());
        
        let parsed = result.unwrap();
        assert_eq!(parsed.intent, "add_todo");
        assert_eq!(parsed.entities.get("description").unwrap(), "finish the project");
        assert_eq!(parsed.confidence, 0.95);
    }

    #[tokio::test]
    async fn test_to_cli_command_add_todo() {
        let mock_client = Box::new(MockLLMClient::new());
        let parser = NLCommandParser::new(mock_client);
        
        let mut entities = HashMap::new();
        entities.insert("description".to_string(), "test task".to_string());
        entities.insert("due_date".to_string(), "tomorrow".to_string());
        entities.insert("tags".to_string(), "work,urgent".to_string());
        
        let parsed = ParsedCommand {
            intent: "add_todo".to_string(),
            entities,
            confidence: 0.95,
            original_input: "Add a todo".to_string(),
            parsed_at: Utc::now(),
            id: "test".to_string(),
            alternatives: Vec::new(),
            needs_disambiguation: false,
        };
        
        let cli_command = parser.to_cli_command(&parsed).unwrap();
        
        match cli_command {
            Commands::Todo(todo_args) => {
                match todo_args.action {
                    TodoAction::Add { description, due_date, tags } => {
                        assert_eq!(description, "test task");
                        assert_eq!(due_date, Some("tomorrow".to_string()));
                        assert_eq!(tags, vec!["work", "urgent"]);
                    }
                    _ => panic!("Expected TodoAction::Add"),
                }
            }
            _ => panic!("Expected Commands::Todo"),
        }
    }

    #[tokio::test]
    async fn test_to_cli_command_list_todos() {
        let mock_client = Box::new(MockLLMClient::new());
        let parser = NLCommandParser::new(mock_client);
        
        let mut entities = HashMap::new();
        entities.insert("status".to_string(), "open".to_string());
        
        let parsed = ParsedCommand {
            intent: "list_todos".to_string(),
            entities,
            confidence: 0.90,
            original_input: "List open todos".to_string(),
            parsed_at: Utc::now(),
            id: "test".to_string(),
            alternatives: Vec::new(),
            needs_disambiguation: false,
        };
        
        let cli_command = parser.to_cli_command(&parsed).unwrap();
        
        match cli_command {
            Commands::Todo(todo_args) => {
                match todo_args.action {
                    TodoAction::List { status } => {
                        assert_eq!(status, Some("open".to_string()));
                    }
                    _ => panic!("Expected TodoAction::List"),
                }
            }
            _ => panic!("Expected Commands::Todo"),
        }
    }

    #[tokio::test]
    async fn test_to_cli_command_create_note() {
        let mock_client = Box::new(MockLLMClient::new());
        let parser = NLCommandParser::new(mock_client);
        
        let mut entities = HashMap::new();
        entities.insert("title".to_string(), "Meeting Notes".to_string());
        entities.insert("content".to_string(), "Discussed project timeline".to_string());
        entities.insert("tags".to_string(), "meeting,project".to_string());
        
        let parsed = ParsedCommand {
            intent: "create_note".to_string(),
            entities,
            confidence: 0.88,
            original_input: "Create a note".to_string(),
            parsed_at: Utc::now(),
            id: "test".to_string(),
            alternatives: Vec::new(),
            needs_disambiguation: false,
        };
        
        let cli_command = parser.to_cli_command(&parsed).unwrap();
        
        match cli_command {
            Commands::Note(note_args) => {
                match note_args.action {
                    NoteAction::Create { title, content, tags } => {
                        assert_eq!(title, "Meeting Notes");
                        assert_eq!(content, Some("Discussed project timeline".to_string()));
                        assert_eq!(tags, vec!["meeting", "project"]);
                    }
                    _ => panic!("Expected NoteAction::Create"),
                }
            }
            _ => panic!("Expected Commands::Note"),
        }
    }

    #[test]
    fn test_parsing_stats() {
        let mock_client = Box::new(MockLLMClient::new());
        let mut parser = NLCommandParser::new(mock_client);
        
        // Add some mock history
        parser.parsing_history.push(ParsedCommand {
            intent: "add_todo".to_string(),
            entities: HashMap::new(),
            confidence: 0.9,
            original_input: "test".to_string(),
            parsed_at: Utc::now(),
            id: "1".to_string(),
            alternatives: Vec::new(),
            needs_disambiguation: false,
        });
        
        parser.parsing_history.push(ParsedCommand {
            intent: "add_todo".to_string(),
            entities: HashMap::new(),
            confidence: 0.8,
            original_input: "test2".to_string(),
            parsed_at: Utc::now(),
            id: "2".to_string(),
            alternatives: Vec::new(),
            needs_disambiguation: false,
        });
        
        let stats = parser.get_parsing_stats();
        
        assert_eq!(stats.get("total_commands_parsed").unwrap().as_u64().unwrap(), 2);
        assert_eq!(stats.get("most_common_intent").unwrap().as_str().unwrap(), "add_todo");
    }

    #[test]
    fn test_clear_history() {
        let mock_client = Box::new(MockLLMClient::new());
        let mut parser = NLCommandParser::new(mock_client);
        
        parser.parsing_history.push(ParsedCommand {
            intent: "test".to_string(),
            entities: HashMap::new(),
            confidence: 0.9,
            original_input: "test".to_string(),
            parsed_at: Utc::now(),
            id: "1".to_string(),
            alternatives: Vec::new(),
            needs_disambiguation: false,
        });
        
        assert!(!parser.parsing_history.is_empty());
        
        parser.clear_history();
        
        assert!(parser.parsing_history.is_empty());
    }

    #[test]
    fn test_disambiguation_question_generation() {
        let mock_client = Box::new(MockLLMClient::new());
        let parser = NLCommandParser::new(mock_client);
        
        let mut alternatives = Vec::new();
        alternatives.push(AlternativeInterpretation {
            intent: "list_todos".to_string(),
            entities: HashMap::new(),
            confidence: 0.6,
            reason: "Could be asking to list todos instead".to_string(),
        });
        
        let parsed = ParsedCommand {
            intent: "add_todo".to_string(),
            entities: HashMap::new(),
            confidence: 0.7,
            original_input: "show me todos".to_string(),
            parsed_at: Utc::now(),
            id: "test".to_string(),
            alternatives,
            needs_disambiguation: true,
        };
        
        let question = parser.generate_disambiguation_question(&parsed);
        assert!(question.is_some());
        
        let question = question.unwrap();
        assert_eq!(question.options.len(), 2);
        assert!(question.question.contains("show me todos"));
    }

    #[test]
    fn test_did_you_mean_suggestions() {
        let mock_client = Box::new(MockLLMClient::new());
        let parser = NLCommandParser::new(mock_client);
        
        let parsed = ParsedCommand {
            intent: "unknown".to_string(),
            entities: HashMap::new(),
            confidence: 0.3,
            original_input: "add task".to_string(),
            parsed_at: Utc::now(),
            id: "test".to_string(),
            alternatives: Vec::new(),
            needs_disambiguation: false,
        };
        
        let suggestions = parser.get_did_you_mean_suggestions(&parsed);
        assert!(!suggestions.is_empty());
        // The suggestions should contain either "Did you mean" or "Try:"
        assert!(suggestions.iter().any(|s| s.contains("Did you mean") || s.contains("Try:")));
    }

    #[test]
    fn test_resolve_disambiguation() {
        let mock_client = Box::new(MockLLMClient::new());
        let parser = NLCommandParser::new(mock_client);
        
        let options = vec![
            DisambiguationOption {
                display_text: "Option 1: Add a todo".to_string(),
                intent: "add_todo".to_string(),
                entities: HashMap::new(),
                description: "Add a new todo item".to_string(),
            },
            DisambiguationOption {
                display_text: "Option 2: List todos".to_string(),
                intent: "list_todos".to_string(),
                entities: HashMap::new(),
                description: "List all todo items".to_string(),
            },
        ];
        
        let question = DisambiguationQuestion {
            question: "What did you mean?".to_string(),
            options,
            context: "Test context".to_string(),
        };
        
        let resolved = parser.resolve_disambiguation(&question, 1).unwrap();
        assert_eq!(resolved.intent, "list_todos");
        assert_eq!(resolved.confidence, 1.0);
        assert!(!resolved.needs_disambiguation);
    }

    #[test]
    fn test_calculate_similarity() {
        let mock_client = Box::new(MockLLMClient::new());
        let parser = NLCommandParser::new(mock_client);
        
        let similarity = parser.calculate_similarity("add todo item", "add a todo");
        assert!(similarity > 0.5);
        
        let similarity = parser.calculate_similarity("completely different", "add todo");
        assert!(similarity < 0.3);
    }
}