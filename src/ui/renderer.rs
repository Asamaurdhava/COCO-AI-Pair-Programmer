use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{
        Block, Borders, Clear, Paragraph, Wrap,
    },
    Frame,
};
use std::sync::Arc;

use crate::app::{Thought, ThoughtType, ViewMode};
use crate::config::Config;
use super::widgets;

pub struct AppData {
    pub current_file: Option<String>,
    pub current_code: String,
    pub thoughts: Vec<Thought>,
    pub mode: ViewMode,
    pub is_recording: bool,
    pub config: Arc<Config>,
}

pub fn render_frame(frame: &mut Frame, app_data: &AppData) {
    let size = frame.size();

    match app_data.mode {
        ViewMode::SideBySide => render_side_by_side(frame, app_data, size),
        ViewMode::Full => render_full_view(frame, app_data, size),
        ViewMode::Minimal => render_minimal_view(frame, app_data, size),
        ViewMode::ThoughtsOnly => render_thoughts_only(frame, app_data, size),
    }

    // Render status bar at the bottom
    render_status_bar(frame, app_data, size);

    // Render help overlay if needed
    // This would be triggered by a help state in the app
}

fn render_side_by_side(frame: &mut Frame, app_data: &AppData, area: Rect) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(area);

    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_layout[0]);

    // Left panel: Code
    render_code_panel(frame, app_data, content_layout[0]);

    // Right panel: AI Thoughts
    render_thoughts_panel(frame, app_data, content_layout[1]);
}

fn render_full_view(frame: &mut Frame, app_data: &AppData, area: Rect) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(37),
            Constraint::Length(3),
        ])
        .split(area);

    // Top: Code
    render_code_panel(frame, app_data, main_layout[0]);

    // Bottom: AI Thoughts
    render_thoughts_panel(frame, app_data, main_layout[1]);
}

fn render_minimal_view(frame: &mut Frame, app_data: &AppData, area: Rect) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(area);

    // Show only current file info and latest thought
    render_minimal_info(frame, app_data, main_layout[0]);
}

fn render_thoughts_only(frame: &mut Frame, app_data: &AppData, area: Rect) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(area);

    // Full area for thoughts
    render_thoughts_panel(frame, app_data, main_layout[0]);
}

fn render_code_panel(frame: &mut Frame, app_data: &AppData, area: Rect) {
    let title = if let Some(ref file) = app_data.current_file {
        format!(" {} ", file)
    } else {
        " No file selected ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue))
        .style(Style::default().bg(Color::Black));

    if app_data.current_code.is_empty() {
        let placeholder = Paragraph::new("No code to display. Open a supported file to start analysis.")
            .block(block)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        frame.render_widget(placeholder, area);
    } else {
        let code_widget = widgets::CodeWidget::new(&app_data.current_code)
            .block(block)
            .style(Style::default().fg(Color::White));

        frame.render_widget(code_widget, area);
    }
}

fn render_thoughts_panel(frame: &mut Frame, app_data: &AppData, area: Rect) {
    let block = Block::default()
        .title(" AI Thoughts ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .style(Style::default().bg(Color::Black));

    if app_data.thoughts.is_empty() {
        let placeholder = Paragraph::new("AI is ready to analyze your code.\nMake changes to see thoughts appear here.")
            .block(block)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        frame.render_widget(placeholder, area);
    } else {
        let thoughts_widget = widgets::ThoughtsWidget::new(&app_data.thoughts)
            .block(block);

        frame.render_widget(thoughts_widget, area);
    }
}

fn render_minimal_info(frame: &mut Frame, app_data: &AppData, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // File info
    let file_info = if let Some(ref file) = app_data.current_file {
        format!("📁 {}", file)
    } else {
        "📁 No file selected".to_string()
    };

    let file_widget = Paragraph::new(file_info)
        .block(
            Block::default()
                .title(" Current File ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(file_widget, layout[0]);

    // Latest thought
    if let Some(latest_thought) = app_data.thoughts.last() {
        let thought_text = format!(
            "{} {}",
            get_thought_icon(&latest_thought.thought_type),
            latest_thought.content
        );

        let thought_widget = Paragraph::new(thought_text)
            .block(
                Block::default()
                    .title(" Latest Thought ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(get_thought_color(&latest_thought.thought_type))),
            )
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true });

        frame.render_widget(thought_widget, layout[1]);
    }
}

fn render_status_bar(frame: &mut Frame, app_data: &AppData, area: Rect) {
    let status_area = Rect {
        x: area.x,
        y: area.bottom() - 3,
        width: area.width,
        height: 3,
    };

    let status_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20),
            Constraint::Min(0),
            Constraint::Length(30),
        ])
        .split(status_area);

    // Left: Recording status
    let recording_text = if app_data.is_recording {
        "🔴 Recording"
    } else {
        "⚫ Not Recording"
    };

    let recording_widget = Paragraph::new(recording_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(if app_data.is_recording {
            Color::Red
        } else {
            Color::DarkGray
        }));

    frame.render_widget(recording_widget, status_layout[0]);

    // Center: Mode and keybindings
    let mode_text = format!("Mode: {:?}", app_data.mode);
    let keybindings = " [q] Quit [v] Mode [c] Clear [f] File [h] Help ";

    let center_text = format!("{} {}", mode_text, keybindings);
    let center_widget = Paragraph::new(center_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center);

    frame.render_widget(center_widget, status_layout[1]);

    // Right: Thoughts count
    let thoughts_count = format!("Thoughts: {}", app_data.thoughts.len());
    let thoughts_widget = Paragraph::new(thoughts_count)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Right);

    frame.render_widget(thoughts_widget, status_layout[2]);
}

fn get_thought_icon(thought_type: &ThoughtType) -> &'static str {
    match thought_type {
        ThoughtType::Analyzing => "🔍",
        ThoughtType::Suggesting => "💡",
        ThoughtType::Warning => "⚠️",
        ThoughtType::Error => "❌",
        ThoughtType::Complete => "✅",
        ThoughtType::Meta => "🧠",
        ThoughtType::Performance => "⚡",
        ThoughtType::Security => "🔒",
        ThoughtType::Style => "🎨",
        ThoughtType::Architecture => "🏗️",
    }
}

fn get_thought_color(thought_type: &ThoughtType) -> Color {
    match thought_type {
        ThoughtType::Analyzing => Color::Blue,
        ThoughtType::Suggesting => Color::Yellow,
        ThoughtType::Warning => Color::Magenta,
        ThoughtType::Error => Color::Red,
        ThoughtType::Complete => Color::Green,
        ThoughtType::Meta => Color::Cyan,
        ThoughtType::Performance => Color::LightYellow,
        ThoughtType::Security => Color::LightRed,
        ThoughtType::Style => Color::LightMagenta,
        ThoughtType::Architecture => Color::LightBlue,
    }
}

pub fn render_help_overlay(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(80, 70, area);

    frame.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from("CoCo v2.0 - AI Pair Programmer"),
        Line::from(""),
        Line::from("Keybindings:"),
        Line::from("  q, Esc, Ctrl+C - Quit"),
        Line::from("  v - Toggle view mode"),
        Line::from("  c - Clear thoughts"),
        Line::from("  f - Select file"),
        Line::from("  y - Accept suggestion"),
        Line::from("  n - Reject suggestion"),
        Line::from("  h, F1 - Show this help"),
        Line::from("  F5 - Refresh"),
        Line::from(""),
        Line::from("View Modes:"),
        Line::from("  Side-by-Side - Code and thoughts side by side"),
        Line::from("  Full - Code on top, thoughts below"),
        Line::from("  Minimal - Essential info only"),
        Line::from("  Thoughts Only - AI thoughts full screen"),
        Line::from(""),
        Line::from("Press any key to close this help"),
    ];

    let help_widget = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .wrap(Wrap { trim: true });

    frame.render_widget(help_widget, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}