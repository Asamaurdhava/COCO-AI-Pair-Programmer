pub mod renderer;
pub mod widgets;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;
use tokio::time::{Duration, Instant};

use crate::app::{App, UiEvent, UiEventType};

pub struct UI {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    app: App,
    last_render: Instant,
    render_interval: Duration,
}

impl UI {
    pub async fn new(app: App) -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            terminal,
            app,
            last_render: Instant::now(),
            render_interval: Duration::from_millis(50), // 20 FPS
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        tracing::info!("Starting UI loop");

        loop {
            // Handle events
            if event::poll(Duration::from_millis(10))? {
                match event::read()? {
                    Event::Key(key) => {
                        if self.handle_key_event(key).await? {
                            break; // Quit requested
                        }
                    }
                    Event::Resize(_, _) => {
                        let ui_event = UiEvent {
                            event_type: UiEventType::Resize,
                            data: None,
                            timestamp: chrono::Utc::now(),
                        };
                        // Non-blocking send to prevent UI deadlock
                        if let Err(_) = self.app.ui_tx.try_send(ui_event) {
                            tracing::warn!("UI channel full, dropping resize event");
                        }
                    }
                    _ => {}
                }
            }

            // Render at controlled intervals
            if self.last_render.elapsed() >= self.render_interval {
                self.render().await?;
                self.last_render = Instant::now();
            }

            // Check if app is still running
            if !self.app.is_running().await {
                break;
            }

            // Small delay to prevent busy waiting
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        self.cleanup()?;
        Ok(())
    }

    async fn handle_key_event(&mut self, key: KeyEvent) -> Result<bool> {
        let ui_event = UiEvent {
            event_type: UiEventType::KeyPressed(key.code),
            data: None,
            timestamp: chrono::Utc::now(),
        };

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                let quit_event = UiEvent {
                    event_type: UiEventType::Quit,
                    data: None,
                    timestamp: chrono::Utc::now(),
                };
                if let Err(_) = self.app.ui_tx.try_send(quit_event) {
                    tracing::warn!("UI channel full, dropping quit event");
                }
                return Ok(true);
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let quit_event = UiEvent {
                    event_type: UiEventType::Quit,
                    data: None,
                    timestamp: chrono::Utc::now(),
                };
                if let Err(_) = self.app.ui_tx.try_send(quit_event) {
                    tracing::warn!("UI channel full, dropping quit event");
                }
                return Ok(true);
            }
            KeyCode::Char('v') => {
                let toggle_event = UiEvent {
                    event_type: UiEventType::ToggleMode,
                    data: None,
                    timestamp: chrono::Utc::now(),
                };
                if let Err(_) = self.app.ui_tx.try_send(toggle_event) {
                    tracing::warn!("UI channel full, dropping toggle event");
                }
            }
            KeyCode::Char('c') => {
                let clear_event = UiEvent {
                    event_type: UiEventType::ClearThoughts,
                    data: None,
                    timestamp: chrono::Utc::now(),
                };
                if let Err(_) = self.app.ui_tx.try_send(clear_event) {
                    tracing::warn!("UI channel full, dropping clear event");
                }
            }
            KeyCode::Char('f') => {
                let select_event = UiEvent {
                    event_type: UiEventType::SelectFile,
                    data: None,
                    timestamp: chrono::Utc::now(),
                };
                if let Err(_) = self.app.ui_tx.try_send(select_event) {
                    tracing::warn!("UI channel full, dropping select event");
                }
            }
            KeyCode::Char('y') => {
                let accept_event = UiEvent {
                    event_type: UiEventType::AcceptSuggestion,
                    data: None,
                    timestamp: chrono::Utc::now(),
                };
                if let Err(_) = self.app.ui_tx.try_send(accept_event) {
                    tracing::warn!("UI channel full, dropping accept event");
                }
            }
            KeyCode::Char('n') => {
                let reject_event = UiEvent {
                    event_type: UiEventType::RejectSuggestion,
                    data: None,
                    timestamp: chrono::Utc::now(),
                };
                if let Err(_) = self.app.ui_tx.try_send(reject_event) {
                    tracing::warn!("UI channel full, dropping reject event");
                }
            }
            KeyCode::Char('h') => {
                let help_event = UiEvent {
                    event_type: UiEventType::Help,
                    data: None,
                    timestamp: chrono::Utc::now(),
                };
                if let Err(_) = self.app.ui_tx.try_send(help_event) {
                    tracing::warn!("UI channel full, dropping help event");
                }
            }
            KeyCode::Char('r') => {
                let refresh_event = UiEvent {
                    event_type: UiEventType::Refresh,
                    data: None,
                    timestamp: chrono::Utc::now(),
                };
                if let Err(_) = self.app.ui_tx.try_send(refresh_event) {
                    tracing::warn!("UI channel full, dropping refresh event");
                }
            }
            _ => {}
        }

        // Send the key event for recording (non-blocking)
        if let Err(_) = self.app.ui_tx.try_send(ui_event) {
            tracing::warn!("UI channel full, dropping key event for recording");
        }
        Ok(false)
    }

    async fn render(&mut self) -> Result<()> {
        let app_data = self.gather_app_data().await;

        self.terminal.draw(|frame| {
            renderer::render_frame(frame, &app_data);
        })?;

        Ok(())
    }

    async fn gather_app_data(&self) -> renderer::AppData {
        renderer::AppData {
            current_file: self.app.get_current_file().await,
            current_code: self.app.get_current_code().await,
            thoughts: self.app.get_thoughts().await,
            mode: self.app.get_mode().await,
            is_recording: *self.app.is_recording.lock().await,
            config: self.app.config.clone(),
        }
    }

    fn cleanup(&mut self) -> Result<()> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

impl Drop for UI {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}