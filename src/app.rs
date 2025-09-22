use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use crate::config::Config;
use crate::session::{SessionRecorder, EventType};

#[derive(Clone)]
pub struct App {
    pub current_file: Arc<Mutex<Option<String>>>,
    pub current_code: Arc<Mutex<String>>,
    pub ai_thoughts: Arc<Mutex<Vec<Thought>>>,
    pub file_tx: mpsc::Sender<FileEvent>,
    pub file_rx: Arc<Mutex<mpsc::Receiver<FileEvent>>>,
    pub ai_tx: mpsc::Sender<AiRequest>,
    pub ai_rx: Arc<Mutex<mpsc::Receiver<AiRequest>>>,
    pub ui_tx: mpsc::Sender<UiEvent>,
    pub ui_rx: Arc<Mutex<mpsc::Receiver<UiEvent>>>,
    pub config: Arc<Config>,
    pub is_recording: Arc<Mutex<bool>>,
    pub mode: Arc<Mutex<ViewMode>>,
    pub session_recorder: Arc<Mutex<Option<SessionRecorder>>>,
    pub running: Arc<Mutex<bool>>,
    pub file_cache: Arc<Mutex<HashMap<String, String>>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ViewMode {
    Full,
    Minimal,
    SideBySide,
    ThoughtsOnly,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Thought {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub thought_type: ThoughtType,
    pub content: String,
    pub file_path: Option<String>,
    pub line_number: Option<usize>,
    pub confidence: f32,
    pub suggestions: Vec<Suggestion>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ThoughtType {
    Analyzing,
    Suggesting,
    Warning,
    Error,
    Complete,
    Meta,
    Performance,
    Security,
    Style,
    Architecture,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Suggestion {
    pub id: String,
    pub title: String,
    pub description: String,
    pub code_snippet: Option<String>,
    pub action_type: ActionType,
    pub priority: Priority,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ActionType {
    Replace,
    Insert,
    Delete,
    Refactor,
    Optimize,
    Fix,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Debug)]
pub struct FileEvent {
    pub path: std::path::PathBuf,
    pub content: String,
    pub event_type: notify::EventKind,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct AiRequest {
    pub id: String,
    pub request_type: AiRequestType,
    pub content: String,
    pub file_path: Option<String>,
    pub context: HashMap<String, String>,
    pub priority: Priority,
}

#[derive(Clone, Debug)]
pub enum AiRequestType {
    Analyze,
    Suggest,
    Fix,
    Optimize,
    Explain,
    Meta,
}

#[derive(Clone, Debug)]
pub struct UiEvent {
    pub event_type: UiEventType,
    pub data: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub enum UiEventType {
    KeyPressed(crossterm::event::KeyCode),
    Refresh,
    Resize,
    SelectFile,
    ToggleMode,
    AcceptSuggestion,
    RejectSuggestion,
    ClearThoughts,
    Help,
    Quit,
}

impl App {
    pub async fn new() -> Result<Self> {
        let config = Arc::new(Config::load().await?);

        let (file_tx, file_rx) = mpsc::channel(5);
        let (ai_tx, ai_rx) = mpsc::channel(5);
        let (ui_tx, ui_rx) = mpsc::channel(10);

        Ok(Self {
            current_file: Arc::new(Mutex::new(None)),
            current_code: Arc::new(Mutex::new(String::new())),
            ai_thoughts: Arc::new(Mutex::new(Vec::new())),
            file_tx,
            file_rx: Arc::new(Mutex::new(file_rx)),
            ai_tx,
            ai_rx: Arc::new(Mutex::new(ai_rx)),
            ui_tx,
            ui_rx: Arc::new(Mutex::new(ui_rx)),
            config,
            is_recording: Arc::new(Mutex::new(false)),
            mode: Arc::new(Mutex::new(ViewMode::SideBySide)),
            session_recorder: Arc::new(Mutex::new(None)),
            running: Arc::new(Mutex::new(true)),
            file_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn new_with_recording() -> Result<Self> {
        let app = Self::new().await?;

        let recorder = SessionRecorder::new()?;
        *app.session_recorder.lock().await = Some(recorder);
        *app.is_recording.lock().await = true;

        Ok(app)
    }

    pub async fn run(&mut self) -> Result<()> {
        tracing::info!("Starting CoCo application loop");

        let app_clone = self.clone();

        // Start file event handler
        let file_handler = tokio::spawn(Self::handle_file_events(app_clone.clone()));

        // Start AI request handler
        let ai_handler = tokio::spawn(Self::handle_ai_requests(app_clone.clone()));

        // Start UI event handler
        let ui_handler = tokio::spawn(Self::handle_ui_events(app_clone.clone()));

        // Start file watcher
        let mut monitor = crate::watcher::FileMonitor::new(self.file_tx.clone()).await?;
        monitor.watch(std::path::Path::new(".")).await?;
        let watcher_task = tokio::spawn(async move {
            monitor.run().await
        });

        // Start UI
        let mut ui = crate::ui::UI::new(self.clone()).await?;
        let ui_task = tokio::spawn(async move {
            ui.run().await
        });

        // Wait for any task to complete (usually UI task when user quits)
        tokio::select! {
            _ = file_handler => tracing::info!("File handler completed"),
            _ = ai_handler => tracing::info!("AI handler completed"),
            _ = ui_handler => tracing::info!("UI handler completed"),
            _ = watcher_task => tracing::info!("Watcher task completed"),
            result = ui_task => {
                match result {
                    Ok(Ok(())) => tracing::info!("UI completed successfully"),
                    Ok(Err(e)) => tracing::error!("UI error: {}", e),
                    Err(e) => tracing::error!("UI task panicked: {}", e),
                }
            }
        }

        *self.running.lock().await = false;

        // Save session if recording
        if let Some(recorder) = self.session_recorder.lock().await.as_mut() {
            recorder.save()?;
        }

        Ok(())
    }

    async fn handle_file_events(app: App) -> Result<()> {
        let mut rx = app.file_rx.lock().await;

        while let Some(event) = rx.recv().await {
            tracing::debug!("Handling file event: {:?}", event.path);

            // Update current file and code
            let path_str = event.path.to_string_lossy().to_string();
            *app.current_file.lock().await = Some(path_str.clone());
            *app.current_code.lock().await = event.content.clone();

            // Cache the file content with size limit
            let mut cache = app.file_cache.lock().await;
            cache.insert(path_str.clone(), event.content.clone());
            // Keep cache size limited to prevent memory growth
            if cache.len() > 3 {
                cache.clear(); // Just clear everything
            }

            // Record event if recording
            if *app.is_recording.lock().await {
                if let Some(recorder) = app.session_recorder.lock().await.as_mut() {
                    recorder.record_event(EventType::FileChanged, serde_json::json!({
                        "path": path_str,
                        "size": event.content.len(),
                        "timestamp": event.timestamp
                    }));
                }
            }

            // Trigger AI analysis only for reasonable file sizes
            if event.content.len() < 5_000 { // Skip analysis for files > 5KB
                let ai_request = AiRequest {
                    id: uuid::Uuid::new_v4().to_string(),
                    request_type: AiRequestType::Analyze,
                    content: event.content,
                    file_path: Some(path_str),
                    context: HashMap::new(),
                    priority: Priority::Medium,
                };

                if let Err(e) = app.ai_tx.send(ai_request).await {
                    tracing::error!("Failed to send AI request: {}", e);
                }
            } else {
                tracing::warn!("Skipping AI analysis for large file: {} bytes", event.content.len());
            }


            if !*app.running.lock().await {
                break;
            }
        }

        Ok(())
    }

    async fn handle_ai_requests(app: App) -> Result<()> {
        let mut rx = app.ai_rx.lock().await;
        let ai_client = crate::ai::ClaudeClient::new(
            app.config.anthropic_api_key.clone()
                .ok_or_else(|| anyhow::anyhow!("Anthropic API key not configured"))?
        )?;

        while let Some(request) = rx.recv().await {
            tracing::debug!("Processing AI request: {}", request.id);

            match ai_client.process_request(&request).await {
                Ok(thoughts) => {
                    let mut ai_thoughts = app.ai_thoughts.lock().await;
                    ai_thoughts.extend(thoughts);

                    // Keep only last 5 thoughts to prevent memory growth
                    if ai_thoughts.len() > 5 {
                        let drain_count = ai_thoughts.len() - 5;
                        ai_thoughts.drain(0..drain_count);
                    }

                    // Record AI response if recording
                    if *app.is_recording.lock().await {
                        if let Some(recorder) = app.session_recorder.lock().await.as_mut() {
                            recorder.record_event(EventType::AiResponse, serde_json::json!({
                                "request_id": request.id,
                                "thoughts_count": ai_thoughts.len(),
                                "timestamp": Utc::now()
                            }));
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("AI request failed: {}", e);

                    // Add error thought
                    let error_thought = Thought {
                        id: uuid::Uuid::new_v4().to_string(),
                        timestamp: Utc::now(),
                        thought_type: ThoughtType::Error,
                        content: format!("AI analysis failed: {}", e),
                        file_path: request.file_path,
                        line_number: None,
                        confidence: 0.0,
                        suggestions: vec![],
                    };

                    app.ai_thoughts.lock().await.push(error_thought);
                }
            }

            if !*app.running.lock().await {
                break;
            }
        }

        Ok(())
    }

    async fn handle_ui_events(app: App) -> Result<()> {
        let mut rx = app.ui_rx.lock().await;

        while let Some(event) = rx.recv().await {
            tracing::debug!("Handling UI event: {:?}", event.event_type);

            match event.event_type {
                UiEventType::ToggleMode => {
                    let mut mode = app.mode.lock().await;
                    *mode = match *mode {
                        ViewMode::SideBySide => ViewMode::ThoughtsOnly,
                        ViewMode::ThoughtsOnly => ViewMode::Full,
                        ViewMode::Full => ViewMode::Minimal,
                        ViewMode::Minimal => ViewMode::SideBySide,
                    };
                    tracing::info!("View mode changed to: {:?}", *mode);
                }
                UiEventType::ClearThoughts => {
                    app.ai_thoughts.lock().await.clear();
                    tracing::info!("Cleared all AI thoughts");
                }
                UiEventType::AcceptSuggestion => {
                    // TODO: Implement suggestion acceptance
                    tracing::info!("Suggestion accepted");
                }
                UiEventType::RejectSuggestion => {
                    // TODO: Implement suggestion rejection
                    tracing::info!("Suggestion rejected");
                }
                UiEventType::Quit => {
                    *app.running.lock().await = false;
                    tracing::info!("Application quit requested");
                    break;
                }
                _ => {}
            }

            // Record UI event if recording
            if *app.is_recording.lock().await {
                if let Some(recorder) = app.session_recorder.lock().await.as_mut() {
                    recorder.record_event(EventType::UiAction, serde_json::json!({
                        "event_type": format!("{:?}", event.event_type),
                        "timestamp": event.timestamp
                    }));
                }
            }

            if !*app.running.lock().await {
                break;
            }
        }

        Ok(())
    }

    pub async fn add_thought(&self, thought: Thought) {
        self.ai_thoughts.lock().await.push(thought);
    }

    pub async fn get_current_file(&self) -> Option<String> {
        self.current_file.lock().await.clone()
    }

    pub async fn get_current_code(&self) -> String {
        self.current_code.lock().await.clone()
    }

    pub async fn get_thoughts(&self) -> Vec<Thought> {
        self.ai_thoughts.lock().await.clone()
    }

    pub async fn get_mode(&self) -> ViewMode {
        self.mode.lock().await.clone()
    }

    pub async fn is_running(&self) -> bool {
        *self.running.lock().await
    }
}