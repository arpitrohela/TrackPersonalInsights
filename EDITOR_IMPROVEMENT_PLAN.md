# Text Editor Improvement Plan: Live Markdown Rendering & Enhanced Editing

## Executive Summary

Your notes editor currently uses `tui-textarea` with basic rendering. This plan introduces **live markdown preview with syntax highlighting** in a split-pane view (editor left, preview right), matching the UX of the micro editor you referenced.

---

## Current State Analysis

### What You Have
- **Text Editor**: `tui-textarea` for multi-line input
- **Rendering**: Basic `Paragraph` widget with `Wrap` trait
- **Limitations**:
  - No live preview of markdown rendering
  - No syntax highlighting for code blocks
  - No visual feedback for markdown elements (#, **, etc.)
  - Cursor positioning and word wrap not refined

### Key Issue
Users type markdown but see raw text. They need immediate visual confirmation that `# heading` renders as a heading, `**bold**` appears bold, etc.

---

## Recommended Solution Architecture

### Split-Pane Editor Design

```
┌─────────────────────────────────────────────────────────────────┐
│                      Notes Editor                               │
├─────────────────────────────────────────────┬───────────────────┤
│                                             │                   │
│  LEFT PANE (Editor)                         │  RIGHT PANE       │
│  • Raw markdown text                        │  (Preview)        │
│  • Cursor visible                           │  • Rendered       │
│  • Line numbers                             │    markdown       │
│  • Code completion hints                    │  • Syntax         │
│                                             │    highlighted    │
│  # My Heading                               │  # My Heading     │
│  This is **bold** text and *italic*        │  This is bold     │
│  ``` rust                                   │    text and       │
│  fn main() {                                │    italic         │
│      println!("hello");                     │  ┌──────────────┐ │
│  }                                          │  │ fn main() {  │ │
│  ```                                        │  │ println!...; │ │
│  |Cursor|here                               │  │ }            │ │
│                                             │  └──────────────┘ │
└─────────────────────────────────────────────┴───────────────────┘
```

### Key Design Points

1. **Synchronized Rendering**
   - Editor updates trigger re-parse of markdown
   - Preview updates in real-time (debounced for performance)
   - Scroll positions sync between panes

2. **Smart Highlighting**
   - Markdown syntax highlighting (headers, bold, italic, etc.)
   - Code block syntax highlighting using Syntect
   - Inline code highlighting

3. **Responsive UX**
   - Debounce preview rendering to avoid lag during fast typing
   - Cache parsed markdown between renders
   - Only re-render changed lines when possible

---

## Library Stack Recommendation

### Core Dependencies to Add

```toml
# Markdown parsing and rendering
pulldown-cmark = "0.11"           # Standard markdown parser
tui-markdown = "0.4"              # Ratatui markdown widget (optional base)
ratatui-markdown = "0.1"          # Alternative: more features

# Syntax highlighting
syntect = "5.1"                   # Code syntax highlighting
ansi-to-tui = "3.0"              # Convert ANSI codes to Ratatui styles

# Performance
crossbeam-channel = "0.5"         # Async rendering (optional)
parking_lot = "0.12"              # Fast mutexes
```

### Why These Libraries?

| Library | Purpose | Why |
|---------|---------|-----|
| **pulldown-cmark** | Markdown → AST | Industry standard, actively maintained, fast |
| **syntect** | Syntax highlighting | Powers VS Code, performance optimized |
| **ansi-to-tui** | Color conversion | Bridges syntect (ANSI) and ratatui styles |
| **tui-markdown** | Ready-made widget | Can use as reference or base implementation |

---

## Implementation Roadmap

### Phase 1: Add Dependencies & Basic Markdown Rendering
**Goal**: Get a proof-of-concept working

1. Add crates to `Cargo.toml`
2. Create `markdown_renderer.rs` module
3. Implement `parse_markdown()` function using pulldown-cmark
4. Create simple `render_markdown_to_text()` that converts AST to `ratatui::text::Text`
5. Test with sample notes

**Code Skeleton**:
```rust
// src/markdown_renderer.rs
use pulldown_cmark::{Parser, html};
use ratatui::text::{Line, Span, Text};

pub fn parse_markdown(content: &str) -> Vec<Line<'static>> {
    let parser = Parser::new(content);
    // Convert parser events to ratatui Text/Spans with styling
    // Return rendered lines
}

pub fn apply_markdown_styles(line: &str) -> Line<'static> {
    // Apply color/style based on markdown syntax
    // Examples: 
    //   - Lines starting with # → Color::Blue + Bold
    //   - Text with ** → Bold
    //   - Text with `` → Different color for code
}
```

### Phase 2: Add Syntax Highlighting for Code Blocks
**Goal**: Colorize code blocks with language-specific highlighting

1. Create `code_highlighter.rs` module
2. Integrate syntect for code block detection
3. Apply Sublime Text themes (syntect comes with defaults)
4. Map syntect ANSI colors to ratatui colors via `ansi-to-tui`

**Code Skeleton**:
```rust
// src/code_highlighter.rs
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

pub struct CodeHighlighter {
    syntax_set: SyntaxSet,
    theme: syntect::highlighting::Theme,
}

impl CodeHighlighter {
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme = ThemeSet::load_defaults().themes["Solarized (dark)"].clone();
        Self { syntax_set, theme }
    }

    pub fn highlight_code(&self, code: &str, language: &str) -> Vec<String> {
        let syntax = self.syntax_set.find_syntax_by_token(language)
            .or_else(|| self.syntax_set.find_syntax_by_first_line(code))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
        
        let mut highlighter = HighlightLines::new(syntax, &self.theme);
        code.lines()
            .map(|line| highlighter.highlight_line(line, &self.syntax_set).unwrap_or_default())
            .collect()
    }
}
```

### Phase 3: Build Split-Pane Layout
**Goal**: Display editor and preview side-by-side

1. Modify rendering logic to split area into two panes
2. Left pane: Keep existing `textarea` with slight enhancements
3. Right pane: Display rendered markdown output
4. Add configuration for pane width (e.g., 50/50 split)

**Code Skeleton**:
```rust
// In render_editing_panel or new render_split_editor function
fn render_split_editor(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    // Left pane: textarea editor
    render_textarea_editor(frame, app, chunks[0], "Edit");

    // Right pane: markdown preview
    render_markdown_preview(frame, app, chunks[1]);
}

fn render_markdown_preview(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let content = app.textarea.lines().join("\n");
    let rendered_lines = parse_markdown(&content);
    
    let preview = Paragraph::new(rendered_lines)
        .block(Block::default().title("Preview").borders(Borders::ALL))
        .scroll((app.preview_scroll, 0));
    
    frame.render_widget(preview, area);
}
```

### Phase 4: Synchronize Scrolling & Cursor Position
**Goal**: Keep editor and preview in sync

1. Track scrolling offset in both panes
2. Sync scroll when user scrolls in editor
3. Optional: Highlight current line in preview based on cursor position
4. Handle resize events to maintain split ratio

**Implementation Tips**:
- Store `preview_scroll` in App struct
- Update during input handling
- Use line mapping (source line → rendered line) for complex markdown

### Phase 5: Performance Optimization
**Goal**: Ensure smooth typing experience

1. Debounce preview updates (only re-render every 100ms during typing)
2. Cache markdown parse results
3. Render only visible lines in preview
4. Profile with `cargo flamegraph`

**Code Skeleton**:
```rust
struct EditorState {
    last_render: Instant,
    render_debounce_ms: u64,
    cached_ast: Option<Vec<pulldown_cmark::Event>>,
}

fn should_rerender(state: &EditorState) -> bool {
    state.last_render.elapsed() > Duration::from_millis(state.render_debounce_ms)
}
```

---

## Reference Implementations

### Production Examples

1. **MDTui** ([mdtui.pages.dev](https://mdtui.pages.dev/))
   - Full markdown editor with live preview
   - Vim keybindings
   - Open source: check their code for split-pane logic

2. **markdown-reader** ([github.com/leboiko/markdown-reader](https://github.com/leboiko/markdown-reader))
   - Hybrid editing (raw on cursor, rendered elsewhere)
   - Mermaid diagram support
   - Good reference for rendering architecture

3. **MDEDIT** ([github.com/thscharler/mdedit](https://github.com/thscharler/mdedit))
   - Clean ratatui markdown editor
   - Worth studying for architectural patterns

### Study These Code Snippets

- How they handle markdown → Text conversion
- Scroll synchronization logic
- Debouncing for performance
- Color scheme mapping

---

## Breaking Changes & Migration

### What Needs to Change

1. **Cargo.toml**: Add new dependencies (~200KB added to binary size)
2. **render_editing_panel()**: Split into two render functions or add pane logic
3. **App struct**: Add new fields:
   ```rust
   pub struct App {
       // ... existing fields ...
       markdown_renderer: MarkdownRenderer,
       code_highlighter: CodeHighlighter,
       preview_scroll: u16,
       last_preview_update: Instant,
       cached_markdown: Option<Vec<Line<'static>>>,
   }
   ```
4. **Input handling**: No major changes needed; textarea logic stays the same

### Backward Compatibility
✅ **Fully compatible** — Only adds visual features, doesn't change data format or save/load logic

---

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let lines = parse_markdown("# Hello World");
        assert_eq!(lines[0].style().fg, Some(Color::Blue));
    }

    #[test]
    fn test_code_block_highlighting() {
        let code = "fn main() {}";
        let highlighted = code_highlighter.highlight_code(code, "rust");
        assert!(!highlighted.is_empty());
    }

    #[test]
    fn test_scroll_sync() {
        // Verify editor and preview scroll in tandem
    }
}
```

### Manual Testing Checklist
- [ ] Type markdown and see live preview update
- [ ] Scroll in editor, preview follows
- [ ] Code blocks render with syntax highlighting
- [ ] Headings, bold, italic, lists render correctly
- [ ] Large documents (50+ KB) render without lag
- [ ] Save/load preserves all content (preview is cosmetic)
- [ ] Window resize maintains split ratio

---

## Alternative: Minimal Approach

If full split-pane feels too heavy, consider **hybrid rendering** (like markdown-reader):

```
┌─────────────────────────────────────────────┐
│ # Heading                                   │
│ Paragraph text with **bold** and *italic*   │
│                                             │
│ Here's code:                                │
│ |fn main() {                                │
│ │    println!("hello");                     │
│ │}                                          │
│                                             │
│ The block under cursor shows raw markdown, │
│ rest is rendered.                           │
└─────────────────────────────────────────────┘
```

This renders everything except the current editing block, reducing UI complexity while still providing live feedback.

---

## Effort Estimate

| Phase | Hours | Complexity |
|-------|-------|-----------|
| 1. Basic markdown parsing | 2-3 | Low |
| 2. Syntax highlighting | 2-3 | Medium |
| 3. Split-pane layout | 1-2 | Low |
| 4. Scroll sync | 2-3 | Medium |
| 5. Performance tuning | 2-4 | Medium-High |
| **Total** | **9-15** | **Medium** |

---

## Next Steps

1. **Start with Phase 1** — Get pulldown-cmark working, parse markdown, render basic styling
2. **Test incrementally** — Don't wait for all phases before testing
3. **Profile early** — Check performance after Phase 2
4. **Get feedback** — Try the split-pane UX with sample content before investing in sync logic
5. **Consider the hybrid approach** — If split-pane feels cramped, try hybrid rendering instead

---

## Questions to Consider

- Do you want vim keybindings in the editor (like micro)?
- Should code blocks be indented in the preview to match rendering?
- What markdown features are most important? (headings, code, lists, tables?)
- Preferred color scheme for syntax highlighting?
- Should preview update on every keystroke or debounce?

---

## Resources

- [pulldown-cmark docs](https://docs.rs/pulldown-cmark/)
- [syntect tutorial](https://github.com/trishume/syntect)
- [ratatui examples](https://github.com/ratatui/ratatui/tree/main/examples)
- [MDTui source](https://github.com/crate-ci/mdtui)
- [Markdown spec](https://spec.commonmark.org/)
