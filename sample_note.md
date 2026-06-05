# Heading 1 — Project Notes

## Heading 2 — Subsection

### Heading 3 — Details

Plain paragraph text. This sentence has **bold text**, *italic text*, ***bold italic***, and ~~strikethrough~~. Here is some `inline code` in a sentence.

A second paragraph to confirm blank-line spacing between blocks works as expected.

---

Unordered list with nesting:

- First item
- Second item
  - Nested item A
  - Nested item B
    - Deeply nested item
- Third item

Ordered list with real numbering:

1. Step one
2. Step two
3. Step three
   1. Sub-step a
   2. Sub-step b

> This is a blockquote.
> It can span multiple lines and should render italic/gray.
>> And this is a nested blockquote.

A fenced Rust code block (should be syntax highlighted):

```rust
fn main() {
    let name = "world";
    println!("Hello, {}!", name);
    for i in 0..3 {
        println!("count = {}", i);
    }
}
```

A Python code block:

```python
def greet(name: str) -> str:
    return f"Hello, {name}!"

print(greet("editor"))
```

A table (rendered by the dedicated table renderer):

| Feature        | Status | Notes              |
|----------------|--------|--------------------|
| Headings       | done   | H1–H6 colored      |
| Bold / italic  | done   | now actually styled |
| Code blocks    | done   | syntect highlight  |
| Tables         | done   | column-aligned     |

Final line of the note.
