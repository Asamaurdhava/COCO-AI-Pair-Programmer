pub mod claude;
pub mod analyzer;

use anyhow::Result;
use std::sync::Arc;

use crate::app::{AiRequest, Thought, Suggestion};

#[async_trait::async_trait]
pub trait AiProvider: Send + Sync {
    async fn analyze_code(&self, request: &AiRequest) -> Result<Vec<Thought>>;
    async fn generate_suggestions(&self, code: &str, context: &str) -> Result<Vec<Suggestion>>;
    async fn explain_code(&self, code: &str) -> Result<String>;
    async fn fix_code(&self, code: &str, error: &str) -> Result<String>;
}

pub struct ClaudeClient {
    inner: Arc<claude::ClaudeProvider>,
}

impl ClaudeClient {
    pub fn new(api_key: String) -> Result<Self> {
        let provider = claude::ClaudeProvider::new(api_key)?;
        Ok(Self {
            inner: Arc::new(provider),
        })
    }

    pub async fn process_request(&self, request: &AiRequest) -> Result<Vec<Thought>> {
        self.inner.analyze_code(request).await
    }
}

