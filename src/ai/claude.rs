use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use chrono::Utc;

use crate::app::{AiRequest, AiRequestType, Thought, ThoughtType, Suggestion, ActionType, Priority};
use super::{AiProvider, analyzer::CodeAnalyzer};

#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ClaudeMessage>,
    temperature: f32,
    system: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
    usage: Option<ClaudeUsage>,
}

#[derive(Deserialize)]
struct ClaudeContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Deserialize)]
struct ClaudeUsage {
    input_tokens: u32,
    output_tokens: u32,
}

pub struct ClaudeProvider {
    client: Client,
    api_key: String,
    model: String,
    max_retries: u32,
    retry_delay: Duration,
    analyzer: CodeAnalyzer,
}

impl ClaudeProvider {
    pub fn new(api_key: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()?;

        Ok(Self {
            client,
            api_key,
            model: "claude-3-5-haiku-20241022".to_string(),
            max_retries: 3,
            retry_delay: Duration::from_millis(1000),
            analyzer: CodeAnalyzer::new(),
        })
    }

    async fn make_request(&self, prompt: &str, system_prompt: Option<&str>) -> Result<String> {
        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 4096,
            messages: vec![ClaudeMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: 0.7,
            system: system_prompt.map(|s| s.to_string()),
        };

        let mut last_error = None;

        for attempt in 0..self.max_retries {
            match self.send_request(&request).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.max_retries - 1 {
                        let delay = self.retry_delay * (2_u32.pow(attempt));
                        tracing::warn!("API request failed, retrying in {:?}. Error: {}", delay, last_error.as_ref().unwrap());
                        sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("All retry attempts failed")))
    }

    async fn send_request(&self, request: &ClaudeRequest) -> Result<String> {
        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("API request failed with status {}: {}", status, error_text));
        }

        let claude_response: ClaudeResponse = response.json().await?;

        if let Some(content) = claude_response.content.first() {
            if let Some(text) = &content.text {
                return Ok(text.clone());
            }
        }

        Err(anyhow!("No text content in response"))
    }

    fn create_analysis_prompt(&self, request: &AiRequest) -> (String, String) {
        let system_prompt = match request.request_type {
            AiRequestType::Analyze => {
                "You are an expert code reviewer and AI pair programmer. Analyze the provided code and provide thoughtful insights about:
1. Code quality and structure
2. Potential bugs or issues
3. Performance considerations
4. Security implications
5. Best practices and improvements
6. Architecture patterns

Format your response as structured thoughts that can help the developer. Be concise but thorough."
            }
            AiRequestType::Suggest => {
                "You are an expert programming assistant. Provide specific, actionable suggestions for improving the given code. Focus on:
1. Code refactoring opportunities
2. Performance optimizations
3. Error handling improvements
4. Code style and readability
5. Modern language features that could be used

Provide concrete code examples where helpful."
            }
            AiRequestType::Fix => {
                "You are a debugging expert. Analyze the provided code to:
1. Identify potential bugs and errors
2. Suggest specific fixes
3. Explain why the issues occur
4. Provide corrected code examples
5. Suggest preventive measures

Be precise and provide working solutions."
            }
            AiRequestType::Optimize => {
                "You are a performance optimization expert. Analyze the code for:
1. Performance bottlenecks
2. Memory usage optimization
3. Algorithm improvements
4. Concurrency opportunities
5. Resource management

Provide specific optimization strategies with examples."
            }
            AiRequestType::Explain => {
                "You are a code educator. Explain the provided code clearly:
1. What the code does (high-level purpose)
2. How it works (step-by-step breakdown)
3. Key concepts and patterns used
4. Context and use cases
5. Related concepts the developer should know

Make explanations accessible but thorough."
            }
            AiRequestType::Meta => {
                "You are a meta-programming expert. Analyze not just the code, but also:
1. The development patterns and practices evident
2. Code organization and architecture decisions
3. Testing strategies that would be appropriate
4. Documentation needs
5. Maintenance considerations
6. Team collaboration aspects

Provide insights about the development process itself."
            }
        };

        let user_prompt = format!(
            "File: {}\n\nCode:\n```\n{}\n```\n\nContext: {}\n\nPlease analyze this code according to your role.",
            request.file_path.as_deref().unwrap_or("unknown"),
            request.content,
            self.format_context(&request.context)
        );

        (system_prompt.to_string(), user_prompt)
    }

    fn format_context(&self, context: &std::collections::HashMap<String, String>) -> String {
        if context.is_empty() {
            "No additional context provided.".to_string()
        } else {
            context
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect::<Vec<_>>()
                .join(", ")
        }
    }

    fn parse_response_to_thoughts(&self, response: &str, request: &AiRequest) -> Vec<Thought> {
        let mut thoughts = Vec::new();

        // Split response into logical sections
        let sections = self.split_response_into_sections(response);

        for (_i, section) in sections.iter().enumerate() {
            if section.trim().is_empty() {
                continue;
            }

            let thought_type = self.infer_thought_type(section, &request.request_type);
            let confidence = self.calculate_confidence(section);
            let suggestions = self.extract_suggestions(section);

            let thought = Thought {
                id: uuid::Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                thought_type,
                content: section.trim().to_string(),
                file_path: request.file_path.clone(),
                line_number: None, // TODO: Extract line numbers from analysis
                confidence,
                suggestions,
            };

            thoughts.push(thought);
        }

        // If no thoughts were generated, create a generic one
        if thoughts.is_empty() {
            thoughts.push(Thought {
                id: uuid::Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                thought_type: ThoughtType::Analyzing,
                content: response.trim().to_string(),
                file_path: request.file_path.clone(),
                line_number: None,
                confidence: 0.5,
                suggestions: Vec::new(),
            });
        }

        thoughts
    }

    fn split_response_into_sections(&self, response: &str) -> Vec<String> {
        // Split by numbered lists, bullet points, or clear paragraph breaks
        let mut sections = Vec::new();
        let mut current_section = String::new();

        for line in response.lines() {
            let trimmed = line.trim();

            // Check if this line starts a new section
            if self.is_section_start(trimmed) && !current_section.trim().is_empty() {
                sections.push(current_section.trim().to_string());
                current_section = String::new();
            }

            current_section.push_str(line);
            current_section.push('\n');
        }

        if !current_section.trim().is_empty() {
            sections.push(current_section.trim().to_string());
        }

        sections
    }

    fn is_section_start(&self, line: &str) -> bool {
        // Detect common section starters
        line.starts_with("1.") ||
        line.starts_with("2.") ||
        line.starts_with("3.") ||
        line.starts_with("4.") ||
        line.starts_with("5.") ||
        line.starts_with("- ") ||
        line.starts_with("* ") ||
        line.starts_with("## ") ||
        line.starts_with("### ") ||
        (line.len() > 20 && line.ends_with(':'))
    }

    fn infer_thought_type(&self, content: &str, request_type: &AiRequestType) -> ThoughtType {
        let content_lower = content.to_lowercase();

        // Look for keywords that indicate thought type
        if content_lower.contains("error") || content_lower.contains("bug") || content_lower.contains("issue") {
            ThoughtType::Error
        } else if content_lower.contains("warning") || content_lower.contains("caution") || content_lower.contains("careful") {
            ThoughtType::Warning
        } else if content_lower.contains("suggest") || content_lower.contains("recommend") || content_lower.contains("consider") {
            ThoughtType::Suggesting
        } else if content_lower.contains("performance") || content_lower.contains("optimization") || content_lower.contains("speed") {
            ThoughtType::Performance
        } else if content_lower.contains("security") || content_lower.contains("vulnerability") || content_lower.contains("safe") {
            ThoughtType::Security
        } else if content_lower.contains("style") || content_lower.contains("format") || content_lower.contains("convention") {
            ThoughtType::Style
        } else if content_lower.contains("architecture") || content_lower.contains("design") || content_lower.contains("pattern") {
            ThoughtType::Architecture
        } else {
            match request_type {
                AiRequestType::Analyze => ThoughtType::Analyzing,
                AiRequestType::Suggest => ThoughtType::Suggesting,
                AiRequestType::Fix => ThoughtType::Error,
                AiRequestType::Optimize => ThoughtType::Performance,
                AiRequestType::Explain => ThoughtType::Complete,
                AiRequestType::Meta => ThoughtType::Meta,
            }
        }
    }

    fn calculate_confidence(&self, content: &str) -> f32 {
        let content_lower = content.to_lowercase();
        let mut confidence: f32 = 0.5; // Base confidence

        // Increase confidence for specific, actionable content
        if content_lower.contains("should") || content_lower.contains("must") {
            confidence += 0.2;
        }

        // Decrease confidence for uncertain language
        if content_lower.contains("might") || content_lower.contains("maybe") || content_lower.contains("possibly") {
            confidence -= 0.2;
        }

        // Increase confidence for code examples
        if content.contains("```") || content.contains("```") {
            confidence += 0.1;
        }

        // Increase confidence for detailed explanations
        if content.len() > 200 {
            confidence += 0.1;
        }

        confidence.clamp(0.0_f32, 1.0_f32)
    }

    fn extract_suggestions(&self, content: &str) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        // Look for action-oriented phrases
        let lines: Vec<&str> = content.lines().collect();

        for line in lines {
            let trimmed = line.trim();

            if self.looks_like_suggestion(trimmed) {
                if let Some(suggestion) = self.parse_suggestion(trimmed) {
                    suggestions.push(suggestion);
                }
            }
        }

        suggestions
    }

    fn looks_like_suggestion(&self, line: &str) -> bool {
        let lower = line.to_lowercase();
        lower.contains("consider") ||
        lower.contains("suggest") ||
        lower.contains("recommend") ||
        lower.contains("should") ||
        lower.contains("could") ||
        lower.contains("try") ||
        lower.starts_with("replace") ||
        lower.starts_with("add") ||
        lower.starts_with("remove") ||
        lower.starts_with("refactor")
    }

    fn parse_suggestion(&self, line: &str) -> Option<Suggestion> {
        let content = line.trim();

        if content.len() < 10 {
            return None; // Too short to be meaningful
        }

        let action_type = if content.to_lowercase().contains("replace") {
            ActionType::Replace
        } else if content.to_lowercase().contains("add") || content.to_lowercase().contains("insert") {
            ActionType::Insert
        } else if content.to_lowercase().contains("remove") || content.to_lowercase().contains("delete") {
            ActionType::Delete
        } else if content.to_lowercase().contains("refactor") {
            ActionType::Refactor
        } else if content.to_lowercase().contains("optimize") {
            ActionType::Optimize
        } else {
            ActionType::Fix
        };

        let priority = if content.to_lowercase().contains("critical") || content.to_lowercase().contains("must") {
            Priority::Critical
        } else if content.to_lowercase().contains("important") || content.to_lowercase().contains("should") {
            Priority::High
        } else if content.to_lowercase().contains("consider") || content.to_lowercase().contains("could") {
            Priority::Medium
        } else {
            Priority::Low
        };

        // Extract title (first part of the suggestion)
        let title = if content.len() > 50 {
            format!("{}...", &content[..47])
        } else {
            content.to_string()
        };

        Some(Suggestion {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            description: content.to_string(),
            code_snippet: None, // TODO: Extract code snippets from response
            action_type,
            priority,
        })
    }
}

#[async_trait::async_trait]
impl AiProvider for ClaudeProvider {
    async fn analyze_code(&self, request: &AiRequest) -> Result<Vec<Thought>> {
        let (system_prompt, user_prompt) = self.create_analysis_prompt(request);

        match self.make_request(&user_prompt, Some(&system_prompt)).await {
            Ok(response) => {
                let thoughts = self.parse_response_to_thoughts(&response, request);
                tracing::debug!("Generated {} thoughts for request {}", thoughts.len(), request.id);
                Ok(thoughts)
            }
            Err(e) => {
                tracing::error!("Claude API request failed: {}", e);

                // Return an error thought instead of failing completely
                let error_thought = Thought {
                    id: uuid::Uuid::new_v4().to_string(),
                    timestamp: Utc::now(),
                    thought_type: ThoughtType::Error,
                    content: format!("AI analysis temporarily unavailable: {}", e),
                    file_path: request.file_path.clone(),
                    line_number: None,
                    confidence: 0.0,
                    suggestions: Vec::new(),
                };

                Ok(vec![error_thought])
            }
        }
    }

    async fn generate_suggestions(&self, code: &str, context: &str) -> Result<Vec<Suggestion>> {
        let prompt = format!(
            "Analyze this code and provide specific, actionable suggestions for improvement:\n\nCode:\n```\n{}\n```\n\nContext: {}\n\nProvide numbered suggestions with clear actions.",
            code, context
        );

        let system_prompt = "You are a code improvement expert. Provide specific, actionable suggestions for improving code quality, performance, and maintainability. Each suggestion should be clear and implementable.";

        let response = self.make_request(&prompt, Some(system_prompt)).await?;
        Ok(self.extract_suggestions(&response))
    }

    async fn explain_code(&self, code: &str) -> Result<String> {
        let prompt = format!(
            "Explain what this code does in clear, educational terms:\n\n```\n{}\n```\n\nProvide a comprehensive but accessible explanation.",
            code
        );

        let system_prompt = "You are a code educator. Explain code clearly and comprehensively, making it accessible to developers who want to understand how it works.";

        self.make_request(&prompt, Some(system_prompt)).await
    }

    async fn fix_code(&self, code: &str, error: &str) -> Result<String> {
        let prompt = format!(
            "Fix the following code that has this error:\n\nError: {}\n\nCode:\n```\n{}\n```\n\nProvide the corrected code with explanation.",
            error, code
        );

        let system_prompt = "You are a debugging expert. Analyze code errors and provide corrected versions with clear explanations of what was wrong and how it was fixed.";

        self.make_request(&prompt, Some(system_prompt)).await
    }
}