use anyhow::Result;
use chrono::Utc;
use serde_json::json;
use std::path::PathBuf;
use tokio::fs;

use super::{Session, SessionEvent, SessionMetadata, EventType, EventContext};

pub struct SessionRecorder {
    session: Session,
    file_path: PathBuf,
    auto_save_interval: usize,
    events_since_save: usize,
    max_events: usize,
}

impl SessionRecorder {
    pub fn new() -> Result<Self> {
        let id = uuid::Uuid::new_v4().to_string();
        let started_at = Utc::now();

        // Get current working directory
        let working_directory = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        // Get current user
        let user = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .ok();

        let metadata = SessionMetadata {
            coco_version: env!("CARGO_PKG_VERSION").to_string(),
            working_directory,
            user,
            ai_provider: "anthropic".to_string(), // TODO: Get from config
            total_duration_ms: None,
            total_file_changes: 0,
            total_ai_requests: 0,
            files_analyzed: Vec::new(),
        };

        let session = Session {
            id: id.clone(),
            started_at,
            ended_at: None,
            events: Vec::new(),
            metadata,
        };

        // Create session file path
        let sessions_dir = super::get_sessions_directory()?;
        let file_path = sessions_dir.join(format!("{}.json", id));

        let mut recorder = Self {
            session,
            file_path,
            auto_save_interval: 10, // Save every 10 events
            events_since_save: 0,
            max_events: 10000, // Limit session size
        };

        // Record session start event
        recorder.record_event_internal(
            EventType::SessionStarted,
            json!({
                "session_id": id,
                "started_at": started_at
            }),
            EventContext::default(),
        );

        // Initial save
        recorder.save()?;

        tracing::info!("Started recording session: {}", id);
        Ok(recorder)
    }

    pub fn record_event(&mut self, event_type: EventType, data: serde_json::Value) {
        self.record_event_with_context(event_type, data, EventContext::default());
    }

    pub fn record_event_with_context(
        &mut self,
        event_type: EventType,
        data: serde_json::Value,
        context: EventContext,
    ) {
        self.record_event_internal(event_type, data, context);

        // Auto-save periodically
        self.events_since_save += 1;
        if self.events_since_save >= self.auto_save_interval {
            if let Err(e) = self.save() {
                tracing::error!("Failed to auto-save session: {}", e);
            }
        }
    }

    fn record_event_internal(
        &mut self,
        event_type: EventType,
        data: serde_json::Value,
        context: EventContext,
    ) {
        // Check if we've hit the max events limit
        if self.session.events.len() >= self.max_events {
            tracing::warn!("Session has reached maximum events limit ({}), rotating events", self.max_events);

            // Keep only the last 80% of events
            let keep_count = (self.max_events as f32 * 0.8) as usize;
            self.session.events.drain(0..self.session.events.len() - keep_count);
        }

        let event = SessionEvent {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type: event_type.clone(),
            data,
            context,
        };

        self.session.events.push(event);

        // Update metadata counters
        match event_type {
            EventType::FileChanged => {
                self.session.metadata.total_file_changes += 1;
            }
            EventType::AiRequest => {
                self.session.metadata.total_ai_requests += 1;
            }
            _ => {}
        }

        tracing::debug!("Recorded event: {:?}", event_type);
    }

    pub fn record_file_change(&mut self, file_path: &str, content_size: usize) {
        let mut context = EventContext::default();
        context.file_path = Some(file_path.to_string());

        // Track unique files analyzed
        if !self.session.metadata.files_analyzed.contains(&file_path.to_string()) {
            self.session.metadata.files_analyzed.push(file_path.to_string());
        }

        self.record_event_with_context(
            EventType::FileChanged,
            json!({
                "file_path": file_path,
                "content_size": content_size,
                "unique_files_count": self.session.metadata.files_analyzed.len()
            }),
            context,
        );
    }

    pub fn record_ai_request(&mut self, request_id: &str, request_type: &str, file_path: Option<&str>) {
        let mut context = EventContext::default();
        context.file_path = file_path.map(|s| s.to_string());

        self.record_event_with_context(
            EventType::AiRequest,
            json!({
                "request_id": request_id,
                "request_type": request_type,
                "file_path": file_path
            }),
            context,
        );
    }

    pub fn record_ai_response(
        &mut self,
        request_id: &str,
        thoughts_count: usize,
        duration_ms: u64,
        success: bool,
    ) {
        let mut context = EventContext::default();
        context.duration_ms = Some(duration_ms);

        self.record_event_with_context(
            EventType::AiResponse,
            json!({
                "request_id": request_id,
                "thoughts_count": thoughts_count,
                "duration_ms": duration_ms,
                "success": success
            }),
            context,
        );
    }

    pub fn record_ui_action(&mut self, action: &str, data: Option<serde_json::Value>) {
        let mut context = EventContext::default();
        context.user_action = Some(action.to_string());

        self.record_event_with_context(
            EventType::UiAction,
            data.unwrap_or_else(|| json!({ "action": action })),
            context,
        );
    }

    pub fn record_error(&mut self, error_message: &str, file_path: Option<&str>) {
        let mut context = EventContext::default();
        context.file_path = file_path.map(|s| s.to_string());

        self.record_event_with_context(
            EventType::Error,
            json!({
                "error_message": error_message,
                "file_path": file_path
            }),
            context,
        );
    }

    pub fn record_thought_generated(
        &mut self,
        thought_id: &str,
        thought_type: &str,
        confidence: f32,
        file_path: Option<&str>,
    ) {
        let mut context = EventContext::default();
        context.file_path = file_path.map(|s| s.to_string());

        self.record_event_with_context(
            EventType::ThoughtGenerated,
            json!({
                "thought_id": thought_id,
                "thought_type": thought_type,
                "confidence": confidence,
                "file_path": file_path
            }),
            context,
        );
    }

    pub fn record_suggestion_action(
        &mut self,
        suggestion_id: &str,
        action: &str, // "accepted" or "rejected"
        file_path: Option<&str>,
    ) {
        let mut context = EventContext::default();
        context.file_path = file_path.map(|s| s.to_string());
        context.user_action = Some(action.to_string());

        let event_type = match action {
            "accepted" => EventType::SuggestionAccepted,
            "rejected" => EventType::SuggestionRejected,
            _ => EventType::UiAction,
        };

        self.record_event_with_context(
            event_type,
            json!({
                "suggestion_id": suggestion_id,
                "action": action,
                "file_path": file_path
            }),
            context,
        );
    }

    pub fn save(&mut self) -> Result<()> {
        // Update session duration
        if let Some(first_event) = self.session.events.first() {
            let duration = Utc::now()
                .signed_duration_since(first_event.timestamp)
                .num_milliseconds();

            if duration > 0 {
                self.session.metadata.total_duration_ms = Some(duration as u64);
            }
        }

        let json = serde_json::to_string_pretty(&self.session)?;
        std::fs::write(&self.file_path, json)?;

        self.events_since_save = 0;
        tracing::debug!("Saved session to: {}", self.file_path.display());
        Ok(())
    }

    pub async fn save_async(&mut self) -> Result<()> {
        // Update session duration
        if let Some(first_event) = self.session.events.first() {
            let duration = Utc::now()
                .signed_duration_since(first_event.timestamp)
                .num_milliseconds();

            if duration > 0 {
                self.session.metadata.total_duration_ms = Some(duration as u64);
            }
        }

        let json = serde_json::to_string_pretty(&self.session)?;
        fs::write(&self.file_path, json).await?;

        self.events_since_save = 0;
        tracing::debug!("Saved session to: {}", self.file_path.display());
        Ok(())
    }

    pub fn end_session(&mut self) -> Result<()> {
        let ended_at = Utc::now();
        self.session.ended_at = Some(ended_at);

        // Record session end event
        self.record_event_internal(
            EventType::SessionEnded,
            json!({
                "session_id": self.session.id,
                "ended_at": ended_at,
                "total_events": self.session.events.len(),
                "duration_ms": self.session.metadata.total_duration_ms
            }),
            EventContext::default(),
        );

        // Final save
        self.save()?;

        tracing::info!(
            "Ended recording session: {} (duration: {:?}ms, events: {})",
            self.session.id,
            self.session.metadata.total_duration_ms,
            self.session.events.len()
        );

        Ok(())
    }

    pub fn get_session_id(&self) -> &str {
        &self.session.id
    }

    pub fn get_events_count(&self) -> usize {
        self.session.events.len()
    }

    pub fn get_session_duration(&self) -> Option<u64> {
        self.session.metadata.total_duration_ms
    }

    pub fn set_auto_save_interval(&mut self, interval: usize) {
        self.auto_save_interval = interval;
    }

    pub fn set_max_events(&mut self, max: usize) {
        self.max_events = max;
    }

    pub fn get_session_stats(&self) -> SessionStats {
        let mut stats = SessionStats::default();

        for event in &self.session.events {
            match event.event_type {
                EventType::FileChanged => stats.file_changes += 1,
                EventType::AiRequest => stats.ai_requests += 1,
                EventType::AiResponse => stats.ai_responses += 1,
                EventType::UiAction => stats.ui_actions += 1,
                EventType::Error => stats.errors += 1,
                EventType::ThoughtGenerated => stats.thoughts_generated += 1,
                EventType::SuggestionAccepted => stats.suggestions_accepted += 1,
                EventType::SuggestionRejected => stats.suggestions_rejected += 1,
                _ => stats.other_events += 1,
            }
        }

        stats.unique_files = self.session.metadata.files_analyzed.len();
        stats.duration_ms = self.session.metadata.total_duration_ms;

        stats
    }

    // Compression for large sessions
    pub fn compress_old_events(&mut self, keep_recent_count: usize) {
        if self.session.events.len() <= keep_recent_count {
            return;
        }

        tracing::info!(
            "Compressing session events, keeping {} most recent events",
            keep_recent_count
        );

        // Keep only the most recent events
        let start_index = self.session.events.len() - keep_recent_count;
        self.session.events.drain(0..start_index);

        // Add a marker event to indicate compression
        let compression_event = SessionEvent {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type: EventType::ConfigChange,
            data: json!({
                "type": "compression",
                "events_removed": start_index,
                "events_remaining": keep_recent_count
            }),
            context: EventContext::default(),
        };

        self.session.events.insert(0, compression_event);
    }
}

#[derive(Debug, Default)]
pub struct SessionStats {
    pub file_changes: usize,
    pub ai_requests: usize,
    pub ai_responses: usize,
    pub ui_actions: usize,
    pub errors: usize,
    pub thoughts_generated: usize,
    pub suggestions_accepted: usize,
    pub suggestions_rejected: usize,
    pub other_events: usize,
    pub unique_files: usize,
    pub duration_ms: Option<u64>,
}

impl Drop for SessionRecorder {
    fn drop(&mut self) {
        if self.session.ended_at.is_none() {
            if let Err(e) = self.end_session() {
                tracing::error!("Failed to properly end session in drop: {}", e);
            }
        }
    }
}