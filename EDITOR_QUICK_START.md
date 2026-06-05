# Live Markdown Editor: Quick Start Guide

## What You're Building

Transform your notes editor from this:

```
┌────────────────────────────────────┐
│ Edit Note                          │
├────────────────────────────────────┤
│ # My Heading                       │
│ This is **bold** and *italic*      │
│ Here is `inline code`              │
│                                    │
│ ``` rust                           │
│ fn main() {                        │
│     println!("hello");             │
│ }                                  │
│ ```                                │
│                                    │
│ |cursor|here                       │
└────────────────────────────────────┘
```

...to this:

```
┌─────────────────────────┬──────────────────────┐
│ Edit                    │ Preview              │
├─────────────────────────┼──────────────────────┤
│ # My Heading            │ My Heading           │
│ This is **bold** and    │ This is bold and     │
│ *italic*                │ italic               │
│ Here is `inline code`   │ Here is inline code  │
│                         │                      │
│ ``` rust                │ ┌─ code ─           │
│ fn main() {             │ │ fn main() {        │
│     println!(...);      │ │     println!(...); │
│ }                       │ │ }                  │
│ ```                     │ └─────────           │
│                         │                      │
│ |cursor|here            │                      │
└─────────────────────────┴──────────────────────┘
```

## Files You Need to Create/Modify

### 1. Update `Cargo.toml`

```toml
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

# NEW DEPENDENCIES
pulldown-cmark = "0.11"    # Markdown parsing
syntect = "5.1"            # Syntax highlighting
ansi-to-tui = "3.0"        # Color conversion
parking_lot = "0.12"       # Fast locks (optional)
```

**Size impact**: ~+300KB to binary (acceptable for TUI app)

### 2. Create `src/markdown_renderer.rs`

Copy the entire `markdown_renderer` module from `EDITOR_IMPLEMENTATION_TEMPLATE.rs`.

Key functions:
- `MarkdownRenderer::new()` — Initialize
- `MarkdownRenderer::render(&mut self, content: &str) -> Vec<Line>` — Main rendering

### 3. Create `src/code_highlighter.rs`

Copy the entire `code_highlighter` module from `EDITOR_IMPLEMENTATION_TEMPLATE.rs`.

Key functions:
- `CodeHighlighter::new()` — Initialize with Solarized theme
- `CodeHighlighter::highlight(&self, code: &str, language: &str) -> Vec<Span>` — Highlight code

### 4. Update `src/main.rs` — Add Module Declarations

At the top of main.rs (after other `use` statements):

```rust
mod markdown_renderer;
mod code_highlighter;
```

### 5. Update `src/main.rs` — Extend App Struct

Find the `struct App` definition and add:

```rust
pub struct App {
    // ... all existing fields ...

    // NEW: Editor enhancement fields
    markdown_renderer: markdown_renderer::MarkdownRenderer,
    code_highlighter: code_highlighter::CodeHighlighter,
    preview_scroll: u16,
    last_preview_render: std::time::Instant,
    cached_preview: Vec<Line<'static>>,
}
```

### 6. Update `src/main.rs` — Initialize New Fields in App::new()

Find `impl App` and the `fn new()` method. Add to initialization:

```rust
impl App {
    fn new() -> Self {
        Self {
            // ... existing field initialization ...

            // NEW: Initialize editor enhancement fields
            markdown_renderer: markdown_renderer::MarkdownRenderer::new(),
            code_highlighter: code_highlighter::CodeHighlighter::new(),
            preview_scroll: 0,
            last_preview_render: std::time::Instant::now(),
            cached_preview: Vec::new(),
        }
    }
}
```

### 7. Update `src/main.rs` — Replace render_textarea_editor()

Find the function `fn render_textarea_editor(...)` and replace it with:

```rust
fn render_split_editor(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    use ratatui::layout::{Direction, Constraint};

    // Split area 50% left (editor) / 50% right (preview)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    // === LEFT PANE: Text Editor ===
    let inner_height = chunks[0].height.saturating_sub(2) as usize;
    let lines_display = textarea_lines_with_cursor(app, inner_height as u16);

    let editor_block = Block::default()
        .title("Edit")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let editor_panel = Paragraph::new(lines_display)
        .block(editor_block)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::Yellow))
        .scroll((app.textarea_scroll, 0));

    frame.render_widget(editor_panel, chunks[0]);

    // === RIGHT PANE: Markdown Preview ===
    let content = app.textarea.lines().join("\n");

    // Only re-render preview every 100ms to avoid performance issues
    if app.last_preview_render.elapsed() > std::time::Duration::from_millis(100) {
        app.cached_preview = app.markdown_renderer.render(&content);
        app.last_preview_render = std::time::Instant::now();
    }

    let preview_block = Block::default()
        .title("Preview")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::Cyan));

    let preview_panel = Paragraph::new(app.cached_preview.clone())
        .block(preview_block)
        .scroll((app.preview_scroll, 0));

    frame.render_widget(preview_panel, chunks[1]);

    // Optional: Render scrollbar
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
```

### 8. Update `src/main.rs` — Find render_editing_panel() and Modify

Find where you call `render_textarea_editor` inside `render_editing_panel`. Replace:

```rust
// OLD:
render_textarea_editor(frame, app, area, "Edit");

// NEW:
render_split_editor(frame, app, area);
```

### 9. Optional: Sync Preview Scroll with Editor

In the main event loop where you handle scrolling, add:

```rust
// When user scrolls in editor
KeyEvent { code: KeyCode::Up, .. } => {
    // After handling editor scroll...
    // Try to keep preview roughly in sync (simplified)
    app.preview_scroll = app.textarea_scroll;
}
```

## Testing Checklist

After implementing, test these:

- [ ] **Live preview updates** — Type `# Heading` and see it render as a heading in preview pane
- [ ] **Bold rendering** — Type `**bold text**` and verify it appears bold in preview
- [ ] **Code blocks** — Paste a code block with language tag (e.g., `\`\`\`rust`) and verify syntax highlighting
- [ ] **Italic text** — Type `*italic*` and verify formatting
- [ ] **Inline code** — Type `` `code` `` and verify different color in preview
- [ ] **Lists** — Type markdown list and verify bullet points render
- [ ] **Performance** — Edit a 50KB document and verify preview doesn't lag
- [ ] **Save/Load** — Verify saved content is unchanged (preview is cosmetic)
- [ ] **Cursor positioning** — Verify cursor works correctly in editor pane
- [ ] **Resize handling** — Resize terminal and verify split panes adjust

## Performance Tips

1. **Debouncing**: The code re-renders preview every 100ms, not every keystroke. Adjust if needed:
   ```rust
   Duration::from_millis(50)   // More responsive but less efficient
   Duration::from_millis(200)  // More efficient but slight lag
   ```

2. **Cache invalidation**: Renderer caches results. Clear if you notice stale renders:
   ```rust
   app.markdown_renderer.clear_cache();
   ```

3. **Large documents**: If you have 100KB+ notes, consider rendering only visible lines:
   ```rust
   // Instead of rendering all, only render lines in viewport
   let start_line = app.preview_scroll as usize;
   let end_line = (app.preview_scroll + area.height) as usize;
   // Render only those lines
   ```

## Next: Enhanced Features

Once the basic split-pane works, consider:

1. **Toggleable preview** — Press `P` to hide/show preview pane
2. **Horizontal split** — Option to stack vertically instead of side-by-side
3. **Markdown features** — Add support for tables, strikethrough, task lists
4. **Theme selection** — Let users pick syntax highlighting theme
5. **Code block language detection** — Auto-detect language if not specified
6. **Export to HTML** — Use `html::push_html()` from pulldown-cmark to export

## Troubleshooting

### Compilation fails with "unknown type pulldown-cmark"
→ Make sure you ran `cargo build` after updating Cargo.toml

### Preview doesn't update when I type
→ Check that `app.markdown_renderer.render()` is being called in `render_split_editor()`

### Memory usage increases over time
→ The renderer caches results. Add `app.markdown_renderer.clear_cache()` periodically

### Syntax highlighting doesn't work
→ Verify language tag is correct (e.g., `\`\`\`rust` not `\`\`\`rs`)

### Terminal colors look wrong
→ Check your terminal supports 24-bit color. Syntect defaults to Solarized which works well in most terminals.

## References

- [pulldown-cmark documentation](https://docs.rs/pulldown-cmark/)
- [syntect examples](https://github.com/trishume/syntect/tree/master/examples)
- [ratatui split layout examples](https://github.com/ratatui/ratatui/blob/main/examples/layout.rs)

## Files in This Package

- **EDITOR_IMPROVEMENT_PLAN.md** — Full architectural guide (read this first)
- **EDITOR_IMPLEMENTATION_TEMPLATE.rs** — Copy/paste code templates
- **EDITOR_QUICK_START.md** — This file (step-by-step integration)

---

**Estimated time to implement**: 1-2 hours for basic split-pane with markdown rendering.

Good luck! The result will feel much closer to a real markdown editor like Micro or VS Code.
