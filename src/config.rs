use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub llm: LLMConfig,
    pub obsidian: ObsidianConfig,
    pub general: GeneralConfig,
}

/// LLM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub provider: String,
    pub gemini: GeminiConfig,
    pub openai: OpenAIConfig,
}

/// Gemini-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiConfig {
    pub api_key: Option<String>,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
}

/// OpenAI-specific configuration (for future use)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIConfig {
    pub api_key: Option<String>,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
}

/// Obsidian configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObsidianConfig {
    pub api_key: Option<String>,
    pub base_url: String,
}

/// General application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub verbose: bool,
    pub auto_save: bool,
    pub max_conversation_history: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            llm: LLMConfig {
                provider: "gemini".to_string(),
                gemini: GeminiConfig {
                    api_key: None,
                    model: "gemini-1.5-flash".to_string(),
                    temperature: 0.7,
                    max_tokens: 8192,
                },
                openai: OpenAIConfig {
                    api_key: None,
                    model: "gpt-4o-mini".to_string(),
                    temperature: 0.7,
                    max_tokens: 4096,
                },
            },
            obsidian: ObsidianConfig {
                api_key: None,
                base_url: "https://127.0.0.1:27124".to_string(),
            },
            general: GeneralConfig {
                verbose: false,
                auto_save: true,
                max_conversation_history: 100,
            },
        }
    }
}

impl Config {
    /// Load configuration from file and environment variables
    pub fn load() -> Result<Self> {
        let mut config = Self::load_from_file().unwrap_or_default();
        config.load_from_env();
        Ok(config)
    }

    /// Load configuration from file
    fn load_from_file() -> Option<Self> {
        let config_path = Self::get_config_path();
        if config_path.exists() {
            let contents = fs::read_to_string(config_path).ok()?;
            toml::from_str(&contents).ok()
        } else {
            None
        }
    }

    /// Load configuration from environment variables
    fn load_from_env(&mut self) {
        if let Ok(api_key) = env::var("GEMINI_API_KEY") {
            self.llm.gemini.api_key = Some(api_key);
        }
        
        if let Ok(api_key) = env::var("OPENAI_API_KEY") {
            self.llm.openai.api_key = Some(api_key);
        }
        
        if let Ok(api_key) = env::var("OBSIDIAN_API_KEY") {
            self.obsidian.api_key = Some(api_key);
        }
        
        if let Ok(base_url) = env::var("OBSIDIAN_BASE_URL") {
            self.obsidian.base_url = base_url;
        }
        
        if let Ok(provider) = env::var("LLM_PROVIDER") {
            self.llm.provider = provider;
        }
        
        if let Ok(verbose) = env::var("ARROWHEAD_VERBOSE") {
            self.general.verbose = verbose.parse().unwrap_or(false);
        }
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path();
        
        // Create config directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let contents = toml::to_string_pretty(self)?;
        fs::write(config_path, contents)?;
        
        Ok(())
    }

    /// Get the configuration file path
    fn get_config_path() -> PathBuf {
        // Use ~/.config/arrowhead/config.toml consistently across platforms
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(".config");
        path.push("arrowhead");
        path.push("config.toml");
        path
    }

    /// Get the API key for the current LLM provider
    pub fn get_llm_api_key(&self) -> Option<String> {
        match self.llm.provider.as_str() {
            "gemini" => self.llm.gemini.api_key.clone(),
            "openai" => self.llm.openai.api_key.clone(),
            _ => None,
        }
    }

    /// Get the model name for the current LLM provider
    pub fn get_llm_model(&self) -> String {
        match self.llm.provider.as_str() {
            "gemini" => self.llm.gemini.model.clone(),
            "openai" => self.llm.openai.model.clone(),
            _ => "gemini-1.5-flash".to_string(),
        }
    }

    /// Get the temperature for the current LLM provider
    pub fn get_llm_temperature(&self) -> f32 {
        match self.llm.provider.as_str() {
            "gemini" => self.llm.gemini.temperature,
            "openai" => self.llm.openai.temperature,
            _ => 0.7,
        }
    }

    /// Get the max tokens for the current LLM provider
    pub fn get_llm_max_tokens(&self) -> u32 {
        match self.llm.provider.as_str() {
            "gemini" => self.llm.gemini.max_tokens,
            "openai" => self.llm.openai.max_tokens,
            _ => 8192,
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Check that we have an API key for the selected provider
        if self.get_llm_api_key().is_none() {
            return Err(anyhow::anyhow!(
                "No API key found for LLM provider '{}'. Please set the appropriate environment variable:\n\
                 - For Gemini: GEMINI_API_KEY\n\
                 - For OpenAI: OPENAI_API_KEY",
                self.llm.provider
            ));
        }

        // Validate temperature range
        let temperature = self.get_llm_temperature();
        if temperature < 0.0 || temperature > 2.0 {
            return Err(anyhow::anyhow!(
                "Temperature must be between 0.0 and 2.0, got {}",
                temperature
            ));
        }

        // Validate max tokens
        let max_tokens = self.get_llm_max_tokens();
        if max_tokens == 0 {
            return Err(anyhow::anyhow!("Max tokens must be greater than 0"));
        }

        Ok(())
    }

    /// Create a sample configuration file
    pub fn create_sample_config() -> Result<()> {
        let config = Self::default();
        let config_path = Self::get_config_path();
        
        if config_path.exists() {
            println!("Configuration file already exists at: {}", config_path.display());
            return Ok(());
        }
        
        config.save()?;
        
        println!("Created sample configuration file at: {}", config_path.display());
        println!("Please edit the file and set your API keys, or use environment variables:");
        println!("  GEMINI_API_KEY=your_gemini_api_key");
        println!("  OBSIDIAN_API_KEY=your_obsidian_api_key");
        
        Ok(())
    }

    /// Set a configuration value
    pub fn set_value(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "gemini.api_key" => {
                self.llm.gemini.api_key = Some(value.to_string());
            }
            "gemini.model" => {
                self.llm.gemini.model = value.to_string();
            }
            "gemini.temperature" => {
                self.llm.gemini.temperature = value.parse::<f32>()
                    .map_err(|_| anyhow::anyhow!("Invalid temperature value: {}", value))?;
            }
            "gemini.max_tokens" => {
                self.llm.gemini.max_tokens = value.parse::<u32>()
                    .map_err(|_| anyhow::anyhow!("Invalid max_tokens value: {}", value))?;
            }
            "openai.api_key" => {
                self.llm.openai.api_key = Some(value.to_string());
            }
            "openai.model" => {
                self.llm.openai.model = value.to_string();
            }
            "openai.temperature" => {
                self.llm.openai.temperature = value.parse::<f32>()
                    .map_err(|_| anyhow::anyhow!("Invalid temperature value: {}", value))?;
            }
            "openai.max_tokens" => {
                self.llm.openai.max_tokens = value.parse::<u32>()
                    .map_err(|_| anyhow::anyhow!("Invalid max_tokens value: {}", value))?;
            }
            "obsidian.api_key" => {
                self.obsidian.api_key = Some(value.to_string());
            }
            "obsidian.base_url" => {
                self.obsidian.base_url = value.to_string();
            }
            "provider" => {
                match value {
                    "gemini" | "openai" => {
                        self.llm.provider = value.to_string();
                    }
                    _ => {
                        return Err(anyhow::anyhow!("Invalid provider: {}. Must be 'gemini' or 'openai'", value));
                    }
                }
            }
            "general.verbose" => {
                self.general.verbose = value.parse::<bool>()
                    .map_err(|_| anyhow::anyhow!("Invalid verbose value: {}. Must be true or false", value))?;
            }
            "general.auto_save" => {
                self.general.auto_save = value.parse::<bool>()
                    .map_err(|_| anyhow::anyhow!("Invalid auto_save value: {}. Must be true or false", value))?;
            }
            "general.max_conversation_history" => {
                self.general.max_conversation_history = value.parse::<usize>()
                    .map_err(|_| anyhow::anyhow!("Invalid max_conversation_history value: {}", value))?;
            }
            _ => {
                return Err(anyhow::anyhow!("Unknown configuration key: {}", key));
            }
        }
        Ok(())
    }

    /// Get available configuration keys
    pub fn get_available_keys() -> Vec<&'static str> {
        vec![
            "gemini.api_key",
            "gemini.model",
            "gemini.temperature",
            "gemini.max_tokens",
            "openai.api_key",
            "openai.model",
            "openai.temperature",
            "openai.max_tokens",
            "obsidian.api_key",
            "obsidian.base_url",
            "provider",
            "general.verbose",
            "general.auto_save",
            "general.max_conversation_history",
        ]
    }
}

/// Configuration builder for programmatic configuration
pub struct ConfigBuilder {
    config: Config,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: Config::default(),
        }
    }

    pub fn llm_provider(mut self, provider: &str) -> Self {
        self.config.llm.provider = provider.to_string();
        self
    }

    pub fn gemini_api_key(mut self, api_key: &str) -> Self {
        self.config.llm.gemini.api_key = Some(api_key.to_string());
        self
    }

    pub fn gemini_model(mut self, model: &str) -> Self {
        self.config.llm.gemini.model = model.to_string();
        self
    }

    pub fn verbose(mut self, verbose: bool) -> Self {
        self.config.general.verbose = verbose;
        self
    }

    pub fn build(self) -> Config {
        self.config
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.llm.provider, "gemini");
        assert_eq!(config.llm.gemini.model, "gemini-1.5-flash");
        assert_eq!(config.llm.gemini.temperature, 0.7);
        assert_eq!(config.llm.gemini.max_tokens, 8192);
    }

    #[test]
    fn test_config_builder() {
        let config = ConfigBuilder::new()
            .llm_provider("gemini")
            .gemini_api_key("test_key")
            .gemini_model("gemini-1.5-flash")
            .verbose(true)
            .build();

        assert_eq!(config.llm.provider, "gemini");
        assert_eq!(config.llm.gemini.api_key, Some("test_key".to_string()));
        assert_eq!(config.llm.gemini.model, "gemini-1.5-flash");
        assert_eq!(config.general.verbose, true);
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        
        // Should fail without API key
        assert!(config.validate().is_err());
        
        // Should pass with API key
        config.llm.gemini.api_key = Some("test_key".to_string());
        assert!(config.validate().is_ok());
        
        // Should fail with invalid temperature
        config.llm.gemini.temperature = 3.0;
        assert!(config.validate().is_err());
    }
}