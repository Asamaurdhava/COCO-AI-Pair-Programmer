pub mod recorder;
pub mod replay;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub events: Vec<SessionEvent>,
    pub metadata: SessionMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub coco_version: String,
    pub working_directory: String,
    pub user: Option<String>,
    pub ai_provider: String,
    pub total_duration_ms: Option<u64>,
    pub total_file_changes: usize,
    pub total_ai_requests: usize,
    pub files_analyzed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEvent {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub data: serde_json::Value,
    pub context: EventContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EventType {
    SessionStarted,
    SessionEnded,
    FileChanged,
    AiRequest,
    AiResponse,
    UiAction,
    Error,
    ConfigChange,
    ThoughtGenerated,
    SuggestionAccepted,
    SuggestionRejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventContext {
    pub file_path: Option<String>,
    pub line_number: Option<usize>,
    pub user_action: Option<String>,
    pub duration_ms: Option<u64>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl Default for EventContext {
    fn default() -> Self {
        Self {
            file_path: None,
            line_number: None,
            user_action: None,
            duration_ms: None,
            metadata: std::collections::HashMap::new(),
        }
    }
}

// Re-export main types
pub use recorder::SessionRecorder;
pub use replay::SessionPlayer;

// Helper functions
pub fn load_session(id: &str) -> Result<Session> {
    let session_path = get_session_path(id)?;
    let content = std::fs::read_to_string(&session_path)?;
    let session: Session = serde_json::from_str(&content)?;
    Ok(session)
}

pub fn list_sessions() -> Result<Vec<Session>> {
    let sessions_dir = get_sessions_directory()?;

    if !sessions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();

    for entry in std::fs::read_dir(&sessions_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    match serde_json::from_str::<Session>(&content) {
                        Ok(session) => sessions.push(session),
                        Err(e) => {
                            tracing::warn!("Failed to parse session file {}: {}", path.display(), e);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to read session file {}: {}", path.display(), e);
                }
            }
        }
    }

    // Sort by start time (newest first)
    sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));

    Ok(sessions)
}

pub async fn replay(session: Session) -> Result<()> {
    let mut player = SessionPlayer::new(session);
    player.play().await
}

pub fn delete_session(id: &str) -> Result<()> {
    let session_path = get_session_path(id)?;
    if session_path.exists() {
        std::fs::remove_file(&session_path)?;
        tracing::info!("Deleted session: {}", id);
    }
    Ok(())
}

pub fn export_session(id: &str, output_path: &str, format: ExportFormat) -> Result<()> {
    let session = load_session(id)?;

    match format {
        ExportFormat::Json => {
            let json = serde_json::to_string_pretty(&session)?;
            std::fs::write(output_path, json)?;
        }
        ExportFormat::Csv => {
            export_session_to_csv(&session, output_path)?;
        }
        ExportFormat::Html => {
            export_session_to_html(&session, output_path)?;
        }
    }

    tracing::info!("Exported session {} to {} (format: {:?})", id, output_path, format);
    Ok(())
}

#[derive(Debug, Clone)]
pub enum ExportFormat {
    Json,
    Csv,
    Html,
}

pub fn get_sessions_directory() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

    let sessions_dir = home.join(".coco").join("sessions");
    std::fs::create_dir_all(&sessions_dir)?;

    Ok(sessions_dir)
}

fn get_session_path(id: &str) -> Result<PathBuf> {
    let sessions_dir = get_sessions_directory()?;
    Ok(sessions_dir.join(format!("{}.json", id)))
}

fn export_session_to_csv(session: &Session, output_path: &str) -> Result<()> {
    let mut csv_content = String::new();

    // CSV header
    csv_content.push_str("timestamp,event_type,file_path,duration_ms,data\n");

    // CSV rows
    for event in &session.events {
        let timestamp = event.timestamp.format("%Y-%m-%d %H:%M:%S%.3f").to_string();
        let event_type = format!("{:?}", event.event_type);
        let file_path = event.context.file_path.as_deref().unwrap_or("");
        let duration = event.context.duration_ms.map(|d| d.to_string()).unwrap_or_default();
        let data = event.data.to_string().replace('"', "'");

        csv_content.push_str(&format!(
            "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
            timestamp, event_type, file_path, duration, data
        ));
    }

    std::fs::write(output_path, csv_content)?;
    Ok(())
}

fn export_session_to_html(session: &Session, output_path: &str) -> Result<()> {
    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
    html.push_str("<title>CoCo Session Report</title>\n");
    html.push_str("<style>\n");
    html.push_str(SESSION_REPORT_CSS);
    html.push_str("</style>\n");
    html.push_str("</head>\n<body>\n");

    // Header
    html.push_str(&format!("<h1>CoCo Session Report</h1>\n"));
    html.push_str(&format!("<h2>Session ID: {}</h2>\n", session.id));
    html.push_str(&format!(
        "<p><strong>Started:</strong> {}</p>\n",
        session.started_at.format("%Y-%m-%d %H:%M:%S UTC")
    ));

    if let Some(ended_at) = session.ended_at {
        html.push_str(&format!(
            "<p><strong>Ended:</strong> {}</p>\n",
            ended_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));
    }

    // Metadata
    html.push_str("<h3>Session Metadata</h3>\n");
    html.push_str("<ul>\n");
    html.push_str(&format!("<li><strong>CoCo Version:</strong> {}</li>\n", session.metadata.coco_version));
    html.push_str(&format!("<li><strong>Working Directory:</strong> {}</li>\n", session.metadata.working_directory));
    html.push_str(&format!("<li><strong>AI Provider:</strong> {}</li>\n", session.metadata.ai_provider));
    html.push_str(&format!("<li><strong>Total Events:</strong> {}</li>\n", session.events.len()));
    html.push_str(&format!("<li><strong>File Changes:</strong> {}</li>\n", session.metadata.total_file_changes));
    html.push_str(&format!("<li><strong>AI Requests:</strong> {}</li>\n", session.metadata.total_ai_requests));
    html.push_str("</ul>\n");

    // Events timeline
    html.push_str("<h3>Events Timeline</h3>\n");
    html.push_str("<div class=\"timeline\">\n");

    for event in &session.events {
        let event_class = match event.event_type {
            EventType::FileChanged => "file-event",
            EventType::AiRequest | EventType::AiResponse => "ai-event",
            EventType::UiAction => "ui-event",
            EventType::Error => "error-event",
            _ => "other-event",
        };

        html.push_str(&format!(
            "<div class=\"event {}\">\n",
            event_class
        ));

        html.push_str(&format!(
            "<div class=\"event-time\">{}</div>\n",
            event.timestamp.format("%H:%M:%S%.3f")
        ));

        html.push_str(&format!(
            "<div class=\"event-type\">{:?}</div>\n",
            event.event_type
        ));

        if let Some(ref file_path) = event.context.file_path {
            html.push_str(&format!(
                "<div class=\"event-file\">{}</div>\n",
                file_path
            ));
        }

        html.push_str(&format!(
            "<div class=\"event-data\">{}</div>\n",
            serde_json::to_string_pretty(&event.data).unwrap_or_default()
        ));

        html.push_str("</div>\n");
    }

    html.push_str("</div>\n");
    html.push_str("</body>\n</html>\n");

    std::fs::write(output_path, html)?;
    Ok(())
}

// CSS content for HTML reports
const SESSION_REPORT_CSS: &str = r#"
body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    margin: 40px;
    background-color: #f5f5f5;
    color: #333;
}

h1, h2, h3 {
    color: #2c3e50;
}

.timeline {
    margin-top: 20px;
}

.event {
    background: white;
    border-left: 4px solid #3498db;
    margin-bottom: 10px;
    padding: 15px;
    border-radius: 4px;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
}

.event.file-event {
    border-left-color: #2ecc71;
}

.event.ai-event {
    border-left-color: #9b59b6;
}

.event.ui-event {
    border-left-color: #f39c12;
}

.event.error-event {
    border-left-color: #e74c3c;
}

.event-time {
    font-size: 12px;
    color: #7f8c8d;
    margin-bottom: 5px;
}

.event-type {
    font-weight: bold;
    color: #2c3e50;
    margin-bottom: 8px;
}

.event-file {
    font-family: monospace;
    font-size: 12px;
    color: #27ae60;
    margin-bottom: 8px;
}

.event-data {
    font-family: monospace;
    font-size: 11px;
    background: #ecf0f1;
    padding: 8px;
    border-radius: 3px;
    max-height: 200px;
    overflow-y: auto;
    white-space: pre-wrap;
}

ul {
    background: white;
    padding: 20px;
    border-radius: 4px;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
}

li {
    margin-bottom: 8px;
}
"#;