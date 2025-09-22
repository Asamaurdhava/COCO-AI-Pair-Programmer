use anyhow::Result;
use chrono::{DateTime, Utc};
use std::time::Duration;
use tokio::time::{sleep, Instant};

use super::{Session, SessionEvent, EventType};

pub struct SessionPlayer {
    session: Session,
    options: PlaybackOptions,
    current_event_index: usize,
    playback_start_time: Option<Instant>,
    session_start_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct PlaybackOptions {
    pub speed_multiplier: f64,
    pub skip_events: Vec<EventType>,
    pub only_events: Option<Vec<EventType>>,
    pub max_delay_ms: Option<u64>,
    pub interactive: bool,
    pub show_timing: bool,
    pub filter_file_path: Option<String>,
    pub start_from_event: Option<usize>,
    pub end_at_event: Option<usize>,
}

impl Default for PlaybackOptions {
    fn default() -> Self {
        Self {
            speed_multiplier: 1.0,
            skip_events: vec![],
            only_events: None,
            max_delay_ms: Some(5000), // Maximum 5 second delay between events
            interactive: false,
            show_timing: true,
            filter_file_path: None,
            start_from_event: None,
            end_at_event: None,
        }
    }
}

impl SessionPlayer {
    pub fn new(session: Session) -> Self {
        Self {
            session,
            options: PlaybackOptions::default(),
            current_event_index: 0,
            playback_start_time: None,
            session_start_time: None,
        }
    }

    pub fn with_options(mut self, options: PlaybackOptions) -> Self {
        self.options = options;
        self
    }

    pub fn with_speed(mut self, speed: f64) -> Self {
        self.options.speed_multiplier = speed;
        self
    }

    pub fn skip_event_types(mut self, event_types: Vec<EventType>) -> Self {
        self.options.skip_events = event_types;
        self
    }

    pub fn only_event_types(mut self, event_types: Vec<EventType>) -> Self {
        self.options.only_events = Some(event_types);
        self
    }

    pub fn interactive(mut self, interactive: bool) -> Self {
        self.options.interactive = interactive;
        self
    }

    pub fn filter_by_file(mut self, file_path: String) -> Self {
        self.options.filter_file_path = Some(file_path);
        self
    }

    pub async fn play(&mut self) -> Result<()> {
        self.print_session_info();

        if self.session.events.is_empty() {
            println!("No events to replay in this session.");
            return Ok(());
        }

        // Set starting point
        if let Some(start_index) = self.options.start_from_event {
            self.current_event_index = start_index.min(self.session.events.len());
        }

        // Filter events if needed
        let events_to_play = self.filter_events();

        if events_to_play.is_empty() {
            println!("No events match the current filters.");
            return Ok(());
        }

        println!("\n🎬 Starting playback of {} events...\n", events_to_play.len());

        if self.options.interactive {
            println!("Interactive mode: Press Enter to continue to next event, 'q' to quit");
        }

        let playback_start = Instant::now();
        let session_start = self.session.started_at;

        self.playback_start_time = Some(playback_start);
        self.session_start_time = Some(session_start);

        for (index, event) in events_to_play.iter().enumerate() {
            if let Some(end_index) = self.options.end_at_event {
                if index >= end_index {
                    break;
                }
            }

            self.play_event(event, index).await?;

            if self.options.interactive {
                if self.wait_for_user_input().await? {
                    break; // User wants to quit
                }
            }
        }

        println!("\n✅ Playback completed!");
        self.print_playback_stats();

        Ok(())
    }

    fn print_session_info(&self) {
        println!("📼 Session Replay");
        println!("================");
        println!("Session ID: {}", self.session.id);
        println!("Started: {}", self.session.started_at.format("%Y-%m-%d %H:%M:%S UTC"));

        if let Some(ended_at) = self.session.ended_at {
            println!("Ended: {}", ended_at.format("%Y-%m-%d %H:%M:%S UTC"));
            let duration = ended_at.signed_duration_since(self.session.started_at);
            println!("Duration: {}m {}s", duration.num_minutes(), duration.num_seconds() % 60);
        }

        println!("Total Events: {}", self.session.events.len());
        println!("Files Analyzed: {}", self.session.metadata.files_analyzed.len());
        println!("Working Directory: {}", self.session.metadata.working_directory);
        println!("CoCo Version: {}", self.session.metadata.coco_version);

        if self.options.speed_multiplier != 1.0 {
            println!("Playback Speed: {}x", self.options.speed_multiplier);
        }
    }

    fn filter_events(&self) -> Vec<SessionEvent> {
        self.session
            .events
            .iter()
            .filter(|event| {
                // Check skip list
                if self.options.skip_events.contains(&event.event_type) {
                    return false;
                }

                // Check only list
                if let Some(ref only_events) = self.options.only_events {
                    if !only_events.contains(&event.event_type) {
                        return false;
                    }
                }

                // Check file filter
                if let Some(ref filter_path) = self.options.filter_file_path {
                    if let Some(ref event_path) = event.context.file_path {
                        if !event_path.contains(filter_path) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect()
    }

    async fn play_event(&self, event: &SessionEvent, index: usize) -> Result<()> {
        // Calculate timing delay
        if let Some(session_start) = self.session_start_time {
            let event_offset = event.timestamp.signed_duration_since(session_start);
            let target_delay_ms = (event_offset.num_milliseconds() as f64 / self.options.speed_multiplier) as u64;

            if let Some(max_delay) = self.options.max_delay_ms {
                let actual_delay = target_delay_ms.min(max_delay);
                if actual_delay > 0 && !self.options.interactive {
                    sleep(Duration::from_millis(actual_delay)).await;
                }
            }
        }

        self.display_event(event, index);

        Ok(())
    }

    fn display_event(&self, event: &SessionEvent, index: usize) {
        let timestamp = if self.options.show_timing {
            format!("[{}] ", event.timestamp.format("%H:%M:%S%.3f"))
        } else {
            format!("[{}] ", index + 1)
        };

        let event_icon = self.get_event_icon(&event.event_type);
        let event_name = format!("{:?}", event.event_type);

        print!("{}{} {}", timestamp, event_icon, event_name);

        // Add file path if available
        if let Some(ref file_path) = event.context.file_path {
            print!(" ({})", Self::truncate_path(file_path, 50));
        }

        // Add duration if available
        if let Some(duration) = event.context.duration_ms {
            print!(" [{} ms]", duration);
        }

        println!();

        // Display event data for important events
        match event.event_type {
            EventType::FileChanged => {
                if let Some(size) = event.data.get("content_size") {
                    println!("  📄 File size: {} bytes", size);
                }
            }
            EventType::AiRequest => {
                if let Some(request_type) = event.data.get("request_type") {
                    println!("  🤖 Request type: {}", request_type.as_str().unwrap_or("unknown"));
                }
            }
            EventType::AiResponse => {
                if let Some(thoughts) = event.data.get("thoughts_count") {
                    println!("  💭 Generated {} thoughts", thoughts);
                }
                if let Some(success) = event.data.get("success") {
                    let status = if success.as_bool().unwrap_or(false) { "✅" } else { "❌" };
                    println!("  {} Request {}", status, if success.as_bool().unwrap_or(false) { "succeeded" } else { "failed" });
                }
            }
            EventType::Error => {
                if let Some(error_msg) = event.data.get("error_message") {
                    println!("  ❌ Error: {}", error_msg.as_str().unwrap_or("Unknown error"));
                }
            }
            EventType::ThoughtGenerated => {
                if let Some(thought_type) = event.data.get("thought_type") {
                    println!("  💡 Thought type: {}", thought_type.as_str().unwrap_or("unknown"));
                }
                if let Some(confidence) = event.data.get("confidence") {
                    println!("  📊 Confidence: {:.1}%", confidence.as_f64().unwrap_or(0.0) * 100.0);
                }
            }
            EventType::SuggestionAccepted | EventType::SuggestionRejected => {
                let action = if matches!(event.event_type, EventType::SuggestionAccepted) { "accepted" } else { "rejected" };
                println!("  👤 User {} suggestion", action);
            }
            _ => {}
        }

        // Add extra spacing for readability
        if !self.options.interactive {
            println!();
        }
    }

    fn get_event_icon(&self, event_type: &EventType) -> &'static str {
        match event_type {
            EventType::SessionStarted => "🚀",
            EventType::SessionEnded => "🏁",
            EventType::FileChanged => "📝",
            EventType::AiRequest => "🤖",
            EventType::AiResponse => "💭",
            EventType::UiAction => "👤",
            EventType::Error => "❌",
            EventType::ConfigChange => "⚙️",
            EventType::ThoughtGenerated => "💡",
            EventType::SuggestionAccepted => "✅",
            EventType::SuggestionRejected => "❌",
        }
    }

    async fn wait_for_user_input(&self) -> Result<bool> {
        use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

        print!("Press Enter to continue (or 'q' to quit): ");
        io::stdout().flush().await?;

        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        reader.read_line(&mut line).await?;

        Ok(line.trim().to_lowercase() == "q")
    }

    fn print_playback_stats(&self) {
        if let Some(start_time) = self.playback_start_time {
            let elapsed = start_time.elapsed();
            println!("Playback time: {:.2} seconds", elapsed.as_secs_f64());
        }

        // Calculate event type statistics
        let mut event_counts = std::collections::HashMap::new();
        for event in &self.session.events {
            *event_counts.entry(&event.event_type).or_insert(0) += 1;
        }

        println!("\nEvent Statistics:");
        for (event_type, count) in event_counts {
            println!("  {:?}: {}", event_type, count);
        }
    }

    fn truncate_path(path: &str, max_len: usize) -> String {
        if path.len() <= max_len {
            path.to_string()
        } else {
            let start_len = max_len / 2 - 2;
            let end_len = max_len - start_len - 3;
            format!("{}...{}", &path[..start_len], &path[path.len() - end_len..])
        }
    }

    // Export functionality
    pub fn export_summary(&self) -> SessionSummary {
        let mut file_changes = 0;
        let mut ai_requests = 0;
        let mut ai_responses = 0;
        let mut ui_actions = 0;
        let mut errors = 0;
        let mut total_ai_duration = 0u64;
        let mut successful_ai_requests = 0;

        for event in &self.session.events {
            match event.event_type {
                EventType::FileChanged => file_changes += 1,
                EventType::AiRequest => ai_requests += 1,
                EventType::AiResponse => {
                    ai_responses += 1;
                    if let Some(duration) = event.context.duration_ms {
                        total_ai_duration += duration;
                    }
                    if event.data.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
                        successful_ai_requests += 1;
                    }
                }
                EventType::UiAction => ui_actions += 1,
                EventType::Error => errors += 1,
                _ => {}
            }
        }

        let duration = self.session.ended_at
            .map(|end| end.signed_duration_since(self.session.started_at))
            .map(|d| d.num_milliseconds() as u64);

        SessionSummary {
            session_id: self.session.id.clone(),
            started_at: self.session.started_at,
            ended_at: self.session.ended_at,
            duration_ms: duration,
            total_events: self.session.events.len(),
            file_changes,
            ai_requests,
            ai_responses,
            ui_actions,
            errors,
            successful_ai_requests,
            ai_success_rate: if ai_requests > 0 {
                successful_ai_requests as f64 / ai_requests as f64
            } else {
                0.0
            },
            average_ai_response_time: if ai_responses > 0 {
                total_ai_duration / ai_responses as u64
            } else {
                0
            },
            unique_files: self.session.metadata.files_analyzed.len(),
            files_analyzed: self.session.metadata.files_analyzed.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub session_id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub total_events: usize,
    pub file_changes: usize,
    pub ai_requests: usize,
    pub ai_responses: usize,
    pub ui_actions: usize,
    pub errors: usize,
    pub successful_ai_requests: usize,
    pub ai_success_rate: f64,
    pub average_ai_response_time: u64,
    pub unique_files: usize,
    pub files_analyzed: Vec<String>,
}

impl SessionSummary {
    pub fn print(&self) {
        println!("Session Summary");
        println!("===============");
        println!("ID: {}", self.session_id);
        println!("Started: {}", self.started_at.format("%Y-%m-%d %H:%M:%S UTC"));

        if let Some(ended) = self.ended_at {
            println!("Ended: {}", ended.format("%Y-%m-%d %H:%M:%S UTC"));
        }

        if let Some(duration) = self.duration_ms {
            let seconds = duration / 1000;
            let minutes = seconds / 60;
            println!("Duration: {}m {}s", minutes, seconds % 60);
        }

        println!("\nActivity:");
        println!("  Total Events: {}", self.total_events);
        println!("  File Changes: {}", self.file_changes);
        println!("  AI Requests: {}", self.ai_requests);
        println!("  AI Responses: {}", self.ai_responses);
        println!("  UI Actions: {}", self.ui_actions);
        println!("  Errors: {}", self.errors);

        println!("\nAI Performance:");
        println!("  Success Rate: {:.1}%", self.ai_success_rate * 100.0);
        println!("  Average Response Time: {} ms", self.average_ai_response_time);

        println!("\nFiles:");
        println!("  Unique Files Analyzed: {}", self.unique_files);

        if self.files_analyzed.len() <= 10 {
            for file in &self.files_analyzed {
                println!("    {}", file);
            }
        } else {
            for file in &self.files_analyzed[..5] {
                println!("    {}", file);
            }
            println!("    ... and {} more files", self.files_analyzed.len() - 5);
        }
    }
}