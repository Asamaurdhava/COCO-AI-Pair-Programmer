use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub anthropic_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub ai_provider: AiProvider,
    pub file_patterns: Vec<String>,
    pub ignore_patterns: Vec<String>,
    pub max_file_size: u64,
    pub analysis_delay_ms: u64,
    pub ui_theme: UiTheme,
    pub session_auto_save: bool,
    pub session_max_events: usize,
    pub log_level: LogLevel,
    pub watch_directories: Vec<String>,
    pub auto_suggestions: bool,
    pub suggestion_confidence_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiProvider {
    Anthropic,
    OpenAI,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiTheme {
    pub primary_color: String,
    pub secondary_color: String,
    pub background_color: String,
    pub text_color: String,
    pub accent_color: String,
    pub error_color: String,
    pub warning_color: String,
    pub success_color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            anthropic_api_key: None,
            openai_api_key: None,
            ai_provider: AiProvider::Anthropic,
            file_patterns: vec![
                "*.rs".to_string(),
                "*.py".to_string(),
                "*.js".to_string(),
                "*.ts".to_string(),
                "*.jsx".to_string(),
                "*.tsx".to_string(),
                "*.go".to_string(),
                "*.java".to_string(),
                "*.c".to_string(),
                "*.cpp".to_string(),
                "*.h".to_string(),
                "*.hpp".to_string(),
                "*.cs".to_string(),
                "*.rb".to_string(),
                "*.php".to_string(),
                "*.swift".to_string(),
                "*.kt".to_string(),
                "*.scala".to_string(),
                "*.clj".to_string(),
                "*.ex".to_string(),
                "*.exs".to_string(),
            ],
            ignore_patterns: vec![
                "target/*".to_string(),
                "node_modules/*".to_string(),
                ".git/*".to_string(),
                "*.log".to_string(),
                "*.tmp".to_string(),
                "*.temp".to_string(),
                "build/*".to_string(),
                "dist/*".to_string(),
                "out/*".to_string(),
                "*.lock".to_string(),
                ".env".to_string(),
                ".env.local".to_string(),
                "*.min.js".to_string(),
                "*.min.css".to_string(),
            ],
            max_file_size: 1024 * 1024, // 1MB
            analysis_delay_ms: 500,
            ui_theme: UiTheme::default(),
            session_auto_save: true,
            session_max_events: 10000,
            log_level: LogLevel::Info,
            watch_directories: vec![".".to_string()],
            auto_suggestions: true,
            suggestion_confidence_threshold: 0.7,
        }
    }
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            primary_color: "#3b82f6".to_string(),   // Blue
            secondary_color: "#6b7280".to_string(), // Gray
            background_color: "#1f2937".to_string(), // Dark gray
            text_color: "#f3f4f6".to_string(),     // Light gray
            accent_color: "#10b981".to_string(),    // Green
            error_color: "#ef4444".to_string(),     // Red
            warning_color: "#f59e0b".to_string(),   // Amber
            success_color: "#22c55e".to_string(),   // Green
        }
    }
}

impl Config {
    pub async fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        let mut config = if config_path.exists() {
            let content = fs::read_to_string(&config_path).await?;
            toml::from_str(&content)
                .map_err(|e| anyhow::anyhow!("Failed to parse config file: {}", e))?
        } else {
            // Create default config
            Self::default()
        };

        // Always load from environment variables (including .env file)
        config.load_from_env();

        // Save the config if it doesn't exist
        if !config_path.exists() {
            config.save().await?;
        }

        Ok(config)
    }

    pub async fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        // Create config directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))?;

        fs::write(&config_path, content).await?;

        tracing::info!("Config saved to: {}", config_path.display());
        Ok(())
    }

    fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

        Ok(home.join(".coco").join("config.toml"))
    }

    fn load_from_env(&mut self) {
        // Load API keys from environment
        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            if !key.is_empty() {
                self.anthropic_api_key = Some(key);
                tracing::info!("Loaded Anthropic API key from environment");
            }
        } else {
            tracing::warn!("ANTHROPIC_API_KEY not found in environment");
        }

        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            self.openai_api_key = Some(key);
        }

        // Load AI provider
        if let Ok(provider) = std::env::var("COCO_AI_PROVIDER") {
            match provider.to_lowercase().as_str() {
                "anthropic" => self.ai_provider = AiProvider::Anthropic,
                "openai" => self.ai_provider = AiProvider::OpenAI,
                "local" => self.ai_provider = AiProvider::Local,
                _ => tracing::warn!("Unknown AI provider: {}", provider),
            }
        }

        // Load log level
        if let Ok(level) = std::env::var("COCO_LOG_LEVEL") {
            match level.to_lowercase().as_str() {
                "error" => self.log_level = LogLevel::Error,
                "warn" => self.log_level = LogLevel::Warn,
                "info" => self.log_level = LogLevel::Info,
                "debug" => self.log_level = LogLevel::Debug,
                "trace" => self.log_level = LogLevel::Trace,
                _ => tracing::warn!("Unknown log level: {}", level),
            }
        }

        // Load max file size
        if let Ok(size) = std::env::var("COCO_MAX_FILE_SIZE") {
            if let Ok(size) = size.parse::<u64>() {
                self.max_file_size = size;
            }
        }

        // Load analysis delay
        if let Ok(delay) = std::env::var("COCO_ANALYSIS_DELAY_MS") {
            if let Ok(delay) = delay.parse::<u64>() {
                self.analysis_delay_ms = delay;
            }
        }

        // Load auto suggestions setting
        if let Ok(auto) = std::env::var("COCO_AUTO_SUGGESTIONS") {
            self.auto_suggestions = auto.to_lowercase() == "true";
        }

        // Load confidence threshold
        if let Ok(threshold) = std::env::var("COCO_CONFIDENCE_THRESHOLD") {
            if let Ok(threshold) = threshold.parse::<f32>() {
                self.suggestion_confidence_threshold = threshold;
            }
        }
    }

    pub fn is_file_supported(&self, path: &std::path::Path) -> bool {
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        let path_str = path.to_string_lossy();

        // Check ignore patterns first
        for pattern in &self.ignore_patterns {
            if Self::matches_pattern(&path_str, pattern) {
                return false;
            }
        }

        // Check file patterns
        for pattern in &self.file_patterns {
            if Self::matches_pattern(file_name, pattern) {
                return true;
            }
        }

        false
    }

    fn matches_pattern(text: &str, pattern: &str) -> bool {
        if pattern.contains('*') {
            // Simple glob matching
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                let prefix = parts[0];
                let suffix = parts[1];
                text.starts_with(prefix) && text.ends_with(suffix)
            } else {
                // More complex patterns could be implemented here
                false
            }
        } else {
            text == pattern
        }
    }

    pub fn should_watch_directory(&self, path: &std::path::Path) -> bool {
        let path_str = path.to_string_lossy();

        for watch_dir in &self.watch_directories {
            if path_str.starts_with(watch_dir) {
                return true;
            }
        }

        false
    }

    pub async fn validate(&self) -> Result<()> {
        // Validate API keys based on provider
        match self.ai_provider {
            AiProvider::Anthropic => {
                if self.anthropic_api_key.is_none() {
                    return Err(anyhow::anyhow!(
                        "Anthropic API key is required. Set ANTHROPIC_API_KEY environment variable."
                    ));
                }
            }
            AiProvider::OpenAI => {
                if self.openai_api_key.is_none() {
                    return Err(anyhow::anyhow!(
                        "OpenAI API key is required. Set OPENAI_API_KEY environment variable."
                    ));
                }
            }
            AiProvider::Local => {
                // No API key validation needed for local provider
            }
        }

        // Validate file size limits
        if self.max_file_size == 0 {
            return Err(anyhow::anyhow!("Max file size must be greater than 0"));
        }

        // Validate confidence threshold
        if !(0.0..=1.0).contains(&self.suggestion_confidence_threshold) {
            return Err(anyhow::anyhow!(
                "Confidence threshold must be between 0.0 and 1.0"
            ));
        }

        // Validate watch directories exist
        for dir in &self.watch_directories {
            let path = std::path::Path::new(dir);
            if !path.exists() {
                tracing::warn!("Watch directory does not exist: {}", dir);
            }
        }

        Ok(())
    }

    pub fn get_tracing_level(&self) -> tracing::Level {
        match self.log_level {
            LogLevel::Error => tracing::Level::ERROR,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Trace => tracing::Level::TRACE,
        }
    }
}