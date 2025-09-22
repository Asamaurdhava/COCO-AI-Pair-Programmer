use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, List, ListItem, Paragraph, Widget, Wrap,
    },
};

use crate::app::{Thought, ThoughtType, Suggestion};

pub struct CodeWidget<'a> {
    content: &'a str,
    block: Option<Block<'a>>,
    style: Style,
    line_numbers: bool,
    highlight_lines: Vec<usize>,
    syntax_highlighting: bool,
}

impl<'a> CodeWidget<'a> {
    pub fn new(content: &'a str) -> Self {
        Self {
            content,
            block: None,
            style: Style::default(),
            line_numbers: true,
            highlight_lines: Vec::new(),
            syntax_highlighting: true,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn line_numbers(mut self, show: bool) -> Self {
        self.line_numbers = show;
        self
    }

    pub fn highlight_lines(mut self, lines: Vec<usize>) -> Self {
        self.highlight_lines = lines;
        self
    }

    pub fn syntax_highlighting(mut self, enable: bool) -> Self {
        self.syntax_highlighting = enable;
        self
    }

    fn create_lines(&self) -> Vec<Line<'static>> {
        let lines: Vec<&str> = self.content.lines().collect();
        let mut result = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let line_num = i + 1;
            let is_highlighted = self.highlight_lines.contains(&line_num);

            let mut spans = Vec::new();

            if self.line_numbers {
                let line_num_str = format!("{:4} │ ", line_num);
                spans.push(Span::styled(
                    line_num_str,
                    Style::default().fg(Color::DarkGray),
                ));
            }

            if self.syntax_highlighting {
                spans.extend(self.highlight_syntax(line));
            } else {
                spans.push(Span::styled(
                    line.to_string(),
                    if is_highlighted {
                        Style::default().bg(Color::DarkGray)
                    } else {
                        self.style
                    },
                ));
            }

            result.push(Line::from(spans));
        }

        result
    }

    fn highlight_syntax(&self, line: &str) -> Vec<Span<'static>> {
        // Simple syntax highlighting for common programming constructs
        let mut spans = Vec::new();
        let _current_pos = 0;

        // Keywords for various languages
        let keywords = [
            "fn", "let", "mut", "const", "if", "else", "for", "while", "loop", "match",
            "return", "break", "continue", "struct", "enum", "impl", "trait", "mod",
            "use", "pub", "async", "await", "def", "class", "import", "from", "try",
            "except", "finally", "with", "as", "pass", "lambda", "yield", "global",
            "nonlocal", "function", "var", "const", "class", "extends", "implements",
            "interface", "public", "private", "protected", "static", "final", "abstract",
        ];

        // Simple tokenization - this is a basic implementation
        let tokens = line.split_whitespace();

        for token in tokens {
            let trimmed = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');

            let style = if keywords.contains(&trimmed) {
                Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
            } else if trimmed.starts_with('"') && trimmed.ends_with('"') {
                Style::default().fg(Color::Green)
            } else if trimmed.starts_with('\'') && trimmed.ends_with('\'') {
                Style::default().fg(Color::Green)
            } else if trimmed.starts_with("//") || trimmed.starts_with('#') {
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)
            } else if trimmed.chars().all(|c| c.is_ascii_digit()) {
                Style::default().fg(Color::Magenta)
            } else {
                self.style
            };

            spans.push(Span::styled(format!("{} ", token).to_string(), style));
        }

        if spans.is_empty() {
            spans.push(Span::styled(line.to_string(), self.style));
        }

        spans
    }
}

impl<'a> Widget for CodeWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let lines = self.create_lines();
        let text = Text::from(lines);

        let paragraph = Paragraph::new(text)
            .style(self.style)
            .wrap(Wrap { trim: false });

        let paragraph = if let Some(block) = self.block {
            paragraph.block(block)
        } else {
            paragraph
        };

        paragraph.render(area, buf);
    }
}

#[derive(Clone)]
pub struct ThoughtsWidget<'a> {
    thoughts: &'a [Thought],
    block: Option<Block<'a>>,
    style: Style,
    show_timestamps: bool,
    show_confidence: bool,
    max_items: Option<usize>,
}

impl<'a> ThoughtsWidget<'a> {
    pub fn new(thoughts: &'a [Thought]) -> Self {
        Self {
            thoughts,
            block: None,
            style: Style::default(),
            show_timestamps: true,
            show_confidence: true,
            max_items: None,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn show_timestamps(mut self, show: bool) -> Self {
        self.show_timestamps = show;
        self
    }

    pub fn show_confidence(mut self, show: bool) -> Self {
        self.show_confidence = show;
        self
    }

    pub fn max_items(mut self, max: usize) -> Self {
        self.max_items = Some(max);
        self
    }

    fn create_list_items(&self) -> Vec<ListItem> {
        let thoughts = if let Some(max) = self.max_items {
            if self.thoughts.len() > max {
                &self.thoughts[self.thoughts.len() - max..]
            } else {
                self.thoughts
            }
        } else {
            self.thoughts
        };

        thoughts
            .iter()
            .map(|thought| self.create_thought_item(thought))
            .collect()
    }

    fn create_thought_item(&self, thought: &Thought) -> ListItem {
        let mut spans = Vec::new();

        // Thought type icon and color
        let icon = get_thought_icon(&thought.thought_type);
        let color = get_thought_color(&thought.thought_type);

        spans.push(Span::styled(
            format!("{} ", icon),
            Style::default().fg(color),
        ));

        // Timestamp
        if self.show_timestamps {
            let time_str = thought.timestamp.format("%H:%M:%S").to_string();
            spans.push(Span::styled(
                format!("[{}] ", time_str),
                Style::default().fg(Color::DarkGray),
            ));
        }

        // Confidence
        if self.show_confidence && thought.confidence > 0.0 {
            let confidence_str = format!("({:.0}%) ", thought.confidence * 100.0);
            let confidence_color = if thought.confidence >= 0.8 {
                Color::Green
            } else if thought.confidence >= 0.6 {
                Color::Yellow
            } else {
                Color::Red
            };
            spans.push(Span::styled(
                confidence_str,
                Style::default().fg(confidence_color),
            ));
        }

        // Content
        spans.push(Span::styled(
            thought.content.clone(),
            Style::default().fg(Color::White),
        ));

        // File path and line number
        if let Some(ref file_path) = thought.file_path {
            let location = if let Some(line_num) = thought.line_number {
                format!(" ({}:{})", file_path, line_num)
            } else {
                format!(" ({})", file_path)
            };
            spans.push(Span::styled(
                location,
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            ));
        }

        let mut lines = vec![Line::from(spans)];

        // Add suggestions if any
        for (i, suggestion) in thought.suggestions.iter().enumerate() {
            if i < 3 {  // Limit to 3 suggestions per thought
                let suggestion_line = self.create_suggestion_line(suggestion, i + 1);
                lines.push(suggestion_line);
            }
        }

        ListItem::new(lines)
    }

    fn create_suggestion_line(&self, suggestion: &Suggestion, index: usize) -> Line {
        let priority_icon = match suggestion.priority {
            crate::app::Priority::Critical => "🔥",
            crate::app::Priority::High => "⚡",
            crate::app::Priority::Medium => "💡",
            crate::app::Priority::Low => "💭",
        };

        let action_icon = match suggestion.action_type {
            crate::app::ActionType::Replace => "🔄",
            crate::app::ActionType::Insert => "➕",
            crate::app::ActionType::Delete => "❌",
            crate::app::ActionType::Refactor => "🔧",
            crate::app::ActionType::Optimize => "⚡",
            crate::app::ActionType::Fix => "🩹",
        };

        vec![
            Span::styled(
                format!("  {}. {} {} ", index, priority_icon, action_icon),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(
                suggestion.title.clone(),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" - {}", suggestion.description),
                Style::default().fg(Color::Gray),
            ),
        ].into()
    }
}

impl<'a> Widget for ThoughtsWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let binding = self.clone();
        let items = binding.create_list_items();
        let style = self.style;
        let block = self.block;

        let list = List::new(items)
            .style(style);

        let list = if let Some(block) = block {
            list.block(block)
        } else {
            list
        };

        Widget::render(list, area, buf);
    }
}

#[derive(Clone)]
pub struct SuggestionWidget<'a> {
    suggestion: &'a Suggestion,
    block: Option<Block<'a>>,
    style: Style,
    show_code: bool,
}

impl<'a> SuggestionWidget<'a> {
    pub fn new(suggestion: &'a Suggestion) -> Self {
        Self {
            suggestion,
            block: None,
            style: Style::default(),
            show_code: true,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn show_code(mut self, show: bool) -> Self {
        self.show_code = show;
        self
    }

    fn create_content(&self) -> Text {
        let mut lines = Vec::new();

        // Title and priority
        let priority_color = match self.suggestion.priority {
            crate::app::Priority::Critical => Color::Red,
            crate::app::Priority::High => Color::Yellow,
            crate::app::Priority::Medium => Color::Blue,
            crate::app::Priority::Low => Color::Gray,
        };

        lines.push(Line::from(vec![
            Span::styled(
                self.suggestion.title.clone(),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" [{}]", format!("{:?}", self.suggestion.priority).to_uppercase()),
                Style::default().fg(priority_color),
            ),
        ]));

        lines.push(Line::from(""));

        // Description
        lines.push(Line::from(Span::styled(
            self.suggestion.description.clone(),
            Style::default().fg(Color::White),
        )));

        // Code snippet if available
        if self.show_code {
            if let Some(ref code) = self.suggestion.code_snippet {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "Suggested code:",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));

                for code_line in code.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", code_line),
                        Style::default().fg(Color::Green),
                    )));
                }
            }
        }

        Text::from(lines)
    }
}

impl<'a> Widget for SuggestionWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let binding = self.clone();
        let content = binding.create_content();
        let style = self.style;
        let block = self.block;

        let paragraph = Paragraph::new(content)
            .style(style)
            .wrap(Wrap { trim: true });

        let paragraph = if let Some(block) = block {
            paragraph.block(block)
        } else {
            paragraph
        };

        paragraph.render(area, buf);
    }
}

// Helper functions for thought styling
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

// Stateful widgets for scrolling and selection
pub struct ScrollableThoughts {
    pub scroll_state: usize,
    pub selected_index: Option<usize>,
}

impl ScrollableThoughts {
    pub fn new() -> Self {
        Self {
            scroll_state: 0,
            selected_index: None,
        }
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_state > 0 {
            self.scroll_state -= 1;
        }
    }

    pub fn scroll_down(&mut self, max_items: usize) {
        if self.scroll_state < max_items.saturating_sub(1) {
            self.scroll_state += 1;
        }
    }

    pub fn select_next(&mut self, max_items: usize) {
        if max_items == 0 {
            return;
        }

        self.selected_index = match self.selected_index {
            None => Some(0),
            Some(i) => {
                if i >= max_items - 1 {
                    Some(0)
                } else {
                    Some(i + 1)
                }
            }
        };
    }

    pub fn select_previous(&mut self, max_items: usize) {
        if max_items == 0 {
            return;
        }

        self.selected_index = match self.selected_index {
            None => Some(max_items - 1),
            Some(i) => {
                if i == 0 {
                    Some(max_items - 1)
                } else {
                    Some(i - 1)
                }
            }
        };
    }
}