use crate::code_highlighter::CodeHighlighter;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use std::collections::HashMap;

/// Tracks the kind of list we're inside so items get the right marker.
enum ListKind {
    /// Ordered list with the next item number.
    Ordered(u64),
    Unordered,
}

#[derive(Clone)]
pub struct MarkdownRenderer {
    cache: HashMap<String, Vec<Line<'static>>>,
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Convert a markdown string into renderable Ratatui lines.
    ///
    /// Results are cached by content. The cache is bounded to avoid unbounded
    /// growth as the user types.
    pub fn render(&mut self, content: &str, highlighter: &CodeHighlighter) -> Vec<Line<'static>> {
        if let Some(cached) = self.cache.get(content) {
            return cached.clone();
        }

        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        let parser = Parser::new_ext(content, options);
        let lines = Self::parse_events(parser, highlighter);

        if self.cache.len() > 100 {
            self.cache.clear();
        }
        self.cache.insert(content.to_string(), lines.clone());
        lines
    }

    fn parse_events(parser: Parser, highlighter: &CodeHighlighter) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = Vec::new();
        let mut current: Vec<Span<'static>> = Vec::new();

        // Stack of styles for nested inline formatting (bold, italic, etc.).
        let mut style_stack: Vec<Style> = Vec::new();
        // Stack of active lists for nesting + numbering.
        let mut list_stack: Vec<ListKind> = Vec::new();
        // Blockquote nesting depth.
        let mut quote_depth: usize = 0;

        let mut in_code_block = false;
        let mut code_language = String::new();
        let mut code_content = String::new();

        // Combine the style stack into a single effective style.
        let effective_style = |stack: &[Style]| -> Style {
            let mut s = Style::default();
            for layer in stack {
                s = s.patch(*layer);
            }
            s
        };

        let flush = |lines: &mut Vec<Line<'static>>, current: &mut Vec<Span<'static>>| {
            if !current.is_empty() {
                lines.push(Line::from(std::mem::take(current)));
            }
        };

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    flush(&mut lines, &mut current);
                    let color = match level {
                        HeadingLevel::H1 => Color::Blue,
                        HeadingLevel::H2 => Color::Cyan,
                        HeadingLevel::H3 => Color::Green,
                        _ => Color::Yellow,
                    };
                    style_stack.push(Style::default().fg(color).add_modifier(Modifier::BOLD));
                    let prefix = "#".repeat(heading_number(level));
                    current.push(Span::styled(
                        format!("{} ", prefix),
                        effective_style(&style_stack),
                    ));
                }
                Event::End(TagEnd::Heading(_)) => {
                    style_stack.pop();
                    flush(&mut lines, &mut current);
                    lines.push(Line::from(""));
                }

                Event::Start(Tag::CodeBlock(kind)) => {
                    flush(&mut lines, &mut current);
                    in_code_block = true;
                    code_language = match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                        pulldown_cmark::CodeBlockKind::Indented => String::new(),
                    };
                    code_content.clear();
                }
                Event::End(TagEnd::CodeBlock) => {
                    in_code_block = false;
                    let label = if code_language.is_empty() {
                        "┌─ code ─".to_string()
                    } else {
                        format!("┌─ {} ─", code_language)
                    };
                    lines.push(Line::from(Span::styled(
                        label,
                        Style::default().fg(Color::DarkGray),
                    )));
                    for hl in highlighter.highlight_lines(&code_content, &code_language) {
                        lines.push(hl);
                    }
                    lines.push(Line::from(Span::styled(
                        "└─────────",
                        Style::default().fg(Color::DarkGray),
                    )));
                    code_content.clear();
                }

                Event::Code(code) => {
                    let style = effective_style(&style_stack)
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::DIM);
                    current.push(Span::styled(format!("`{}`", code), style));
                }

                Event::Start(Tag::Strong) => {
                    style_stack.push(Style::default().add_modifier(Modifier::BOLD));
                }
                Event::End(TagEnd::Strong) => {
                    style_stack.pop();
                }
                Event::Start(Tag::Emphasis) => {
                    style_stack.push(Style::default().add_modifier(Modifier::ITALIC));
                }
                Event::End(TagEnd::Emphasis) => {
                    style_stack.pop();
                }
                Event::Start(Tag::Strikethrough) => {
                    style_stack.push(Style::default().add_modifier(Modifier::CROSSED_OUT));
                }
                Event::End(TagEnd::Strikethrough) => {
                    style_stack.pop();
                }

                Event::Text(text) => {
                    if in_code_block {
                        code_content.push_str(&text);
                    } else {
                        current.push(Span::styled(text.to_string(), effective_style(&style_stack)));
                    }
                }

                Event::SoftBreak | Event::HardBreak => {
                    flush(&mut lines, &mut current);
                }

                Event::Start(Tag::List(start)) => {
                    list_stack.push(match start {
                        Some(n) => ListKind::Ordered(n),
                        None => ListKind::Unordered,
                    });
                }
                Event::End(TagEnd::List(_)) => {
                    list_stack.pop();
                }
                Event::Start(Tag::Item) => {
                    flush(&mut lines, &mut current);
                    let depth = list_stack.len().saturating_sub(1);
                    let indent = "  ".repeat(depth);
                    let marker = match list_stack.last_mut() {
                        Some(ListKind::Ordered(n)) => {
                            let m = format!("{}. ", n);
                            *n += 1;
                            m
                        }
                        _ => "• ".to_string(),
                    };
                    current.push(Span::styled(
                        format!("{}{}", indent, marker),
                        Style::default().fg(Color::Yellow),
                    ));
                }
                Event::End(TagEnd::Item) => {
                    flush(&mut lines, &mut current);
                }

                Event::Start(Tag::BlockQuote(_)) => {
                    quote_depth += 1;
                    flush(&mut lines, &mut current);
                    current.push(Span::styled(
                        "║ ".repeat(quote_depth),
                        Style::default().fg(Color::DarkGray),
                    ));
                    style_stack
                        .push(Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC));
                }
                Event::End(TagEnd::BlockQuote) => {
                    style_stack.pop();
                    quote_depth = quote_depth.saturating_sub(1);
                    flush(&mut lines, &mut current);
                }

                Event::Start(Tag::Paragraph) => {
                    flush(&mut lines, &mut current);
                }
                Event::End(TagEnd::Paragraph) => {
                    flush(&mut lines, &mut current);
                    if list_stack.is_empty() {
                        lines.push(Line::from(""));
                    }
                }

                Event::Rule => {
                    flush(&mut lines, &mut current);
                    lines.push(Line::from(Span::styled(
                        "─".repeat(40),
                        Style::default().fg(Color::DarkGray),
                    )));
                }

                _ => {}
            }
        }

        flush(&mut lines, &mut current);
        lines
    }
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

fn heading_number(level: HeadingLevel) -> usize {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}
