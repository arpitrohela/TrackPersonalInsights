# TrackPersonalInsights

**Your all-in-one terminal productivity powerhouse**: unified notes, daily planner, journal, habit tracker, finances, calorie counter, kanban board, and flashcard builder designed for speed and clarity.

A feature-rich, terminal-based productivity app combining hierarchical note taking, finance summaries with monthly/yearly totals, category filtering, and flashcard workflows all in aTUI(Terminal User Interface).

Change Terminal settings for changing fonts of the application.

The dictionary installation is a separate system-level package that users need to install manually with sudo apt install wamerican (Ubuntu) or sudo pacman -S words (Arch). Press F7 for spell check.

Press ? for help in the application.

---

## Keyboard Shortcuts

### Global

*   `Ctrl+Q`: Quit the application
*   `Esc`:
    *   Close Calendar picker
    *   Close Help overlay
    *   Close Spell check popup
    *   Close Card import help
    *   Close Global search overlay
    *   Exit Find and Replace mode
    *   Dismiss validation error popup
    *   Dismiss success popup
    *   Cancel editing without saving
*   `?`: Open Quick Help (when not editing)
*   `Ctrl+F`: Open Global Fuzzy Search overlay (when not editing)
*   `F7`: Run Spell Check (when editing)

### Calendar Picker

*   `Left Arrow`: Previous month
*   `Right Arrow`: Next month
*   `Up Arrow`: Next year
*   `Down Arrow`: Previous year
*   `0-9` (digits): Type day number to select a date

### Help Overlay

*   `Esc`: Close Help overlay
*   `Enter`: Close Help overlay
*   `Up Arrow`: Scroll up
*   `Down Arrow`: Scroll down
*   `PageUp`: Scroll up 10 lines
*   `PageDown`: Scroll down 10 lines
*   `Backspace`: Delete last character in search query
*   `Char(c)`: Push character to search query

### Spell Check Popup

*   `Esc`: Close Spell check popup
*   `Up Arrow`: Select previous suggestion
*   `Down Arrow`: Select next suggestion
*   `PageUp`: Scroll up 10 suggestions
*   `PageDown`: Scroll down 10 suggestions
*   `Enter`: Replace word with first suggestion
*   `A` (or `a`): Add word to custom dictionary
*   `1-9` (digits): Quick replace with numbered suggestion

### Card Import Help View

*   `Esc`: Close Card import help, clear editing input, and set edit target to None
*   `Enter`: Switch to editable path entry mode for Card Import
*   `Up Arrow`: Scroll up
*   `Down Arrow`: Scroll down
*   `PageUp`: Scroll up 10 lines
*   `PageDown`: Scroll down 10 lines

### Global Search Overlay

*   `Esc`: Close Global search overlay
*   `Enter`: Navigate to selected search result
*   `Up Arrow`: Select previous search result
*   `Down Arrow`: Select next search result
*   `Backspace`: Delete last character in search query
*   `Char(c)`: Push character to search query

### Find and Replace Mode (Notes View)

*   `Esc`: Exit Find and Replace mode
*   `Tab`: Toggle focus between Find input and Replace input
*   `Backspace`: Delete last character in focused input field
*   `Enter`: Perform replacement
*   `Char(c)`: Push character to focused input field

### Flashcards View (when not editing)

*   `Space`: Show/hide card answer (in review mode)
*   `0-5` (digits): Rate card quality (in review mode, after showing answer)
*   `Shift+Up Arrow`: Select previous card (with selection anchor)
*   `Shift+Down Arrow`: Select next card (with selection anchor)
*   `Up Arrow`: Select previous card
*   `Down Arrow`: Select next card
*   `Enter`: Enter review mode for selected card
*   `Esc`: Exit review mode

### Finance View (when summary is open and not editing)

*   `Up Arrow`: Scroll up
*   `Down Arrow`: Scroll down
*   `PageUp`: Scroll up 10 lines
*   `PageDown`: Scroll down 10 lines
*   `Left Arrow`: Select previous category
*   `Right Arrow`: Select next category

### Habits View (when summary is open and not editing)

*   `Up Arrow`: Scroll up
*   `Down Arrow`: Scroll down
*   `PageUp`: Scroll up 10 lines
*   `PageDown`: Scroll down 10 lines

### Notes View (scrolling when not editing and not in search)

*   `Up Arrow`: Scroll up
*   `Down Arrow`: Scroll down
*   `PageUp`: Scroll up 10 lines
*   `PageDown`: Scroll down 10 lines

### Editing (General, when in edit mode)

*   `Ctrl+S`: Save current editing content
*   `Ctrl+A`: Select all
*   `Ctrl+Z`: Undo
*   `Ctrl+Y`: Redo
*   `Ctrl+K`: Delete current line
*   `Delete` / `Backspace`: Clear all (if `Ctrl+A` is active)
*   All other standard text editing keys (e.g., character input, arrow keys, Enter, Tab, Home, End, PageUp, PageDown, Esc, F-keys) are handled by the text area.

---

## Screenshots

![Screenshot 1](Screenshots/Screenshot%20from%202025-12-27%2019-10-50.png)

![Screenshot 2](Screenshots/Screenshot%20from%202025-12-27%2019-10-58.png)

![Screenshot 3](Screenshots/Screenshot%20from%202025-12-27%2019-11-06.png)

![Screenshot 4](Screenshots/Screenshot%20from%202025-12-27%2019-11-15.png)

![Screenshot 5](Screenshots/Screenshot%20from%202025-12-27%2019-11-22.png)

![Screenshot 6](Screenshots/Screenshot%20from%202025-12-27%2019-11-29.png)

![Screenshot 7](Screenshots/Screenshot%20from%202025-12-27%2019-11-36.png)

![Screenshot 8](Screenshots/Screenshot%20from%202025-12-27%2019-11-46.png)

![Screenshot 9](Screenshots/Screenshot%20from%202025-12-27%2019-11-56.png)

![Screenshot 10](Screenshots/Screenshot%20from%202025-12-27%2019-12-07.png)
