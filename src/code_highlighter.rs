use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Theme, Style as SyntectStyle, FontStyle};
use syntect::parsing::SyntaxSet;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Syntax highlighter for fenced code blocks.
///
/// The `SyntaxSet` and `Theme` are loaded once at construction time and reused
/// for every call. Previously they were rebuilt on every `highlight()` call,
/// which is expensive (parsing all default syntaxes/themes) and happened on the
/// render hot path.
#[derive(Clone)]
pub struct CodeHighlighter {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl CodeHighlighter {
    pub fn new() -> Self {
        let theme_set = ThemeSet::load_defaults();
        let theme = theme_set
            .themes
            .get("Solarized (dark)")
            .or_else(|| theme_set.themes.get("base16-ocean.dark"))
            .cloned()
            .unwrap_or_else(|| theme_set.themes.values().next().unwrap().clone());

        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme,
        }
    }

    /// Highlight a code block, returning one `Line` per source line so the
    /// caller can render it directly without re-splitting on newlines.
    pub fn highlight_lines(&self, code: &str, language: &str) -> Vec<Line<'static>> {
        let syntax = self
            .syntax_set
            .find_syntax_by_token(language)
            .or_else(|| self.syntax_set.find_syntax_by_extension(language))
            .or_else(|| self.syntax_set.find_syntax_by_first_line(code))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let mut highlighter = HighlightLines::new(syntax, &self.theme);
        let mut lines = Vec::new();

        for line in code.lines() {
            match highlighter.highlight_line(line, &self.syntax_set) {
                Ok(regions) => {
                    let spans: Vec<Span<'static>> = regions
                        .into_iter()
                        .map(|(style, text)| {
                            Span::styled(text.to_string(), self.syntect_to_ratatui_style(style))
                        })
                        .collect();
                    lines.push(Line::from(spans));
                }
                Err(_) => lines.push(Line::from(Span::raw(line.to_string()))),
            }
        }

        lines
    }

    fn syntect_to_ratatui_style(&self, style: SyntectStyle) -> Style {
        let mut out = Style::default().fg(Color::Rgb(
            style.foreground.r,
            style.foreground.g,
            style.foreground.b,
        ));

        if style.font_style.contains(FontStyle::BOLD) {
            out = out.add_modifier(Modifier::BOLD);
        }
        if style.font_style.contains(FontStyle::ITALIC) {
            out = out.add_modifier(Modifier::ITALIC);
        }
        if style.font_style.contains(FontStyle::UNDERLINE) {
            out = out.add_modifier(Modifier::UNDERLINED);
        }

        out
    }
}

impl Default for CodeHighlighter {
    fn default() -> Self {
        Self::new()
    }
}
