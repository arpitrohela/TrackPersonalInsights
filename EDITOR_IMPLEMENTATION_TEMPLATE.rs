// ============================================================================
// EDITOR_IMPLEMENTATION_TEMPLATE.rs
//
// Concrete code templates for implementing live markdown rendering.
// Copy patterns from here into your actual src/main.rs or new modules.
// ============================================================================

// ============================================================================
// PART 1: Add to Cargo.toml
// ============================================================================

/*
Add these to your [dependencies] section:

pulldown-cmark = "0.11"
syntect = "5.1"
ansi-to-tui = "3.0"
parking_lot = "0.12"

Full updated section example:
[dependencies]
anyhow = "1"
bincode = "1.3"
chrono = { version = "0.4", features = ["serde"] }
crossterm = "0.27"
dirs = "5"
open = "5"
ratatui = "0.26"
csv = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
strsim = "0.11"
tui-textarea = "0.4"
pulldown-cmark = "0.11"
syntect = "5.1"
ansi-to-tui = "3.0"
parking_lot = "0.12"
*/

// ============================================================================
// PART 2: New Module - markdown_renderer.rs
// ============================================================================

/*
Save this as: src/markdown_renderer.rs
Then add to main.rs: mod markdown_renderer;
*/

use pulldown_cmark::{Parser, Event, CowStr, html};
use ratatui::text::{Line, Span, Text};
use ratatui::style::{Color, Modifier, Style};
use std::collections::HashMap;

pub struct MarkdownRenderer {
    cache: HashMap<String, Vec<Line<'static>>>,
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Main entry point: convert markdown string to renderable Ratatui Lines
    pub fn render(&mut self, content: &str) -> Vec<Line<'static>> {
        // Check cache first
        if let Some(cached) = self.cache.get(content) {
            return cached.clone();
        }

        let parser = Parser::new(content);
        let lines = self.parse_events(parser);

        // Cache result
        self.cache.insert(content.to_string(), lines.clone());
        lines
    }

    fn parse_events(&self, parser: Parser) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let mut current_line = Line::from(Span::raw(""));
        let mut in_code_block = false;
        let mut code_language = String::new();
        let mut in_list = false;
        let mut list_indent = 0;

        for event in parser {
            match event {
                // Headings: render with color and bold
                Event::Start(pulldown_cmark::Tag::Heading { level, .. }) => {
                    let color = match level {
                        pulldown_cmark::HeadingLevel::H1 => Color::Blue,
                        pulldown_cmark::HeadingLevel::H2 => Color::Cyan,
                        pulldown_cmark::HeadingLevel::H3 => Color::Green,
                        _ => Color::Yellow,
                    };
                    let style = Style::default().fg(color).add_modifier(Modifier::BOLD);
                    current_line = Line::from(Span::styled("", style));
                }
                Event::End(pulldown_cmark::TagEnd::Heading(_)) => {
                    lines.push(current_line.clone());
                    current_line = Line::from(Span::raw(""));
                }

                // Code blocks: different styling, preserve formatting
                Event::Start(pulldown_cmark::Tag::CodeBlock(kind)) => {
                    in_code_block = true;
                    if let pulldown_cmark::CodeBlockKind::Fenced(lang) = kind {
                        code_language = lang.to_string();
                    }
                    lines.push(Line::from(Span::styled("┌─ code ─", Style::default().fg(Color::Gray))));
                    current_line = Line::from(Span::raw(""));
                }
                Event::End(pulldown_cmark::TagEnd::CodeBlock) => {
                    in_code_block = false;
                    if !current_line.spans.is_empty() {
                        lines.push(current_line.clone());
                    }
                    lines.push(Line::from(Span::styled("└─────────", Style::default().fg(Color::Gray))));
                    current_line = Line::from(Span::raw(""));
                }

                // Code spans (inline): backticks
                Event::Code(code) => {
                    let style = Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::DIM);
                    current_line.spans.push(Span::styled(
                        format!("`{}`", code),
                        style,
                    ));
                }

                // Bold text
                Event::Start(pulldown_cmark::Tag::Strong) => {
                    let style = Style::default().add_modifier(Modifier::BOLD);
                    current_line.spans.push(Span::styled("", style));
                }
                Event::End(pulldown_cmark::TagEnd::Strong) => {
                    // Reset style handled by SoftBreak/HardBreak
                }

                // Italic text
                Event::Start(pulldown_cmark::Tag::Emphasis) => {
                    let style = Style::default().add_modifier(Modifier::ITALIC);
                    current_line.spans.push(Span::styled("", style));
                }
                Event::End(pulldown_cmark::TagEnd::Emphasis) => {
                    // Reset style
                }

                // Text content
                Event::Text(text) => {
                    let span = if in_code_block {
                        Span::styled(&text, Style::default().fg(Color::Green))
                    } else {
                        Span::raw(&text)
                    };
                    current_line.spans.push(span);
                }

                // Line breaks
                Event::SoftBreak | Event::HardBreak => {
                    lines.push(current_line.clone());
                    current_line = Line::from(Span::raw(""));
                }

                // Lists
                Event::Start(pulldown_cmark::Tag::List(_)) => {
                    in_list = true;
                    list_indent += 2;
                }
                Event::End(pulldown_cmark::TagEnd::List(_)) => {
                    in_list = false;
                    list_indent = list_indent.saturating_sub(2);
                }
                Event::Start(pulldown_cmark::Tag::Item) => {
                    if in_list {
                        current_line = Line::from(Span::raw("  ".repeat(list_indent / 2) + "• "));
                    }
                }
                Event::End(pulldown_cmark::TagEnd::Item) => {
                    lines.push(current_line.clone());
                    current_line = Line::from(Span::raw(""));
                }

                // Blockquotes
                Event::Start(pulldown_cmark::Tag::BlockQuote(_)) => {
                    current_line.spans.push(Span::styled(
                        "║ ",
                        Style::default().fg(Color::DarkGray),
                    ));
                }
                Event::End(pulldown_cmark::TagEnd::BlockQuote) => {
                    lines.push(current_line.clone());
                    current_line = Line::from(Span::raw(""));
                }

                // Ignore other events for now
                _ => {}
            }
        }

        // Don't forget the last line
        if !current_line.spans.is_empty() {
            lines.push(current_line);
        }

        lines
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

// ============================================================================
// PART 3: New Module - code_highlighter.rs
// ============================================================================

/*
Save this as: src/code_highlighter.rs
Then add to main.rs: mod code_highlighter;
*/

use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Style as SyntectStyle};
use syntect::parsing::SyntaxSet;
use ratatui::style::{Color, Style};
use ratatui::text::Span;

pub struct CodeHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl CodeHighlighter {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    /// Highlight a code block. Returns vector of Ratatui Spans.
    pub fn highlight(&self, code: &str, language: &str) -> Vec<Span<'static>> {
        let syntax = self.syntax_set
            .find_syntax_by_token(language)
            .or_else(|| self.syntax_set.find_syntax_by_first_line(code))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = &self.theme_set.themes["Solarized (dark)"];
        let mut highlighter = HighlightLines::new(syntax, theme);

        let mut spans = Vec::new();

        for line in code.lines() {
            let regions = highlighter
                .highlight_line(line, &self.syntax_set)
                .unwrap_or_default();

            for (style, text) in regions {
                let ratatui_style = self.syntect_to_ratatui_style(style);
                spans.push(Span::styled(text.to_string(), ratatui_style));
            }

            spans.push(Span::raw("\n"));
        }

        spans
    }

    fn syntect_to_ratatui_style(&self, style: SyntectStyle) -> Style {
        let fg = if let Some(color) = style.foreground.a.checked_sub(1) {
            // Convert RGB to terminal color (simplified)
            Color::Rgb(
                style.foreground.r,
                style.foreground.g,
                style.foreground.b,
            )
        } else {
            Color::White
        };

        Style::default().fg(fg)
    }
}

// ============================================================================
// PART 4: Update App struct (in main.rs)
// ============================================================================

/*
Add these fields to your App struct:

pub struct App {
    // ... existing fields ...

    // NEW FIELDS FOR EDITOR
    markdown_renderer: markdown_renderer::MarkdownRenderer,
    code_highlighter: code_highlighter::CodeHighlighter,
    preview_scroll: u16,
    last_preview_render: std::time::Instant,
    cached_preview: Vec<ratatui::text::Line<'static>>,
}

And in App::new():
    markdown_renderer: markdown_renderer::MarkdownRenderer::new(),
    code_highlighter: code_highlighter::CodeHighlighter::new(),
    preview_scroll: 0,
    last_preview_render: std::time::Instant::now(),
    cached_preview: Vec::new(),
*/

// ============================================================================
// PART 5: Update rendering (replace/enhance render_textarea_editor)
// ============================================================================

/*
Replace the render_textarea_editor function or create new render_split_editor.

Key changes:
1. Split the editing area into left (editor) and right (preview) panes
2. Render editor on left as before
3. Render markdown preview on right
4. Sync scrolling between panes
*/

// Pseudo-code for new rendering function:
/*
fn render_split_editor(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    use ratatui::layout::{Layout, Direction, Constraint};

    // Split area 50/50 horizontally
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    // LEFT PANE: Text Editor (existing code)
    render_textarea_editor(frame, app, chunks[0], "Edit");

    // RIGHT PANE: Markdown Preview (new code)
    let content = app.textarea.lines().join("\n");

    // Debounce: only re-render if 100ms have passed
    if app.last_preview_render.elapsed() > std::time::Duration::from_millis(100) {
        app.cached_preview = app.markdown_renderer.render(&content);
        app.last_preview_render = std::time::Instant::now();
    }

    let preview = Paragraph::new(app.cached_preview.clone())
        .block(Block::default()
            .title("Preview")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan)))
        .scroll((app.preview_scroll, 0));

    frame.render_widget(preview, chunks[1]);

    // Optional: Render scrollbar for preview
    let mut scrollbar_state = ScrollbarState::default()
        .content_length(app.cached_preview.len() as u16)
        .viewport_content_length(chunks[1].height.saturating_sub(2));

    frame.render_stateful_widget(
        Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight),
        chunks[1],
        &mut scrollbar_state,
    );
}
*/

// ============================================================================
// PART 6: Handle input for preview scrolling
// ============================================================================

/*
In your input handling, add:

When user scrolls in editor:
    app.preview_scroll = some_calculated_value;

Optional: Sync scroll positions
    - Line number in editor → estimated line in preview
    - More complex due to markdown expansion
*/

// ============================================================================
// PART 7: Testing
// ============================================================================

/*
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heading_renders_blue() {
        let mut renderer = markdown_renderer::MarkdownRenderer::new();
        let lines = renderer.render("# Hello World");
        assert!(!lines.is_empty());
        // Check color of first span
    }

    #[test]
    fn test_code_block_highlighted() {
        let highlighter = code_highlighter::CodeHighlighter::new();
        let spans = highlighter.highlight("fn main() {}", "rust");
        assert!(!spans.is_empty());
    }

    #[test]
    fn test_bold_text() {
        let mut renderer = markdown_renderer::MarkdownRenderer::new();
        let lines = renderer.render("This is **bold** text");
        assert!(!lines.is_empty());
    }
}
*/

// ============================================================================
// PART 8: Configuration (optional)
// ============================================================================

/*
Create a config struct for easy tweaking:

pub struct EditorConfig {
    pub split_ratio: (u16, u16),        // (left%, right%)
    pub preview_debounce_ms: u64,       // Delay before re-rendering preview
    pub highlight_theme: String,         // "Solarized (dark)", "InspiredGitHub", etc.
    pub enable_line_numbers: bool,
    pub tab_width: usize,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            split_ratio: (50, 50),
            preview_debounce_ms: 100,
            highlight_theme: "Solarized (dark)".to_string(),
            enable_line_numbers: false,
            tab_width: 4,
        }
    }
}
*/

// ============================================================================
// INTEGRATION CHECKLIST
// ============================================================================

/*
1. [ ] Add dependencies to Cargo.toml
2. [ ] Create src/markdown_renderer.rs and add mod declaration
3. [ ] Create src/code_highlighter.rs and add mod declaration
4. [ ] Add new fields to App struct
5. [ ] Initialize new fields in App::new()
6. [ ] Create render_split_editor() function
7. [ ] Update rendering pipeline to call render_split_editor()
8. [ ] Test with sample markdown
9. [ ] Add scrolling synchronization
10. [ ] Profile and optimize for large documents
11. [ ] Test saving/loading (should be transparent)
12. [ ] Add configuration options
*/
