use anyhow::{bail, Context, Result}; // Using anyhow for error handling
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use chrono::{DateTime, Utc};
use uuid;
use crate::ai_conversation::{LLMClient, Message, MessageRole};
use nalgebra::{DVector, Norm};
use std::path::Path;
use std::fs;
use rayon::prelude::*;

const MCP_SERVER_URL: &str = "https://127.0.0.1:27124"; // Default for Obsidian Local REST API
const ANALYSIS_VERSION: &str = "1.0.0";
const EMBEDDING_CACHE_FILE: &str = ".arrowhead_embeddings.bin";
const TEMPLATE_CACHE_FILE: &str = ".arrowhead_templates.bin";
const EMBEDDING_DIMENSION: usize = 768; // Common embedding dimension for many models

/// Content analysis results from AI processing
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContentAnalysis {
    /// Key themes and topics extracted from the content
    pub themes: Vec<String>,
    /// Sentiment analysis results
    pub sentiment: SentimentAnalysis,
    /// Key entities (people, places, organizations, etc.)
    pub entities: Vec<Entity>,
    /// Conceptual relationships and connections
    pub concepts: Vec<Concept>,
    /// Summary of the content
    pub summary: Option<String>,
    /// Keywords extracted from the content
    pub keywords: Vec<String>,
    /// Content classification/category
    pub category: Option<String>,
    /// Complexity score (0-10)
    pub complexity_score: Option<f32>,
    /// Reading time estimate in minutes
    pub reading_time_minutes: Option<u32>,
}

impl Default for ContentAnalysis {
    fn default() -> Self {
        Self {
            themes: Vec::new(),
            sentiment: SentimentAnalysis::default(),
            entities: Vec::new(),
            concepts: Vec::new(),
            summary: None,
            keywords: Vec::new(),
            category: None,
            complexity_score: None,
            reading_time_minutes: None,
        }
    }
}

/// Sentiment analysis results
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SentimentAnalysis {
    /// Overall sentiment: "positive", "negative", "neutral"
    pub overall: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Emotional tone indicators
    pub emotions: Vec<String>,
}

impl Default for SentimentAnalysis {
    fn default() -> Self {
        Self {
            overall: "neutral".to_string(),
            confidence: 0.0,
            emotions: Vec::new(),
        }
    }
}

/// Named entity with type and confidence
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Entity {
    /// The entity text
    pub text: String,
    /// Entity type (PERSON, ORG, LOCATION, etc.)
    pub entity_type: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Additional context or description
    pub context: Option<String>,
}

/// Conceptual relationship or idea
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Concept {
    /// The concept name
    pub name: String,
    /// Concept description
    pub description: Option<String>,
    /// Related concepts
    pub related_concepts: Vec<String>,
    /// Importance score (0.0 - 1.0)
    pub importance: f32,
}

/// Configuration for content analysis
#[derive(Debug, Clone)]
pub struct AnalysisConfig {
    /// Whether to extract themes
    pub extract_themes: bool,
    /// Whether to perform sentiment analysis
    pub analyze_sentiment: bool,
    /// Whether to extract entities
    pub extract_entities: bool,
    /// Whether to identify concepts
    pub identify_concepts: bool,
    /// Whether to generate summary
    pub generate_summary: bool,
    /// Maximum number of themes to extract
    pub max_themes: usize,
    /// Maximum number of entities to extract
    pub max_entities: usize,
    /// Maximum number of concepts to extract
    pub max_concepts: usize,
    /// Minimum confidence threshold for entities
    pub entity_confidence_threshold: f32,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            extract_themes: true,
            analyze_sentiment: true,
            extract_entities: true,
            identify_concepts: true,
            generate_summary: true,
            max_themes: 10,
            max_entities: 20,
            max_concepts: 15,
            entity_confidence_threshold: 0.7,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)] // Added Default and Clone
pub struct Frontmatter {
    // Define common frontmatter fields
    pub tags: Option<Vec<String>>,
    pub due_date: Option<String>,
    pub status: Option<String>, // Added for todos and goals
    pub target_date: Option<String>, // Added for goals
    
    // AI Analysis fields
    pub ai_analysis: Option<ContentAnalysis>,
    pub ai_analysis_version: Option<String>,
    pub ai_analysis_timestamp: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)] // Added Clone
pub struct MarkdownFile {
    pub frontmatter: Frontmatter, // Made public for easier access in handlers
    pub content: String,          // Made public
}

impl MarkdownFile {
    /// Helper to serialize just the frontmatter part to a YAML string.
    /// Useful if you need to reconstruct/update frontmatter specifically.
    pub fn frontmatter_to_string(&self) -> Result<String> {
        serde_yaml::to_string(&self.frontmatter).context("Failed to serialize frontmatter to YAML")
    }
}

/// Document embedding with metadata
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocumentEmbedding {
    /// Path to the document in the vault
    pub path: String,
    /// The embedding vector
    pub embedding: Vec<f32>,
    /// Hash of the content for cache validation
    pub content_hash: String,
    /// Timestamp when the embedding was created
    pub created_at: DateTime<Utc>,
    /// Document metadata
    pub metadata: DocumentMetadata,
}

/// Metadata for documents
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocumentMetadata {
    /// Document title
    pub title: String,
    /// Document tags from frontmatter
    pub tags: Vec<String>,
    /// Document length in characters
    pub length: usize,
    /// Document excerpt for preview
    pub excerpt: String,
    /// Last modified timestamp
    pub modified_at: Option<DateTime<Utc>>,
}

/// Semantic search result
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SemanticSearchResult {
    /// Path to the document
    pub path: String,
    /// Similarity score (0.0 to 1.0)
    pub similarity: f32,
    /// Document metadata
    pub metadata: DocumentMetadata,
    /// Matching content snippet
    pub snippet: String,
    /// Highlighted matches
    pub highlights: Vec<String>,
}

/// Semantic search query configuration
#[derive(Debug, Clone)]
pub struct SemanticSearchConfig {
    /// Maximum number of results to return
    pub max_results: usize,
    /// Minimum similarity threshold (0.0 to 1.0)
    pub min_similarity: f32,
    /// Whether to include content snippets
    pub include_snippets: bool,
    /// Length of content snippets
    pub snippet_length: usize,
    /// Whether to boost results based on recency
    pub boost_recent: bool,
    /// Whether to boost results based on tags
    pub boost_tags: bool,
}

impl Default for SemanticSearchConfig {
    fn default() -> Self {
        Self {
            max_results: 10,
            min_similarity: 0.5,
            include_snippets: true,
            snippet_length: 200,
            boost_recent: false,
            boost_tags: false,
        }
    }
}

/// Vector database for storing and searching embeddings
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VectorDatabase {
    /// All document embeddings
    pub embeddings: Vec<DocumentEmbedding>,
    /// Index for fast lookup by path
    pub path_index: HashMap<String, usize>,
    /// Version of the database
    pub version: String,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

/// Template component types
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum TemplateComponent {
    /// Static text content
    Text(String),
    /// Dynamic placeholder with hint
    Placeholder { name: String, hint: String, required: bool },
    /// Conditional section
    Conditional { condition: String, content: Vec<TemplateComponent> },
    /// Repeated section
    Repeating { item_name: String, content: Vec<TemplateComponent> },
    /// AI-generated content suggestion
    AiSuggestion { prompt: String, fallback: String },
    /// Link to other notes
    Link { target: String, display_text: Option<String> },
    /// Tag reference
    Tag(String),
}

/// Note template structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NoteTemplate {
    /// Template identifier
    pub id: String,
    /// Template name
    pub name: String,
    /// Template description
    pub description: String,
    /// Template category (e.g., "meeting", "project", "daily")
    pub category: String,
    /// Template components
    pub components: Vec<TemplateComponent>,
    /// Required frontmatter fields
    pub frontmatter_fields: Vec<FrontmatterField>,
    /// Template tags
    pub tags: Vec<String>,
    /// Usage statistics
    pub usage_stats: TemplateUsageStats,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modified timestamp
    pub modified_at: DateTime<Utc>,
}

/// Frontmatter field definition
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FrontmatterField {
    /// Field name
    pub name: String,
    /// Field type (string, array, date, etc.)
    pub field_type: String,
    /// Default value
    pub default_value: Option<String>,
    /// Whether the field is required
    pub required: bool,
    /// Field description
    pub description: Option<String>,
}

/// Template usage statistics
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TemplateUsageStats {
    /// Number of times used
    pub usage_count: u32,
    /// Last used timestamp
    pub last_used: Option<DateTime<Utc>>,
    /// User satisfaction rating (1-5)
    pub satisfaction_rating: Option<f32>,
    /// Number of customizations made
    pub customization_count: u32,
}

impl Default for TemplateUsageStats {
    fn default() -> Self {
        Self {
            usage_count: 0,
            last_used: None,
            satisfaction_rating: None,
            customization_count: 0,
        }
    }
}

/// Template pattern recognition result
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TemplatePattern {
    /// Pattern identifier
    pub id: String,
    /// Pattern name
    pub name: String,
    /// Pattern description
    pub description: String,
    /// Common structure elements
    pub structure_elements: Vec<String>,
    /// Frequently used tags
    pub common_tags: Vec<String>,
    /// Typical frontmatter fields
    pub typical_frontmatter: Vec<String>,
    /// Pattern confidence score (0.0-1.0)
    pub confidence: f32,
    /// Number of documents matching this pattern
    pub match_count: u32,
    /// Example document paths
    pub examples: Vec<String>,
}

/// Template generation request
#[derive(Debug, Clone)]
pub struct TemplateGenerationRequest {
    /// Template type/category
    pub template_type: String,
    /// Specific topic or domain
    pub topic: Option<String>,
    /// User preferences
    pub preferences: TemplatePreferences,
    /// Context from existing notes
    pub context: Option<String>,
    /// Required fields
    pub required_fields: Vec<String>,
    /// Example content for inspiration
    pub example_content: Option<String>,
}

/// User preferences for template generation
#[derive(Debug, Clone)]
pub struct TemplatePreferences {
    /// Preferred complexity level (simple, medium, detailed)
    pub complexity: String,
    /// Include AI suggestions
    pub include_ai_suggestions: bool,
    /// Preferred frontmatter fields
    pub preferred_frontmatter: Vec<String>,
    /// Preferred tags
    pub preferred_tags: Vec<String>,
    /// Writing style preference
    pub writing_style: Option<String>,
    /// Language preference
    pub language: Option<String>,
}

impl Default for TemplatePreferences {
    fn default() -> Self {
        Self {
            complexity: "medium".to_string(),
            include_ai_suggestions: true,
            preferred_frontmatter: vec!["tags".to_string(), "created".to_string()],
            preferred_tags: Vec::new(),
            writing_style: None,
            language: Some("en".to_string()),
        }
    }
}

/// Template generation result
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TemplateGenerationResult {
    /// Generated template
    pub template: NoteTemplate,
    /// Generation metadata
    pub metadata: TemplateGenerationMetadata,
    /// Suggestions for improvement
    pub suggestions: Vec<String>,
    /// Confidence score of the generation
    pub confidence: f32,
}

/// Metadata about template generation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TemplateGenerationMetadata {
    /// Generation timestamp
    pub generated_at: DateTime<Utc>,
    /// AI model used
    pub model_used: String,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
    /// Source patterns used
    pub source_patterns: Vec<String>,
    /// Number of examples analyzed
    pub examples_analyzed: u32,
}

/// Template database for storing and managing templates
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TemplateDatabase {
    /// All templates
    pub templates: Vec<NoteTemplate>,
    /// Template patterns discovered
    pub patterns: Vec<TemplatePattern>,
    /// Template index by category
    pub category_index: HashMap<String, Vec<usize>>,
    /// Template index by tags
    pub tag_index: HashMap<String, Vec<usize>>,
    /// Database version
    pub version: String,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

/// Tag suggestion with confidence score
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TagSuggestion {
    /// Suggested tag
    pub tag: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Reason for suggestion
    pub reason: String,
    /// Source of suggestion (theme, entity, keyword, etc.)
    pub source: TagSource,
}

/// Source of tag suggestion
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TagSource {
    /// From content themes
    Theme,
    /// From extracted entities
    Entity,
    /// From keywords
    Keyword,
    /// From category classification
    Category,
    /// From semantic similarity
    Similarity,
    /// From manual rules
    Rule,
}

/// Folder organization suggestion
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FolderSuggestion {
    /// Suggested folder path
    pub folder_path: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Reason for suggestion
    pub reason: String,
    /// Category that led to this suggestion
    pub category: String,
}

/// Auto-linking suggestion
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinkSuggestion {
    /// Target note path
    pub target_path: String,
    /// Suggested link text
    pub link_text: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Context where link should be inserted
    pub context: String,
    /// Reason for suggestion
    pub reason: String,
}

/// Organization recommendations for a note
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrganizationRecommendations {
    /// Suggested tags to add
    pub suggested_tags: Vec<TagSuggestion>,
    /// Suggested folder locations
    pub folder_suggestions: Vec<FolderSuggestion>,
    /// Suggested links to other notes
    pub link_suggestions: Vec<LinkSuggestion>,
    /// Overall confidence in recommendations
    pub overall_confidence: f32,
    /// Timestamp of analysis
    pub generated_at: DateTime<Utc>,
}

/// Configuration for automated organization
#[derive(Debug, Clone)]
pub struct OrganizationConfig {
    /// Minimum confidence for auto-applying tags
    pub auto_tag_confidence_threshold: f32,
    /// Maximum number of tags to suggest
    pub max_tag_suggestions: usize,
    /// Maximum number of folder suggestions
    pub max_folder_suggestions: usize,
    /// Maximum number of link suggestions
    pub max_link_suggestions: usize,
    /// Whether to auto-apply high-confidence tags
    pub auto_apply_tags: bool,
    /// Whether to suggest folder moves
    pub suggest_folder_moves: bool,
    /// Whether to suggest auto-linking
    pub suggest_auto_links: bool,
    /// Custom tag rules
    pub custom_tag_rules: Vec<TagRule>,
}

/// Custom tag rule
#[derive(Debug, Clone)]
pub struct TagRule {
    /// Rule name
    pub name: String,
    /// Content pattern to match
    pub pattern: String,
    /// Tag to apply
    pub tag: String,
    /// Confidence to assign
    pub confidence: f32,
}

impl Default for OrganizationConfig {
    fn default() -> Self {
        Self {
            auto_tag_confidence_threshold: 0.8,
            max_tag_suggestions: 10,
            max_folder_suggestions: 3,
            max_link_suggestions: 5,
            auto_apply_tags: false,
            suggest_folder_moves: true,
            suggest_auto_links: true,
            custom_tag_rules: Vec::new(),
        }
    }
}

/// Content suggestion while writing
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContentSuggestion {
    /// Suggested content text
    pub text: String,
    /// Type of suggestion
    pub suggestion_type: SuggestionType,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Context where suggestion applies
    pub context: String,
    /// Reason for the suggestion
    pub reason: String,
    /// Related source documents
    pub source_documents: Vec<String>,
    /// Position in document where suggestion applies
    pub position: Option<ContentPosition>,
}

/// Type of content suggestion
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SuggestionType {
    /// Continue writing based on context
    ContentContinuation,
    /// Suggest related information
    RelatedContent,
    /// Suggest a link to another note
    LinkSuggestion,
    /// Suggest completing a sentence/paragraph
    TextCompletion,
    /// Suggest adding a heading
    HeadingSuggestion,
    /// Suggest adding a bullet point
    BulletPointSuggestion,
    /// Suggest adding a code block
    CodeBlockSuggestion,
}

/// Position in document for content suggestions
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContentPosition {
    /// Line number (0-based)
    pub line: usize,
    /// Column number (0-based)
    pub column: usize,
    /// Length of text to replace (if any)
    pub length: Option<usize>,
}

/// Auto-link insertion result
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AutoLinkResult {
    /// Original text
    pub original_text: String,
    /// Text with auto-links inserted
    pub linked_text: String,
    /// Links that were inserted
    pub inserted_links: Vec<InsertedLink>,
    /// Total number of links added
    pub links_added: usize,
}

/// Information about an inserted link
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InsertedLink {
    /// Original text that was linked
    pub original_text: String,
    /// Link target (note path)
    pub target: String,
    /// Link display text
    pub display_text: String,
    /// Position where link was inserted
    pub position: ContentPosition,
    /// Confidence score for the link
    pub confidence: f32,
}

/// Content suggestion request
#[derive(Debug, Clone)]
pub struct ContentSuggestionRequest {
    /// Current document content
    pub content: String,
    /// Cursor position in document
    pub cursor_position: ContentPosition,
    /// Number of suggestions to return
    pub max_suggestions: usize,
    /// Context around cursor (characters before/after)
    pub context_window: usize,
    /// Types of suggestions to include
    pub suggestion_types: Vec<SuggestionType>,
}

/// Configuration for content suggestions
#[derive(Debug, Clone)]
pub struct ContentSuggestionConfig {
    /// Whether to enable real-time suggestions
    pub enable_real_time: bool,
    /// Minimum confidence threshold for suggestions
    pub min_confidence: f32,
    /// Maximum number of suggestions to return
    pub max_suggestions: usize,
    /// Context window size (characters)
    pub context_window: usize,
    /// Debounce delay for real-time suggestions (milliseconds)
    pub debounce_delay_ms: u64,
    /// Whether to auto-insert high-confidence links
    pub auto_insert_links: bool,
    /// Minimum confidence for auto-inserted links
    pub auto_link_confidence_threshold: f32,
    /// Cache timeout for suggestions (seconds)
    pub cache_timeout_seconds: u64,
}

impl Default for ContentSuggestionConfig {
    fn default() -> Self {
        Self {
            enable_real_time: true,
            min_confidence: 0.6,
            max_suggestions: 5,
            context_window: 200,
            debounce_delay_ms: 300,
            auto_insert_links: false,
            auto_link_confidence_threshold: 0.8,
            cache_timeout_seconds: 300,
        }
    }
}

/// Cached suggestion entry
#[derive(Debug, Clone)]
struct CachedSuggestion {
    /// The suggestion
    suggestion: ContentSuggestion,
    /// When it was cached
    cached_at: DateTime<Utc>,
    /// Content hash when cached
    content_hash: String,
}

/// Content suggestion cache
#[derive(Debug, Clone)]
struct SuggestionCache {
    /// Cached suggestions by content hash
    suggestions: HashMap<String, Vec<CachedSuggestion>>,
    /// Cache hit statistics
    hit_count: usize,
    /// Cache miss statistics
    miss_count: usize,
}

pub struct ObsidianAdapter {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    llm_client: Option<Box<dyn LLMClient>>,
    analysis_config: AnalysisConfig,
    analysis_cache: HashMap<String, (ContentAnalysis, DateTime<Utc>)>,
    vector_database: VectorDatabase,
    search_config: SemanticSearchConfig,
    embedding_cache_path: String,
    template_database: TemplateDatabase,
    template_cache_path: String,
    organization_config: OrganizationConfig,
    content_suggestion_config: ContentSuggestionConfig,
    suggestion_cache: SuggestionCache,
}

impl ObsidianAdapter {
    pub fn new(base_url: Option<String>, api_key: Option<String>) -> Self {
        // Create a client that accepts self-signed certificates for localhost
        let client = Client::builder()
            .danger_accept_invalid_certs(true) // For self-signed certificates
            .build()
            .expect("Failed to create HTTP client");

        let vector_db = VectorDatabase {
            embeddings: Vec::new(),
            path_index: HashMap::new(),
            version: "1.0.0".to_string(),
            last_updated: Utc::now(),
        };

        let template_db = TemplateDatabase {
            templates: Vec::new(),
            patterns: Vec::new(),
            category_index: HashMap::new(),
            tag_index: HashMap::new(),
            version: "1.0.0".to_string(),
            last_updated: Utc::now(),
        };

        ObsidianAdapter {
            client,
            base_url: base_url.unwrap_or_else(|| MCP_SERVER_URL.to_string()),
            api_key,
            llm_client: None,
            analysis_config: AnalysisConfig::default(),
            analysis_cache: HashMap::new(),
            vector_database: vector_db,
            search_config: SemanticSearchConfig::default(),
            embedding_cache_path: EMBEDDING_CACHE_FILE.to_string(),
            template_database: template_db,
            template_cache_path: TEMPLATE_CACHE_FILE.to_string(),
            organization_config: OrganizationConfig::default(),
            content_suggestion_config: ContentSuggestionConfig::default(),
            suggestion_cache: SuggestionCache {
                suggestions: HashMap::new(),
                hit_count: 0,
                miss_count: 0,
            },
        }
    }

    /// Create a new ObsidianAdapter with AI analysis capabilities
    pub fn with_ai_client(
        base_url: Option<String>, 
        api_key: Option<String>, 
        llm_client: Box<dyn LLMClient>,
        analysis_config: Option<AnalysisConfig>
    ) -> Self {
        let client = Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .expect("Failed to create HTTP client");

        let vector_db = VectorDatabase {
            embeddings: Vec::new(),
            path_index: HashMap::new(),
            version: "1.0.0".to_string(),
            last_updated: Utc::now(),
        };

        let template_db = TemplateDatabase {
            templates: Vec::new(),
            patterns: Vec::new(),
            category_index: HashMap::new(),
            tag_index: HashMap::new(),
            version: "1.0.0".to_string(),
            last_updated: Utc::now(),
        };

        ObsidianAdapter {
            client,
            base_url: base_url.unwrap_or_else(|| MCP_SERVER_URL.to_string()),
            api_key,
            llm_client: Some(llm_client),
            analysis_config: analysis_config.unwrap_or_default(),
            analysis_cache: HashMap::new(),
            vector_database: vector_db,
            search_config: SemanticSearchConfig::default(),
            embedding_cache_path: EMBEDDING_CACHE_FILE.to_string(),
            template_database: template_db,
            template_cache_path: TEMPLATE_CACHE_FILE.to_string(),
            organization_config: OrganizationConfig::default(),
            content_suggestion_config: ContentSuggestionConfig::default(),
            suggestion_cache: SuggestionCache {
                suggestions: HashMap::new(),
                hit_count: 0,
                miss_count: 0,
            },
        }
    }

    /// Set the LLM client for AI analysis
    pub fn set_llm_client(&mut self, llm_client: Box<dyn LLMClient>) {
        self.llm_client = Some(llm_client);
    }

    /// Update analysis configuration
    pub fn set_analysis_config(&mut self, config: AnalysisConfig) {
        self.analysis_config = config;
    }

    /// Clear the analysis cache
    pub fn clear_analysis_cache(&mut self) {
        self.analysis_cache.clear();
    }

    // Helper method to add authorization header if API key is present
    fn add_auth_header(&self, request_builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(ref api_key) = self.api_key {
            request_builder.header("Authorization", format!("Bearer {}", api_key))
        } else {
            request_builder
        }
    }

    // Made public for potential direct use if needed, though get_markdown_file_data is preferred
    pub fn parse_markdown_file(raw_content: &str) -> Result<MarkdownFile> {
        let parts: Vec<&str> = raw_content.splitn(3, "---").collect();
        if parts.len() < 3 {
            // No frontmatter or malformed, return with default frontmatter and all content as body
            return Ok(MarkdownFile {
                frontmatter: Frontmatter::default(),
                content: raw_content.to_string(),
            });
        }

        let yaml_str = parts[1].trim();
        let content_str = parts[2].trim_start().to_string();

        // If YAML is empty, serde_yaml::from_str("") might error or return unit type.
        // We want a default Frontmatter in this case.
        let frontmatter: Frontmatter = if yaml_str.is_empty() {
            Frontmatter::default()
        } else {
            serde_yaml::from_str(yaml_str)
                .context(format!("Failed to parse YAML frontmatter: '{}'", yaml_str))?
        };

        Ok(MarkdownFile {
            frontmatter,
            content: content_str,
        })
    }

    // Made public for potential direct use
    pub fn serialize_markdown_file(file: &MarkdownFile) -> Result<String> {
        // Ensure frontmatter isn't just defaults if we don't want to write empty "--- \n ---"
        // However, always writing it is consistent. Serde_yaml handles Option types well (omits if None).
        let fm_yaml = serde_yaml::to_string(&file.frontmatter)
            .context("Failed to serialize frontmatter to YAML")?;

        // Avoid serializing an empty "null" or "{}" if frontmatter is truly empty/default
        // and all its fields are None.
        // A simple check: if the serialized YAML is just "---" or "--- {}" or "--- null", treat as empty.
        // For now, `trim()` handles `--- \n---` becoming `--- \n---`
        // An empty `Frontmatter::default()` serializes to `status: null\ntarget_date: null\n` if these fields are present.
        // `serde_yaml` with `skip_serializing_if = "Option::is_none"` on struct fields is better.
        // Let's add that to Frontmatter struct. (Will do this in a follow up if needed, for now it's fine)

        Ok(format!("---\n{}---\n\n{}", fm_yaml.trim(), file.content))
    }

    pub async fn get_file(&self, vault_path: &str) -> Result<String> {
        let url = format!("{}/vault/{}", self.base_url, vault_path);
        let response = self
            .add_auth_header(self.client.get(&url).header("Accept", "text/markdown"))
            .send()
            .await
            .context(format!("Failed to send GET request to {}", url))?;

        if response.status().is_success() {
            response
                .text()
                .await
                .context("Failed to read response text")
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            bail!(
                "MCP server returned error {}: {}. URL: {}",
                status,
                error_text,
                url
            )
        }
    }

    pub async fn create_file(&self, vault_path: &str, content: &str) -> Result<()> {
        let url = format!("{}/vault/{}", self.base_url, vault_path);
        let response = self
            .add_auth_header(
                self.client
                    .post(&url)
                    .header("Content-Type", "text/markdown")
                    .body(content.to_string()),
            )
            .send()
            .await
            .context(format!("Failed to send POST request to {}", url))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            bail!(
                "MCP server returned error {}: {}. URL: {}",
                status,
                error_text,
                url
            )
        }
    }

    pub async fn update_file(&self, vault_path: &str, content: &str) -> Result<()> {
        let url = format!("{}/vault/{}", self.base_url, vault_path);
        let response = self
            .add_auth_header(
                self.client
                    .put(&url)
                    .header("Content-Type", "text/markdown")
                    .body(content.to_string()),
            )
            .send()
            .await
            .context(format!("Failed to send PUT request to {}", url))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            bail!(
                "MCP server returned error {}: {}. URL: {}",
                status,
                error_text,
                url
            )
        }
    }

    pub async fn get_markdown_file_data(&self, vault_path: &str) -> Result<MarkdownFile> {
        let raw_content = self.get_file(vault_path).await?;
        Self::parse_markdown_file(&raw_content)
    }

    pub async fn save_markdown_file_data(
        &self,
        vault_path: &str,
        file_data: &MarkdownFile,
        overwrite: bool,
    ) -> Result<()> {
        let serialized_content = Self::serialize_markdown_file(file_data)?;
        if overwrite {
            // For Obsidian Local REST API, PUT is typically used for update/overwrite
            self.update_file(vault_path, &serialized_content).await
        } else {
            // POST can often create or overwrite. If strict create-only is needed,
            // one might need to check for file existence first (e.g. with a GET).
            // Assuming POST here means "create or replace".
            self.create_file(vault_path, &serialized_content).await
        }
    }

    /// Analyze the content of a markdown file using AI
    pub async fn analyze_content(&mut self, content: &str) -> Result<ContentAnalysis> {
        let llm_client = self.llm_client.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No LLM client configured for content analysis"))?;

        // Check cache first
        let cache_key = self.generate_cache_key(content);
        if let Some((cached_analysis, timestamp)) = self.analysis_cache.get(&cache_key) {
            // Check if cache is still valid (24 hours)
            if Utc::now().signed_duration_since(*timestamp).num_hours() < 24 {
                return Ok(cached_analysis.clone());
            }
        }

        // Create analysis prompt
        let analysis_prompt = self.create_analysis_prompt(content);
        
        let messages = vec![
            Message {
                id: uuid::Uuid::new_v4().to_string(),
                role: MessageRole::System,
                content: "You are an expert content analyst. Analyze the provided text and return a detailed JSON response with the requested information.".to_string(),
                timestamp: Utc::now(),
                function_call: None,
            },
            Message {
                id: uuid::Uuid::new_v4().to_string(),
                role: MessageRole::User,
                content: analysis_prompt,
                timestamp: Utc::now(),
                function_call: None,
            }
        ];

        let response = llm_client.send_message(messages).await
            .context("Failed to get AI analysis response")?;

        let analysis = self.parse_analysis_response(&response.content)?;
        
        // Cache the result
        self.analysis_cache.insert(cache_key, (analysis.clone(), Utc::now()));
        
        Ok(analysis)
    }

    /// Analyze a markdown file and update its frontmatter with AI analysis
    pub async fn analyze_and_update_file(&mut self, vault_path: &str) -> Result<MarkdownFile> {
        let mut file_data = self.get_markdown_file_data(vault_path).await?;
        
        // Check if analysis already exists and is recent
        if let Some(ref _ai_analysis) = file_data.frontmatter.ai_analysis {
            if let Some(ref timestamp_str) = file_data.frontmatter.ai_analysis_timestamp {
                if let Ok(timestamp) = DateTime::parse_from_rfc3339(timestamp_str) {
                    let age_hours = Utc::now().signed_duration_since(timestamp.with_timezone(&Utc)).num_hours();
                    if age_hours < 24 {
                        // Analysis is recent, return as-is
                        return Ok(file_data);
                    }
                }
            }
        }

        // Perform analysis
        let analysis = self.analyze_content(&file_data.content).await?;
        
        // Update frontmatter
        file_data.frontmatter.ai_analysis = Some(analysis);
        file_data.frontmatter.ai_analysis_version = Some(ANALYSIS_VERSION.to_string());
        file_data.frontmatter.ai_analysis_timestamp = Some(Utc::now().to_rfc3339());
        
        // Save updated file
        self.save_markdown_file_data(vault_path, &file_data, true).await?;
        
        Ok(file_data)
    }

    /// Get analysis for content without updating the file
    pub async fn get_content_analysis(&mut self, content: &str) -> Result<ContentAnalysis> {
        self.analyze_content(content).await
    }

    /// Batch analyze multiple files
    pub async fn batch_analyze_files(&mut self, vault_paths: Vec<&str>) -> Result<Vec<(String, Result<ContentAnalysis>)>> {
        let mut results = Vec::new();
        
        for path in vault_paths {
            let result = match self.get_markdown_file_data(path).await {
                Ok(file_data) => self.analyze_content(&file_data.content).await,
                Err(e) => Err(e),
            };
            results.push((path.to_string(), result));
        }
        
        Ok(results)
    }

    /// Helper method to generate cache key from content
    fn generate_cache_key(&self, content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("content_{:x}", hasher.finish())
    }

    /// Create the analysis prompt for the AI
    fn create_analysis_prompt(&self, content: &str) -> String {
        let word_count = content.split_whitespace().count();
        let reading_time = (word_count as f32 / 200.0).ceil() as u32; // Assume 200 words per minute
        
        format!(r#"Please analyze the following content and return a JSON response with the following structure:

{{
  "themes": ["theme1", "theme2", ...], // Max {} themes
  "sentiment": {{
    "overall": "positive|negative|neutral",
    "confidence": 0.0-1.0,
    "emotions": ["emotion1", "emotion2", ...]
  }},
  "entities": [
    {{
      "text": "entity text",
      "entity_type": "PERSON|ORG|LOCATION|MISC",
      "confidence": 0.0-1.0,
      "context": "optional context"
    }}
  ], // Max {} entities
  "concepts": [
    {{
      "name": "concept name",
      "description": "concept description",
      "related_concepts": ["related1", "related2"],
      "importance": 0.0-1.0
    }}
  ], // Max {} concepts
  "summary": "Brief summary of the content",
  "keywords": ["keyword1", "keyword2", ...],
  "category": "category name",
  "complexity_score": 0.0-10.0,
  "reading_time_minutes": {}
}}

Focus on extracting meaningful insights. For themes, identify the main topics discussed. For sentiment, analyze the overall emotional tone. For entities, extract people, organizations, locations, and other important entities. For concepts, identify key ideas and their relationships.

Content to analyze:
{}
"#, 
            self.analysis_config.max_themes,
            self.analysis_config.max_entities,
            self.analysis_config.max_concepts,
            reading_time,
            content
        )
    }

    /// Parse the AI response into ContentAnalysis struct
    fn parse_analysis_response(&self, response_content: &str) -> Result<ContentAnalysis> {
        // Try to extract JSON from the response
        let json_start = response_content.find('{').unwrap_or(0);
        let json_end = response_content.rfind('}').map(|i| i + 1).unwrap_or(response_content.len());
        let json_str = &response_content[json_start..json_end];
        
        let parsed: serde_json::Value = serde_json::from_str(json_str)
            .context("Failed to parse AI response as JSON")?;
        
        // Extract themes
        let themes = parsed.get("themes")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .take(self.analysis_config.max_themes)
                .collect())
            .unwrap_or_default();
        
        // Extract sentiment
        let sentiment = if let Some(sentiment_obj) = parsed.get("sentiment") {
            SentimentAnalysis {
                overall: sentiment_obj.get("overall")
                    .and_then(|v| v.as_str())
                    .unwrap_or("neutral")
                    .to_string(),
                confidence: sentiment_obj.get("confidence")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as f32,
                emotions: sentiment_obj.get("emotions")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect())
                    .unwrap_or_default(),
            }
        } else {
            SentimentAnalysis::default()
        };
        
        // Extract entities
        let entities = parsed.get("entities")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|entity| {
                    let text = entity.get("text")?.as_str()?.to_string();
                    let entity_type = entity.get("entity_type")?.as_str()?.to_string();
                    let confidence = entity.get("confidence")?.as_f64()? as f32;
                    
                    if confidence >= self.analysis_config.entity_confidence_threshold {
                        Some(Entity {
                            text,
                            entity_type,
                            confidence,
                            context: entity.get("context")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                        })
                    } else {
                        None
                    }
                })
                .take(self.analysis_config.max_entities)
                .collect())
            .unwrap_or_default();
        
        // Extract concepts
        let concepts = parsed.get("concepts")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|concept| {
                    let name = concept.get("name")?.as_str()?.to_string();
                    let importance = concept.get("importance")?.as_f64()? as f32;
                    
                    Some(Concept {
                        name,
                        description: concept.get("description")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        related_concepts: concept.get("related_concepts")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect())
                            .unwrap_or_default(),
                        importance,
                    })
                })
                .take(self.analysis_config.max_concepts)
                .collect())
            .unwrap_or_default();
        
        // Extract other fields
        let summary = parsed.get("summary")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let keywords = parsed.get("keywords")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect())
            .unwrap_or_default();
        
        let category = parsed.get("category")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let complexity_score = parsed.get("complexity_score")
            .and_then(|v| v.as_f64())
            .map(|f| f as f32);
        
        let reading_time_minutes = parsed.get("reading_time_minutes")
            .and_then(|v| v.as_u64())
            .map(|u| u as u32);
        
        Ok(ContentAnalysis {
            themes,
            sentiment,
            entities,
            concepts,
            summary,
            keywords,
            category,
            complexity_score,
            reading_time_minutes,
        })
    }

    /// Load the vector database from disk
    pub fn load_vector_database(&mut self) -> Result<()> {
        if Path::new(&self.embedding_cache_path).exists() {
            let data = fs::read(&self.embedding_cache_path)
                .context("Failed to read embedding cache file")?;
            self.vector_database = bincode::deserialize(&data)
                .context("Failed to deserialize vector database")?;
        }
        Ok(())
    }

    /// Save the vector database to disk
    pub fn save_vector_database(&self) -> Result<()> {
        let data = bincode::serialize(&self.vector_database)
            .context("Failed to serialize vector database")?;
        fs::write(&self.embedding_cache_path, data)
            .context("Failed to write embedding cache file")?;
        Ok(())
    }

    /// Set semantic search configuration
    pub fn set_search_config(&mut self, config: SemanticSearchConfig) {
        self.search_config = config;
    }

    /// Generate embeddings for content using the LLM client
    pub async fn generate_embeddings(&self, content: &str) -> Result<Vec<f32>> {
        let llm_client = self.llm_client.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No LLM client configured for embeddings"))?;

        let messages = vec![
            Message {
                id: uuid::Uuid::new_v4().to_string(),
                role: MessageRole::System,
                content: "You are an embedding generator. Generate a 768-dimensional vector embedding for the given text. Return only a JSON array of 768 floating point numbers.".to_string(),
                timestamp: Utc::now(),
                function_call: None,
            },
            Message {
                id: uuid::Uuid::new_v4().to_string(),
                role: MessageRole::User,
                content: format!("Generate embedding for: {}", content),
                timestamp: Utc::now(),
                function_call: None,
            }
        ];

        let response = llm_client.send_message(messages).await
            .context("Failed to generate embeddings")?;

        // Parse the embedding from the response
        let embedding_json = response.content.trim();
        let embedding: Vec<f32> = serde_json::from_str(embedding_json)
            .context("Failed to parse embedding JSON")?;

        if embedding.len() != EMBEDDING_DIMENSION {
            bail!("Embedding dimension mismatch: expected {}, got {}", EMBEDDING_DIMENSION, embedding.len());
        }

        Ok(embedding)
    }

    /// Generate embedding for a document and store it
    pub async fn embed_document(&mut self, vault_path: &str) -> Result<()> {
        let file_data = self.get_markdown_file_data(vault_path).await?;
        let content_hash = self.generate_cache_key(&file_data.content);
        
        // Check if we already have a recent embedding
        if let Some(index) = self.vector_database.path_index.get(vault_path) {
            if let Some(existing_embedding) = self.vector_database.embeddings.get(*index) {
                if existing_embedding.content_hash == content_hash {
                    // Content hasn't changed, no need to re-embed
                    return Ok(());
                }
            }
        }

        // Generate new embedding
        let embedding = self.generate_embeddings(&file_data.content).await?;
        
        // Extract document metadata
        let title = file_data.frontmatter.tags.as_ref()
            .and_then(|tags| tags.first().cloned())
            .unwrap_or_else(|| vault_path.to_string());
        
        let tags = file_data.frontmatter.tags.unwrap_or_default();
        let excerpt = file_data.content.chars().take(200).collect::<String>();
        
        let metadata = DocumentMetadata {
            title,
            tags,
            length: file_data.content.len(),
            excerpt,
            modified_at: Some(Utc::now()),
        };

        let doc_embedding = DocumentEmbedding {
            path: vault_path.to_string(),
            embedding,
            content_hash,
            created_at: Utc::now(),
            metadata,
        };

        // Update the vector database
        if let Some(index) = self.vector_database.path_index.get(vault_path) {
            // Update existing embedding
            self.vector_database.embeddings[*index] = doc_embedding;
        } else {
            // Add new embedding
            let index = self.vector_database.embeddings.len();
            self.vector_database.embeddings.push(doc_embedding);
            self.vector_database.path_index.insert(vault_path.to_string(), index);
        }

        self.vector_database.last_updated = Utc::now();
        self.save_vector_database()?;

        Ok(())
    }

    /// Batch embed multiple documents
    pub async fn batch_embed_documents(&mut self, vault_paths: Vec<&str>) -> Result<Vec<(String, Result<()>)>> {
        let mut results = Vec::new();
        
        for path in vault_paths {
            let result = self.embed_document(path).await;
            results.push((path.to_string(), result));
        }
        
        Ok(results)
    }

    /// Calculate cosine similarity between two vectors
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        let vec_a = DVector::from_row_slice(a);
        let vec_b = DVector::from_row_slice(b);
        
        let dot_product = vec_a.dot(&vec_b);
        let norm_a = vec_a.norm();
        let norm_b = vec_b.norm();
        
        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_product / (norm_a * norm_b)
        }
    }

    /// Perform semantic search across the vault
    pub async fn semantic_search(&mut self, query: &str) -> Result<Vec<SemanticSearchResult>> {
        // Generate embedding for the query
        let query_embedding = self.generate_embeddings(query).await?;
        
        // Calculate similarities with all documents
        let mut similarities: Vec<(usize, f32)> = self.vector_database.embeddings
            .par_iter()
            .enumerate()
            .map(|(i, doc_embedding)| {
                let similarity = self.cosine_similarity(&query_embedding, &doc_embedding.embedding);
                (i, similarity)
            })
            .collect();

        // Sort by similarity (descending)
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Filter by minimum similarity and take top results
        let results: Vec<SemanticSearchResult> = similarities
            .into_iter()
            .filter(|(_, sim)| *sim >= self.search_config.min_similarity)
            .take(self.search_config.max_results)
            .map(|(index, similarity)| {
                let doc_embedding = &self.vector_database.embeddings[index];
                let snippet = if self.search_config.include_snippets {
                    self.generate_snippet(&doc_embedding.path, query).unwrap_or_else(|_| doc_embedding.metadata.excerpt.clone())
                } else {
                    String::new()
                };
                
                SemanticSearchResult {
                    path: doc_embedding.path.clone(),
                    similarity,
                    metadata: doc_embedding.metadata.clone(),
                    snippet,
                    // TODO: Implement search result highlighting
                    // Current state: Empty highlights vector, no text highlighting
                    // 
                    // Implementation requirements:
                    // 1. Query term extraction:
                    //    - Parse search query to extract individual terms
                    //    - Handle quoted phrases, wildcards, and special operators
                    //    - Support stemming and fuzzy matching
                    // 2. Text highlighting:
                    //    - Find all occurrences of query terms in the snippet
                    //    - Calculate character positions for highlighting
                    //    - Handle case-insensitive matching
                    //    - Support partial word matches and synonyms
                    // 3. Highlight data structure:
                    //    - Create highlight spans with start/end positions
                    //    - Include highlight type (exact match, fuzzy, etc.)
                    //    - Support multiple highlight colors/styles
                    // 4. Context preservation:
                    //    - Ensure highlights align with snippet boundaries
                    //    - Handle multi-line highlights properly
                    //    - Preserve markdown formatting around highlights
                    // 5. Performance considerations:
                    //    - Optimize for large documents and many search terms
                    //    - Cache highlighting results for repeated searches
                    //    - Limit highlight processing time for responsiveness
                    highlights: vec![], // TODO: Implement highlighting
                }
            })
            .collect();

        Ok(results)
    }

    /// Generate a relevant snippet from a document for a query
    fn generate_snippet(&self, _vault_path: &str, _query: &str) -> Result<String> {
        // This is a simplified implementation
        // In a real implementation, you would:
        // 1. Load the document content
        // 2. Find the most relevant passages
        // 3. Extract snippets around those passages
        // 4. Highlight matching terms
        
        // For now, return a placeholder
        Ok("Document snippet...".to_string())
    }

    /// Get all documents in the vector database
    pub fn get_indexed_documents(&self) -> Vec<&DocumentEmbedding> {
        self.vector_database.embeddings.iter().collect()
    }

    /// Remove a document from the vector database
    pub fn remove_document_embedding(&mut self, vault_path: &str) -> Result<()> {
        if let Some(index) = self.vector_database.path_index.remove(vault_path) {
            self.vector_database.embeddings.remove(index);
            
            // Update indices for remaining documents
            for (path, idx) in self.vector_database.path_index.iter_mut() {
                if *idx > index {
                    *idx -= 1;
                }
            }
            
            self.vector_database.last_updated = Utc::now();
            self.save_vector_database()?;
        }
        Ok(())
    }

    /// Clear all embeddings from the vector database
    pub fn clear_vector_database(&mut self) -> Result<()> {
        self.vector_database.embeddings.clear();
        self.vector_database.path_index.clear();
        self.vector_database.last_updated = Utc::now();
        self.save_vector_database()?;
        Ok(())
    }

    /// Get vector database statistics
    pub fn get_vector_database_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();
        stats.insert("total_documents".to_string(), serde_json::Value::Number(self.vector_database.embeddings.len().into()));
        stats.insert("last_updated".to_string(), serde_json::Value::String(self.vector_database.last_updated.to_rfc3339()));
        stats.insert("version".to_string(), serde_json::Value::String(self.vector_database.version.clone()));
        stats.insert("embedding_dimension".to_string(), serde_json::Value::Number(EMBEDDING_DIMENSION.into()));
        stats
    }

    /// Load the template database from disk
    pub fn load_template_database(&mut self) -> Result<()> {
        if Path::new(&self.template_cache_path).exists() {
            let data = fs::read(&self.template_cache_path)
                .context("Failed to read template cache file")?;
            self.template_database = bincode::deserialize(&data)
                .context("Failed to deserialize template database")?;
        }
        Ok(())
    }

    /// Save the template database to disk
    pub fn save_template_database(&self) -> Result<()> {
        let data = bincode::serialize(&self.template_database)
            .context("Failed to serialize template database")?;
        fs::write(&self.template_cache_path, data)
            .context("Failed to write template cache file")?;
        Ok(())
    }

    /// Analyze existing notes to discover template patterns
    pub async fn discover_template_patterns(&mut self, vault_paths: Vec<&str>) -> Result<Vec<TemplatePattern>> {
        let mut patterns = Vec::new();
        let mut document_analyses = Vec::new();

        // Analyze all provided documents
        for path in vault_paths {
            match self.get_markdown_file_data(path).await {
                Ok(file_data) => {
                    let analysis = self.analyze_content(&file_data.content).await?;
                    document_analyses.push((path.to_string(), file_data, analysis));
                }
                Err(e) => {
                    log::warn!("Failed to analyze document {}: {}", path, e);
                }
            }
        }

        // Group documents by similar patterns
        let grouped_patterns = self.group_documents_by_patterns(&document_analyses)?;

        // Generate template patterns from groups
        for (pattern_name, group) in grouped_patterns {
            let pattern = self.create_template_pattern(pattern_name, group)?;
            patterns.push(pattern);
        }

        // Update template database with discovered patterns
        self.template_database.patterns = patterns.clone();
        self.template_database.last_updated = Utc::now();
        self.save_template_database()?;

        Ok(patterns)
    }

    /// Group documents by similar patterns
    fn group_documents_by_patterns(&self, documents: &[(String, MarkdownFile, ContentAnalysis)]) -> Result<HashMap<String, Vec<(String, MarkdownFile, ContentAnalysis)>>> {
        let mut groups: HashMap<String, Vec<(String, MarkdownFile, ContentAnalysis)>> = HashMap::new();

        for (path, file_data, analysis) in documents {
            // Determine pattern category based on analysis
            let pattern_category = self.determine_pattern_category(analysis, file_data);
            
            groups.entry(pattern_category)
                .or_insert_with(Vec::new)
                .push((path.clone(), file_data.clone(), analysis.clone()));
        }

        Ok(groups)
    }

    /// Determine pattern category for a document
    fn determine_pattern_category(&self, analysis: &ContentAnalysis, file_data: &MarkdownFile) -> String {
        // Use AI analysis and content structure to determine category
        if let Some(ref category) = analysis.category {
            return category.clone();
        }

        // Fallback to tag-based categorization
        if let Some(ref tags) = file_data.frontmatter.tags {
            if let Some(primary_tag) = tags.first() {
                return primary_tag.clone();
            }
        }

        // Default category
        "general".to_string()
    }

    /// Create a template pattern from a group of similar documents
    fn create_template_pattern(&self, pattern_name: String, group: Vec<(String, MarkdownFile, ContentAnalysis)>) -> Result<TemplatePattern> {
        let mut common_tags = HashMap::new();
        let mut structure_elements = HashMap::new();
        let mut frontmatter_fields = HashMap::new();

        // Analyze common elements across the group
        for (path, file_data, analysis) in &group {
            // Count common tags
            if let Some(ref tags) = file_data.frontmatter.tags {
                for tag in tags {
                    *common_tags.entry(tag.clone()).or_insert(0) += 1;
                }
            }

            // Count structure elements from themes
            for theme in &analysis.themes {
                *structure_elements.entry(theme.clone()).or_insert(0) += 1;
            }

            // Count frontmatter fields
            if file_data.frontmatter.tags.is_some() {
                *frontmatter_fields.entry("tags".to_string()).or_insert(0) += 1;
            }
            if file_data.frontmatter.status.is_some() {
                *frontmatter_fields.entry("status".to_string()).or_insert(0) += 1;
            }
            if file_data.frontmatter.due_date.is_some() {
                *frontmatter_fields.entry("due_date".to_string()).or_insert(0) += 1;
            }
            if file_data.frontmatter.target_date.is_some() {
                *frontmatter_fields.entry("target_date".to_string()).or_insert(0) += 1;
            }
        }

        let group_size = group.len();
        let threshold = (group_size as f32 * 0.5) as usize; // 50% threshold

        // Filter common elements by threshold
        let filtered_tags: Vec<String> = common_tags.into_iter()
            .filter(|(_, count)| *count >= threshold)
            .map(|(tag, _)| tag)
            .collect();

        let filtered_structure: Vec<String> = structure_elements.into_iter()
            .filter(|(_, count)| *count >= threshold)
            .map(|(element, _)| element)
            .collect();

        let filtered_frontmatter: Vec<String> = frontmatter_fields.into_iter()
            .filter(|(_, count)| *count >= threshold)
            .map(|(field, _)| field)
            .collect();

        // Calculate confidence based on consistency
        let confidence = self.calculate_pattern_confidence(&group);

        // Get example paths (limit to 3)
        let examples: Vec<String> = group.iter()
            .take(3)
            .map(|(path, _, _)| path.clone())
            .collect();

        Ok(TemplatePattern {
            id: uuid::Uuid::new_v4().to_string(),
            name: pattern_name.clone(),
            description: format!("Pattern discovered from {} similar documents", group_size),
            structure_elements: filtered_structure,
            common_tags: filtered_tags,
            typical_frontmatter: filtered_frontmatter,
            confidence,
            match_count: group_size as u32,
            examples,
        })
    }

    /// Calculate confidence score for a pattern
    fn calculate_pattern_confidence(&self, group: &[(String, MarkdownFile, ContentAnalysis)]) -> f32 {
        if group.len() < 2 {
            return 0.5;
        }

        // Calculate confidence based on similarity of documents in the group
        let mut total_similarity = 0.0;
        let mut comparison_count = 0;

        for i in 0..group.len() {
            for j in (i + 1)..group.len() {
                let similarity = self.calculate_document_similarity(&group[i].2, &group[j].2);
                total_similarity += similarity;
                comparison_count += 1;
            }
        }

        if comparison_count > 0 {
            total_similarity / comparison_count as f32
        } else {
            0.5
        }
    }

    /// Calculate similarity between two content analyses
    fn calculate_document_similarity(&self, analysis1: &ContentAnalysis, analysis2: &ContentAnalysis) -> f32 {
        let mut similarity_score = 0.0;
        let mut total_factors = 0;

        // Compare themes
        if !analysis1.themes.is_empty() && !analysis2.themes.is_empty() {
            let theme_similarity = self.calculate_list_similarity(&analysis1.themes, &analysis2.themes);
            similarity_score += theme_similarity;
            total_factors += 1;
        }

        // Compare categories
        if analysis1.category.is_some() && analysis2.category.is_some() {
            let category_similarity = if analysis1.category == analysis2.category { 1.0 } else { 0.0 };
            similarity_score += category_similarity;
            total_factors += 1;
        }

        // Compare keywords
        if !analysis1.keywords.is_empty() && !analysis2.keywords.is_empty() {
            let keyword_similarity = self.calculate_list_similarity(&analysis1.keywords, &analysis2.keywords);
            similarity_score += keyword_similarity;
            total_factors += 1;
        }

        if total_factors > 0 {
            similarity_score / total_factors as f32
        } else {
            0.0
        }
    }

    /// Calculate similarity between two lists of strings
    fn calculate_list_similarity(&self, list1: &[String], list2: &[String]) -> f32 {
        let set1: std::collections::HashSet<_> = list1.iter().collect();
        let set2: std::collections::HashSet<_> = list2.iter().collect();
        
        let intersection = set1.intersection(&set2).count();
        let union = set1.union(&set2).count();
        
        if union > 0 {
            intersection as f32 / union as f32
        } else {
            0.0
        }
    }

    /// Generate a new template using AI based on a request
    pub async fn generate_template(&mut self, request: TemplateGenerationRequest) -> Result<TemplateGenerationResult> {
        let start_time = std::time::Instant::now();
        
        // Find relevant patterns for the request
        let relevant_patterns = self.find_relevant_patterns(&request);
        
        // Generate template using AI
        let template = self.generate_template_with_ai(&request, &relevant_patterns).await?;
        
        // Create generation metadata
        let processing_time = start_time.elapsed().as_millis() as u64;
        let metadata = TemplateGenerationMetadata {
            generated_at: Utc::now(),
            model_used: "AI Assistant".to_string(),
            processing_time_ms: processing_time,
            source_patterns: relevant_patterns.iter().map(|p| p.name.clone()).collect(),
            examples_analyzed: relevant_patterns.iter().map(|p| p.match_count).sum(),
        };

        // Generate suggestions for improvement
        let suggestions = self.generate_template_suggestions(&template, &request);

        // Calculate confidence score
        let confidence = self.calculate_template_confidence(&template, &relevant_patterns);

        // Store the template in the database
        self.add_template_to_database(template.clone())?;

        Ok(TemplateGenerationResult {
            template,
            metadata,
            suggestions,
            confidence,
        })
    }

    /// Find relevant patterns for a template generation request
    fn find_relevant_patterns(&self, request: &TemplateGenerationRequest) -> Vec<TemplatePattern> {
        self.template_database.patterns.iter()
            .filter(|pattern| {
                // Match by category
                if pattern.name.to_lowercase().contains(&request.template_type.to_lowercase()) {
                    return true;
                }
                
                // Match by tags
                if let Some(ref topic) = request.topic {
                    if pattern.common_tags.iter().any(|tag| tag.to_lowercase().contains(&topic.to_lowercase())) {
                        return true;
                    }
                }
                
                // Match by preferred tags
                if pattern.common_tags.iter().any(|tag| request.preferences.preferred_tags.contains(tag)) {
                    return true;
                }
                
                false
            })
            .cloned()
            .collect()
    }

    /// Generate template using AI
    async fn generate_template_with_ai(&self, request: &TemplateGenerationRequest, patterns: &[TemplatePattern]) -> Result<NoteTemplate> {
        let llm_client = self.llm_client.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No LLM client configured for template generation"))?;

        // Create generation prompt
        let prompt = self.create_template_generation_prompt(request, patterns);
        
        let messages = vec![
            Message {
                id: uuid::Uuid::new_v4().to_string(),
                role: MessageRole::System,
                content: "You are an expert note template generator. Create structured, useful templates for note-taking in Obsidian. Return your response as a JSON object with the requested template structure.".to_string(),
                timestamp: Utc::now(),
                function_call: None,
            },
            Message {
                id: uuid::Uuid::new_v4().to_string(),
                role: MessageRole::User,
                content: prompt,
                timestamp: Utc::now(),
                function_call: None,
            }
        ];

        let response = llm_client.send_message(messages).await
            .context("Failed to generate template with AI")?;

        // Parse the template from the response
        self.parse_template_from_response(&response.content, request)
    }

    /// Create a prompt for template generation
    fn create_template_generation_prompt(&self, request: &TemplateGenerationRequest, patterns: &[TemplatePattern]) -> String {
        let mut prompt = format!(
            "Generate a {} template for note-taking. The template should be {} complexity.\n\n",
            request.template_type, request.preferences.complexity
        );

        if let Some(ref topic) = request.topic {
            prompt.push_str(&format!("Topic/Domain: {}\n", topic));
        }

        if let Some(ref context) = request.context {
            prompt.push_str(&format!("Context: {}\n", context));
        }

        if !request.required_fields.is_empty() {
            prompt.push_str(&format!("Required fields: {}\n", request.required_fields.join(", ")));
        }

        if !patterns.is_empty() {
            prompt.push_str("\nBased on these discovered patterns:\n");
            for pattern in patterns {
                prompt.push_str(&format!("- {}: {} (used in {} documents)\n", 
                    pattern.name, pattern.description, pattern.match_count));
            }
        }

        prompt.push_str(&format!(r#"
Return a JSON object with this structure:
{{
  "id": "unique-template-id",
  "name": "Template Name",
  "description": "Template description",
  "category": "{}",
  "components": [
    {{"type": "Text", "content": "Static text"}},
    {{"type": "Placeholder", "name": "field_name", "hint": "Description", "required": true}},
    {{"type": "AiSuggestion", "prompt": "Generate suggestion for...", "fallback": "Default text"}}
  ],
  "frontmatter_fields": [
    {{
      "name": "field_name",
      "field_type": "string",
      "default_value": "default",
      "required": true,
      "description": "Field description"
    }}
  ],
  "tags": ["template", "{}"]
}}
"#, request.template_type, request.template_type));

        prompt
    }

    /// Parse template from AI response
    fn parse_template_from_response(&self, response: &str, request: &TemplateGenerationRequest) -> Result<NoteTemplate> {
        // Extract JSON from the response
        let json_start = response.find('{').unwrap_or(0);
        let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
        let json_str = &response[json_start..json_end];
        
        let parsed: serde_json::Value = serde_json::from_str(json_str)
            .context("Failed to parse template JSON")?;

        // Extract template fields
        let id = parsed.get("id")
            .and_then(|v| v.as_str())
            .unwrap_or(&uuid::Uuid::new_v4().to_string())
            .to_string();

        let name = parsed.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Generated Template")
            .to_string();

        let description = parsed.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("AI-generated template")
            .to_string();

        let category = parsed.get("category")
            .and_then(|v| v.as_str())
            .unwrap_or(&request.template_type)
            .to_string();

        // Parse components
        let components = self.parse_template_components(&parsed)?;

        // Parse frontmatter fields
        let frontmatter_fields = self.parse_frontmatter_fields(&parsed)?;

        // Parse tags
        let tags = parsed.get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect())
            .unwrap_or_else(|| vec![category.clone()]);

        Ok(NoteTemplate {
            id,
            name,
            description,
            category,
            components,
            frontmatter_fields,
            tags,
            usage_stats: TemplateUsageStats::default(),
            created_at: Utc::now(),
            modified_at: Utc::now(),
        })
    }

    /// Parse template components from JSON
    fn parse_template_components(&self, json: &serde_json::Value) -> Result<Vec<TemplateComponent>> {
        let empty_vec = vec![];
        let components_array = json.get("components")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);

        let mut components = Vec::new();
        for component_value in components_array {
            if let Some(component_type) = component_value.get("type").and_then(|v| v.as_str()) {
                match component_type {
                    "Text" => {
                        if let Some(content) = component_value.get("content").and_then(|v| v.as_str()) {
                            components.push(TemplateComponent::Text(content.to_string()));
                        }
                    }
                    "Placeholder" => {
                        let name = component_value.get("name").and_then(|v| v.as_str()).unwrap_or("field").to_string();
                        let hint = component_value.get("hint").and_then(|v| v.as_str()).unwrap_or("Enter value").to_string();
                        let required = component_value.get("required").and_then(|v| v.as_bool()).unwrap_or(false);
                        components.push(TemplateComponent::Placeholder { name, hint, required });
                    }
                    "AiSuggestion" => {
                        let prompt = component_value.get("prompt").and_then(|v| v.as_str()).unwrap_or("Generate content").to_string();
                        let fallback = component_value.get("fallback").and_then(|v| v.as_str()).unwrap_or("Content here").to_string();
                        components.push(TemplateComponent::AiSuggestion { prompt, fallback });
                    }
                    "Tag" => {
                        if let Some(tag) = component_value.get("tag").and_then(|v| v.as_str()) {
                            components.push(TemplateComponent::Tag(tag.to_string()));
                        }
                    }
                    _ => {
                        // Unknown component type, skip
                    }
                }
            }
        }

        Ok(components)
    }

    /// Parse frontmatter fields from JSON
    fn parse_frontmatter_fields(&self, json: &serde_json::Value) -> Result<Vec<FrontmatterField>> {
        let empty_vec = vec![];
        let fields_array = json.get("frontmatter_fields")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);

        let mut fields = Vec::new();
        for field_value in fields_array {
            let name = field_value.get("name").and_then(|v| v.as_str()).unwrap_or("field").to_string();
            let field_type = field_value.get("field_type").and_then(|v| v.as_str()).unwrap_or("string").to_string();
            let default_value = field_value.get("default_value").and_then(|v| v.as_str()).map(|s| s.to_string());
            let required = field_value.get("required").and_then(|v| v.as_bool()).unwrap_or(false);
            let description = field_value.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());

            fields.push(FrontmatterField {
                name,
                field_type,
                default_value,
                required,
                description,
            });
        }

        Ok(fields)
    }

    /// Generate suggestions for template improvement
    fn generate_template_suggestions(&self, template: &NoteTemplate, request: &TemplateGenerationRequest) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Check for missing common elements
        if template.frontmatter_fields.is_empty() {
            suggestions.push("Consider adding frontmatter fields like 'created', 'tags', or 'status'".to_string());
        }

        // Check for complexity appropriateness
        if request.preferences.complexity == "simple" && template.components.len() > 5 {
            suggestions.push("Template might be too complex for 'simple' preference".to_string());
        }

        // Check for AI suggestions
        if request.preferences.include_ai_suggestions && !template.components.iter().any(|c| matches!(c, TemplateComponent::AiSuggestion { .. })) {
            suggestions.push("Consider adding AI suggestions for dynamic content".to_string());
        }

        suggestions
    }

    /// Calculate confidence score for a generated template
    fn calculate_template_confidence(&self, template: &NoteTemplate, patterns: &[TemplatePattern]) -> f32 {
        let mut confidence = 0.5; // Base confidence

        // Boost confidence if based on patterns
        if !patterns.is_empty() {
            let pattern_confidence: f32 = patterns.iter().map(|p| p.confidence).sum::<f32>() / patterns.len() as f32;
            confidence += pattern_confidence * 0.3;
        }

        // Boost confidence if template has good structure
        if !template.components.is_empty() {
            confidence += 0.1;
        }

        if !template.frontmatter_fields.is_empty() {
            confidence += 0.1;
        }

        confidence.min(1.0)
    }

    /// Add a template to the database
    fn add_template_to_database(&mut self, template: NoteTemplate) -> Result<()> {
        let template_index = self.template_database.templates.len();
        
        // Update category index
        self.template_database.category_index
            .entry(template.category.clone())
            .or_insert_with(Vec::new)
            .push(template_index);

        // Update tag index
        for tag in &template.tags {
            self.template_database.tag_index
                .entry(tag.clone())
                .or_insert_with(Vec::new)
                .push(template_index);
        }

        // Add template
        self.template_database.templates.push(template);
        self.template_database.last_updated = Utc::now();

        // Save to disk
        self.save_template_database()?;

        Ok(())
    }

    /// Get templates by category
    pub fn get_templates_by_category(&self, category: &str) -> Vec<&NoteTemplate> {
        self.template_database.category_index.get(category)
            .map(|indices| indices.iter()
                .filter_map(|&i| self.template_database.templates.get(i))
                .collect())
            .unwrap_or_default()
    }

    /// Get templates by tag
    pub fn get_templates_by_tag(&self, tag: &str) -> Vec<&NoteTemplate> {
        self.template_database.tag_index.get(tag)
            .map(|indices| indices.iter()
                .filter_map(|&i| self.template_database.templates.get(i))
                .collect())
            .unwrap_or_default()
    }

    /// Get all templates
    pub fn get_all_templates(&self) -> Vec<&NoteTemplate> {
        self.template_database.templates.iter().collect()
    }

    /// Get template by ID
    pub fn get_template_by_id(&self, id: &str) -> Option<&NoteTemplate> {
        self.template_database.templates.iter().find(|t| t.id == id)
    }

    /// Update template usage statistics
    pub fn update_template_usage(&mut self, template_id: &str, satisfaction_rating: Option<f32>) -> Result<()> {
        if let Some(template) = self.template_database.templates.iter_mut().find(|t| t.id == template_id) {
            template.usage_stats.usage_count += 1;
            template.usage_stats.last_used = Some(Utc::now());
            if let Some(rating) = satisfaction_rating {
                template.usage_stats.satisfaction_rating = Some(rating);
            }
            template.modified_at = Utc::now();
            
            self.template_database.last_updated = Utc::now();
            self.save_template_database()?;
        }
        Ok(())
    }

    /// Render a template with provided values
    pub fn render_template(&self, template: &NoteTemplate, values: &HashMap<String, String>) -> Result<String> {
        let mut rendered_content = String::new();
        
        // Add frontmatter
        rendered_content.push_str("---\n");
        for field in &template.frontmatter_fields {
            let empty_string = String::new();
            let value = values.get(&field.name)
                .or(field.default_value.as_ref())
                .unwrap_or(&empty_string);
            rendered_content.push_str(&format!("{}: {}\n", field.name, value));
        }
        rendered_content.push_str("---\n\n");
        
        // Render components
        for component in &template.components {
            match component {
                TemplateComponent::Text(text) => {
                    rendered_content.push_str(text);
                    rendered_content.push('\n');
                }
                TemplateComponent::Placeholder { name, hint, required: _ } => {
                    let placeholder_text = format!("{{{{ {} - {} }}}}", name, hint);
                    let value = values.get(name).unwrap_or(&placeholder_text);
                    rendered_content.push_str(value);
                    rendered_content.push('\n');
                }
                TemplateComponent::AiSuggestion { prompt: _, fallback } => {
                    // For now, use fallback. In a real implementation, this would trigger AI generation
                    rendered_content.push_str(fallback);
                    rendered_content.push('\n');
                }
                TemplateComponent::Tag(tag) => {
                    rendered_content.push_str(&format!("#{}", tag));
                    rendered_content.push('\n');
                }
                TemplateComponent::Link { target, display_text } => {
                    let link_text = display_text.as_ref().unwrap_or(target);
                    rendered_content.push_str(&format!("[[{}|{}]]", target, link_text));
                    rendered_content.push('\n');
                }
                TemplateComponent::Conditional { condition: _, content } => {
                    // For now, always render conditional content
                    // In a real implementation, this would evaluate the condition
                    for sub_component in content {
                        // Recursively render sub-components (simplified)
                        match sub_component {
                            TemplateComponent::Text(text) => rendered_content.push_str(text),
                            _ => {} // Simplified for now
                        }
                    }
                    rendered_content.push('\n');
                }
                TemplateComponent::Repeating { item_name: _, content } => {
                    // For now, render once
                    // In a real implementation, this would repeat based on provided data
                    for sub_component in content {
                        match sub_component {
                            TemplateComponent::Text(text) => rendered_content.push_str(text),
                            _ => {} // Simplified for now
                        }
                    }
                    rendered_content.push('\n');
                }
            }
        }

        Ok(rendered_content)
    }

    /// Get template database statistics
    pub fn get_template_database_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();
        stats.insert("total_templates".to_string(), serde_json::Value::Number(self.template_database.templates.len().into()));
        stats.insert("total_patterns".to_string(), serde_json::Value::Number(self.template_database.patterns.len().into()));
        stats.insert("categories".to_string(), serde_json::Value::Number(self.template_database.category_index.len().into()));
        stats.insert("last_updated".to_string(), serde_json::Value::String(self.template_database.last_updated.to_rfc3339()));
        stats.insert("version".to_string(), serde_json::Value::String(self.template_database.version.clone()));
        stats
    }

    /// Set organization configuration
    pub fn set_organization_config(&mut self, config: OrganizationConfig) {
        self.organization_config = config;
    }

    /// Generate organization recommendations for a note
    pub async fn generate_organization_recommendations(&mut self, vault_path: &str) -> Result<OrganizationRecommendations> {
        // Get content analysis for the note
        let file_data = self.get_markdown_file_data(vault_path).await?;
        let analysis = self.analyze_content(&file_data.content).await?;
        
        // Generate tag suggestions
        let suggested_tags = self.generate_tag_suggestions(&analysis, &file_data).await?;
        
        // Generate folder suggestions
        let folder_suggestions = self.generate_folder_suggestions(&analysis, vault_path).await?;
        
        // Generate link suggestions
        let link_suggestions = self.generate_link_suggestions(&analysis, vault_path).await?;
        
        // Calculate overall confidence
        let overall_confidence = self.calculate_overall_confidence(&suggested_tags, &folder_suggestions, &link_suggestions);
        
        Ok(OrganizationRecommendations {
            suggested_tags,
            folder_suggestions,
            link_suggestions,
            overall_confidence,
            generated_at: Utc::now(),
        })
    }

    /// Generate tag suggestions from content analysis
    pub async fn generate_tag_suggestions(&self, analysis: &ContentAnalysis, file_data: &MarkdownFile) -> Result<Vec<TagSuggestion>> {
        let mut suggestions = Vec::new();
        
        // Get existing tags to avoid duplicates
        let existing_tags: std::collections::HashSet<String> = file_data.frontmatter.tags
            .as_ref()
            .unwrap_or(&Vec::new())
            .iter()
            .cloned()
            .collect();
        
        // Suggestions from themes
        for theme in &analysis.themes {
            if !existing_tags.contains(theme) {
                let tag = self.normalize_tag(theme);
                suggestions.push(TagSuggestion {
                    tag: tag.clone(),
                    confidence: 0.8,
                    reason: format!("Found as major theme in content"),
                    source: TagSource::Theme,
                });
            }
        }
        
        // Suggestions from keywords
        for keyword in &analysis.keywords {
            if !existing_tags.contains(keyword) {
                let tag = self.normalize_tag(keyword);
                suggestions.push(TagSuggestion {
                    tag: tag.clone(),
                    confidence: 0.7,
                    reason: format!("Found as important keyword"),
                    source: TagSource::Keyword,
                });
            }
        }
        
        // Suggestions from category
        if let Some(ref category) = analysis.category {
            if !existing_tags.contains(category) {
                let tag = self.normalize_tag(category);
                suggestions.push(TagSuggestion {
                    tag: tag.clone(),
                    confidence: 0.9,
                    reason: format!("Content classified as this category"),
                    source: TagSource::Category,
                });
            }
        }
        
        // Suggestions from entities
        for entity in &analysis.entities {
            if entity.confidence >= 0.8 {
                let tag = self.normalize_tag(&entity.text);
                if !existing_tags.contains(&tag) {
                    suggestions.push(TagSuggestion {
                        tag: tag.clone(),
                        confidence: entity.confidence * 0.9, // Slightly reduce confidence
                        reason: format!("Important {} entity found", entity.entity_type),
                        source: TagSource::Entity,
                    });
                }
            }
        }
        
        // Apply custom tag rules
        for rule in &self.organization_config.custom_tag_rules {
            if file_data.content.contains(&rule.pattern) {
                let tag = self.normalize_tag(&rule.tag);
                if !existing_tags.contains(&tag) {
                    suggestions.push(TagSuggestion {
                        tag: tag.clone(),
                        confidence: rule.confidence,
                        reason: format!("Matches custom rule: {}", rule.name),
                        source: TagSource::Rule,
                    });
                }
            }
        }
        
        // Sort by confidence and limit results
        suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        suggestions.truncate(self.organization_config.max_tag_suggestions);
        
        Ok(suggestions)
    }

    /// Generate folder organization suggestions
    pub async fn generate_folder_suggestions(&self, analysis: &ContentAnalysis, vault_path: &str) -> Result<Vec<FolderSuggestion>> {
        let mut suggestions = Vec::new();
        
        // Get current folder
        let current_folder = std::path::Path::new(vault_path)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string();
        
        // Suggest based on category
        if let Some(ref category) = analysis.category {
            let folder_path = format!("{}", category.to_lowercase().replace(' ', "_"));
            if folder_path != current_folder {
                suggestions.push(FolderSuggestion {
                    folder_path: folder_path.clone(),
                    confidence: 0.8,
                    reason: format!("Content is classified as {}", category),
                    category: category.clone(),
                });
            }
        }
        
        // Suggest based on primary theme
        if let Some(primary_theme) = analysis.themes.first() {
            let folder_path = format!("{}", primary_theme.to_lowercase().replace(' ', "_"));
            if folder_path != current_folder {
                suggestions.push(FolderSuggestion {
                    folder_path: folder_path.clone(),
                    confidence: 0.7,
                    reason: format!("Primary theme is {}", primary_theme),
                    category: primary_theme.clone(),
                });
            }
        }
        
        // Suggest based on complexity
        if let Some(complexity) = analysis.complexity_score {
            let folder_suggestion = if complexity >= 8.0 {
                "advanced"
            } else if complexity >= 5.0 {
                "intermediate"
            } else {
                "basic"
            };
            
            let folder_path = format!("{}", folder_suggestion);
            if folder_path != current_folder {
                suggestions.push(FolderSuggestion {
                    folder_path: folder_path.clone(),
                    confidence: 0.6,
                    reason: format!("Content complexity level: {}", complexity),
                    category: "complexity".to_string(),
                });
            }
        }
        
        // Remove duplicates and sort by confidence
        suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        suggestions.dedup_by(|a, b| a.folder_path == b.folder_path);
        suggestions.truncate(self.organization_config.max_folder_suggestions);
        
        Ok(suggestions)
    }

    /// Generate auto-linking suggestions
    pub async fn generate_link_suggestions(&self, analysis: &ContentAnalysis, vault_path: &str) -> Result<Vec<LinkSuggestion>> {
        let mut suggestions = Vec::new();
        
        // Find related documents using semantic search
        for theme in &analysis.themes {
            if let Ok(search_results) = self.semantic_search_immutable(theme).await {
                for result in search_results.into_iter().take(2) {
                    // Don't suggest linking to the same document
                    if result.path != vault_path {
                        suggestions.push(LinkSuggestion {
                            target_path: result.path.clone(),
                            link_text: result.metadata.title.clone(),
                            confidence: result.similarity * 0.8,
                            context: format!("Related content about {}", theme),
                            reason: format!("Semantically similar ({}% match)", (result.similarity * 100.0) as u32),
                        });
                    }
                }
            }
        }
        
        // Find links based on entities
        for entity in &analysis.entities {
            if entity.confidence >= 0.8 {
                // This is a simplified version - in practice, you'd search for documents
                // that mention this entity
                let entity_search = format!("entity:{}", entity.text);
                if let Ok(search_results) = self.semantic_search_immutable(&entity_search).await {
                    for result in search_results.into_iter().take(1) {
                        if result.path != vault_path && result.similarity > 0.7 {
                            suggestions.push(LinkSuggestion {
                                target_path: result.path.clone(),
                                link_text: entity.text.clone(),
                                confidence: result.similarity * entity.confidence,
                                context: format!("Mentions entity: {}", entity.text),
                                reason: format!("Both documents reference {}", entity.text),
                            });
                        }
                    }
                }
            }
        }
        
        // Sort by confidence and limit results
        suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        suggestions.truncate(self.organization_config.max_link_suggestions);
        
        Ok(suggestions)
    }

    /// Apply organization recommendations to a note
    pub async fn apply_organization_recommendations(&mut self, vault_path: &str, recommendations: &OrganizationRecommendations) -> Result<()> {
        let mut file_data = self.get_markdown_file_data(vault_path).await?;
        let mut updated = false;
        
        // Apply high-confidence tags if auto-apply is enabled
        if self.organization_config.auto_apply_tags {
            let mut existing_tags = file_data.frontmatter.tags.clone().unwrap_or_default();
            
            for tag_suggestion in &recommendations.suggested_tags {
                if tag_suggestion.confidence >= self.organization_config.auto_tag_confidence_threshold {
                    if !existing_tags.contains(&tag_suggestion.tag) {
                        existing_tags.push(tag_suggestion.tag.clone());
                        updated = true;
                    }
                }
            }
            
            if updated {
                file_data.frontmatter.tags = Some(existing_tags);
            }
        }
        
        // Save the updated file if changes were made
        if updated {
            self.save_markdown_file_data(vault_path, &file_data, true).await?;
        }
        
        Ok(())
    }

    /// Batch process multiple notes for organization
    pub async fn batch_organize_notes(&mut self, vault_paths: Vec<&str>) -> Result<Vec<(String, Result<OrganizationRecommendations>)>> {
        let mut results = Vec::new();
        
        for path in vault_paths {
            let result = self.generate_organization_recommendations(path).await;
            results.push((path.to_string(), result));
        }
        
        Ok(results)
    }

    /// Normalize tag to follow consistent naming conventions
    fn normalize_tag(&self, tag: &str) -> String {
        tag.to_lowercase()
            .replace(' ', "_")
            .replace('-', "_")
            .replace('/', "_")
            .replace('.', "")
            .replace('!', "")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect()
    }

    /// Calculate overall confidence for organization recommendations
    fn calculate_overall_confidence(&self, tags: &[TagSuggestion], folders: &[FolderSuggestion], links: &[LinkSuggestion]) -> f32 {
        let mut total_confidence = 0.0;
        let mut count = 0;
        
        // Include tag confidences
        for tag in tags {
            total_confidence += tag.confidence;
            count += 1;
        }
        
        // Include folder confidences
        for folder in folders {
            total_confidence += folder.confidence;
            count += 1;
        }
        
        // Include link confidences
        for link in links {
            total_confidence += link.confidence;
            count += 1;
        }
        
        if count > 0 {
            total_confidence / count as f32
        } else {
            0.0
        }
    }

    /// Semantic search without mutable self (helper method)
    async fn semantic_search_immutable(&self, query: &str) -> Result<Vec<SemanticSearchResult>> {
        // This is a simplified version that works with immutable self
        // In practice, you might need to restructure the semantic search to work without mutable state
        let query_embedding = if let Some(ref llm_client) = self.llm_client {
            // Generate embedding for query
            let messages = vec![
                crate::ai_conversation::Message {
                    id: uuid::Uuid::new_v4().to_string(),
                    role: crate::ai_conversation::MessageRole::System,
                    content: "Generate a 768-dimensional vector embedding for the given text. Return only a JSON array of 768 floating point numbers.".to_string(),
                    timestamp: Utc::now(),
                    function_call: None,
                },
                crate::ai_conversation::Message {
                    id: uuid::Uuid::new_v4().to_string(),
                    role: crate::ai_conversation::MessageRole::User,
                    content: format!("Generate embedding for: {}", query),
                    timestamp: Utc::now(),
                    function_call: None,
                }
            ];

            match llm_client.send_message(messages).await {
                Ok(response) => {
                    let embedding_json = response.content.trim();
                    serde_json::from_str::<Vec<f32>>(embedding_json).unwrap_or_default()
                }
                Err(_) => return Ok(Vec::new()),
            }
        } else {
            return Ok(Vec::new());
        };

        // Calculate similarities with all documents
        let mut similarities: Vec<(usize, f32)> = self.vector_database.embeddings
            .iter()
            .enumerate()
            .map(|(i, doc_embedding)| {
                let similarity = self.cosine_similarity(&query_embedding, &doc_embedding.embedding);
                (i, similarity)
            })
            .collect();

        // Sort by similarity (descending)
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Filter by minimum similarity and take top results
        let results: Vec<SemanticSearchResult> = similarities
            .into_iter()
            .filter(|(_, sim)| *sim >= self.search_config.min_similarity)
            .take(self.search_config.max_results)
            .map(|(index, similarity)| {
                let doc_embedding = &self.vector_database.embeddings[index];
                
                SemanticSearchResult {
                    path: doc_embedding.path.clone(),
                    similarity,
                    metadata: doc_embedding.metadata.clone(),
                    snippet: doc_embedding.metadata.excerpt.clone(),
                    highlights: vec![],
                }
            })
            .collect();

        Ok(results)
    }

    /// Set content suggestion configuration
    pub fn set_content_suggestion_config(&mut self, config: ContentSuggestionConfig) {
        self.content_suggestion_config = config;
    }

    /// Generate content suggestions for real-time writing assistance
    pub async fn generate_content_suggestions(&mut self, request: ContentSuggestionRequest) -> Result<Vec<ContentSuggestion>> {
        // Check cache first
        let content_hash = self.generate_cache_key(&request.content);
        if let Some(cached) = self.get_cached_suggestions(&content_hash) {
            self.suggestion_cache.hit_count += 1;
            return Ok(cached);
        }

        self.suggestion_cache.miss_count += 1;

        // Extract context around cursor
        let context = self.extract_context(&request.content, &request.cursor_position, request.context_window);
        
        // Generate different types of suggestions
        let mut suggestions = Vec::new();
        
        for suggestion_type in &request.suggestion_types {
            match suggestion_type {
                SuggestionType::ContentContinuation => {
                    if let Ok(mut continuation_suggestions) = self.generate_content_continuation(&context, &request).await {
                        suggestions.append(&mut continuation_suggestions);
                    }
                }
                SuggestionType::RelatedContent => {
                    if let Ok(mut related_suggestions) = self.generate_related_content_suggestions(&context, &request).await {
                        suggestions.append(&mut related_suggestions);
                    }
                }
                SuggestionType::LinkSuggestion => {
                    if let Ok(mut link_suggestions) = self.generate_link_suggestions_from_context(&context, &request).await {
                        suggestions.append(&mut link_suggestions);
                    }
                }
                SuggestionType::TextCompletion => {
                    if let Ok(mut completion_suggestions) = self.generate_text_completion(&context, &request).await {
                        suggestions.append(&mut completion_suggestions);
                    }
                }
                SuggestionType::HeadingSuggestion => {
                    if let Ok(mut heading_suggestions) = self.generate_heading_suggestions(&context, &request).await {
                        suggestions.append(&mut heading_suggestions);
                    }
                }
                SuggestionType::BulletPointSuggestion => {
                    if let Ok(mut bullet_suggestions) = self.generate_bullet_point_suggestions(&context, &request).await {
                        suggestions.append(&mut bullet_suggestions);
                    }
                }
                SuggestionType::CodeBlockSuggestion => {
                    if let Ok(mut code_suggestions) = self.generate_code_block_suggestions(&context, &request).await {
                        suggestions.append(&mut code_suggestions);
                    }
                }
            }
        }

        // Filter by confidence and limit results
        suggestions.retain(|s| s.confidence >= self.content_suggestion_config.min_confidence);
        suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        suggestions.truncate(request.max_suggestions);

        // Cache the results
        self.cache_suggestions(&content_hash, &suggestions);

        Ok(suggestions)
    }

    /// Generate content continuation suggestions
    async fn generate_content_continuation(&self, context: &str, request: &ContentSuggestionRequest) -> Result<Vec<ContentSuggestion>> {
        let llm_client = self.llm_client.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No LLM client configured for content suggestions"))?;

        let prompt = format!(
            "Continue writing this content in a natural way. Context: {}\n\nProvide 2-3 different continuation options, each on a separate line.",
            context
        );

        let messages = vec![
            crate::ai_conversation::Message {
                id: uuid::Uuid::new_v4().to_string(),
                role: crate::ai_conversation::MessageRole::System,
                content: "You are a writing assistant. Provide natural, helpful continuations for the given text context.".to_string(),
                timestamp: Utc::now(),
                function_call: None,
            },
            crate::ai_conversation::Message {
                id: uuid::Uuid::new_v4().to_string(),
                role: crate::ai_conversation::MessageRole::User,
                content: prompt,
                timestamp: Utc::now(),
                function_call: None,
            }
        ];

        let response = llm_client.send_message(messages).await?;
        let suggestions = self.parse_continuation_suggestions(&response.content, context, request);

        Ok(suggestions)
    }

    /// Generate related content suggestions
    async fn generate_related_content_suggestions(&self, context: &str, request: &ContentSuggestionRequest) -> Result<Vec<ContentSuggestion>> {
        // Use semantic search to find related content
        let search_results = self.semantic_search_immutable(context).await?;
        
        let mut suggestions = Vec::new();
        
        for result in search_results.into_iter().take(3) {
            // Extract relevant snippet from the related document
            let snippet = result.snippet.chars().take(100).collect::<String>();
            
            suggestions.push(ContentSuggestion {
                text: format!("Related: {}", snippet),
                suggestion_type: SuggestionType::RelatedContent,
                confidence: result.similarity * 0.8,
                context: context.to_string(),
                reason: format!("Found related content in {}", result.path),
                source_documents: vec![result.path],
                position: Some(request.cursor_position.clone()),
            });
        }

        Ok(suggestions)
    }

    /// Generate link suggestions from context
    async fn generate_link_suggestions_from_context(&self, context: &str, request: &ContentSuggestionRequest) -> Result<Vec<ContentSuggestion>> {
        // Find potential link targets based on context
        let search_results = self.semantic_search_immutable(context).await?;
        
        let mut suggestions = Vec::new();
        
        for result in search_results.into_iter().take(3) {
            if result.similarity > 0.7 {
                let link_text = format!("[[{}]]", result.metadata.title);
                
                suggestions.push(ContentSuggestion {
                    text: link_text,
                    suggestion_type: SuggestionType::LinkSuggestion,
                    confidence: result.similarity * 0.9,
                    context: context.to_string(),
                    reason: format!("Relevant link to {}", result.metadata.title),
                    source_documents: vec![result.path],
                    position: Some(request.cursor_position.clone()),
                });
            }
        }

        Ok(suggestions)
    }

    /// Generate text completion suggestions
    async fn generate_text_completion(&self, context: &str, _request: &ContentSuggestionRequest) -> Result<Vec<ContentSuggestion>> {
        let llm_client = self.llm_client.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No LLM client configured for text completion"))?;

        // Check if we're in the middle of a sentence
        if context.trim().ends_with('.') || context.trim().ends_with('!') || context.trim().ends_with('?') {
            return Ok(Vec::new()); // Don't suggest completion for complete sentences
        }

        let prompt = format!(
            "Complete this text naturally: {}\n\nProvide only the completion text, not the full sentence.",
            context
        );

        let messages = vec![
            crate::ai_conversation::Message {
                id: uuid::Uuid::new_v4().to_string(),
                role: crate::ai_conversation::MessageRole::System,
                content: "You are a text completion assistant. Complete the given text naturally and concisely.".to_string(),
                timestamp: Utc::now(),
                function_call: None,
            },
            crate::ai_conversation::Message {
                id: uuid::Uuid::new_v4().to_string(),
                role: crate::ai_conversation::MessageRole::User,
                content: prompt,
                timestamp: Utc::now(),
                function_call: None,
            }
        ];

        let response = llm_client.send_message(messages).await?;
        let completion = response.content.trim();

        if !completion.is_empty() {
            Ok(vec![ContentSuggestion {
                text: completion.to_string(),
                suggestion_type: SuggestionType::TextCompletion,
                confidence: 0.7,
                context: context.to_string(),
                reason: "AI-generated text completion".to_string(),
                source_documents: vec![],
                position: None,
            }])
        } else {
            Ok(vec![])
        }
    }

    /// Generate heading suggestions
    async fn generate_heading_suggestions(&self, context: &str, _request: &ContentSuggestionRequest) -> Result<Vec<ContentSuggestion>> {
        // Simple heuristic: suggest headings if we're at the beginning of a line
        if context.trim().is_empty() || context.ends_with('\n') {
            let llm_client = self.llm_client.as_ref()
                .ok_or_else(|| anyhow::anyhow!("No LLM client configured for heading suggestions"))?;

            let prompt = format!(
                "Based on this content context, suggest 2-3 appropriate headings for the next section: {}\n\nProvide only the heading text, one per line.",
                context
            );

            let messages = vec![
                crate::ai_conversation::Message {
                    id: uuid::Uuid::new_v4().to_string(),
                    role: crate::ai_conversation::MessageRole::System,
                    content: "You are a document structure assistant. Suggest appropriate headings based on context.".to_string(),
                    timestamp: Utc::now(),
                    function_call: None,
                },
                crate::ai_conversation::Message {
                    id: uuid::Uuid::new_v4().to_string(),
                    role: crate::ai_conversation::MessageRole::User,
                    content: prompt,
                    timestamp: Utc::now(),
                    function_call: None,
                }
            ];

            let response = llm_client.send_message(messages).await?;
            let headings = response.content.lines()
                .filter(|line| !line.trim().is_empty())
                .take(3)
                .enumerate()
                .map(|(i, heading)| {
                    ContentSuggestion {
                        text: format!("## {}", heading.trim()),
                        suggestion_type: SuggestionType::HeadingSuggestion,
                        confidence: 0.8 - (i as f32 * 0.1),
                        context: context.to_string(),
                        reason: "AI-suggested heading".to_string(),
                        source_documents: vec![],
                        position: None,
                    }
                })
                .collect();

            Ok(headings)
        } else {
            Ok(vec![])
        }
    }

    /// Generate bullet point suggestions
    async fn generate_bullet_point_suggestions(&self, context: &str, _request: &ContentSuggestionRequest) -> Result<Vec<ContentSuggestion>> {
        // Check if we're in a list context
        if context.contains("- ") || context.contains("* ") || context.contains("1. ") {
            Ok(vec![ContentSuggestion {
                text: "- ".to_string(),
                suggestion_type: SuggestionType::BulletPointSuggestion,
                confidence: 0.9,
                context: context.to_string(),
                reason: "Continue bullet point list".to_string(),
                source_documents: vec![],
                position: None,
            }])
        } else {
            Ok(vec![])
        }
    }

    /// Generate code block suggestions
    async fn generate_code_block_suggestions(&self, context: &str, _request: &ContentSuggestionRequest) -> Result<Vec<ContentSuggestion>> {
        // Check if we're likely to need a code block
        if context.to_lowercase().contains("code") || context.to_lowercase().contains("example") || context.contains("`") {
            Ok(vec![ContentSuggestion {
                text: "```\n\n```".to_string(),
                suggestion_type: SuggestionType::CodeBlockSuggestion,
                confidence: 0.6,
                context: context.to_string(),
                reason: "Code block detected in context".to_string(),
                source_documents: vec![],
                position: None,
            }])
        } else {
            Ok(vec![])
        }
    }

    /// Auto-insert links in content
    pub async fn auto_insert_links(&self, content: &str) -> Result<AutoLinkResult> {
        let mut result = AutoLinkResult {
            original_text: content.to_string(),
            linked_text: content.to_string(),
            inserted_links: Vec::new(),
            links_added: 0,
        };

        // Find potential link targets
        let words: Vec<&str> = content.split_whitespace().collect();
        let mut current_position = 0;
        
        for word in words {
            // Search for documents that might match this word
            if word.len() > 3 { // Only consider words longer than 3 characters
                if let Ok(search_results) = self.semantic_search_immutable(word).await {
                    for search_result in search_results {
                        if search_result.similarity >= self.content_suggestion_config.auto_link_confidence_threshold {
                            let link_text = format!("[[{}]]", search_result.metadata.title);
                            
                            // Replace the word with the link
                            result.linked_text = result.linked_text.replace(word, &link_text);
                            
                            result.inserted_links.push(InsertedLink {
                                original_text: word.to_string(),
                                target: search_result.path,
                                display_text: search_result.metadata.title,
                                position: ContentPosition { line: 0, column: current_position, length: Some(word.len()) },
                                confidence: search_result.similarity,
                            });
                            
                            result.links_added += 1;
                            break; // Only link to the first high-confidence match
                        }
                    }
                }
            }
            current_position += word.len() + 1; // +1 for space
        }

        Ok(result)
    }

    /// Extract context around cursor position
    fn extract_context(&self, content: &str, cursor_position: &ContentPosition, window_size: usize) -> String {
        let lines: Vec<&str> = content.lines().collect();
        
        if cursor_position.line >= lines.len() {
            return String::new();
        }
        
        let current_line = lines[cursor_position.line];
        let start_pos = cursor_position.column.saturating_sub(window_size / 2);
        let end_pos = (cursor_position.column + window_size / 2).min(current_line.len());
        
        // Also include some context from previous and next lines
        let mut context_lines = Vec::new();
        
        if cursor_position.line > 0 {
            context_lines.push(lines[cursor_position.line - 1]);
        }
        
        context_lines.push(&current_line[start_pos..end_pos]);
        
        if cursor_position.line + 1 < lines.len() {
            context_lines.push(lines[cursor_position.line + 1]);
        }
        
        context_lines.join(" ")
    }

    /// Parse continuation suggestions from AI response
    fn parse_continuation_suggestions(&self, response: &str, context: &str, request: &ContentSuggestionRequest) -> Vec<ContentSuggestion> {
        response.lines()
            .filter(|line| !line.trim().is_empty())
            .take(3)
            .enumerate()
            .map(|(i, suggestion)| {
                ContentSuggestion {
                    text: suggestion.trim().to_string(),
                    suggestion_type: SuggestionType::ContentContinuation,
                    confidence: 0.8 - (i as f32 * 0.1),
                    context: context.to_string(),
                    reason: "AI-generated continuation".to_string(),
                    source_documents: vec![],
                    position: Some(request.cursor_position.clone()),
                }
            })
            .collect()
    }

    /// Get cached suggestions
    fn get_cached_suggestions(&self, content_hash: &str) -> Option<Vec<ContentSuggestion>> {
        if let Some(cached_suggestions) = self.suggestion_cache.suggestions.get(content_hash) {
            // Check if cache is still valid
            let now = Utc::now();
            let valid_suggestions: Vec<ContentSuggestion> = cached_suggestions.iter()
                .filter(|cached| {
                    let age = now.signed_duration_since(cached.cached_at);
                    age.num_seconds() < self.content_suggestion_config.cache_timeout_seconds as i64
                })
                .map(|cached| cached.suggestion.clone())
                .collect();
            
            if !valid_suggestions.is_empty() {
                return Some(valid_suggestions);
            }
        }
        None
    }

    /// Cache suggestions
    fn cache_suggestions(&mut self, content_hash: &str, suggestions: &[ContentSuggestion]) {
        let cached_suggestions: Vec<CachedSuggestion> = suggestions.iter()
            .map(|suggestion| CachedSuggestion {
                suggestion: suggestion.clone(),
                cached_at: Utc::now(),
                content_hash: content_hash.to_string(),
            })
            .collect();

        self.suggestion_cache.suggestions.insert(content_hash.to_string(), cached_suggestions);
    }

    /// Get suggestion cache statistics
    pub fn get_suggestion_cache_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();
        stats.insert("hit_count".to_string(), serde_json::Value::Number(self.suggestion_cache.hit_count.into()));
        stats.insert("miss_count".to_string(), serde_json::Value::Number(self.suggestion_cache.miss_count.into()));
        stats.insert("cached_entries".to_string(), serde_json::Value::Number(self.suggestion_cache.suggestions.len().into()));
        
        let hit_rate = if self.suggestion_cache.hit_count + self.suggestion_cache.miss_count > 0 {
            self.suggestion_cache.hit_count as f64 / (self.suggestion_cache.hit_count + self.suggestion_cache.miss_count) as f64
        } else {
            0.0
        };
        
        stats.insert("hit_rate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(hit_rate).unwrap_or(serde_json::Number::from(0))));
        
        stats
    }

    /// Clear suggestion cache
    pub fn clear_suggestion_cache(&mut self) {
        self.suggestion_cache.suggestions.clear();
        self.suggestion_cache.hit_count = 0;
        self.suggestion_cache.miss_count = 0;
    }

    /// Get note by vault and path
    pub async fn get_note(&self, vault: &str, path: &str) -> Result<MarkdownFile> {
        let vault_path = format!("{}:{}", vault, path);
        self.get_markdown_file_data(&vault_path).await
    }

    /// List available vaults
    pub async fn list_vaults(&self) -> Result<Vec<String>> {
        // For now, return a default vault
        // In a real implementation, this would query the Obsidian API
        Ok(vec!["default".to_string()])
    }

    /// List notes in a vault
    pub async fn list_notes(&self, vault: &str) -> Result<Vec<String>> {
        // For now, return empty list
        // In a real implementation, this would query the Obsidian API
        Ok(vec![])
    }

    pub async fn list_files_in_folder(&self, folder_path: &str) -> Result<Vec<String>> {
        // Create a URL to list files in the given folder
        let url = format!("{}/vault/{}/", self.base_url, folder_path);
        
        let response = self
            .add_auth_header(self.client.get(&url).header("Accept", "application/json"))
            .send()
            .await
            .context(format!("Failed to send GET request to {}", url))?;

        if response.status().is_success() {
            let response_text = response
                .text()
                .await
                .context("Failed to read response text")?;
            
            // Try to parse as JSON array of file names
            let files: Vec<String> = serde_json::from_str(&response_text)
                .context("Failed to parse file list response")?;
            
            Ok(files)
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            bail!(
                "MCP server returned error {}: {}. URL: {}",
                status,
                error_text,
                url
            )
        }
    }

    /// Delete a note
    pub async fn delete_note(&self, vault: &str, path: &str) -> Result<()> {
        // For now, this is a no-op
        // In a real implementation, this would delete via the Obsidian API
        Ok(())
    }

    /// Create a new note
    pub async fn create_note(&self, vault: &str, path: &str, content: &str) -> Result<()> {
        let vault_path = format!("{}:{}", vault, path);
        self.create_file(&vault_path, content).await
    }

    /// Update a note
    pub async fn update_note(&self, vault: &str, path: &str, content: &str) -> Result<()> {
        let vault_path = format!("{}:{}", vault, path);
        self.update_file(&vault_path, content).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_markdown_with_full_frontmatter() {
        let raw_md = r#"---
tags: [test, example]
due_date: "2024-01-01"
status: "pending"
target_date: "2024-02-01"
---

This is the content."#;
        let parsed = ObsidianAdapter::parse_markdown_file(raw_md).unwrap();
        assert_eq!(parsed.content, "This is the content.");
        assert_eq!(
            parsed.frontmatter.tags,
            Some(vec!["test".to_string(), "example".to_string()])
        );
        assert_eq!(parsed.frontmatter.due_date, Some("2024-01-01".to_string()));
        assert_eq!(parsed.frontmatter.status, Some("pending".to_string()));
        assert_eq!(
            parsed.frontmatter.target_date,
            Some("2024-02-01".to_string())
        );
    }

    #[test]
    fn test_parse_markdown_with_partial_frontmatter() {
        let raw_md = r#"---
tags: [test]
status: "active"
---

Content here."#;
        let parsed = ObsidianAdapter::parse_markdown_file(raw_md).unwrap();
        assert_eq!(parsed.content, "Content here.");
        assert_eq!(parsed.frontmatter.tags, Some(vec!["test".to_string()]));
        assert!(parsed.frontmatter.due_date.is_none());
        assert_eq!(parsed.frontmatter.status, Some("active".to_string()));
        assert!(parsed.frontmatter.target_date.is_none());
    }

    #[test]
    fn test_parse_markdown_without_frontmatter() {
        let raw_md = "Just content here.";
        let parsed = ObsidianAdapter::parse_markdown_file(raw_md).unwrap();
        assert_eq!(parsed.content, "Just content here.");
        assert!(parsed.frontmatter.tags.is_none());
        assert!(parsed.frontmatter.status.is_none());
    }

    #[test]
    fn test_parse_markdown_empty_frontmatter_section() {
        // "--- \n ---"
        let raw_md = r#"---
---

Content below empty frontmatter."#;
        let parsed = ObsidianAdapter::parse_markdown_file(raw_md).unwrap();
        assert_eq!(parsed.content, "Content below empty frontmatter.");
        // Default Frontmatter should have all Nones
        assert!(parsed.frontmatter.tags.is_none());
        assert!(parsed.frontmatter.due_date.is_none());
        assert!(parsed.frontmatter.status.is_none());
        assert!(parsed.frontmatter.target_date.is_none());
    }

    #[test]
    fn test_serialize_markdown_file_with_all_fields() {
        let file_data = MarkdownFile {
            frontmatter: Frontmatter {
                tags: Some(vec!["rust".to_string(), "dev".to_string()]),
                due_date: Some("tomorrow".to_string()),
                status: Some("in progress".to_string()),
                target_date: Some("next week".to_string()),
                ai_analysis: None,
                ai_analysis_version: None,
                ai_analysis_timestamp: None,
            },
            content: "Writing some Rust code.".to_string(),
        };
        let serialized = ObsidianAdapter::serialize_markdown_file(&file_data).unwrap();
        // Using contains to avoid strict YAML key order issues, though serde_yaml is usually consistent.
        // A better test would parse the YAML back.
        assert!(serialized.contains("tags:\n- rust\n- dev"));
        assert!(serialized.contains("due_date: tomorrow"));
        assert!(serialized.contains("status: in progress"));
        assert!(serialized.contains("target_date: next week"));
        assert!(serialized.ends_with("\n\nWriting some Rust code."));
    }

    #[test]
    fn test_serialize_markdown_file_with_some_fields_none() {
        let file_data = MarkdownFile {
            frontmatter: Frontmatter {
                tags: Some(vec!["task".to_string()]),
                due_date: None,
                status: Some("open".to_string()),
                target_date: None,
                ai_analysis: None,
                ai_analysis_version: None,
                ai_analysis_timestamp: None,
            },
            content: "A simple task.".to_string(),
        };
        let serialized = ObsidianAdapter::serialize_markdown_file(&file_data).unwrap();
        let expected_fm_yaml = "tags:\n- task\nstatus: open"; // due_date and target_date should be omitted by serde_yaml if None

        // Parse the frontmatter part of the serialized string
        let parts: Vec<&str> = serialized.splitn(3, "---").collect();
        assert!(parts.len() >= 3, "Serialized output not in expected format");
        let parsed_fm_yaml: serde_yaml::Value = serde_yaml::from_str(parts[1].trim()).unwrap();

        assert_eq!(
            parsed_fm_yaml
                .get("tags")
                .unwrap()
                .as_sequence()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            parsed_fm_yaml.get("tags").unwrap()[0].as_str().unwrap(),
            "task"
        );
        assert_eq!(
            parsed_fm_yaml.get("status").unwrap().as_str().unwrap(),
            "open"
        );
        assert!(
            parsed_fm_yaml.get("due_date").is_none()
                || parsed_fm_yaml.get("due_date").unwrap().is_null()
        );
        assert!(
            parsed_fm_yaml.get("target_date").is_none()
                || parsed_fm_yaml.get("target_date").unwrap().is_null()
        );

        assert!(serialized.ends_with("\n\nA simple task."));
    }

    #[test]
    fn test_frontmatter_to_string_method() {
        let fm = Frontmatter {
            tags: Some(vec!["a".to_string()]),
            status: Some("active".to_string()),
            ..Default::default()
        };
        let md_file = MarkdownFile {
            frontmatter: fm,
            content: "test".to_string(),
        };
        let fm_str = md_file.frontmatter_to_string().unwrap();
        assert!(fm_str.contains("tags:\n- a"));
        assert!(fm_str.contains("status: active"));
    }

    #[test]
    fn test_content_analysis_default() {
        let analysis = ContentAnalysis::default();
        assert!(analysis.themes.is_empty());
        assert_eq!(analysis.sentiment.overall, "neutral");
        assert_eq!(analysis.sentiment.confidence, 0.0);
        assert!(analysis.entities.is_empty());
        assert!(analysis.concepts.is_empty());
        assert!(analysis.summary.is_none());
        assert!(analysis.keywords.is_empty());
        assert!(analysis.category.is_none());
        assert!(analysis.complexity_score.is_none());
        assert!(analysis.reading_time_minutes.is_none());
    }

    #[test]
    fn test_sentiment_analysis_creation() {
        let sentiment = SentimentAnalysis {
            overall: "positive".to_string(),
            confidence: 0.85,
            emotions: vec!["happy".to_string(), "excited".to_string()],
        };
        
        assert_eq!(sentiment.overall, "positive");
        assert_eq!(sentiment.confidence, 0.85);
        assert_eq!(sentiment.emotions.len(), 2);
    }

    #[test]
    fn test_entity_creation() {
        let entity = Entity {
            text: "John Doe".to_string(),
            entity_type: "PERSON".to_string(),
            confidence: 0.95,
            context: Some("Software engineer".to_string()),
        };
        
        assert_eq!(entity.text, "John Doe");
        assert_eq!(entity.entity_type, "PERSON");
        assert_eq!(entity.confidence, 0.95);
        assert!(entity.context.is_some());
    }

    #[test]
    fn test_concept_creation() {
        let concept = Concept {
            name: "Machine Learning".to_string(),
            description: Some("AI technology".to_string()),
            related_concepts: vec!["AI".to_string(), "Neural Networks".to_string()],
            importance: 0.8,
        };
        
        assert_eq!(concept.name, "Machine Learning");
        assert!(concept.description.is_some());
        assert_eq!(concept.related_concepts.len(), 2);
        assert_eq!(concept.importance, 0.8);
    }

    #[test]
    fn test_analysis_config_default() {
        let config = AnalysisConfig::default();
        assert!(config.extract_themes);
        assert!(config.analyze_sentiment);
        assert!(config.extract_entities);
        assert!(config.identify_concepts);
        assert!(config.generate_summary);
        assert_eq!(config.max_themes, 10);
        assert_eq!(config.max_entities, 20);
        assert_eq!(config.max_concepts, 15);
        assert_eq!(config.entity_confidence_threshold, 0.7);
    }

    #[test]
    fn test_frontmatter_with_ai_analysis() {
        let analysis = ContentAnalysis {
            themes: vec!["technology".to_string(), "programming".to_string()],
            sentiment: SentimentAnalysis {
                overall: "positive".to_string(),
                confidence: 0.8,
                emotions: vec!["enthusiasm".to_string()],
            },
            entities: vec![Entity {
                text: "Rust".to_string(),
                entity_type: "TECHNOLOGY".to_string(),
                confidence: 0.9,
                context: Some("Programming language".to_string()),
            }],
            concepts: vec![Concept {
                name: "Systems Programming".to_string(),
                description: Some("Low-level programming".to_string()),
                related_concepts: vec!["Memory Management".to_string()],
                importance: 0.85,
            }],
            summary: Some("Article about Rust programming".to_string()),
            keywords: vec!["rust".to_string(), "programming".to_string(), "systems".to_string()],
            category: Some("Technology".to_string()),
            complexity_score: Some(7.5),
            reading_time_minutes: Some(5),
        };

        let frontmatter = Frontmatter {
            tags: Some(vec!["programming".to_string()]),
            ai_analysis: Some(analysis.clone()),
            ai_analysis_version: Some("1.0.0".to_string()),
            ai_analysis_timestamp: Some(Utc::now().to_rfc3339()),
            ..Default::default()
        };

        assert!(frontmatter.ai_analysis.is_some());
        assert_eq!(frontmatter.ai_analysis.as_ref().unwrap().themes.len(), 2);
        assert_eq!(frontmatter.ai_analysis.as_ref().unwrap().sentiment.overall, "positive");
        assert_eq!(frontmatter.ai_analysis.as_ref().unwrap().entities.len(), 1);
        assert_eq!(frontmatter.ai_analysis.as_ref().unwrap().concepts.len(), 1);
    }

    #[test]
    fn test_cache_key_generation() {
        let adapter = ObsidianAdapter::new(None, None);
        let content1 = "This is some test content";
        let content2 = "This is some different content";
        let content3 = "This is some test content"; // Same as content1

        let key1 = adapter.generate_cache_key(content1);
        let key2 = adapter.generate_cache_key(content2);
        let key3 = adapter.generate_cache_key(content3);

        assert_ne!(key1, key2); // Different content should have different keys
        assert_eq!(key1, key3); // Same content should have same keys
        assert!(key1.starts_with("content_"));
    }

    #[test]
    fn test_analysis_prompt_creation() {
        let adapter = ObsidianAdapter::new(None, None);
        let content = "This is a test content with multiple words to test reading time calculation.";
        let prompt = adapter.create_analysis_prompt(content);

        assert!(prompt.contains("JSON response"));
        assert!(prompt.contains("themes"));
        assert!(prompt.contains("sentiment"));
        assert!(prompt.contains("entities"));
        assert!(prompt.contains("concepts"));
        assert!(prompt.contains(content));
        assert!(prompt.contains("reading_time_minutes"));
    }

    #[test]
    fn test_parse_analysis_response_valid_json() {
        let adapter = ObsidianAdapter::new(None, None);
        let response = r#"{
            "themes": ["technology", "programming"],
            "sentiment": {
                "overall": "positive",
                "confidence": 0.8,
                "emotions": ["excitement"]
            },
            "entities": [
                {
                    "text": "Rust",
                    "entity_type": "TECHNOLOGY", 
                    "confidence": 0.9,
                    "context": "Programming language"
                }
            ],
            "concepts": [
                {
                    "name": "Memory Safety",
                    "description": "Preventing memory errors",
                    "related_concepts": ["Ownership"],
                    "importance": 0.85
                }
            ],
            "summary": "Article about Rust programming",
            "keywords": ["rust", "memory", "safety"],
            "category": "Technology",
            "complexity_score": 7.5,
            "reading_time_minutes": 5
        }"#;

        let result = adapter.parse_analysis_response(response);
        assert!(result.is_ok());
        
        let analysis = result.unwrap();
        assert_eq!(analysis.themes.len(), 2);
        assert_eq!(analysis.sentiment.overall, "positive");
        assert_eq!(analysis.entities.len(), 1);
        assert_eq!(analysis.concepts.len(), 1);
        assert!(analysis.summary.is_some());
        assert_eq!(analysis.keywords.len(), 3);
        assert!(analysis.category.is_some());
        assert!(analysis.complexity_score.is_some());
        assert!(analysis.reading_time_minutes.is_some());
    }

    #[test] 
    fn test_parse_analysis_response_with_markdown_wrapper() {
        let adapter = ObsidianAdapter::new(None, None);
        let response = r#"Here's the analysis:
        
```json
{
    "themes": ["test"],
    "sentiment": {
        "overall": "neutral",
        "confidence": 0.5,
        "emotions": []
    },
    "entities": [],
    "concepts": [],
    "summary": "Test content",
    "keywords": ["test"],
    "category": "Test",
    "complexity_score": 3.0,
    "reading_time_minutes": 1
}
```

That's the complete analysis."#;

        let result = adapter.parse_analysis_response(response);
        assert!(result.is_ok());
        
        let analysis = result.unwrap();
        assert_eq!(analysis.themes.len(), 1);
        assert_eq!(analysis.themes[0], "test");
    }

    #[test]
    fn test_tag_normalization() {
        let adapter = ObsidianAdapter::new(None, None);
        
        // Test various tag normalization scenarios
        assert_eq!(adapter.normalize_tag("Machine Learning"), "machine_learning");
        assert_eq!(adapter.normalize_tag("AI/ML"), "ai_ml");
        assert_eq!(adapter.normalize_tag("Data-Science"), "data_science");
        assert_eq!(adapter.normalize_tag("Programming!"), "programming");
        assert_eq!(adapter.normalize_tag("Web Development"), "web_development");
        assert_eq!(adapter.normalize_tag("React.js"), "reactjs");
    }

    #[test]
    fn test_organization_config_default() {
        let config = OrganizationConfig::default();
        
        assert_eq!(config.auto_tag_confidence_threshold, 0.8);
        assert_eq!(config.max_tag_suggestions, 10);
        assert_eq!(config.max_folder_suggestions, 3);
        assert_eq!(config.max_link_suggestions, 5);
        assert!(!config.auto_apply_tags);
        assert!(config.suggest_folder_moves);
        assert!(config.suggest_auto_links);
        assert!(config.custom_tag_rules.is_empty());
    }

    #[test]
    fn test_tag_suggestion_creation() {
        let suggestion = TagSuggestion {
            tag: "machine_learning".to_string(),
            confidence: 0.9,
            reason: "Found as major theme in content".to_string(),
            source: TagSource::Theme,
        };
        
        assert_eq!(suggestion.tag, "machine_learning");
        assert_eq!(suggestion.confidence, 0.9);
        assert!(matches!(suggestion.source, TagSource::Theme));
    }

    #[test]
    fn test_folder_suggestion_creation() {
        let suggestion = FolderSuggestion {
            folder_path: "technology".to_string(),
            confidence: 0.8,
            reason: "Content is classified as Technology".to_string(),
            category: "Technology".to_string(),
        };
        
        assert_eq!(suggestion.folder_path, "technology");
        assert_eq!(suggestion.confidence, 0.8);
        assert_eq!(suggestion.category, "Technology");
    }

    #[test]
    fn test_link_suggestion_creation() {
        let suggestion = LinkSuggestion {
            target_path: "notes/related_article.md".to_string(),
            link_text: "Related Article".to_string(),
            confidence: 0.75,
            context: "Related content about machine learning".to_string(),
            reason: "Semantically similar (75% match)".to_string(),
        };
        
        assert_eq!(suggestion.target_path, "notes/related_article.md");
        assert_eq!(suggestion.link_text, "Related Article");
        assert_eq!(suggestion.confidence, 0.75);
    }

    #[test]
    fn test_organization_recommendations_structure() {
        let recommendations = OrganizationRecommendations {
            suggested_tags: vec![
                TagSuggestion {
                    tag: "ai".to_string(),
                    confidence: 0.9,
                    reason: "Primary theme".to_string(),
                    source: TagSource::Theme,
                }
            ],
            folder_suggestions: vec![
                FolderSuggestion {
                    folder_path: "technology".to_string(),
                    confidence: 0.8,
                    reason: "Technology content".to_string(),
                    category: "Technology".to_string(),
                }
            ],
            link_suggestions: vec![
                LinkSuggestion {
                    target_path: "other.md".to_string(),
                    link_text: "Other".to_string(),
                    confidence: 0.7,
                    context: "Related".to_string(),
                    reason: "Similar content".to_string(),
                }
            ],
            overall_confidence: 0.8,
            generated_at: Utc::now(),
        };
        
        assert_eq!(recommendations.suggested_tags.len(), 1);
        assert_eq!(recommendations.folder_suggestions.len(), 1);
        assert_eq!(recommendations.link_suggestions.len(), 1);
        assert_eq!(recommendations.overall_confidence, 0.8);
    }

    #[test]
    fn test_content_suggestion_config_default() {
        let config = ContentSuggestionConfig::default();
        
        assert!(config.enable_real_time);
        assert_eq!(config.min_confidence, 0.6);
        assert_eq!(config.max_suggestions, 5);
        assert_eq!(config.context_window, 200);
        assert_eq!(config.debounce_delay_ms, 300);
        assert!(!config.auto_insert_links);
        assert_eq!(config.auto_link_confidence_threshold, 0.8);
        assert_eq!(config.cache_timeout_seconds, 300);
    }

    #[test]
    fn test_content_suggestion_creation() {
        let suggestion = ContentSuggestion {
            text: "This is a test suggestion".to_string(),
            suggestion_type: SuggestionType::ContentContinuation,
            confidence: 0.8,
            context: "Previous context".to_string(),
            reason: "AI-generated".to_string(),
            source_documents: vec!["doc1.md".to_string()],
            position: Some(ContentPosition { line: 1, column: 10, length: Some(5) }),
        };
        
        assert_eq!(suggestion.text, "This is a test suggestion");
        assert!(matches!(suggestion.suggestion_type, SuggestionType::ContentContinuation));
        assert_eq!(suggestion.confidence, 0.8);
        assert_eq!(suggestion.context, "Previous context");
        assert_eq!(suggestion.source_documents.len(), 1);
        assert!(suggestion.position.is_some());
    }

    #[test]
    fn test_content_position() {
        let position = ContentPosition {
            line: 5,
            column: 15,
            length: Some(10),
        };
        
        assert_eq!(position.line, 5);
        assert_eq!(position.column, 15);
        assert_eq!(position.length, Some(10));
    }

    #[test]
    fn test_auto_link_result() {
        let result = AutoLinkResult {
            original_text: "This is original text".to_string(),
            linked_text: "This is [[linked]] text".to_string(),
            inserted_links: vec![
                InsertedLink {
                    original_text: "linked".to_string(),
                    target: "target.md".to_string(),
                    display_text: "Linked Note".to_string(),
                    position: ContentPosition { line: 0, column: 8, length: Some(6) },
                    confidence: 0.9,
                }
            ],
            links_added: 1,
        };
        
        assert_eq!(result.original_text, "This is original text");
        assert_eq!(result.linked_text, "This is [[linked]] text");
        assert_eq!(result.inserted_links.len(), 1);
        assert_eq!(result.links_added, 1);
    }

    #[test]
    fn test_content_suggestion_request() {
        let request = ContentSuggestionRequest {
            content: "This is test content".to_string(),
            cursor_position: ContentPosition { line: 0, column: 10, length: None },
            max_suggestions: 5,
            context_window: 100,
            suggestion_types: vec![SuggestionType::ContentContinuation, SuggestionType::LinkSuggestion],
        };
        
        assert_eq!(request.content, "This is test content");
        assert_eq!(request.cursor_position.column, 10);
        assert_eq!(request.max_suggestions, 5);
        assert_eq!(request.context_window, 100);
        assert_eq!(request.suggestion_types.len(), 2);
    }

    #[test]
    fn test_extract_context() {
        let adapter = ObsidianAdapter::new(None, None);
        let content = "Line 1\nLine 2 with some content\nLine 3\nLine 4";
        let position = ContentPosition { line: 1, column: 10, length: None };
        
        let context = adapter.extract_context(content, &position, 20);
        
        assert!(context.contains("Line 1"));
        assert!(context.contains("with some"));
        assert!(context.contains("Line 3"));
    }

    #[test]
    fn test_suggestion_type_variants() {
        let types = vec![
            SuggestionType::ContentContinuation,
            SuggestionType::RelatedContent,
            SuggestionType::LinkSuggestion,
            SuggestionType::TextCompletion,
            SuggestionType::HeadingSuggestion,
            SuggestionType::BulletPointSuggestion,
            SuggestionType::CodeBlockSuggestion,
        ];
        
        assert_eq!(types.len(), 7);
    }

    #[test]
    fn test_suggestion_cache_initialization() {
        let cache = SuggestionCache {
            suggestions: HashMap::new(),
            hit_count: 0,
            miss_count: 0,
        };
        
        assert!(cache.suggestions.is_empty());
        assert_eq!(cache.hit_count, 0);
        assert_eq!(cache.miss_count, 0);
    }
}
