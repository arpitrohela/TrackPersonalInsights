// ============================================================================
// MYNOTES - Multi-purpose Note-taking & Productivity Application
// ============================================================================
// 
// MODULE STRUCTURE:
// 1. Imports & Constants        - Dependencies and security limits
// 2. Data Structures            - App, Task, Habit, Finance, Calories, etc.
// 3. Serialization              - AppData persistence layer
// 4. Helpers                    - Validation, popup rendering, edit completion
// 5. Main Loop                  - Terminal UI initialization and event loop
// 6. Event Handling             - Keyboard, mouse, UI interactions
// 7. Editing Functions          - Input capture and data saving
// 8. Rendering (Drawing)        - All UI output functions
// 9. Parsers & Validators       - Input validation with error messages
// 10. Formatters & Templates    - Editor templates and display helpers
// 11. Utilities                 - Label functions and helper logic
//
// Each section is clearly marked with section headers for easy navigation.
// ============================================================================

use std::fs;
use std::io;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::{Datelike, Local, NaiveDate, NaiveTime};
use crossterm::{
    event::{
        self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
        MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, BorderType, Clear, List, ListItem, Paragraph, Wrap},
};
use std::collections::{BTreeSet, HashSet};
use tui_textarea::{CursorMove, Input, Key, TextArea};
use strsim::jaro_winkler;
use std::env;

// Persistence functions with security checks
const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024; // 50 MB max per file

fn get_data_dir() -> Result<PathBuf> {
    if let Some(data_home) = dirs::data_dir() {
        Ok(data_home.join("mynotes"))
    } else {
        Err(anyhow::anyhow!("Could not determine data directory"))
    }
}

fn get_current_year_file() -> Result<PathBuf> {
    let data_dir = get_data_dir()?;
    fs::create_dir_all(&data_dir)?;
    let year = Local::now().year();
    Ok(data_dir.join(format!("{}.bin", year)))
}

fn save_app_data(app: &App) -> Result<()> {
    let file_path = get_current_year_file()?;
    let serialized = bincode::serialize(&AppData::from_app(app))?;

    // Security: Check serialized size before writing
    if serialized.len() > MAX_FILE_SIZE as usize {
        return Err(anyhow::anyhow!(
            "Serialized data exceeds maximum size limit"
        ));
    }

    // Write to temporary file first, then atomic rename (safer)
    let temp_path = file_path.with_extension("bin.tmp");
    fs::write(&temp_path, serialized)?;
    fs::rename(temp_path, file_path)?;

    Ok(())
}

fn load_app_data() -> Result<App> {
    match get_current_year_file() {
        Ok(file_path) if file_path.exists() => {
            // Security: Check file size before reading
            let metadata = fs::metadata(&file_path)?;
            if metadata.len() > MAX_FILE_SIZE {
                return Err(anyhow::anyhow!(
                    "Data file exceeds maximum size limit - possible corruption or attack"
                ));
            }

            let data = fs::read(&file_path)?;

            // Security: Deserialize and validate
            let app_data: AppData = bincode::deserialize(&data).map_err(|e| {
                anyhow::anyhow!("Failed to deserialize data (file may be corrupted): {}", e)
            })?;

            // Security: Validate indices before using them
            let mut app = app_data.into_app();
            app.validate_indices();

            Ok(app)
        }
        _ => Ok(App::new()),
    }
}

// Serializable wrapper for App data (excludes UI state and regexes)
#[derive(serde::Serialize, serde::Deserialize)]
struct AppData {
    notebooks: Vec<Notebook>,
    tasks: Vec<Task>,
    journal_entries: Vec<JournalEntry>,
    habits: Vec<Habit>,
    finances: Vec<FinanceEntry>,
    calories: Vec<CalorieEntry>,
    kanban_cards: Vec<KanbanCard>,
    cards: Vec<Card>,
    current_notebook_idx: usize,
    current_section_idx: usize,
    current_page_idx: usize,
    current_task_idx: usize,
    current_habit_idx: usize,
    current_finance_idx: usize,
    current_calorie_idx: usize,
    current_kanban_card_idx: usize,
    current_card_idx: usize,
    current_journal_date: NaiveDate,
    view_mode: ViewMode,
}

impl AppData {
    fn from_app(app: &App) -> Self {
        Self {
            notebooks: app.notebooks.clone(),
            tasks: app.tasks.clone(),
            journal_entries: app.journal_entries.clone(),
            habits: app.habits.clone(),
            finances: app.finances.clone(),
            calories: app.calories.clone(),
            kanban_cards: app.kanban_cards.clone(),
            cards: app.cards.clone(),
            current_notebook_idx: app.current_notebook_idx,
            current_section_idx: app.current_section_idx,
            current_page_idx: app.current_page_idx,
            current_task_idx: app.current_task_idx,
            current_habit_idx: app.current_habit_idx,
            current_finance_idx: app.current_finance_idx,
            current_calorie_idx: app.current_calorie_idx,
            current_kanban_card_idx: app.current_kanban_card_idx,
            current_card_idx: app.current_card_idx,
            current_journal_date: app.current_journal_date,
            view_mode: app.view_mode,
        }
    }

    fn into_app(self) -> App {
        let mut app = App::new();
        app.notebooks = self.notebooks;
        app.tasks = self.tasks;
        app.journal_entries = self.journal_entries;
        app.habits = self.habits;
        app.finances = self.finances;
        app.calories = self.calories;
        app.kanban_cards = self.kanban_cards;
        app.cards = self.cards;
        app.current_notebook_idx = self
            .current_notebook_idx
            .min(app.notebooks.len().saturating_sub(1));
        app.current_section_idx = self.current_section_idx;
        app.current_page_idx = self.current_page_idx;
        app.current_task_idx = self.current_task_idx;
        app.current_habit_idx = self.current_habit_idx;
        app.current_finance_idx = self.current_finance_idx;
        app.current_calorie_idx = self.current_calorie_idx;
        app.current_kanban_card_idx = self.current_kanban_card_idx;
        app.current_card_idx = self.current_card_idx;
        app.current_journal_date = self.current_journal_date;
        app.view_mode = self.view_mode;
        app
    }
}

// ============================================================================
// HELPER FUNCTIONS - Consolidate repeated logic for cleaner code
// ============================================================================

/// Show validation error popup without saving
#[inline]
fn handle_validation_error(app: &mut App, error_msg: &str, context: &str) {
    app.show_validation_error = true;
    app.validation_error_message = format!("{} Error: {}\n\nPlease correct and try again.", context, error_msg);
}

/// Complete editing session and save data
#[inline]
fn complete_edit(app: &mut App) -> std::result::Result<(), Box<dyn std::error::Error>> {
    app.edit_target = EditTarget::None;
    app.inline_edit_mode = false;
    app.editing_input.clear();
    save_app_data(app)?;
    Ok(())
}

/// Get centered popup area for overlays
fn get_popup_area(frame_width: u16, frame_height: u16, width_percent: u16, height_percent: u16) -> Rect {
    let width = frame_width.saturating_mul(width_percent as u16) / 100;
    let height = frame_height.saturating_mul(height_percent as u16) / 100;
    let x = (frame_width.saturating_sub(width)) / 2;
    let y = (frame_height.saturating_sub(height)) / 2;
    Rect { x, y, width, height }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:?}");
    }
}

fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, event::EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal);

    disable_raw_mode().ok();
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        event::DisableMouseCapture
    )
    .ok();
    terminal.show_cursor().ok();

    res
}

// Hierarchical Note Structure

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Page {
    title: String,
    content: String,
    modified_at: NaiveDate,
    links: Vec<String>,  // URLs or note references
    images: Vec<String>, // Image file paths
}

impl Page {
    fn new(title: String) -> Self {
        Self {
            title,
            content: String::new(),
            modified_at: Local::now().date_naive(),
            links: Vec::new(),
            images: Vec::new(),
        }
    }

    fn extract_links_and_images(&mut self) {
        self.links.clear();
        self.images.clear();

        let mut seen_links = std::collections::BTreeSet::new();
        let mut seen_images = std::collections::BTreeSet::new();

        for line in self.content.lines() {
            // Extract HTTP/HTTPS URLs as links
            for part in line.split_whitespace() {
                let lower = part.to_lowercase();
                if (lower.starts_with("http://") || lower.starts_with("https://"))
                    && !seen_links.contains(part)
                {
                    seen_links.insert(part.to_string());
                    self.links.push(part.to_string());
                }
            }

            // Extract local file paths as potential images (only image extensions)
            if let Some(token) = extract_path(line) {
                let lower = token.to_lowercase();
                let is_image = [
                    ".png", ".jpg", ".jpeg", ".gif", ".bmp", ".webp", ".tiff", ".tif", ".svg",
                ]
                .iter()
                .any(|ext| lower.ends_with(ext));

                if is_image && !seen_images.contains(&token) {
                    seen_images.insert(token.clone());
                    self.images.push(token);
                }
            }
        }
    }

    // Automatically update the page title from the first few words of content
    fn update_title_from_content(&mut self) {
        if let Some(first_line) = self.content.lines().next() {
            let words: Vec<&str> = first_line.split_whitespace().take(6).collect();
            if !words.is_empty() {
                self.title = words.join(" ");
                // Truncate if too long (max 50 chars for display)
                if self.title.len() > 50 {
                    self.title.truncate(47);
                    self.title.push_str("...");
                }
            }
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Section {
    title: String,
    pages: Vec<Page>,
    created_at: NaiveDate,
}

impl Section {
    fn new(title: String) -> Self {
        Self {
            title,
            pages: Vec::new(),
            created_at: Local::now().date_naive(),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Notebook {
    title: String,
    sections: Vec<Section>,
    created_at: NaiveDate,
}

impl Notebook {
    fn new(title: String) -> Self {
        Self {
            title,
            sections: Vec::new(),
            created_at: Local::now().date_naive(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Task {
    title: String,
    description: String,
    completed: bool,
    priority: TaskPriority,
    due_date: Option<NaiveDate>,
    reminder_text: Option<String>,
    reminder_date: Option<NaiveDate>,
    #[serde(default)]
    reminder_time: Option<NaiveTime>,
    recurrence: Recurrence,
    created_at: NaiveDate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
enum TaskPriority {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum Recurrence {
    None,
    Daily,
    Weekly,
    Monthly,
    Range { start: NaiveDate, end: NaiveDate, time: Option<NaiveTime> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum KanbanStage {
    Todo,
    Doing,
    Done,
}

impl KanbanStage {
    fn label(&self) -> &'static str {
        match self {
            KanbanStage::Todo => "To Do",
            KanbanStage::Doing => "In Progress",
            KanbanStage::Done => "Done",
        }
    }

    fn color(&self) -> Color {
        match self {
            KanbanStage::Todo => Color::Cyan,
            KanbanStage::Doing => Color::Yellow,
            KanbanStage::Done => Color::Green,
        }
    }

    fn move_left(self) -> KanbanStage {
        match self {
            KanbanStage::Todo => KanbanStage::Todo,
            KanbanStage::Doing => KanbanStage::Todo,
            KanbanStage::Done => KanbanStage::Doing,
        }
    }

    fn move_right(self) -> KanbanStage {
        match self {
            KanbanStage::Todo => KanbanStage::Doing,
            KanbanStage::Doing => KanbanStage::Done,
            KanbanStage::Done => KanbanStage::Done,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct KanbanCard {
    title: String,
    note: String,
    stage: KanbanStage,
    created_at: NaiveDate,
}

impl KanbanCard {
    fn new(title: String, note: String) -> Self {
        Self {
            title,
            note,
            stage: KanbanStage::Todo,
            created_at: Local::now().date_naive(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum HabitStatus {
    Active,
    Paused,
}

fn default_habit_status() -> HabitStatus {
    HabitStatus::Active
}

fn default_habit_start_date() -> NaiveDate {
    Local::now().date_naive()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Habit {
    name: String,
    frequency: Recurrence, // use Recurrence for simplicity
    streak: u32,
    marks: HashSet<NaiveDate>,
    #[serde(default = "default_habit_status")]
    status: HabitStatus,
    #[serde(default = "default_habit_start_date")]
    start_date: NaiveDate,
    #[serde(default)]
    notes: String,
}

impl Habit {
    fn new(name: String) -> Self {
        Self {
            name,
            frequency: Recurrence::Daily,
            streak: 0,
            marks: HashSet::new(),
            status: HabitStatus::Active,
            start_date: Local::now().date_naive(),
            notes: String::new(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct FinanceEntry {
    date: NaiveDate,
    category: String,
    note: String,
    amount: f64,
}

impl FinanceEntry {
    fn new(date: NaiveDate, category: String, note: String, amount: f64) -> Self {
        Self {
            date,
            category,
            note,
            amount,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CalorieEntry {
    date: NaiveDate,
    meal: String,
    note: String,
    calories: u32,
}

impl CalorieEntry {
    fn new(date: NaiveDate, meal: String, note: String, calories: u32) -> Self {
        Self {
            date,
            meal,
            note,
            calories,
        }
    }
}

// Spaced Repetition Card (SM-2 Algorithm)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Card {
    front: String,
    back: String,
    card_type: CardType,
    created_at: NaiveDate,
    last_reviewed: Option<NaiveDate>,
    next_review: NaiveDate,
    ease_factor: f32, // SM-2 algorithm
    interval: u32,     // days
    repetitions: u32,
    tags: Vec<String>,
    collection: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
enum CardType {
    Basic,           // front/back
    Cloze,           // text with {{c1::deletion}}
    MultipleChoice,  // front with options in back
}

impl<'de> serde::Deserialize<'de> for CardType {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = <String as serde::Deserialize>::deserialize(deserializer)?;
        let norm = raw.trim().to_lowercase();
        match norm.as_str() {
            "basic" | "frontback" | "front_back" => Ok(CardType::Basic),
            "cloze" => Ok(CardType::Cloze),
            "mc" | "multiplechoice" | "multiple choice" | "multiple_choice" => {
                Ok(CardType::MultipleChoice)
            }
            other => Err(serde::de::Error::custom(format!(
                "unknown card_type '{}'; use basic, cloze, or mc/multiplechoice",
                other
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum CardFilter {
    All,
    New,                    // Never reviewed
    Due,                    // Due for review today
    Blackout,               // Quality 0: Complete failure
    Hard,                   // Quality 1-2: Difficult
    Medium,                 // Quality 3: Passing
    Easy,                   // Quality 4: Good
    Perfect,                // Quality 5: Perfect
    Mastered,               // High repetitions and ease
    Collection(String),     // By collection name
}

impl Card {
    fn new(front: String, back: String, card_type: CardType) -> Self {
        Self {
            front,
            back,
            card_type,
            created_at: Local::now().date_naive(),
            last_reviewed: None,
            next_review: Local::now().date_naive(),
            ease_factor: 2.5,
            interval: 0,
            repetitions: 0,
            tags: Vec::new(),
            collection: None,
        }
    }

    // SM-2 Algorithm for spaced repetition
    fn review(&mut self, quality: u8) {
        // quality: 0-5 (0=total blackout, 5=perfect response)
        let quality = quality.min(5) as f32;
        
        if quality < 3.0 {
            // Failed recall - reset
            self.repetitions = 0;
            self.interval = 1;
        } else {
            // Successful recall
            if self.repetitions == 0 {
                self.interval = 1;
            } else if self.repetitions == 1 {
                self.interval = 6;
            } else {
                self.interval = (self.interval as f32 * self.ease_factor).round() as u32;
            }
            self.repetitions += 1;
        }
        
        // Update ease factor
        self.ease_factor = (self.ease_factor + (0.1 - (5.0 - quality) * (0.08 + (5.0 - quality) * 0.02))).max(1.3);
        
        // Set next review date
        self.last_reviewed = Some(Local::now().date_naive());
        self.next_review = Local::now().date_naive() + chrono::Duration::days(self.interval as i64);
    }
    
    fn is_due(&self) -> bool {
        self.next_review <= Local::now().date_naive()
    }
}

impl Task {
    fn new(title: String, description: String) -> Self {
        Self {
            title,
            description,
            completed: false,
            priority: TaskPriority::Medium,
            due_date: None,
            reminder_text: None,
            reminder_date: None,
            reminder_time: None,
            recurrence: Recurrence::None,
            created_at: Local::now().date_naive(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct JournalEntry {
    date: NaiveDate,
    content: String,
    mood: Option<String>,
}

impl JournalEntry {
    fn new(date: NaiveDate) -> Self {
        Self {
            date,
            content: String::new(),
            mood: None,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum HierarchyLevel {
    Notebook,
    Section,
    Page,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum FindMode {
    Content,  // Current page content
    AllNotes, // All notebooks
}

#[allow(dead_code)]
enum EditTarget {
    None,
    NotebookTitle,
    SectionTitle,
    PageTitle,
    PageContent,
    JournalEntry,
    TaskTitle,
    TaskDetails,
    HabitNew,
    Habit,
    FinanceNew,
    Finance,
    CaloriesNew,
    Calories,
    KanbanNew,
    KanbanEdit,
    CardNew,
    CardEdit,
    CardImport,
    FindReplace,
}

#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum ViewMode {
    Notes,
    Planner,
    Journal,
    Habits,
    Finance,
    Calories,
    Kanban,
    Flashcards,
}

#[derive(Clone, Copy)]
enum SearchTarget {
    Note { notebook_idx: usize, section_idx: usize, page_idx: usize },
    Task { idx: usize },
    Journal { date: NaiveDate },
    Habit { idx: usize, date: Option<NaiveDate> },
    Finance { idx: usize, date: NaiveDate },
    Calorie { idx: usize, date: NaiveDate },
    Kanban { idx: usize },
    Card { idx: usize },
    Help,
}

#[derive(Clone)]
struct SearchHit {
    title: String,
    detail: String,
    target: SearchTarget,
    score: i32,
}

struct HelpTopic {
    title: &'static str,
    detail: &'static str,
}

const HELP_TOPICS: &[HelpTopic] = &[
    HelpTopic {
        title: "Open Help",
        detail: "Press ? to pop this help open, type to filter, Esc to hide it.",
    },
    HelpTopic {
        title: "Global Search",
        detail: "Hit Ctrl+F (or Search button), type what you need, move with ↑/↓, press Enter to jump there.",
    },
    HelpTopic {
        title: "Spell Check",
        detail: "Press F7 while editing. Walk results with ↑/↓, fix with Enter or keys 1-5, add with 'a'. For a real dictionary: point SPELL_DICT_PATH (or MYNOTES_SPELL_DICT) to your wordlist, or install /usr/share/dict/words on Linux. On Windows, you must supply a wordlist via the env var. Otherwise I fall back to the bundled basic list.",
    },
    HelpTopic {
        title: "Flashcard Bulk Actions",
        detail: "Go to List View, Shift+Up/Down to multi-select cards, then click Bulk Delete or Bulk Disassociate at the bottom.",
    },
    HelpTopic {
        title: "Flashcard Filters",
        detail: "Click Filter to cycle New, Due, difficulty bands, or collections. Bulk actions only touch what the current filter shows.",
    },
    HelpTopic {
        title: "Mouse Basics",
        detail: "Left-click to select, double-click a flashcard to review, middle-click a tree item to rename, right-click for context actions.",
    },
    HelpTopic {
        title: "Editing & Saving",
        detail: "Ctrl+S saves, Esc cancels, Space reveals a flashcard answer, Enter starts review from the card list.",
    },
    HelpTopic {
        title: "Add Images & Files",
        detail: "Paste a full path (e.g., /home/you/Pictures/pic.png or ~/Pictures/pic.png). Markdown links [alt](~/path) and [alt][~/path] work too. Leave edit mode and click the line to open it with your system app.",
    },
    HelpTopic {
        title: "Notes Section View",
        detail: "Click a section in the tree to read all its pages in one stream. Scroll to skim; pick a specific page to edit it.",
    },
    HelpTopic {
        title: "Cloud Backup & Sync",
        detail: "I save to ~/.local/share/mynotes/{year}.bin. Upload that file to Drive/Dropbox/OneDrive to back up. Pull it down on another machine to continue where you left off.",
    },
];

#[derive(Clone)]
struct SpellCheckResult {
    word: String,
    suggestions: Vec<String>,
    line_number: usize,
    column: usize,
}

struct SimpleDictionary {
    words: HashSet<String>,
}

impl SimpleDictionary {
    fn from_wordlist(list: &str) -> Self {
        let mut words = HashSet::new();
        for line in list.lines() {
            let w = line.trim().to_lowercase();
            if !w.is_empty() {
                words.insert(w);
            }
        }
        Self { words }
    }

    fn check_word(&self, word: &str, custom: &HashSet<String>) -> bool {
        let w = word.to_lowercase();
        custom.contains(&w) || self.words.contains(&w)
    }

    fn suggest(&self, word: &str, custom: &HashSet<String>, limit: usize) -> Vec<String> {
        let target = word.to_lowercase();
        let mut candidates: Vec<(f64, &str)> = self
            .words
            .iter()
            .filter(|w| !custom.contains(*w))
            .map(|w| (jaro_winkler(&target, w), w.as_str()))
            .collect();
        candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        candidates
            .into_iter()
            .take(limit)
            .map(|(_, w)| w.to_string())
            .collect()
    }
}

struct App {
    notebooks: Vec<Notebook>,
    current_notebook_idx: usize,
    current_section_idx: usize,
    current_page_idx: usize,
    hierarchy_level: HierarchyLevel,
    editing_input: String,
    textarea: TextArea<'static>, // Professional text editor
    edit_target: EditTarget,

    // View mode
    view_mode: ViewMode,

    // Planner & Journal
    tasks: Vec<Task>,
    current_task_idx: usize,
    journal_entries: Vec<JournalEntry>,
    current_journal_date: NaiveDate,
    // Habits
    habits: Vec<Habit>,
    current_habit_idx: usize,
    // Finance
    finances: Vec<FinanceEntry>,
    current_finance_idx: usize,
    // Calories
    calories: Vec<CalorieEntry>,
    current_calorie_idx: usize,
    // Kanban
    kanban_cards: Vec<KanbanCard>,
    current_kanban_card_idx: usize,
    // Flashcards (Spaced Repetition)
    cards: Vec<Card>,
    current_card_idx: usize,
    show_card_answer: bool,
    card_review_mode: bool,
    card_filter: CardFilter,
    card_selection_anchor: Option<usize>,
    selected_card_indices: BTreeSet<usize>,

    // UI areas for mouse support
    tree_items: Vec<(HierarchyLevel, usize, usize, usize, Rect)>,
    task_items: Vec<(usize, Rect)>, // (task_idx, clickable area)
    habit_items: Vec<(usize, Rect)>,
    finance_items: Vec<(usize, Rect)>,
    calorie_items: Vec<(usize, Rect)>,
    kanban_items: Vec<(usize, Rect)>,
    card_items: Vec<(usize, Rect)>,
    content_edit_area: Rect,
    add_notebook_btn: Rect,
    add_section_btn: Rect,
    add_page_btn: Rect,
    delete_btn: Rect,
    view_mode_btns: Vec<(ViewMode, Rect)>,
    add_task_btn: Rect,
    edit_task_btn: Rect,
    delete_task_btn: Rect,
    add_habit_btn: Rect,
    mark_done_btn: Rect,
    edit_habit_btn: Rect,
    delete_habit_btn: Rect,
    add_fin_btn: Rect,
    edit_fin_btn: Rect,
    delete_fin_btn: Rect,
    add_cal_btn: Rect,
    edit_cal_btn: Rect,
    delete_cal_btn: Rect,
    summary_btn: Rect,
    show_finance_summary: bool,
    finance_summary_scroll: u16,
    selected_finance_category_idx: usize,
    show_habits_summary: bool,
    habits_summary_scroll: u16,
    card_import_help_btn: Rect,
    card_import_edit_btn: Rect,
    show_card_import_help: bool,
    card_import_help_scroll: u16,
    card_import_help_text_area: Rect,
    // Store a pending path typed for import (saved via Ctrl+S)
    pending_card_import_path: Option<String>,
    add_kanban_btn: Rect,
    move_left_kanban_btn: Rect,
    move_right_kanban_btn: Rect,
    delete_kanban_btn: Rect,
    add_card_btn: Rect,
    review_card_btn: Rect,
    edit_card_btn: Rect,
    delete_card_btn: Rect,
    import_card_btn: Rect,
    show_answer_btn: Rect,
    quality_btns: Vec<(u8, Rect)>,
    filter_collection_btn: Rect,
    bulk_delete_btn: Rect,
    bulk_unassign_btn: Rect,
    prev_day_btn: Rect,
    next_day_btn: Rect,
    date_btn: Rect,
    today_btn: Rect,
    search_btn: Rect,
    search_result_items: Vec<(usize, Rect)>,

    // Content scrolling (Notes view)
    content_scroll: u16,

    // Selection state for editing
    selection_all: bool,

    // Editing caret support
    editing_cursor_line: usize,
    editing_cursor_col: usize,

    // Calendar picker state
    show_calendar: bool,
    calendar_year: i32,
    calendar_month: u32,
    calendar_day_rects: Vec<(u32, Rect)>, // (day, clickable rect)

    // Inline editing (click line to edit)
    editing_line_index: usize, // Which line is being edited
    inline_edit_mode: bool,    // Are we editing a single line inline?


    // Find and Replace
    find_text: String,
    replace_text: String,
    #[allow(dead_code)]
    find_mode: FindMode,
    find_input_focus: bool, // true = find field, false = replace field

    // Global fuzzy search
    show_global_search: bool,
    global_search_query: String,
    global_search_results: Vec<SearchHit>,
    global_search_selected: usize,
    show_help_overlay: bool,
    help_search_query: String,
    help_scroll: u16,

    // Validation error popup
    show_validation_error: bool,
    validation_error_message: String,
    // Success popup
    show_success_popup: bool,
    success_message: String,

    // Editor undo/redo stacks (only for content editor)
    undo_stack: Vec<String>,

    // Spell checker
    spell_dict: Option<SimpleDictionary>,
    show_spell_check: bool,
    spell_check_results: Vec<SpellCheckResult>,
    spell_check_selected: usize,
    spell_check_scroll: u16,
    custom_words: HashSet<String>,
    redo_stack: Vec<String>,
}

impl App {
    fn new() -> Self {
        let mut default_notebook = Notebook::new("My Notes".to_string());
        default_notebook
            .sections
            .push(Section::new("Getting Started".to_string()));
        if let Some(section) = default_notebook.sections.get_mut(0) {
            section
                .pages
                .push(Page::new("Welcome & Tutorial".to_string()));
            if let Some(page) = section.pages.get_mut(0) {
                page.content = r#"MYNOTES - COMPLETE TUTORIAL

NAVIGATION & SELECTION
------------------------------
- Click tree items to navigate notebooks/sections/pages
- Middle-click items to rename them
- Right-click items to delete them
- In Planner: Middle-click a task to mark it done/undone

TEXT EDITING IN CONTENT
------------------------------
- Click anywhere in the content area to start editing
- Type to add text
- Backspace: delete character before cursor
- Delete: delete character at cursor
- Enter: create new line
- Tab: indent (4 spaces)
- Ctrl+S: save your changes
- Esc: cancel editing without saving
- Ctrl+A: select all text
- Ctrl+K: delete current line

FORMATTING & FEATURES
------------------------------

Links & Files - Add an absolute or ~ path (supports spaces and quotes; also works with [alt][~/path/to/file]). Stay in read mode and click the line to open it with your system's default application (PDF, images, audio, archives, etc.).

Code Blocks - wrap with ```:
```rust
fn example() {
    println!("hello!");
}
```

KEYBOARD SHORTCUTS
------------------------------
Ctrl+S: Save current edit
Esc: Cancel current edit
Ctrl+A: Select all text (in editor)
Ctrl+K: Delete current line (in editor)
Ctrl+Z: Undo (in editor)
Ctrl+Y: Redo (in editor)
Ctrl+F: Global search
Up/Down/PgUp/PgDn: Scroll content
Mouse wheel: Scroll content (no edit mode needed!)

OTHER SECTIONS (tabs at top)
------------------------------
- PLANNER: Tasks, habits, reminders, goal tracking
- JOURNAL: Daily journal with calendar date picker
- FINANCE: Track expenses and income
- HEALTH: Log meals and calories
- KANBAN: Organize work in columns
- FLASHCARDS: Spaced repetition flashcards for memorization

FLASHCARDS (SPACED REPETITION)
--------------------------------
- Create flashcards with front/back content
- Uses SM-2 algorithm for optimal review scheduling
- Rate your recall: 0 (blackout) to 5 (perfect)
- Import flashcards from CSV or JSON files
- CSV format: front,back,type,collection (last 2 optional: type=basic/cloze/mc)
- JSON format: array of card objects
- Review Mode: Space to show answer, 0-5 keys to rate quality
- List View: Up/Down to navigate, Enter to review, Double-click to start review
- Single-click to select/highlight, double-click to enter review mode
- Press Esc to exit review mode
- Filters: Click 'Filter' to cycle through:
  • All - Show all flashcards
  • New - Never reviewed cards
  • Due - Cards scheduled for review today
  • Blackout - Complete failures (ease < 1.3)
  • Hard - Struggling cards (ease 1.3-1.8)
  • Medium - Average cards (ease 1.8-2.3)
  • Easy - Good cards (ease 2.3-2.8)
  • Perfect - Excellent cards (ease ≥ 2.8)
  • Mastered - Well-learned cards (5+ reviews, high ease)
  • Collections - Group related cards (use 'Set Collection' to assign)

TIPS & TRICKS
------------------------------
- All changes auto-save when you press **Ctrl+s**
- Use mouse wheel to scroll and read - NO NEED TO ENTER EDIT MODE!
- Click Date button in Journal to pick any date with calendar
- Create multiple notebooks for different purposes
- Use sections to organize notes by topic
- Mix text, code, tables, and flow steps on the same page!

CREATING TABLES:
- Start lines with | to create a table
- Use --- to create a separator row
- Example:
  | Column1 | Column2 |
  |---------|---------|
  | Value1  | Value2  |

CREATING FLOW STEPS:
- Use > to start a step, - for bullet details, 1. for numbered lists.
- Example:
  > First step
  - detail
  1. next

  Project Flow:
[Requirements] -> [Design] -> [Development] -> [Testing] -> [Release]

EXAMPLE - Mixed Content
------------------------------
Project Status Table:

| Task        | Status      | Owner |
|-------------|-------------|-------|
| Design      | Complete    | Ada   |
| Development | In Progress | Bob   |
| Testing     | Pending     | Chen  |

Happy note-taking! Start by clicking a page to edit, use mouse wheel to read. Tables and flow steps render automatically!"#
                    .to_string();
                page.extract_links_and_images();
            }
        }

        let default_kanban = vec![
            KanbanCard {
                title: "Sketch backlog".to_string(),
                note: "Status: Planned\nOwner: (assign)\nRoadblocks: None yet\nNext step: Draft 5-7 candidate tasks\nLinks/Refs: --".to_string(),
                stage: KanbanStage::Todo,
                created_at: Local::now().date_naive(),
            },
            KanbanCard {
                title: "Prioritize top 3".to_string(),
                note: "Status: In Progress\nOwner: (assign)\nRoadblocks: Waiting on estimates?\nNext step: Rank top 3, mark owners\nLinks/Refs: --".to_string(),
                stage: KanbanStage::Doing,
                created_at: Local::now().date_naive(),
            },
            KanbanCard {
                title: "Wrap a win".to_string(),
                note: "Status: Done (template)\nOwner: (assign)\nRoadblocks: None\nNext step: Demo & announce\nLinks/Refs: --".to_string(),
                stage: KanbanStage::Done,
                created_at: Local::now().date_naive(),
            },
        ];

        Self {
            notebooks: vec![default_notebook],
            current_notebook_idx: 0,
            current_section_idx: 0,
            current_page_idx: 0,
            hierarchy_level: HierarchyLevel::Notebook,
            editing_input: String::new(),
            edit_target: EditTarget::None,
            view_mode: ViewMode::Notes,
            tasks: Vec::new(),
            current_task_idx: 0,
            journal_entries: Vec::new(),
            current_journal_date: Local::now().date_naive(),
            habits: Vec::new(),
            current_habit_idx: 0,
            finances: Vec::new(),
            current_finance_idx: 0,
            calories: Vec::new(),
            current_calorie_idx: 0,
            kanban_cards: default_kanban,
            current_kanban_card_idx: 0,
            cards: Vec::new(),
            current_card_idx: 0,
            show_card_answer: false,
            card_review_mode: false,
            card_filter: CardFilter::All,
            card_selection_anchor: None,
            selected_card_indices: BTreeSet::new(),
            tree_items: Vec::new(),
            task_items: Vec::new(),
            habit_items: Vec::new(),
            finance_items: Vec::new(),
            calorie_items: Vec::new(),
            kanban_items: Vec::new(),
            card_items: Vec::new(),
            content_edit_area: Rect::default(),
            add_notebook_btn: Rect::default(),
            add_section_btn: Rect::default(),
            add_page_btn: Rect::default(),
            delete_btn: Rect::default(),
            view_mode_btns: Vec::new(),
            add_task_btn: Rect::default(),
            edit_task_btn: Rect::default(),
            delete_task_btn: Rect::default(),
            add_habit_btn: Rect::default(),
            mark_done_btn: Rect::default(),
            edit_habit_btn: Rect::default(),
            delete_habit_btn: Rect::default(),
            add_fin_btn: Rect::default(),
            edit_fin_btn: Rect::default(),
            delete_fin_btn: Rect::default(),
            summary_btn: Rect::default(),
            show_finance_summary: false,
            finance_summary_scroll: 0,
            selected_finance_category_idx: 0,
            show_habits_summary: false,
            habits_summary_scroll: 0,
            card_import_help_btn: Rect::default(),
            card_import_edit_btn: Rect::default(),
            show_card_import_help: false,
            card_import_help_scroll: 0,
               card_import_help_text_area: Rect::default(),
            pending_card_import_path: None,
            add_cal_btn: Rect::default(),
            edit_cal_btn: Rect::default(),
            delete_cal_btn: Rect::default(),
            add_kanban_btn: Rect::default(),
            move_left_kanban_btn: Rect::default(),
            move_right_kanban_btn: Rect::default(),
            delete_kanban_btn: Rect::default(),
            add_card_btn: Rect::default(),
            review_card_btn: Rect::default(),
            edit_card_btn: Rect::default(),
            delete_card_btn: Rect::default(),
            import_card_btn: Rect::default(),
            show_answer_btn: Rect::default(),
            quality_btns: Vec::new(),
            filter_collection_btn: Rect::default(),
            bulk_delete_btn: Rect::default(),
            bulk_unassign_btn: Rect::default(),
            prev_day_btn: Rect::default(),
            next_day_btn: Rect::default(),
            date_btn: Rect::default(),
            today_btn: Rect::default(),
            search_btn: Rect::default(),
            search_result_items: Vec::new(),
            content_scroll: 0,
            selection_all: false,
            editing_cursor_line: 0,
            editing_cursor_col: 0,
            find_text: String::new(),
            replace_text: String::new(),
            find_mode: FindMode::Content,
            find_input_focus: true,
            show_global_search: false,
            global_search_query: String::new(),
            global_search_results: Vec::new(),
            global_search_selected: 0,
            show_help_overlay: false,
            help_search_query: String::new(),
            help_scroll: 0,
            show_validation_error: false,
            validation_error_message: String::new(),
            show_success_popup: false,
            success_message: String::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            editing_line_index: 0,
            inline_edit_mode: false,
            textarea: TextArea::default(),
            show_calendar: false,
            calendar_year: Local::now().year(),
            calendar_month: Local::now().month(),
            calendar_day_rects: Vec::new(),
            spell_dict: Self::load_spell_dict(),
            show_spell_check: false,
            spell_check_results: Vec::new(),
            spell_check_selected: 0,
            spell_check_scroll: 0,
            custom_words: HashSet::new(),
        }
    }

    fn load_spell_dict() -> Option<SimpleDictionary> {
        // 1) User-provided path via env (preferred for large dictionaries)
        if let Ok(path) = std::env::var("SPELL_DICT_PATH").or_else(|_| std::env::var("MYNOTES_SPELL_DICT")) {
            if let Ok(contents) = fs::read_to_string(&path) {
                return Some(SimpleDictionary::from_wordlist(&contents));
            }
        }

        // 2) Common system dictionary locations (macOS/Linux)
        for path in ["/usr/share/dict/words", "/usr/share/dict/web2"] {
            if let Ok(contents) = fs::read_to_string(path) {
                return Some(SimpleDictionary::from_wordlist(&contents));
            }
        }

        // 3) Bundled fallback (basic list)
        const EN_WORDS: &str = include_str!("../assets/spell_en_basic.txt");
        Some(SimpleDictionary::from_wordlist(EN_WORDS))
    }

    fn current_notebook(&self) -> Option<&Notebook> {
        self.notebooks.get(self.current_notebook_idx)
    }

    fn current_notebook_mut(&mut self) -> Option<&mut Notebook> {
        self.notebooks.get_mut(self.current_notebook_idx)
    }

    fn current_section(&self) -> Option<&Section> {
        self.current_notebook()
            .and_then(|nb| nb.sections.get(self.current_section_idx))
    }

    fn current_section_mut(&mut self) -> Option<&mut Section> {
        let idx = self.current_section_idx;
        self.current_notebook_mut()
            .and_then(|nb| nb.sections.get_mut(idx))
    }

    fn current_page(&self) -> Option<&Page> {
        self.current_section()
            .and_then(|sec| sec.pages.get(self.current_page_idx))
    }

    fn current_page_mut(&mut self) -> Option<&mut Page> {
        let idx = self.current_page_idx;
        self.current_section_mut()
            .and_then(|sec| sec.pages.get_mut(idx))
    }

    fn add_notebook(&mut self) {
        self.notebooks.push(Notebook::new(format!(
            "Notebook {}",
            self.notebooks.len() + 1
        )));
        self.current_notebook_idx = self.notebooks.len() - 1;
        self.current_section_idx = 0;
        self.current_page_idx = 0;
    }

    fn add_section(&mut self) {
        if let Some(notebook) = self.current_notebook_mut() {
            notebook
                .sections
                .push(Section::new("New Section".to_string()));
            self.current_section_idx = notebook.sections.len() - 1;
            self.current_page_idx = 0;
        }
    }

    fn add_page(&mut self) {
        if let Some(section) = self.current_section_mut() {
            section.pages.push(Page::new("New Page".to_string()));
            self.current_page_idx = section.pages.len() - 1;
        }
    }

    fn delete_current(&mut self) {
        match self.hierarchy_level {
            HierarchyLevel::Notebook => {
                if self.notebooks.len() > 1 {
                    self.notebooks.remove(self.current_notebook_idx);
                    self.current_notebook_idx = self
                        .current_notebook_idx
                        .min(self.notebooks.len().saturating_sub(1));
                    self.current_section_idx = 0;
                    self.current_page_idx = 0;
                }
            }
            HierarchyLevel::Section => {
                let sec_idx = self.current_section_idx;
                if let Some(notebook) = self.current_notebook_mut() {
                    if notebook.sections.len() > 0 {
                        notebook.sections.remove(sec_idx);
                        self.current_section_idx =
                            sec_idx.min(notebook.sections.len().saturating_sub(1));
                        self.current_page_idx = 0;
                    }
                }
            }
            HierarchyLevel::Page => {
                let pg_idx = self.current_page_idx;
                if let Some(section) = self.current_section_mut() {
                    if section.pages.len() > 0 {
                        section.pages.remove(pg_idx);
                        self.current_page_idx = pg_idx.min(section.pages.len().saturating_sub(1));
                    }
                }
            }
        }
    }

    fn start_text_editing(&mut self, content: String) {
        // Initialize textarea with content and set editing input
        self.textarea = TextArea::new(content.lines().map(|s| s.to_string()).collect());
        self.editing_input = content;
        self.undo_stack.clear();
        self.redo_stack.clear();
        let line_count = self.editing_input.lines().count().saturating_sub(1);
        let last_len = self
            .editing_input
            .lines()
            .last()
            .map(|l| l.len())
            .unwrap_or(0);
        self.editing_cursor_line = line_count;
        self.editing_cursor_col = last_len;
        self.textarea
            .move_cursor(CursorMove::Jump(line_count as u16, last_len as u16));
        self.selection_all = false;
    }

    fn save_inline_edit(&mut self) {
        // Save an inline edit of a page content line
        // Get the edited content from textarea first
        let edited_content = self.textarea.lines().join("\n");
        let line_idx = self.editing_line_index;

        if let Some(page) = self.current_page_mut() {
            // Replace the specific line in the page content
            let lines: Vec<&str> = page.content.lines().collect();

            if line_idx < lines.len() {
                // Replacing an existing line - rebuild entire content
                let mut new_lines = Vec::new();
                for (i, line) in lines.iter().enumerate() {
                    if i == line_idx {
                        new_lines.push(edited_content.clone());
                    } else {
                        new_lines.push(line.to_string());
                    }
                }
                page.content = new_lines.join("\n");
            } else if line_idx == lines.len() {
                // Adding a new line at the end
                if !page.content.is_empty() && !page.content.ends_with('\n') {
                    page.content.push('\n');
                }
                page.content.push_str(&edited_content);
            }

            page.modified_at = Local::now().date_naive();
            page.extract_links_and_images();
            page.update_title_from_content();
        }
    }

    fn save_input(&mut self) {
        let input = self.editing_input.clone();
        match self.edit_target {
            EditTarget::None => {}
            EditTarget::NotebookTitle => {
                if let Some(notebook) = self.current_notebook_mut() {
                    notebook.title = input;
                }
            }
            EditTarget::SectionTitle => {
                if let Some(section) = self.current_section_mut() {
                    section.title = input;
                }
            }
            EditTarget::PageTitle => {
                if let Some(page) = self.current_page_mut() {
                    // Validate title length (max 200 characters)
                    page.title = if input.len() <= 200 {
                        input
                    } else {
                        input.chars().take(200).collect()
                    };
                    page.modified_at = Local::now().date_naive();
                }
            }
            EditTarget::PageContent => {
                if let Some(page) = self.current_page_mut() {
                    // Validate content length (max 100,000 characters)
                    page.content = if input.len() <= 100_000 {
                        input
                    } else {
                        input.chars().take(100_000).collect()
                    };
                    page.modified_at = Local::now().date_naive();
                    page.extract_links_and_images();
                    page.update_title_from_content();
                }
            }
            EditTarget::TaskTitle => {
                if !input.trim().is_empty() {
                    match parse_and_validate_task(&input, None) {
                        Ok(task) => {
                            self.tasks.push(task);
                            self.current_task_idx = self.tasks.len().saturating_sub(1);
                            let _ = complete_edit(self);
                            return;
                        }
                        Err(err) => {
                            handle_validation_error(self, &err, "Task");
                            return;
                        }
                    }
                }
            }
            EditTarget::TaskDetails => {
                if let Some(existing) = self.tasks.get(self.current_task_idx).cloned() {
                    match parse_and_validate_task(&input, Some(&existing)) {
                        Ok(updated) => {
                            if let Some(slot) = self.tasks.get_mut(self.current_task_idx) {
                                *slot = updated;
                            }
                            let _ = complete_edit(self);
                            return;
                        }
                        Err(err) => {
                            handle_validation_error(self, &err, "Task");
                            return;
                        }
                    }
                }
            }
            EditTarget::JournalEntry => {
                // Validate journal content length (max 50,000 characters)
                let validated_content = if input.len() <= 50_000 {
                    input.clone()
                } else {
                    input.chars().take(50_000).collect()
                };
                
                // Find or create journal entry for current date
                if let Some(entry) = self
                    .journal_entries
                    .iter_mut()
                    .find(|e| e.date == self.current_journal_date)
                {
                    entry.content = validated_content;
                } else {
                    let mut entry = JournalEntry::new(self.current_journal_date);
                    entry.content = validated_content;
                    self.journal_entries.push(entry);
                }
            }
            EditTarget::HabitNew => {
                match parse_and_validate_habit(&input, None, self.current_journal_date) {
                    Ok(habit) => {
                        self.habits.push(habit);
                        self.current_habit_idx = self.habits.len().saturating_sub(1);
                        let _ = complete_edit(self);
                        return;
                    }
                    Err(err) => {
                        handle_validation_error(self, &err, "Habit");
                        return;
                    }
                }
            }
            EditTarget::Habit => {
                if let Some(existing) = self.habits.get(self.current_habit_idx).cloned() {
                    match parse_and_validate_habit(&input, Some(&existing), existing.start_date) {
                        Ok(updated) => {
                            if let Some(slot) = self.habits.get_mut(self.current_habit_idx) {
                                *slot = updated;
                            }
                            let _ = complete_edit(self);
                            return;
                        }
                        Err(err) => {
                            handle_validation_error(self, &err, "Habit");
                            return;
                        }
                    }
                }
            }
            EditTarget::FinanceNew => {
                if let Some(entry) =
                    parse_finance_editor_content(&input, None, self.current_journal_date)
                {
                    self.finances.push(entry);
                    self.current_finance_idx = self.finances.len().saturating_sub(1);
                }
            }
            EditTarget::Finance => {
                if let Some(existing) = self.finances.get(self.current_finance_idx).cloned() {
                    if let Some(updated) =
                        parse_finance_editor_content(&input, Some(&existing), existing.date)
                    {
                        if let Some(slot) = self.finances.get_mut(self.current_finance_idx) {
                            *slot = updated;
                        }
                    }
                }
            }
            EditTarget::CaloriesNew => {
                if let Some(entry) =
                    parse_calorie_editor_content(&input, None, self.current_journal_date)
                {
                    self.calories.push(entry);
                    self.current_calorie_idx = self.calories.len().saturating_sub(1);
                }
            }
            EditTarget::Calories => {
                if let Some(existing) = self.calories.get(self.current_calorie_idx).cloned() {
                    if let Some(updated) =
                        parse_calorie_editor_content(&input, Some(&existing), existing.date)
                    {
                        if let Some(slot) = self.calories.get_mut(self.current_calorie_idx) {
                            *slot = updated;
                        }
                    }
                }
            }
            EditTarget::KanbanNew => {
                if let Some(card) = parse_kanban_editor_content(&input, None) {
                    self.kanban_cards.push(card);
                    self.current_kanban_card_idx = self.kanban_cards.len().saturating_sub(1);
                }
            }
            EditTarget::KanbanEdit => {
                if let Some(existing) = self.kanban_cards.get(self.current_kanban_card_idx).cloned() {
                    if let Some(updated) = parse_kanban_editor_content(&input, Some(&existing)) {
                        if let Some(slot) =
                            self.kanban_cards.get_mut(self.current_kanban_card_idx)
                        {
                            *slot = updated;
                        }
                    }
                }
            }
            EditTarget::CardNew => {
                if let Some(card) = parse_card_editor_content_structured(&input, None) {
                    self.cards.push(card);
                    self.current_card_idx = self.cards.len().saturating_sub(1);
                }
            }
            EditTarget::CardEdit => {
                if let Some(existing) = self.cards.get(self.current_card_idx).cloned() {
                    if let Some(updated) = parse_card_editor_content_structured(&input, Some(&existing)) {
                        if let Some(slot) = self.cards.get_mut(self.current_card_idx) {
                            *slot = updated;
                        }
                    }
                }
            }
            EditTarget::CardImport => {
                // Do NOT import here. Only store the path for later, and keep editing open.
                // Import should be triggered exclusively by the "Start Import" button.
                let path = input.trim().to_string();
                if !path.is_empty() {
                    self.pending_card_import_path = Some(path);
                }
                // Return early: do not clear editing state for CardImport on Ctrl+S
                return;
            }
            EditTarget::FindReplace => {
                // Find+Replace handled differently via keyboard events, not save_input
            }
        }
        self.edit_target = EditTarget::None;
        self.inline_edit_mode = false;
        self.editing_input.clear();
        self.editing_cursor_line = 0;
        self.editing_cursor_col = 0;
        // Auto-save after data changes
        let _ = save_app_data(self);
    }

    fn is_editing(&self) -> bool {
        !matches!(self.edit_target, EditTarget::None) || self.inline_edit_mode
    }

    fn clear_card_selection(&mut self) {
        self.selected_card_indices.clear();
        self.card_selection_anchor = None;
    }

    fn filtered_card_indices(&self) -> Vec<usize> {
        self
            .cards
            .iter()
            .enumerate()
            .filter(|(_, card)| matches_filter(self, card))
            .map(|(idx, _)| idx)
            .collect()
    }

    fn update_card_selection(&mut self, anchor: usize, current: usize) {
        let visible = self.filtered_card_indices();
        let anchor_pos = visible.iter().position(|idx| *idx == anchor);
        let current_pos = visible.iter().position(|idx| *idx == current);
        self.selected_card_indices.clear();
        if let (Some(a), Some(c)) = (anchor_pos, current_pos) {
            let (start, end) = if a <= c { (a, c) } else { (c, a) };
            for idx in visible[start..=end].iter() {
                self.selected_card_indices.insert(*idx);
            }
        } else {
            self.selected_card_indices.insert(current);
        }
    }

    fn validate_indices(&mut self) {
        // Validate and clamp all indices to prevent out-of-bounds access
        if self.current_notebook_idx >= self.notebooks.len() {
            self.current_notebook_idx = 0;
        }
        if self.current_section_idx
            >= self
                .current_notebook()
                .map(|n| n.sections.len())
                .unwrap_or(0)
        {
            self.current_section_idx = 0;
        }
        if self.current_page_idx >= self.current_section().map(|s| s.pages.len()).unwrap_or(0) {
            self.current_page_idx = 0;
        }
        if self.current_task_idx >= self.tasks.len() {
            self.current_task_idx = 0;
        }
        if self.current_habit_idx >= self.habits.len() {
            self.current_habit_idx = 0;
        }
        if self.current_finance_idx >= self.finances.len() {
            self.current_finance_idx = 0;
        }
        if self.current_calorie_idx >= self.calories.len() {
            self.current_calorie_idx = 0;
        }
        if self.current_kanban_card_idx >= self.kanban_cards.len() {
            self.current_kanban_card_idx = 0;
        }
        if self.current_card_idx >= self.cards.len() {
            self.current_card_idx = 0;
        }
        self.clear_card_selection();
    }

    fn fuzzy_score(&self, haystack: &str, needle: &str) -> i32 {
        if needle.is_empty() {
            return 0;
        }
        let h = haystack.to_lowercase();
        let n = needle.to_lowercase();
        let jw = (jaro_winkler(&h, &n) * 1000.0) as i32;
        let contains_boost = if h.contains(&n) { 400 } else { 0 };
        let start_boost = if h.starts_with(&n) { 200 } else { 0 };
        let eq_boost = if h == n { 800 } else { 0 };
        jw + contains_boost + start_boost + eq_boost
    }

    fn run_spell_check(&mut self) {
        self.spell_check_results.clear();
        self.spell_check_selected = 0;
        self.spell_check_scroll = 0;

        let Some(dict) = &self.spell_dict else {
            self.show_validation_error = true;
            self.validation_error_message = "Spell check dictionary not available".to_string();
            return;
        };

        let text = self.textarea.lines().join("\n");
        let lines: Vec<&str> = text.lines().collect();

        for (line_idx, line) in lines.iter().enumerate() {
            let mut col = 0;
            for word in line.split(|c: char| !c.is_alphanumeric()) {
                if !word.is_empty() && word.len() > 1 {
                    let word_lower = word.to_lowercase();
                    // Skip if in custom dictionary
                    if !self.custom_words.contains(&word_lower) {
                        if !dict.check_word(&word_lower, &self.custom_words) {
                            let suggestions = dict.suggest(&word_lower, &self.custom_words, 5);
                            self.spell_check_results.push(SpellCheckResult {
                                word: word.to_string(),
                                suggestions,
                                line_number: line_idx + 1,
                                column: col,
                            });
                        }
                    }
                }
                col += word.len() + 1;
            }
        }

        if self.spell_check_results.is_empty() {
            self.show_success_popup = true;
            self.success_message = "No spelling errors found!".to_string();
        } else {
            self.show_spell_check = true;
        }
    }

    fn replace_word_in_textarea(&mut self, old_word: &str, new_word: &str) {
        let text = self.textarea.lines().join("\n");
        // Simple replace - first occurrence
        let new_text = text.replacen(old_word, new_word, 1);
        let lines: Vec<String> = new_text.lines().map(|s| s.to_string()).collect();
        let (row, _col) = self.textarea.cursor();
        self.textarea = TextArea::new(lines);
        self.textarea
            .move_cursor(CursorMove::Jump(row as u16, 0));
        self.editing_input = self.textarea.lines().join("\n");
    }

    fn navigate_search_target(&mut self, target: SearchTarget) {
        match target {
            SearchTarget::Note { notebook_idx, section_idx, page_idx } => {
                self.current_notebook_idx = notebook_idx.min(self.notebooks.len().saturating_sub(1));
                self.current_section_idx = section_idx;
                self.current_page_idx = page_idx;
                self.hierarchy_level = HierarchyLevel::Page;
                self.view_mode = ViewMode::Notes;
            }
            SearchTarget::Task { idx } => {
                self.current_task_idx = idx.min(self.tasks.len().saturating_sub(1));
                self.view_mode = ViewMode::Planner;
            }
            SearchTarget::Journal { date } => {
                self.current_journal_date = date;
                self.view_mode = ViewMode::Journal;
            }
            SearchTarget::Habit { idx, date } => {
                self.current_habit_idx = idx.min(self.habits.len().saturating_sub(1));
                if let Some(d) = date { self.current_journal_date = d; }
                self.view_mode = ViewMode::Habits;
            }
            SearchTarget::Finance { idx, date } => {
                self.current_finance_idx = idx.min(self.finances.len().saturating_sub(1));
                self.current_journal_date = date;
                self.view_mode = ViewMode::Finance;
            }
            SearchTarget::Calorie { idx, date } => {
                self.current_calorie_idx = idx.min(self.calories.len().saturating_sub(1));
                self.current_journal_date = date;
                self.view_mode = ViewMode::Calories;
            }
            SearchTarget::Kanban { idx } => {
                self.current_kanban_card_idx = idx.min(self.kanban_cards.len().saturating_sub(1));
                self.view_mode = ViewMode::Kanban;
            }
            SearchTarget::Card { idx } => {
                self.current_card_idx = idx.min(self.cards.len().saturating_sub(1));
                self.view_mode = ViewMode::Flashcards;
                self.card_review_mode = true;
                self.show_card_answer = false;
            }
            SearchTarget::Help => {
                self.show_help_overlay = true;
                self.help_search_query.clear();
            }
        }
    }

    fn rebuild_global_search_results(&mut self) {
        self.global_search_results.clear();
        self.search_result_items.clear();

        let q = self.global_search_query.trim();
        if q.is_empty() {
            return;
        }
        let q_lower = q.to_lowercase();

        let mut hits: Vec<SearchHit> = Vec::new();

        // Notes
        for (nb_idx, nb) in self.notebooks.iter().enumerate() {
            for (sec_idx, sec) in nb.sections.iter().enumerate() {
                for (pg_idx, page) in sec.pages.iter().enumerate() {
                    let title = format!("Note: {}", page.title);
                    let detail = format!("{}/{}", nb.title, sec.title);
                    let score = self.fuzzy_score(&page.title, q) + self.fuzzy_score(&detail, q);
                    if score > 350 {
                        hits.push(SearchHit {
                            title,
                            detail,
                            target: SearchTarget::Note { notebook_idx: nb_idx, section_idx: sec_idx, page_idx: pg_idx },
                            score,
                        });
                    }
                }
            }
        }

        // Tasks
        for (idx, task) in self.tasks.iter().enumerate() {
            let detail = task
                .description
                .lines()
                .next()
                .unwrap_or("")
                .to_string();
            let score = self.fuzzy_score(&task.title, q) + self.fuzzy_score(&detail, q);
            if score > 350 {
                hits.push(SearchHit {
                    title: format!("Task: {}", task.title),
                    detail,
                    target: SearchTarget::Task { idx },
                    score,
                });
            }
        }

        // Journal entries
        for entry in self.journal_entries.iter() {
            let first_line = entry.content.lines().next().unwrap_or("");
            let score = self.fuzzy_score(&entry.date.to_string(), q) + self.fuzzy_score(first_line, q);
            if score > 300 {
                hits.push(SearchHit {
                    title: format!("Journal {}", entry.date),
                    detail: first_line.to_string(),
                    target: SearchTarget::Journal { date: entry.date },
                    score,
                });
            }
        }

        // Habits
        for (idx, habit) in self.habits.iter().enumerate() {
            let score = self.fuzzy_score(&habit.name, q);
            if score > 350 {
                hits.push(SearchHit {
                    title: format!("Habit: {}", habit.name),
                    detail: format!("{} • {}", habit_status_label(habit.status), recurrence_label(habit.frequency)),
                    target: SearchTarget::Habit { idx, date: None },
                    score,
                });
            }
        }

        // Finance
        for (idx, fin) in self.finances.iter().enumerate() {
            let title = format!("Finance {} {:.2}", fin.category, fin.amount);
            let detail = fin.note.lines().next().unwrap_or("").to_string();
            let score = self.fuzzy_score(&title, q) + self.fuzzy_score(&detail, q);
            if score > 300 {
                hits.push(SearchHit {
                    title,
                    detail,
                    target: SearchTarget::Finance { idx, date: fin.date },
                    score,
                });
            }
        }

        // Calories
        for (idx, cal) in self.calories.iter().enumerate() {
            let title = format!("Calories {} {} kcal", cal.meal, cal.calories);
            let detail = cal.note.lines().next().unwrap_or("").to_string();
            let score = self.fuzzy_score(&title, q) + self.fuzzy_score(&detail, q);
            if score > 300 {
                hits.push(SearchHit {
                    title,
                    detail,
                    target: SearchTarget::Calorie { idx, date: cal.date },
                    score,
                });
            }
        }

        // Kanban
        for (idx, card) in self.kanban_cards.iter().enumerate() {
            let score = self.fuzzy_score(&card.title, q) + self.fuzzy_score(&card.note, q);
            if score > 300 {
                hits.push(SearchHit {
                    title: format!("Kanban: {}", card.title),
                    detail: card.note.lines().next().unwrap_or("").to_string(),
                    target: SearchTarget::Kanban { idx },
                    score,
                });
            }
        }

        // Flashcards (spaced repetition)
        for (idx, card) in self.cards.iter().enumerate() {
            let score = self.fuzzy_score(&card.front, q) + self.fuzzy_score(&card.back, q);
            if score > 300 {
                hits.push(SearchHit {
                    title: format!("Flashcard: {}", card.front.chars().take(50).collect::<String>()),
                    detail: card.back.chars().take(50).collect::<String>(),
                    target: SearchTarget::Card { idx },
                    score,
                });
            }
        }

        if q_lower.contains("help") || q_lower.contains("shortcut") || q_lower.contains("tips") || q.contains('?') {
            hits.push(SearchHit {
                title: "Help & Shortcuts".to_string(),
                detail: "Open the quick tips panel (press ?).".to_string(),
                target: SearchTarget::Help,
                score: self.fuzzy_score("help shortcuts", q) + 800,
            });
        }

        hits.sort_by(|a, b| b.score.cmp(&a.score));
        hits.truncate(100);
        self.global_search_selected = 0;
        self.global_search_results = hits;
    }
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let mut app = load_app_data().unwrap_or_else(|_| App::new());
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|frame| draw(frame, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::from_secs(0));

        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if handle_key(&mut app, key)? {
                        // Save before exit
                        let _ = save_app_data(&app);
                        break;
                    }
                }
                Event::Mouse(mouse) => handle_mouse(&mut app, mouse),
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Ok(true);
    }

    // Calendar picker navigation
    if app.show_calendar {
        match key.code {
            KeyCode::Esc => {
                app.show_calendar = false;
            }
            KeyCode::Left => {
                if app.calendar_month > 1 {
                    app.calendar_month -= 1;
                } else {
                    app.calendar_month = 12;
                    app.calendar_year -= 1;
                }
            }
            KeyCode::Right => {
                if app.calendar_month < 12 {
                    app.calendar_month += 1;
                } else {
                    app.calendar_month = 1;
                    app.calendar_year += 1;
                }
            }
            KeyCode::Up => {
                app.calendar_year += 1;
            }
            KeyCode::Down => {
                app.calendar_year -= 1;
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                // Allow typing day number (1-31)
                let digit = c.to_digit(10).unwrap() as u32;
                if let Some(date) = NaiveDate::from_ymd_opt(app.calendar_year, app.calendar_month, digit) {
                    app.current_journal_date = date;
                    app.show_calendar = false;
                }
            }
            _ => {}
        }
        return Ok(false);
    }

    if app.show_help_overlay {
        match key.code {
            KeyCode::Esc => {
                app.show_help_overlay = false;
                app.help_search_query.clear();
                app.help_scroll = 0;
            }
            KeyCode::Enter => {
                app.show_help_overlay = false;
                app.help_search_query.clear();
                app.help_scroll = 0;
            }
            KeyCode::Up => {
                app.help_scroll = app.help_scroll.saturating_sub(1);
            }
            KeyCode::Down => {
                app.help_scroll = app.help_scroll.saturating_add(1);
            }
            KeyCode::PageUp => {
                app.help_scroll = app.help_scroll.saturating_sub(10);
            }
            KeyCode::PageDown => {
                app.help_scroll = app.help_scroll.saturating_add(10);
            }
            KeyCode::Backspace => {
                app.help_search_query.pop();
                app.help_scroll = 0;
            }
            KeyCode::Char(c) => {
                if c == '?' {
                    app.show_help_overlay = false;
                    app.help_search_query.clear();
                    app.help_scroll = 0;
                } else {
                    app.help_search_query.push(c);
                    app.help_scroll = 0;
                }
            }
            _ => {}
        }
        return Ok(false);
    }

    // Spell check popup keyboard handling
    if app.show_spell_check {
        match key.code {
            KeyCode::Esc => {
                app.show_spell_check = false;
                return Ok(false);
            }
            KeyCode::Up => {
                app.spell_check_selected = app.spell_check_selected.saturating_sub(1);
                return Ok(false);
            }
            KeyCode::Down => {
                if app.spell_check_selected + 1 < app.spell_check_results.len() {
                    app.spell_check_selected += 1;
                }
                return Ok(false);
            }
            KeyCode::PageUp => {
                app.spell_check_scroll = app.spell_check_scroll.saturating_sub(10);
                return Ok(false);
            }
            KeyCode::PageDown => {
                app.spell_check_scroll = app.spell_check_scroll.saturating_add(10);
                return Ok(false);
            }
            KeyCode::Enter => {
                // Replace with first suggestion
                if let Some(result) = app.spell_check_results.get(app.spell_check_selected).cloned() {
                    if let Some(replacement) = result.suggestions.first() {
                        app.replace_word_in_textarea(&result.word, replacement);
                        app.spell_check_results.remove(app.spell_check_selected);
                        if app.spell_check_selected >= app.spell_check_results.len() {
                            app.spell_check_selected = app.spell_check_results.len().saturating_sub(1);
                        }
                        if app.spell_check_results.is_empty() {
                            app.show_spell_check = false;
                        }
                    }
                }
                return Ok(false);
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                // Add word to custom dictionary
                if let Some(result) = app.spell_check_results.get(app.spell_check_selected).cloned() {
                    app.custom_words.insert(result.word.clone());
                    app.spell_check_results.remove(app.spell_check_selected);
                    if app.spell_check_selected >= app.spell_check_results.len() {
                        app.spell_check_selected = app.spell_check_results.len().saturating_sub(1);
                    }
                    if app.spell_check_results.is_empty() {
                        app.show_spell_check = false;
                    }
                }
                return Ok(false);
            }
            KeyCode::Char(c @ '1'..='9') => {
                // Quick replace with numbered suggestion
                let num = c.to_digit(10).unwrap() as usize;
                if let Some(result) = app.spell_check_results.get(app.spell_check_selected).cloned() {
                    if let Some(replacement) = result.suggestions.get(num - 1) {
                        app.replace_word_in_textarea(&result.word, replacement);
                        app.spell_check_results.remove(app.spell_check_selected);
                        if app.spell_check_selected >= app.spell_check_results.len() {
                            app.spell_check_selected = app.spell_check_results.len().saturating_sub(1);
                        }
                        if app.spell_check_results.is_empty() {
                            app.show_spell_check = false;
                        }
                    }
                }
                return Ok(false);
            }
            _ => {}
        }
        return Ok(false);
    }

    // Card import help view keyboard handling (read-only help with scrolling)
    if app.show_card_import_help && matches!(app.edit_target, EditTarget::CardImport) {
        match key.code {
            KeyCode::Esc => {
                app.show_card_import_help = false;
                app.edit_target = EditTarget::None;
                app.editing_input.clear();
                return Ok(false);
            }
            KeyCode::Enter => {
                // Switch to editable path entry
                app.show_card_import_help = false;
                app.editing_input.clear();
                start_editing(app, EditTarget::CardImport, String::new());
                return Ok(false);
            }
            KeyCode::Up => {
                app.card_import_help_scroll = app.card_import_help_scroll.saturating_sub(1);
                return Ok(false);
            }
            KeyCode::Down => {
                app.card_import_help_scroll = app.card_import_help_scroll.saturating_add(1);
                return Ok(false);
            }
            KeyCode::PageUp => {
                app.card_import_help_scroll = app.card_import_help_scroll.saturating_sub(10);
                return Ok(false);
            }
            KeyCode::PageDown => {
                app.card_import_help_scroll = app.card_import_help_scroll.saturating_add(10);
                return Ok(false);
            }
            _ => {}
        }
    }

    if app.show_global_search {
        match key.code {
            KeyCode::Esc => {
                app.show_global_search = false;
            }
            KeyCode::Enter => {
                if let Some(hit) = app.global_search_results.get(app.global_search_selected).cloned() {
                    app.navigate_search_target(hit.target);
                }
                app.show_global_search = false;
            }
            KeyCode::Up => {
                if app.global_search_selected > 0 {
                    app.global_search_selected -= 1;
                }
            }
            KeyCode::Down => {
                if app.global_search_selected + 1 < app.global_search_results.len() {
                    app.global_search_selected += 1;
                }
            }
            KeyCode::Backspace => {
                app.global_search_query.pop();
                app.rebuild_global_search_results();
            }
            KeyCode::Char(c) => {
                app.global_search_query.push(c);
                app.rebuild_global_search_results();
            }
            _ => {}
        }
        return Ok(false);
    }

    if key.code == KeyCode::Char('?') && !app.is_editing() {
        app.show_help_overlay = true;
        app.help_search_query.clear();
        return Ok(false);
    }

    // Ctrl+H: Open Find and Replace (only in Notes view)
    if key.code == KeyCode::Char('h') && key.modifiers.contains(KeyModifiers::CONTROL) {
        if matches!(app.view_mode, ViewMode::Notes) && !app.is_editing() {
            app.edit_target = EditTarget::FindReplace;
            app.find_text.clear();
            app.replace_text.clear();
            app.find_input_focus = true;
            return Ok(false);
        }
    }

    // Ctrl+F: Global fuzzy search overlay
    if key.code == KeyCode::Char('f') && key.modifiers.contains(KeyModifiers::CONTROL) {
        if !app.is_editing() {
            app.show_global_search = true;
            app.global_search_query.clear();
            app.rebuild_global_search_results();
            return Ok(false);
        }
    }

    // Flashcards view keyboard shortcuts (when not editing)
    if !app.is_editing() && matches!(app.view_mode, ViewMode::Flashcards) {
        match key.code {
            KeyCode::Char(' ') if app.card_review_mode => {
                app.show_card_answer = !app.show_card_answer;
                return Ok(false);
            }
            KeyCode::Char('0'..='5') if app.card_review_mode && app.show_card_answer => {
                let quality = match key.code {
                    KeyCode::Char('0') => 0,
                    KeyCode::Char('1') => 1,
                    KeyCode::Char('2') => 2,
                    KeyCode::Char('3') => 3,
                    KeyCode::Char('4') => 4,
                    KeyCode::Char('5') => 5,
                    _ => 3,
                };
                if let Some(card) = app.cards.get_mut(app.current_card_idx) {
                    card.review(quality);
                    app.show_card_answer = false;
                    app.current_card_idx = next_card_in_filter(app, app.current_card_idx);
                    let _ = save_app_data(app);
                }
                return Ok(false);
            }
            KeyCode::Up if !app.card_review_mode && key.modifiers.contains(KeyModifiers::SHIFT) => {
                if app.cards.is_empty() {
                    return Ok(false);
                }
                let anchor = app.card_selection_anchor.unwrap_or(app.current_card_idx);
                app.card_selection_anchor = Some(anchor);
                app.current_card_idx = prev_card_in_filter(app, app.current_card_idx);
                app.update_card_selection(anchor, app.current_card_idx);
                return Ok(false);
            }
            KeyCode::Down if !app.card_review_mode && key.modifiers.contains(KeyModifiers::SHIFT) => {
                if app.cards.is_empty() {
                    return Ok(false);
                }
                let anchor = app.card_selection_anchor.unwrap_or(app.current_card_idx);
                app.card_selection_anchor = Some(anchor);
                app.current_card_idx = next_card_in_filter(app, app.current_card_idx);
                app.update_card_selection(anchor, app.current_card_idx);
                return Ok(false);
            }
            KeyCode::Up if !app.card_review_mode => {
                app.current_card_idx = prev_card_in_filter(app, app.current_card_idx);
                app.clear_card_selection();
                return Ok(false);
            }
            KeyCode::Down if !app.card_review_mode => {
                app.current_card_idx = next_card_in_filter(app, app.current_card_idx);
                app.clear_card_selection();
                return Ok(false);
            }
            KeyCode::Enter if !app.card_review_mode && !app.cards.is_empty() => {
                // Ensure current selection is within filter
                if !matches_filter(app, &app.cards[app.current_card_idx]) {
                    if let Some((first_idx, _)) = app
                        .cards
                        .iter()
                        .enumerate()
                        .find(|(_, c)| matches_filter(app, c))
                    {
                        app.current_card_idx = first_idx;
                    }
                }
                app.clear_card_selection();
                app.card_review_mode = true;
                app.show_card_answer = false;
                return Ok(false);
            }
            KeyCode::Esc if app.card_review_mode => {
                app.card_review_mode = false;
                app.show_card_answer = false;
                app.clear_card_selection();
                return Ok(false);
            }
            _ => {}
        }
    }

    // Finance view keyboard controls (when summary is open and not editing)
    if !app.is_editing() && matches!(app.view_mode, ViewMode::Finance) && app.show_finance_summary {
        match key.code {
            KeyCode::Up => {
                app.finance_summary_scroll = app.finance_summary_scroll.saturating_sub(1);
                return Ok(false);
            }
            KeyCode::Down => {
                app.finance_summary_scroll = app.finance_summary_scroll.saturating_add(1);
                return Ok(false);
            }
            KeyCode::PageUp => {
                app.finance_summary_scroll = app.finance_summary_scroll.saturating_sub(10);
                return Ok(false);
            }
            KeyCode::PageDown => {
                app.finance_summary_scroll = app.finance_summary_scroll.saturating_add(10);
                return Ok(false);
            }
            KeyCode::Left => {
                // Get unique categories
                let categories: Vec<String> = app
                    .finances
                    .iter()
                    .map(|e| e.category.clone())
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect();
                
                if !categories.is_empty() {
                    app.selected_finance_category_idx = if app.selected_finance_category_idx > 0 {
                        app.selected_finance_category_idx - 1
                    } else {
                        categories.len() - 1
                    };
                    app.finance_summary_scroll = 0; // Reset scroll when changing category
                }
                return Ok(false);
            }
            KeyCode::Right => {
                // Get unique categories
                let categories: Vec<String> = app
                    .finances
                    .iter()
                    .map(|e| e.category.clone())
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect();
                
                if !categories.is_empty() {
                    app.selected_finance_category_idx = (app.selected_finance_category_idx + 1) % categories.len();
                    app.finance_summary_scroll = 0; // Reset scroll when changing category
                }
                return Ok(false);
            }
            _ => {}
        }
    }

    // Habits view keyboard controls (when summary is open and not editing)
    if !app.is_editing() && matches!(app.view_mode, ViewMode::Habits) && app.show_habits_summary {
        match key.code {
            KeyCode::Up => {
                app.habits_summary_scroll = app.habits_summary_scroll.saturating_sub(1);
                return Ok(false);
            }
            KeyCode::Down => {
                app.habits_summary_scroll = app.habits_summary_scroll.saturating_add(1);
                return Ok(false);
            }
            KeyCode::PageUp => {
                app.habits_summary_scroll = app.habits_summary_scroll.saturating_sub(10);
                return Ok(false);
            }
            KeyCode::PageDown => {
                app.habits_summary_scroll = app.habits_summary_scroll.saturating_add(10);
                return Ok(false);
            }
            _ => {}
        }
    }

    // Notes view scrolling when not editing and not in search
    if !app.is_editing() && matches!(app.view_mode, ViewMode::Notes) {
        match key.code {
            KeyCode::Up => {
                app.content_scroll = app.content_scroll.saturating_sub(1);
                return Ok(false);
            }
            KeyCode::Down => {
                app.content_scroll = app.content_scroll.saturating_add(1);
                return Ok(false);
            }
            KeyCode::PageUp => {
                app.content_scroll = app.content_scroll.saturating_sub(10);
                return Ok(false);
            }
            KeyCode::PageDown => {
                app.content_scroll = app.content_scroll.saturating_add(10);
                return Ok(false);
            }
            _ => {}
        }
    }

    // Handle Find and Replace mode
    if matches!(app.edit_target, EditTarget::FindReplace) {
        match key.code {
            KeyCode::Esc => {
                app.edit_target = EditTarget::None;
                app.find_text.clear();
                app.replace_text.clear();
            }
            KeyCode::Tab => {
                app.find_input_focus = !app.find_input_focus;
            }
            KeyCode::Backspace => {
                if app.find_input_focus {
                    app.find_text.pop();
                } else {
                    app.replace_text.pop();
                }
            }
            KeyCode::Enter => {
                // Perform the replacement
                if !app.find_text.is_empty() {
                    let find_text = app.find_text.clone();
                    let replace_text = app.replace_text.clone();

                    if let Some(page) = app.current_page_mut() {
                        page.content = page.content.replace(&find_text, &replace_text);
                        page.modified_at = Local::now().date_naive();
                        page.extract_links_and_images();

                        app.edit_target = EditTarget::None;
                        app.find_text.clear();
                        app.replace_text.clear();
                        let _ = save_app_data(app);
                    }
                }
            }
            KeyCode::Char(c) => {
                if app.find_input_focus {
                    app.find_text.push(c);
                } else {
                    app.replace_text.push(c);
                }
            }
            _ => {}
        }
        return Ok(false);
    }

    // Ctrl+S: Save current editing content
    if key.code == KeyCode::Char('s')
        && key.modifiers.contains(KeyModifiers::CONTROL)
        && app.is_editing()
    {
        // For inline edits, sync textarea first then save
        if app.inline_edit_mode {
            app.editing_input = app.textarea.lines().join("\n");
            app.save_inline_edit();
        } else {
            app.editing_input = app.textarea.lines().join("\n");
            app.save_input();
        }
        app.inline_edit_mode = false;
        app.editing_input.clear();
        return Ok(false);
    }

    // Esc: Dismiss validation error popup
    if key.code == KeyCode::Esc && app.show_validation_error {
        app.show_validation_error = false;
        app.validation_error_message.clear();
        return Ok(false);
    }

    // Esc: Dismiss success popup
    if key.code == KeyCode::Esc && app.show_success_popup {
        app.show_success_popup = false;
        app.success_message.clear();
        return Ok(false);
    }

    // Esc: Cancel editing without saving
    if key.code == KeyCode::Esc && app.is_editing() {
        app.edit_target = EditTarget::None;
        app.inline_edit_mode = false;
        app.editing_input.clear();
        app.textarea.delete_line_by_head(); // Clear textarea
        app.undo_stack.clear();
        app.redo_stack.clear();
        return Ok(false);
    }

    if app.is_editing() {
        // Ctrl+A: select all (cleared on other edits)
        if key.code == KeyCode::Char('a') && key.modifiers.contains(KeyModifiers::CONTROL) {
            app.selection_all = true;
            return Ok(false);
        }

        // Ctrl+Z: Undo
        if key.code == KeyCode::Char('z') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if let Some(prev) = app.undo_stack.pop() {
                let current = app.textarea.lines().join("\n");
                app.redo_stack.push(current);
                let lines: Vec<String> = prev.lines().map(|s| s.to_string()).collect();
                app.textarea = TextArea::new(lines);
                let end_row = app.textarea.lines().len().saturating_sub(1) as u16;
                let end_col = app.textarea.lines().last().map(|l| l.len()).unwrap_or(0) as u16;
                app.textarea.move_cursor(CursorMove::Jump(end_row, end_col));
                app.editing_input = app.textarea.lines().join("\n");
                return Ok(false);
            }
        }

        // Ctrl+Y: Redo
        if key.code == KeyCode::Char('y') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if let Some(next) = app.redo_stack.pop() {
                let current = app.textarea.lines().join("\n");
                app.undo_stack.push(current);
                let lines: Vec<String> = next.lines().map(|s| s.to_string()).collect();
                app.textarea = TextArea::new(lines);
                let end_row = app.textarea.lines().len().saturating_sub(1) as u16;
                let end_col = app.textarea.lines().last().map(|l| l.len()).unwrap_or(0) as u16;
                app.textarea.move_cursor(CursorMove::Jump(end_row, end_col));
                app.editing_input = app.textarea.lines().join("\n");
                return Ok(false);
            }
        }

        // Ctrl+K: delete current line
        if key.code == KeyCode::Char('k') && key.modifiers.contains(KeyModifiers::CONTROL) {
            let (row, col) = app.textarea.cursor();
            let mut lines: Vec<String> = app.textarea.lines().to_vec();
            if !lines.is_empty() {
                let row_usize = row as usize;
                if row_usize < lines.len() {
                    lines.remove(row_usize);
                    if lines.is_empty() {
                        lines.push(String::new());
                    }
                    let new_row = row_usize.min(lines.len().saturating_sub(1));
                    let new_col = col.min(lines[new_row].len());
                    app.textarea = TextArea::new(lines);
                    app.textarea.move_cursor(CursorMove::Jump(new_row as u16, new_col as u16));
                    app.editing_input = app.textarea.lines().join("\n");
                    app.editing_cursor_line = new_row;
                    app.editing_cursor_col = new_col;
                    app.selection_all = false;
                }
            }
            return Ok(false);
        }

        // F7: Spell Check
        if key.code == KeyCode::F(7) {
            app.run_spell_check();
            return Ok(false);
        }

        // Delete/Backspace clears all when select-all is active
        if app.selection_all && matches!(key.code, KeyCode::Delete | KeyCode::Backspace) {
            app.textarea = TextArea::new(vec![String::new()]);
            app.textarea.move_cursor(CursorMove::Jump(0, 0));
            app.editing_input.clear();
            app.editing_cursor_line = 0;
            app.editing_cursor_col = 0;
            app.selection_all = false;
            return Ok(false);
        }

        // Forward all key events to the textarea for normal text editing (arrow keys, etc.)
        let input = Input {
            key: match key.code {
                KeyCode::Char(c) => Key::Char(c),
                KeyCode::Enter => Key::Enter,
                KeyCode::Backspace => Key::Backspace,
                KeyCode::Delete => Key::Delete,
                KeyCode::Left => Key::Left,
                KeyCode::Right => Key::Right,
                KeyCode::Up => Key::Up,
                KeyCode::Down => Key::Down,
                KeyCode::Tab => Key::Tab,
                KeyCode::Home => Key::Home,
                KeyCode::End => Key::End,
                KeyCode::PageUp => Key::PageUp,
                KeyCode::PageDown => Key::PageDown,
                KeyCode::Esc => Key::Esc,
                KeyCode::F(n) => Key::F(n),
                _ => Key::Null,
            },
            ctrl: key.modifiers.contains(KeyModifiers::CONTROL),
            alt: key.modifiers.contains(KeyModifiers::ALT),
        };
        app.selection_all = false;
        // Push current state to undo stack before a mutating key
        let mutates = matches!(input.key, Key::Char(_)|Key::Enter|Key::Backspace|Key::Delete|Key::Tab)
            || (matches!(input.key, Key::Null) && input.ctrl);
        if mutates {
            let current = app.textarea.lines().join("\n");
            app.undo_stack.push(current);
            app.redo_stack.clear();
        }
        app.textarea.input(input);
        app.editing_input = app.textarea.lines().join("\n");
        let (row, col) = app.textarea.cursor();
        app.editing_cursor_line = row;
        app.editing_cursor_col = col;
        return Ok(false);
    }

    match key.code {
        KeyCode::Char('q') => return Ok(true),
        _ => {}
    }

    Ok(false)
}

fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    // Mouse scroll support for card import help; do not swallow clicks
    if app.show_card_import_help && matches!(app.edit_target, EditTarget::CardImport) {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                app.card_import_help_scroll = app.card_import_help_scroll.saturating_sub(3);
            }
            MouseEventKind::ScrollDown => {
                app.card_import_help_scroll = app.card_import_help_scroll.saturating_add(3);
            }
            _ => {}
        }
        // Continue to process clicks below
    }

    // Handle mouse wheel scrolling in help overlay
    if app.show_help_overlay {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                app.help_scroll = app.help_scroll.saturating_sub(3);
            }
            MouseEventKind::ScrollDown => {
                app.help_scroll = app.help_scroll.saturating_add(3);
            }
            _ => {}
        }
        return;
    }

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // Handle calendar picker
            if app.show_calendar {
                for (day, rect) in app.calendar_day_rects.clone() {
                    if inside_rect(mouse, rect) {
                        if let Some(date) =
                            NaiveDate::from_ymd_opt(app.calendar_year, app.calendar_month, day)
                        {
                            app.current_journal_date = date;
                            app.show_calendar = false;
                        }
                        return;
                    }
                }
                return;
            }

            if app.show_global_search {
                if let Some(idx) = find_clicked_item(mouse, &app.search_result_items.clone()) {
                    app.global_search_selected =
                        idx.min(app.global_search_results.len().saturating_sub(1));
                    if let Some(hit) =
                        app.global_search_results.get(app.global_search_selected).cloned()
                    {
                        app.navigate_search_target(hit.target);
                        app.show_global_search = false;
                    }
                }
                return;
            }

            // Check view mode buttons
            for (mode, rect) in app.view_mode_btns.clone() {
                if inside_rect(mouse, rect) {
                    app.view_mode = mode;
                    app.edit_target = EditTarget::None;
                    app.validate_indices();
                    return;
                }
            }

            // Global search button
            if inside_rect(mouse, app.search_btn) {
                app.show_global_search = true;
                app.global_search_query.clear();
                app.rebuild_global_search_results();
                return;
            }

            match app.view_mode {
                ViewMode::Notes => handle_notes_mouse_left(app, mouse),
                ViewMode::Planner => handle_planner_mouse_left(app, mouse),
                ViewMode::Journal => handle_journal_mouse_left(app, mouse),
                ViewMode::Habits => handle_habits_mouse_left(app, mouse),
                ViewMode::Finance => handle_finance_mouse_left(app, mouse),
                ViewMode::Calories => handle_calories_mouse_left(app, mouse),
                ViewMode::Kanban => handle_kanban_mouse_left(app, mouse),
                ViewMode::Flashcards => handle_flashcards_mouse_left(app, mouse),
            }
        }
        MouseEventKind::Up(MouseButton::Left) | MouseEventKind::Drag(MouseButton::Left) => {}
        MouseEventKind::Down(MouseButton::Right) => match app.view_mode {
            ViewMode::Notes => handle_notes_mouse_right(app, mouse),
            ViewMode::Planner => handle_planner_mouse_right(app, mouse),
            ViewMode::Habits => handle_habits_mouse_right(app, mouse),
            ViewMode::Kanban => handle_kanban_mouse_right(app, mouse),
            _ => {}
        },
        MouseEventKind::Down(MouseButton::Middle) => {
            match app.view_mode {
                ViewMode::Notes => handle_notes_mouse_middle(app, mouse),
                ViewMode::Planner => handle_planner_mouse_middle(app, mouse),
                _ => {}
            }
        }
        MouseEventKind::ScrollUp => {
            // Scroll up in content when not editing
            if !app.is_editing() && matches!(app.view_mode, ViewMode::Notes) {
                app.content_scroll = app.content_scroll.saturating_sub(3);
            }
        }
        MouseEventKind::ScrollDown => {
            // Scroll down in content when not editing
            if !app.is_editing() && matches!(app.view_mode, ViewMode::Notes) {
                app.content_scroll = app.content_scroll.saturating_add(3);
            }
        }
        _ => {}
    }
}

fn handle_notes_mouse_left(app: &mut App, mouse: MouseEvent) {
    // Check tree items - single click to select
    for (level, nb_idx, sec_idx, pg_idx, rect) in app.tree_items.clone() {
        if inside_rect(mouse, rect) {
            app.current_notebook_idx = nb_idx;
            app.current_section_idx = sec_idx;
            app.current_page_idx = pg_idx;
            app.hierarchy_level = level;
            return;
        }
    }

    // Check buttons
    if inside_rect(mouse, app.add_notebook_btn) {
        app.add_notebook();
        return;
    }
    if inside_rect(mouse, app.add_section_btn) {
        app.add_section();
        return;
    }
    if inside_rect(mouse, app.add_page_btn) {
        app.add_page();
        return;
    }
    if inside_rect(mouse, app.delete_btn) {
        app.delete_current();
        return;
    }

    // Check content area
    if inside_rect(mouse, app.content_edit_area) {
        if !app.is_editing() {
            let rel_y = mouse.row.saturating_sub(app.content_edit_area.y + 1);
            let content = app
                .current_page()
                .map(|p| p.content.clone())
                .unwrap_or_default();
            let lines: Vec<&str> = content.lines().collect();
            let target_idx = app.content_scroll as usize + rel_y as usize;
            let mut debug_lines = Vec::new();

            if let Some(line) = lines.get(target_idx) {
                debug_lines.push(format!("clicked line: {}", line));
                if let Some(path) = extract_path(line) {
                    debug_lines.push(format!("found path token: {}", path));
                    if let Some(resolved) = resolve_image_path(&path) {
                        debug_lines.push(format!("resolved path: {}", resolved.display()));
                        let _ = open::that(&resolved).map_err(|e| {
                            debug_lines.push(format!("open error: {}", e));
                        });
                        let _ = std::fs::write("/tmp/mynotes_image_debug.log", debug_lines.join("\n"));
                        return;
                    } else {
                        debug_lines.push("resolve_image_path returned None".to_string());
                    }
                } else {
                    debug_lines.push("extract_path returned None".to_string());
                }
            } else {
                debug_lines.push(format!("line index out of bounds: {} of {}", target_idx, lines.len()));
            }

            let _ = std::fs::write("/tmp/mynotes_image_debug.log", debug_lines.join("\n"));
        }

        let rel_y = mouse.row.saturating_sub(app.content_edit_area.y + 1);
        let rel_x = mouse.column.saturating_sub(app.content_edit_area.x + 1);

        // Click inside content starts a full-page text editor, and clicking while editing moves the caret
        if matches!(app.edit_target, EditTarget::PageContent) {
            app.textarea
                .move_cursor(CursorMove::Jump(rel_y as u16, rel_x as u16));
        } else if matches!(app.hierarchy_level, HierarchyLevel::Page) {
            let content = app
                .current_page()
                .map(|p| p.content.clone())
                .unwrap_or_default();
            start_editing(app, EditTarget::PageContent, content);
            app.inline_edit_mode = false;
            app.textarea
                .move_cursor(CursorMove::Jump(rel_y as u16, rel_x as u16));
        } else {
            // In Section/Notebook view, do not enter edit mode on content click
            return;
        }
        let (row, col) = app.textarea.cursor();
        app.editing_cursor_line = row;
        app.editing_cursor_col = col;
        return;
    }
}

// Helper function to handle mouse clicks in textarea editors across all views
fn handle_textarea_mouse_click(app: &mut App, mouse: MouseEvent) {
    if inside_rect(mouse, app.content_edit_area) && app.is_editing() {
        let rel_y = mouse.row.saturating_sub(app.content_edit_area.y + 1);
        let rel_x = mouse.column.saturating_sub(app.content_edit_area.x + 1);
        
        app.textarea
            .move_cursor(CursorMove::Jump(rel_y as u16, rel_x as u16));
        
        let (row, col) = app.textarea.cursor();
        app.editing_cursor_line = row;
        app.editing_cursor_col = col;
    }
}

fn handle_planner_mouse_left(app: &mut App, mouse: MouseEvent) {
    // Handle textarea mouse clicks for editing
    handle_textarea_mouse_click(app, mouse);
    
    // Check task items to select
    if let Some(idx) = find_clicked_item(mouse, &app.task_items.clone()) {
        app.current_task_idx = idx;
        return;
    }

    // Check add task button
    if inside_rect(mouse, app.add_task_btn) {
        start_editing(app, EditTarget::TaskTitle, new_task_editor_template());
        // Position cursor after first parameter (title line)
        app.textarea.move_cursor(CursorMove::Head);
        return;
    }

    // Check edit task button
    if inside_rect(mouse, app.edit_task_btn) {
        if let Some(task) = app.tasks.get(app.current_task_idx) {
            let content = format_task_editor_content(task);
            start_editing(app, EditTarget::TaskDetails, content);
            // Position cursor at end of first line (title)
            app.textarea.move_cursor(CursorMove::Head);
            app.textarea.move_cursor(CursorMove::End);
        }
        return;
    }

    // Check delete task button
    if inside_rect(mouse, app.delete_task_btn) {
        delete_and_adjust_index(&mut app.tasks, &mut app.current_task_idx);
        let _ = save_app_data(app);
        return;
    }

    // Open reminder edit (same as Edit Task)
}

fn handle_planner_mouse_right(app: &mut App, mouse: MouseEvent) {
    // Right-click on task to delete
    for (idx, rect) in app.task_items.clone() {
        if inside_rect(mouse, rect) {
            app.current_task_idx = idx;
            delete_and_adjust_index(&mut app.tasks, &mut app.current_task_idx);
            let _ = save_app_data(app);
            return;
        }
    }
}

fn handle_planner_mouse_middle(app: &mut App, mouse: MouseEvent) {
    // Middle-click to toggle completion
    if let Some(idx) = find_clicked_item(mouse, &app.task_items.clone()) {
        app.current_task_idx = idx;
        if let Some(task) = app.tasks.get_mut(idx) {
            task.completed = !task.completed;
        }
        let _ = save_app_data(app);
    }
}

fn handle_journal_mouse_left(app: &mut App, mouse: MouseEvent) {
    // Handle textarea mouse clicks for editing
    handle_textarea_mouse_click(app, mouse);
    
    // Check navigation buttons
    if handle_date_nav(app, mouse) {
        return;
    }

    // Check content area for editing
    if inside_rect(mouse, app.content_edit_area) && !app.is_editing() {
        let entry = app
            .journal_entries
            .iter()
            .find(|e| e.date == app.current_journal_date)
            .cloned();

        let content = entry.map(|e| e.content).unwrap_or_default();
        let is_empty = content.is_empty();
        start_editing(app, EditTarget::JournalEntry, content);
        // Position cursor at start for new entry or at end for existing
        if is_empty {
            app.textarea.move_cursor(CursorMove::Head);
        }
    }
}

fn handle_habits_mouse_left(app: &mut App, mouse: MouseEvent) {
    // Handle textarea mouse clicks for editing
    handle_textarea_mouse_click(app, mouse);
    
    // Check Summary button
    if inside_rect(mouse, app.summary_btn) {
        app.show_habits_summary = !app.show_habits_summary;
        return;
    }
    
    // Check date navigation buttons first
    if handle_date_nav(app, mouse) {
        return;
    }

    // Check habit list items for selection
    if let Some(idx) = find_clicked_item(mouse, &app.habit_items.clone()) {
        app.current_habit_idx = idx;
        return;
    }

    // Buttons
    if inside_rect(mouse, app.add_habit_btn) {
        let template = new_habit_editor_template(app.current_journal_date);
        start_editing(app, EditTarget::HabitNew, template);
        // Position cursor at end of name line
        app.textarea.move_cursor(CursorMove::Head);
        app.textarea.move_cursor(CursorMove::End);
        return;
    }
    if inside_rect(mouse, app.mark_done_btn) {
        if let Some(h) = app.habits.get_mut(app.current_habit_idx) {
            let d = app.current_journal_date;
            if h.marks.contains(&d) {
                h.marks.remove(&d);
            } else {
                h.marks.insert(d);
            }
            // Recompute streak from the most recent marked date backwards
            if let Some(mut day) = h.marks.iter().copied().max() {
                let mut streak = 0u32;
                loop {
                    if h.marks.contains(&day) {
                        streak += 1;
                    } else {
                        break;
                    }
                    if let Some(prev) = day.pred_opt() {
                        day = prev;
                    } else {
                        break;
                    }
                }
                h.streak = streak;
            } else {
                h.streak = 0;
            }
        }
        let _ = save_app_data(app);
        return;
    }
    if inside_rect(mouse, app.edit_habit_btn) {
        if let Some(h) = app.habits.get(app.current_habit_idx) {
            let content = format_habit_editor_content(h);
            start_editing(app, EditTarget::Habit, content);
            // Position cursor at end of name line
            app.textarea.move_cursor(CursorMove::Head);
            app.textarea.move_cursor(CursorMove::End);
        }
        return;
    }
    if inside_rect(mouse, app.delete_habit_btn) {
        delete_and_adjust_index(&mut app.habits, &mut app.current_habit_idx);
        let _ = save_app_data(app);
        return;
    }
}

fn handle_habits_mouse_right(_app: &mut App, _mouse: MouseEvent) {}

fn handle_finance_mouse_left(app: &mut App, mouse: MouseEvent) {
    // Handle textarea mouse clicks for editing
    handle_textarea_mouse_click(app, mouse);
    
    // Check Summary button
    if inside_rect(mouse, app.summary_btn) {
        app.show_finance_summary = !app.show_finance_summary;
        return;
    }
    
    // Check date navigation buttons
    if handle_date_nav(app, mouse) {
        return;
    }

    // Check finance list items for selection
    if let Some(idx) = find_clicked_item(mouse, &app.finance_items.clone()) {
        app.current_finance_idx = idx;
        return;
    }

    if inside_rect(mouse, app.add_fin_btn) {
        let template = new_finance_editor_template(app.current_journal_date);
        start_editing(app, EditTarget::FinanceNew, template);
        // Position cursor at end of category line
        app.textarea.move_cursor(CursorMove::Head);
        app.textarea.move_cursor(CursorMove::End);
        return;
    }

    if inside_rect(mouse, app.edit_fin_btn) {
        if let Some(entry) = app.finances.get(app.current_finance_idx) {
            let content = format_finance_editor_content(entry);
            start_editing(app, EditTarget::Finance, content);
            // Position cursor at end of category line
            app.textarea.move_cursor(CursorMove::Head);
            app.textarea.move_cursor(CursorMove::End);
        }
        return;
    }

    if inside_rect(mouse, app.delete_fin_btn) {
        delete_and_adjust_index(&mut app.finances, &mut app.current_finance_idx);
        let _ = save_app_data(app);
    }
}

fn handle_calories_mouse_left(app: &mut App, mouse: MouseEvent) {
    // Handle textarea mouse clicks for editing
    handle_textarea_mouse_click(app, mouse);
    
    // Check date navigation buttons
    if handle_date_nav(app, mouse) {
        return;
    }

    // Check calorie list items for selection
    if let Some(idx) = find_clicked_item(mouse, &app.calorie_items.clone()) {
        app.current_calorie_idx = idx;
        return;
    }

    if inside_rect(mouse, app.add_cal_btn) {
        let template = new_calorie_editor_template(app.current_journal_date);
        start_editing(app, EditTarget::CaloriesNew, template);
        // Position cursor at end of meal name line
        app.textarea.move_cursor(CursorMove::Head);
        app.textarea.move_cursor(CursorMove::End);
        return;
    }

    if inside_rect(mouse, app.edit_cal_btn) {
        if let Some(entry) = app.calories.get(app.current_calorie_idx) {
            let content = format_calorie_editor_content(entry);
            start_editing(app, EditTarget::Calories, content);
            // Position cursor at end of meal name line
            app.textarea.move_cursor(CursorMove::Head);
            app.textarea.move_cursor(CursorMove::End);
        }
        return;
    }

    if inside_rect(mouse, app.delete_cal_btn) {
        delete_and_adjust_index(&mut app.calories, &mut app.current_calorie_idx);
        let _ = save_app_data(app);
    }
}

fn handle_kanban_mouse_left(app: &mut App, mouse: MouseEvent) {
    // Handle textarea mouse clicks for editing
    handle_textarea_mouse_click(app, mouse);
    
    if inside_rect(mouse, app.add_kanban_btn) {
        let template = new_kanban_editor_template();
        start_editing(app, EditTarget::KanbanNew, template);
        // Position cursor at end of title line
        app.textarea.move_cursor(CursorMove::Head);
        app.textarea.move_cursor(CursorMove::End);
        return;
    }

    if inside_rect(mouse, app.move_left_kanban_btn) {
        if let Some(card) = app.kanban_cards.get_mut(app.current_kanban_card_idx) {
            card.stage = card.stage.move_left();
            let _ = save_app_data(app);
        }
        return;
    }

    if inside_rect(mouse, app.move_right_kanban_btn) {
        if let Some(card) = app.kanban_cards.get_mut(app.current_kanban_card_idx) {
            card.stage = card.stage.move_right();
            let _ = save_app_data(app);
        }
        return;
    }

    if inside_rect(mouse, app.delete_kanban_btn) {
        delete_and_adjust_index(&mut app.kanban_cards, &mut app.current_kanban_card_idx);
        let _ = save_app_data(app);
        return;
    }

    for (idx, rect) in app.kanban_items.clone() {
        if inside_rect(mouse, rect) {
            app.current_kanban_card_idx = idx;
            if let Some(card) = app.kanban_cards.get(idx) {
                let content = format_kanban_editor_content(card);
                start_editing(app, EditTarget::KanbanEdit, content);
                // Position cursor at end of title line
                app.textarea.move_cursor(CursorMove::Head);
                app.textarea.move_cursor(CursorMove::End);
            }
            return;
        }
    }
}

fn handle_kanban_mouse_right(app: &mut App, mouse: MouseEvent) {
    for (idx, rect) in app.kanban_items.clone() {
        if inside_rect(mouse, rect) {
            app.current_kanban_card_idx = idx;
            delete_and_adjust_index(&mut app.kanban_cards, &mut app.current_kanban_card_idx);
            let _ = save_app_data(app);
            return;
        }
    }
}

fn handle_notes_mouse_right(app: &mut App, mouse: MouseEvent) {
    // Right click to delete
    for (level, nb_idx, sec_idx, pg_idx, rect) in app.tree_items.clone() {
        if inside_rect(mouse, rect) {
            app.current_notebook_idx = nb_idx;
            app.current_section_idx = sec_idx;
            app.current_page_idx = pg_idx;
            app.hierarchy_level = level;
            app.delete_current();
            return;
        }
    }
}

fn handle_notes_mouse_middle(app: &mut App, mouse: MouseEvent) {
    // Middle click to rename
    for (level, nb_idx, sec_idx, pg_idx, rect) in app.tree_items.clone() {
        if inside_rect(mouse, rect) {
            app.current_notebook_idx = nb_idx;
            app.current_section_idx = sec_idx;
            app.current_page_idx = pg_idx;
            app.hierarchy_level = level;

            // Start editing title
            match level {
                HierarchyLevel::Notebook => {
                    let content = app
                        .current_notebook()
                        .map(|n| n.title.clone())
                        .unwrap_or_default();
                    app.start_text_editing(content);
                    app.edit_target = EditTarget::NotebookTitle;
                }
                HierarchyLevel::Section => {
                    let content = app
                        .current_section()
                        .map(|s| s.title.clone())
                        .unwrap_or_default();
                    app.start_text_editing(content);
                    app.edit_target = EditTarget::SectionTitle;
                }
                HierarchyLevel::Page => {
                    let content = app
                        .current_page()
                        .map(|p| p.title.clone())
                        .unwrap_or_default();
                    app.start_text_editing(content);
                    app.edit_target = EditTarget::PageTitle;
                }
            }
            return;
        }
    }
}

// Parse and render markdown tables
fn parse_and_render_table(table_text: &str) -> Option<Vec<Line<'static>>> {
    let lines: Vec<&str> = table_text.lines().collect();
    if lines.len() < 2 {
        return None;
    }

    // Parse header row
    let header_line = lines[0].trim();
    if !header_line.starts_with('|') || !header_line.ends_with('|') {
        return None;
    }

    let headers: Vec<&str> = header_line
        .trim_start_matches('|')
        .trim_end_matches('|')
        .split('|')
        .map(|s| s.trim())
        .collect();

    // Check separator line
    let sep_line = lines.get(1).map(|s| s.trim()).unwrap_or("");
    if !sep_line.contains("---") {
        return None;
    }

    let mut result_lines = Vec::new();

    // Header row
    let header_spans: Vec<Span> = headers
        .iter()
        .enumerate()
        .flat_map(|(i, h)| {
            let mut spans = vec![Span::styled(
                format!(" {:^20} ", h),
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )];
            if i < headers.len() - 1 {
                spans.push(Span::raw("│"));
            }
            spans
        })
        .collect();
    result_lines.push(Line::from(header_spans));

    // Separator
    let sep = "─".repeat(headers.len() * 23 - 1);
    result_lines.push(Line::from(Span::styled(sep, Style::default().fg(Color::Gray))));

    // Data rows
    for line_idx in 2..lines.len() {
        let data_line = lines[line_idx].trim();
        if !data_line.starts_with('|') || !data_line.ends_with('|') {
            continue;
        }

        let cells: Vec<&str> = data_line
            .trim_start_matches('|')
            .trim_end_matches('|')
            .split('|')
            .map(|s| s.trim())
            .collect();

        let row_spans: Vec<Span> = cells
            .iter()
            .enumerate()
            .flat_map(|(i, cell)| {
                let mut spans = vec![Span::styled(
                    format!(" {:20} ", cell),
                    Style::default().fg(Color::White),
                )];
                if i < cells.len() - 1 {
                    spans.push(Span::raw("│"));
                }
                spans
            })
            .collect();
        result_lines.push(Line::from(row_spans));
    }

    Some(result_lines)
}

// Diagram rendering removed (feature disabled)

// Parse and render simple flowchart: Line starting with `>` or bullet points
fn parse_and_render_flowchart(flowchart_text: &str) -> Option<Vec<Line<'static>>> {
    let lines: Vec<&str> = flowchart_text.lines().collect();
    if lines.is_empty() {
        return None;
    }

    let mut result = Vec::new();
    let mut is_flowchart = false;

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        
        // Detect flowchart markers: lines starting with >, -, or numbers
        if trimmed.starts_with('>') || trimmed.starts_with("- ") || trimmed.starts_with("1. ") {
            is_flowchart = true;
            
            let (marker, content) = if trimmed.starts_with('>') {
                (trimmed.chars().next().unwrap().to_string(), trimmed[1..].trim())
            } else if trimmed.starts_with("- ") {
                ("-".to_string(), trimmed[2..].trim())
            } else {
                let dot_pos = trimmed.find('.').unwrap_or(0);
                (trimmed[..=dot_pos].to_string(), trimmed[dot_pos + 1..].trim())
            };

            let indent = line.len() - trimmed.len();
            let indent_str = " ".repeat(indent);

            result.push(Line::from(vec![
                Span::raw(indent_str),
                Span::styled(
                    format!("{} ", marker),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    content.to_string(),
                    Style::default().fg(Color::White),
                ),
            ]));

            // Add connector if not last
            if idx < lines.len() - 1 {
                result.push(Line::from(vec![
                    Span::raw(format!("{}  ", " ".repeat(indent))),
                    Span::styled("↓", Style::default().fg(Color::Cyan)),
                ]));
            }
        }
    }

    if is_flowchart && !result.is_empty() {
        Some(result)
    } else {
        None
    }
}

fn looks_like_path(path: &str) -> bool {
    let trimmed = path.trim_matches(|c: char| c == '"');
    trimmed.starts_with('/') || trimmed.starts_with('~')
}

fn normalize_token(token: &str) -> String {
    token
        .trim_matches(|c: char| " ,;')\"].[".contains(c))
        .trim_matches('(')
    .trim_matches('[')
    .trim_matches(']')
        .to_string()
}

fn extract_path(line: &str) -> Option<String> {
    // Whole-line path (supports spaces), possibly quoted
    let trimmed = line.trim();
    let whole = trimmed.trim_matches('"');
    if looks_like_path(whole) {
        return Some(normalize_token(whole));
    }

    // Quoted substring anywhere in line: "..." or '...'
    if let Some(start) = line.find('"') {
        if let Some(end) = line[start + 1..].find('"') {
            let inner = &line[start + 1..start + 1 + end];
            let cleaned = normalize_token(inner);
            if looks_like_path(&cleaned) {
                return Some(cleaned);
            }
        }
    }
    if let Some(start) = line.find('\'') {
        if let Some(end) = line[start + 1..].find('\'') {
            let inner = &line[start + 1..start + 1 + end];
            let cleaned = normalize_token(inner);
            if looks_like_path(&cleaned) {
                return Some(cleaned);
            }
        }
    }

    // Markdown link/image style [alt](path)
    if let Some(start) = line.find('[') {
        if let Some(open) = line[start..].find("](") {
            let after = start + open + 2;
            if let Some(close) = line[after..].find(')') {
                let path = line[after..after + close].trim();
                let cleaned = normalize_token(path);
                if looks_like_path(&cleaned) {
                    return Some(cleaned);
                }
            }
        }
    }

    // Bracketed path form: [alt][path/to/file]
    if let Some(mid) = line.find("][") {
        let path_start = mid + 2;
        if let Some(end) = line[path_start..].find(']') {
            let path = &line[path_start..path_start + end];
            let cleaned = normalize_token(path);
            if looks_like_path(&cleaned) {
                return Some(cleaned);
            }
        }
    }

    // Plain path tokens
    for token in line.split_whitespace() {
        let cleaned = normalize_token(token);
        if looks_like_path(&cleaned) {
            return Some(cleaned);
        }
    }
    None
}

fn resolve_image_path(raw: &str) -> Option<PathBuf> {
    let expanded = if raw.starts_with('~') {
        env::home_dir().map(|h| h.join(raw.trim_start_matches('~')))
    } else {
        Some(PathBuf::from(raw))
    }?;
    if expanded.exists() {
        return Some(expanded);
    }
    std::fs::canonicalize(&expanded).ok()
}

    // Removed image feature; helper no longer needed
    // fn clear_inline_images() {}

fn inside_rect(mouse: MouseEvent, rect: Rect) -> bool {
    mouse.row >= rect.y
        && mouse.row < rect.y + rect.height
        && mouse.column >= rect.x
        && mouse.column < rect.x + rect.width
}

// Helper: Find clicked item index from mouse event
fn find_clicked_item(mouse: MouseEvent, items: &[(usize, Rect)]) -> Option<usize> {
    items
        .iter()
        .find(|(_, rect)| inside_rect(mouse, *rect))
        .map(|(idx, _)| *idx)
}

// Helper: Set up editor for a given target with initial content
fn start_editing(app: &mut App, target: EditTarget, content: String) {
    app.start_text_editing(content);
    app.edit_target = target;
    app.editing_cursor_line = 0;
    app.editing_cursor_col = 0;
}

// Helper: Delete item and adjust current index if needed
fn delete_and_adjust_index<T>(items: &mut Vec<T>, current_idx: &mut usize) {
    if *current_idx < items.len() {
        items.remove(*current_idx);
        if *current_idx >= items.len() && *current_idx > 0 {
            *current_idx -= 1;
        }
    }
}

// Helper: Render button with color
fn render_button(frame: &mut ratatui::Frame, text: &str, area: Rect, color: Color) {
    let btn = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(color));
    frame.render_widget(btn, area);
}

// Helper: Split a rectangular area into N equal horizontal chunks
fn split_equal_horizontal(area: Rect, count: usize) -> Vec<Rect> {
    if count == 0 {
        return Vec::new();
    }
    let pct = 100 / count.max(1) as u16;
    let mut constraints = Vec::with_capacity(count);
    for _ in 0..count {
        constraints.push(Constraint::Percentage(pct));
    }
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area)
        .to_vec()
}

// Helper: Handle date navigation button clicks
fn handle_date_nav(app: &mut App, mouse: MouseEvent) -> bool {
    if inside_rect(mouse, app.prev_day_btn) {
        app.current_journal_date = app
            .current_journal_date
            .pred_opt()
            .unwrap_or(app.current_journal_date);
        return true;
    }
    if inside_rect(mouse, app.next_day_btn) {
        app.current_journal_date = app
            .current_journal_date
            .succ_opt()
            .unwrap_or(app.current_journal_date);
        return true;
    }
    if inside_rect(mouse, app.date_btn) {
        // Open calendar picker
        app.show_calendar = true;
        app.calendar_year = app.current_journal_date.year();
        app.calendar_month = app.current_journal_date.month();
        return true;
    }
    if inside_rect(mouse, app.today_btn) {
        app.current_journal_date = Local::now().date_naive();
        return true;
    }
    false
}

// Helper: Build and track list items with rects for mouse interaction
fn build_list_items(
    items_iter: Vec<(usize, String, bool)>,
    current_idx: usize,
    area: Rect,
    item_rects: &mut Vec<(usize, Rect)>,
) -> Vec<ListItem<'_>> {
    let inner_y = area.y + 1;
    let mut items = Vec::new();
    let mut row_idx = 0;

    for (idx, text, is_completed) in items_iter {
        let style = if idx == current_idx {
            Style::default().bg(Color::Blue).fg(Color::White)
        } else if is_completed {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default()
        };

        let item_rect = Rect {
            x: area.x,
            y: inner_y + row_idx as u16,
            width: area.width,
            height: 1,
        };
        item_rects.push((idx, item_rect));

        items.push(ListItem::new(text).style(style));
        row_idx += 1;
    }

    items
}

fn draw(frame: &mut ratatui::Frame, app: &mut App) {
    app.validate_indices();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(frame.size());

    // View mode selector
    draw_view_mode_selector(frame, app, chunks[0]);

    // Body based on view mode
    match app.view_mode {
        ViewMode::Notes => {
            let body = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(chunks[1]);
            draw_left_panel(frame, app, body[0]);
            draw_content_panel(frame, app, body[1]);
        }
        ViewMode::Planner => {
            draw_planner_view(frame, app, chunks[1]);
        }
        ViewMode::Journal => {
            draw_journal_view(frame, app, chunks[1]);
        }
        ViewMode::Habits => {
            draw_habits_view(frame, app, chunks[1]);
        }
        ViewMode::Finance => {
            draw_finance_view(frame, app, chunks[1]);
        }
        ViewMode::Calories => {
            draw_calories_view(frame, app, chunks[1]);
        }
        ViewMode::Kanban => {
            draw_kanban_view(frame, app, chunks[1]);
        }
        ViewMode::Flashcards => {
            draw_flashcards_view(frame, app, chunks[1]);
        }
    }

    if app.show_validation_error {
        draw_validation_error_popup(frame, app);
    }

    if app.show_success_popup {
        draw_success_popup(frame, app);
    }

    if app.show_global_search {
        draw_global_search_overlay(frame, app);
    }

    if app.show_help_overlay {
        draw_help_overlay(frame, app);
    }

    if app.show_spell_check {
        draw_spell_check_popup(frame, app);
    }

    if app.show_calendar {
        draw_calendar_picker(frame, app);
    }
}

fn draw_view_mode_selector(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(11),
            Constraint::Percentage(11),
            Constraint::Percentage(11),
            Constraint::Percentage(11),
            Constraint::Percentage(11),
            Constraint::Percentage(11),
            Constraint::Percentage(11),
            Constraint::Percentage(11),
            Constraint::Percentage(12),
        ])
        .split(area);

    app.view_mode_btns.clear();

    let notes_style = if matches!(app.view_mode, ViewMode::Notes) {
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    };
    let notes_btn = Paragraph::new("Notes")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(notes_style);
    app.view_mode_btns.push((ViewMode::Notes, chunks[0]));
    frame.render_widget(notes_btn, chunks[0]);

    let planner_style = if matches!(app.view_mode, ViewMode::Planner) {
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };
    let planner_btn = Paragraph::new("Planner")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(planner_style);
    app.view_mode_btns.push((ViewMode::Planner, chunks[1]));
    frame.render_widget(planner_btn, chunks[1]);

    let journal_style = if matches!(app.view_mode, ViewMode::Journal) {
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Yellow)
    };
    let journal_btn = Paragraph::new("Journal")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(journal_style);
    app.view_mode_btns.push((ViewMode::Journal, chunks[2]));
    frame.render_widget(journal_btn, chunks[2]);

    let habits_style = if matches!(app.view_mode, ViewMode::Habits) {
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Magenta)
    };
    let habits_btn = Paragraph::new("Habits")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(habits_style);
    app.view_mode_btns.push((ViewMode::Habits, chunks[3]));
    frame.render_widget(habits_btn, chunks[3]);

    let finance_style = if matches!(app.view_mode, ViewMode::Finance) {
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };
    let finance_btn = Paragraph::new("Finances")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(finance_style);
    app.view_mode_btns.push((ViewMode::Finance, chunks[4]));
    frame.render_widget(finance_btn, chunks[4]);

    let cal_style = if matches!(app.view_mode, ViewMode::Calories) {
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red)
    };
    let cal_btn = Paragraph::new("Calories")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(cal_style);
    app.view_mode_btns.push((ViewMode::Calories, chunks[5]));
    frame.render_widget(cal_btn, chunks[5]);

    let kanban_style = if matches!(app.view_mode, ViewMode::Kanban) {
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::LightBlue)
    };
    let kanban_btn = Paragraph::new("Kanban")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(kanban_style);
    app.view_mode_btns.push((ViewMode::Kanban, chunks[6]));
    frame.render_widget(kanban_btn, chunks[6]);

    let cards_style = if matches!(app.view_mode, ViewMode::Flashcards) {
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::LightMagenta)
    };
    let cards_btn = Paragraph::new("Flashcards")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(cards_style);
    app.view_mode_btns.push((ViewMode::Flashcards, chunks[7]));
    frame.render_widget(cards_btn, chunks[7]);

    let search_style = if app.show_global_search {
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::LightGreen)
    };
    let search_btn = Paragraph::new("Search (Ctrl+F)")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(search_style);
    app.search_btn = chunks[8];
    frame.render_widget(search_btn, chunks[8]);
}

fn draw_left_panel(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    // Tree hierarchy
    draw_tree_panel(frame, app, chunks[0]);

    // Buttons
    let btn_chunks = split_equal_horizontal(chunks[1], 4);

    let add_nb_btn = Paragraph::new("New Notebook")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green));
    app.add_notebook_btn = btn_chunks[0];
    frame.render_widget(add_nb_btn, btn_chunks[0]);

    let add_sec_btn = Paragraph::new("New Section")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow));
    app.add_section_btn = btn_chunks[1];
    frame.render_widget(add_sec_btn, btn_chunks[1]);

    let add_pg_btn = Paragraph::new("New Page")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Blue));
    app.add_page_btn = btn_chunks[2];
    frame.render_widget(add_pg_btn, btn_chunks[2]);

    let del_btn = Paragraph::new("Delete Item")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Red));
    app.delete_btn = btn_chunks[3];
    frame.render_widget(del_btn, btn_chunks[3]);
}

fn draw_tree_panel(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let mut items = Vec::new();
    let mut tree_items = Vec::new();
    let mut row = 0u16;

    let inner_y = area.y + 1;
    let item_height = 1;

    for (nb_idx, notebook) in app.notebooks.iter().enumerate() {
        let is_current = nb_idx == app.current_notebook_idx;
        let selected = is_current && matches!(app.hierarchy_level, HierarchyLevel::Notebook);

        let nb_style = if selected {
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else if is_current {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let item_rect = Rect {
            x: area.x,
            y: inner_y + row,
            width: area.width,
            height: item_height,
        };
        tree_items.push((HierarchyLevel::Notebook, nb_idx, 0, 0, item_rect));
        items.push(ListItem::new(format!(" {}", notebook.title)).style(nb_style));
        row += 1;

        for (sec_idx, section) in notebook.sections.iter().enumerate() {
            let is_current_section = is_current && sec_idx == app.current_section_idx;
            let selected_section =
                is_current_section && matches!(app.hierarchy_level, HierarchyLevel::Section);

            let sec_style = if selected_section {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if is_current_section {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };

            let item_rect = Rect {
                x: area.x,
                y: inner_y + row,
                width: area.width,
                height: item_height,
            };
            tree_items.push((HierarchyLevel::Section, nb_idx, sec_idx, 0, item_rect));
            items.push(ListItem::new(format!("   {}", section.title)).style(sec_style));
            row += 1;

            for (pg_idx, page) in section.pages.iter().enumerate() {
                let is_current_page = is_current_section && pg_idx == app.current_page_idx;
                let selected_page =
                    is_current_page && matches!(app.hierarchy_level, HierarchyLevel::Page);

                let pg_style = if selected_page {
                    Style::default()
                        .bg(Color::Blue)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else if is_current_page {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                };

                let item_rect = Rect {
                    x: area.x,
                    y: inner_y + row,
                    width: area.width,
                    height: item_height,
                };
                tree_items.push((HierarchyLevel::Page, nb_idx, sec_idx, pg_idx, item_rect));
                items.push(ListItem::new(format!("      {}", page.title)).style(pg_style));
                row += 1;
            }
        }
    }

    app.tree_items = tree_items;

    let list = List::new(items).block(
        Block::default()
            .title("Tree (Left: select - Middle: rename - Right: delete)")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(list, area);
}

fn draw_content_panel(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(5)])
        .split(area);

    // Info panel with links and images count
    let info_text = match app.hierarchy_level {
        HierarchyLevel::Notebook => {
            if let Some(notebook) = app.current_notebook() {
                format!(
                    "Notes {}\nSections: {} | Created: {}",
                    notebook.title,
                    notebook.sections.len(),
                    notebook.created_at
                )
            } else {
                "No notebook selected".to_string()
            }
        }
        HierarchyLevel::Section => {
            if let Some(section) = app.current_section() {
                // Aggregate link/image counts across pages
                let mut links = 0usize;
                let mut images = 0usize;
                for p in &section.pages {
                    links += p.links.len();
                    images += p.images.len();
                }
                format!(
                    "Section {}\nPages: {} | Links {} | Images {} | Created: {}",
                    section.title,
                    section.pages.len(),
                    links,
                    images,
                    section.created_at
                )
            } else {
                "No section selected".to_string()
            }
        }
        HierarchyLevel::Page => {
            if let Some(page) = app.current_page() {
                format!(
                    "Page {} | Modified: {}\nLinks {} links | Images  {} images",
                    page.title,
                    page.modified_at,
                    page.links.len(),
                    page.images.len()
                )
            } else {
                "No page selected".to_string()
            }
        }
    };

    let info_panel = Paragraph::new(info_text)
        .block(Block::default().title("Info").borders(Borders::ALL))
        .style(Style::default().fg(Color::White));
    frame.render_widget(info_panel, chunks[0]);

    // Content panel - render with enhanced formatting
    if app.is_editing() {
        render_editing_panel(frame, app, chunks[1]);
    } else {
        render_formatted_content(frame, app, chunks[1]);
    }
}

fn render_editing_panel(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    // Inline mode is no longer used for content editing; fall through to textarea-based editing.

    let (title, _content) = match app.edit_target {
        EditTarget::NotebookTitle => (
            "Renaming Notebook (Ctrl+S to save, Esc to cancel)",
            app.editing_input.clone(),
        ),
        EditTarget::SectionTitle => (
            "Edit Renaming Section (Ctrl+S to save, Esc to cancel)",
            app.editing_input.clone(),
        ),
        EditTarget::PageTitle => (
            "Edit Renaming Page (Ctrl+S to save, Esc to cancel)",
            app.editing_input.clone(),
        ),
        EditTarget::PageContent => (
            "Editing Content (Ctrl+S to save, Esc to cancel)",
            app.editing_input.clone(),
        ),
        EditTarget::TaskTitle => (
            "Edit New Task (Ctrl+S to save, Esc to cancel)",
            app.editing_input.clone(),
        ),
        EditTarget::TaskDetails => (
            "Edit Task (Ctrl+S to save, Esc to cancel)",
            app.editing_input.clone(),
        ),
        EditTarget::JournalEntry => (
            "Edit Journal Entry (Ctrl+S to save, Esc to cancel)",
            app.editing_input.clone(),
        ),
        EditTarget::HabitNew => (
            "Edit New Habit - Fill Name/Frequency/Status fields (Ctrl+S to save, Esc to cancel)",
            app.editing_input.clone(),
        ),
        EditTarget::Habit => (
            "Edit Habit - Update Name/Frequency/Status fields (Ctrl+S to save, Esc to cancel)",
            app.editing_input.clone(),
        ),
        EditTarget::FinanceNew => (
            "Finance New Finance Entry (Ctrl+S to save, Esc to cancel)",
            app.editing_input.clone(),
        ),
        EditTarget::Finance => (
            "Finance Edit Finance Entry (Ctrl+S to save, Esc to cancel)",
            app.editing_input.clone(),
        ),
        EditTarget::CaloriesNew => (
            "Calories New Meal (Ctrl+S to save, Esc to cancel)",
            app.editing_input.clone(),
        ),
        EditTarget::Calories => (
            "Calories Edit Meal (Ctrl+S to save, Esc to cancel)",
            app.editing_input.clone(),
        ),
        EditTarget::KanbanNew => (
            "Kanban New Card (Ctrl+S to save, Esc to cancel)",
            app.editing_input.clone(),
        ),
        EditTarget::KanbanEdit => (
            "Kanban Edit Card (Ctrl+S to save, Esc to cancel)",
            app.editing_input.clone(),
        ),
        EditTarget::CardNew => (
            "New Flashcard - Format: front text\\n---\\nback text\\n---\\ncollection (optional) (Ctrl+S to save, Esc to cancel)",
            app.editing_input.clone(),
        ),
        EditTarget::CardEdit => (
            "Edit Flashcard - Format: front text\\n---\\nback text\\n---\\ncollection (optional) (Ctrl+S to save, Esc to cancel)",
            app.editing_input.clone(),
        ),
        EditTarget::CardImport => (
            "Import Flashcards - Enter file path (Ctrl+S to import, Esc to cancel)",
            app.editing_input.clone(),
        ),
        EditTarget::FindReplace => ("Find Find & Replace (Ctrl+H)", app.find_text.clone()),
        EditTarget::None => ("Content", String::new()),
    };

    // Special handling for Find and Replace
    if matches!(app.edit_target, EditTarget::FindReplace) {
        draw_find_replace_ui(frame, app, area);
        return;
    }

    app.content_edit_area = area;
    render_textarea_editor(frame, app, area, title);
}

fn render_formatted_content(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    app.content_edit_area = area;

    // Determine what to render based on the current hierarchy selection
    let content = match app.hierarchy_level {
        HierarchyLevel::Page => {
            if let Some(page) = app.current_page() {
                page.content.clone()
            } else {
                "(Select a page to view content)".to_string()
            }
        }
        HierarchyLevel::Section => {
            if let Some(section) = app.current_section() {
                // Aggregate all pages in the section into a single readable view
                let mut aggregated = String::new();
                for (idx, p) in section.pages.iter().enumerate() {
                    if idx > 0 {
                        aggregated.push_str("\n\n----------------------------------------\n\n");
                    }
                    aggregated.push_str(&format!("{}\n\n{}", p.title, p.content));
                }
                if aggregated.trim().is_empty() {
                    "(This section has no pages yet)".to_string()
                } else {
                    aggregated
                }
            } else {
                "(No section selected)".to_string()
            }
        }
        HierarchyLevel::Notebook => {
            if let Some(notebook) = app.current_notebook() {
                let mut overview = String::new();
                for (sidx, s) in notebook.sections.iter().enumerate() {
                    if sidx > 0 {
                        overview.push_str("\n\n----------------------------------------\n\n");
                    }
                    overview.push_str(&format!("Section: {} ({} pages)\n", s.title, s.pages.len()));
                    for p in &s.pages {
                        overview.push_str(&format!("  - {}\n", p.title));
                    }
                }
                if overview.trim().is_empty() {
                    "(This notebook has no sections yet)".to_string()
                } else {
                    overview
                }
            } else {
                "(No notebook selected)".to_string()
            }
        }
    };

    // Parse and render with highlighting
    let mut lines = Vec::new();
    let mut _y_offset = area.y + 1;
    let mut in_code_block = false;
    let mut code_lang = String::new();

    let content_lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < content_lines.len() {
        let line = content_lines[i];

        // Check for table start
        if line.trim().starts_with('|') && !in_code_block {
            let table_start = i;
            let mut table_end = i + 1;
            
            // Find end of table
            while table_end < content_lines.len() && content_lines[table_end].trim().starts_with('|') {
                table_end += 1;
            }

            // Extract and render table
            let table_text = content_lines[table_start..table_end].join("\n");
            if let Some(table_lines) = parse_and_render_table(&table_text) {
                let table_len = table_lines.len() as u16;
                lines.extend(table_lines);
                i = table_end;
                _y_offset += table_len;
                continue;
            }
        }

        // Check for flowchart markers - only if starting with > or numbered lists (not plain -)
        if (line.trim().starts_with('>') || line.trim().starts_with("1. ")) && !in_code_block {
            let flowchart_start = i;
            let mut flowchart_end = i + 1;
            
            // Find consecutive flowchart lines (>, -, or numbered)
            while flowchart_end < content_lines.len() {
                let next_line = content_lines[flowchart_end].trim();
                if next_line.is_empty() || (!next_line.starts_with('>') && !next_line.starts_with("- ") && !next_line.starts_with("1. ") && !next_line.starts_with("2. ")) {
                    break;
                }
                flowchart_end += 1;
            }

            // Extract and render flowchart
            let flowchart_text = content_lines[flowchart_start..flowchart_end].join("\n");
            if let Some(flowchart_lines) = parse_and_render_flowchart(&flowchart_text) {
                let flowchart_len = flowchart_lines.len() as u16;
                lines.extend(flowchart_lines);
                i = flowchart_end;
                _y_offset += flowchart_len;
                continue;
            }
        }

        // Regular line processing
        if line.starts_with("```") {
            in_code_block = !in_code_block;
            if in_code_block {
                code_lang = line.trim_start_matches("```").to_string();
                lines.push(Line::from(Span::styled(
                    line,
                    Style::default().fg(Color::DarkGray),
                )));
            } else {
                code_lang.clear();
                lines.push(Line::from(Span::styled(
                    line,
                    Style::default().fg(Color::DarkGray),
                )));
            }
        } else if in_code_block {
            // Syntax highlighted code
            lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(Color::Green),
            )));
        } else {
            // Regular text (links not rendered as clickable)
            lines.push(Line::from(line.to_string()));
        }

        i += 1;
        _y_offset += 1;
    }

    let title = match app.hierarchy_level {
        HierarchyLevel::Page => "Page Content (Scroll: Mouse wheel/Up/Down/PgUp/PgDn - Click to edit)",
        HierarchyLevel::Section => "Section View (aggregated) — scroll to read; select a page to edit",
        HierarchyLevel::Notebook => "Notebook Overview — sections and pages",
    };

    let content_block = Block::default()
        .title(title)
        .borders(Borders::ALL);

    let content_panel = Paragraph::new(lines)
        .block(content_block)
        .wrap(Wrap { trim: false })
        .scroll((app.content_scroll, 0));

    frame.render_widget(content_panel, area);
}

fn draw_find_replace_ui(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    // Split the area into sections: title, find input, replace input, buttons, and instructions
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Find input
            Constraint::Length(3), // Replace input
            Constraint::Length(3), // Buttons and info
            Constraint::Min(1),    // Status
        ])
        .split(area);

    // Find input field
    let find_style = if app.find_input_focus {
        Style::default().fg(Color::White).bg(Color::Blue)
    } else {
        Style::default().fg(Color::Gray)
    };

    let find_label = if !app.find_text.is_empty() {
        format!(
            "Find: {} | {} matches",
            app.find_text,
            app.current_page()
                .map(|p| p.content.matches(&app.find_text).count())
                .unwrap_or(0)
        )
    } else {
        "Find: (type search term)".to_string()
    };

    let find_widget = Paragraph::new(app.find_text.clone())
        .block(Block::default().title(find_label).borders(Borders::ALL))
        .style(find_style);
    frame.render_widget(find_widget, chunks[0]);

    // Replace input field
    let replace_style = if !app.find_input_focus {
        Style::default().fg(Color::White).bg(Color::Blue)
    } else {
        Style::default().fg(Color::Gray)
    };

    let replace_widget = Paragraph::new(app.replace_text.clone())
        .block(
            Block::default()
                .title("Replace with: (Tab to switch)")
                .borders(Borders::ALL),
        )
        .style(replace_style);
    frame.render_widget(replace_widget, chunks[1]);

    // Instructions
    let instructions = vec![
        Line::from("Tab: Switch field | Enter: Replace all | Esc: Cancel"),
        Line::from(format!(
            "Press Enter to replace all {} matches with '{}'",
            app.current_page()
                .map(|p| p.content.matches(&app.find_text).count())
                .unwrap_or(0),
            app.replace_text
        )),
    ];

    let info_widget = Paragraph::new(instructions)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(info_widget, chunks[2]);
}

fn draw_global_search_overlay(frame: &mut ratatui::Frame, app: &mut App) {
    let size = frame.size();
    let width = size.width.saturating_mul(3) / 4;
    let height = size.height.saturating_mul(3) / 4;
    let x = size.x + (size.width.saturating_sub(width)) / 2;
    let y = size.y + (size.height.saturating_sub(height)) / 2;
    let area = Rect { x, y, width, height };

    frame.render_widget(Clear, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    let input_label = format!(
        "Global Search (Esc to close, Enter to open, ↑↓ navigate) — {} results",
        app.global_search_results.len()
    );
    let input_widget = Paragraph::new(app.global_search_query.clone())
        .block(Block::default().title(input_label).borders(Borders::ALL))
        .style(Style::default().fg(Color::White).bg(Color::DarkGray));
    frame.render_widget(input_widget, layout[0]);

    let list_area = layout[1];
    app.search_result_items.clear();

    if app.global_search_results.is_empty() {
        let hint = Paragraph::new("Type to search across notes, tasks, journal, habits, finance, calories, and kanban.")
            .block(Block::default().title("Results").borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(hint, list_area);
        return;
    }

    let max_rows = list_area.height.saturating_sub(2) as usize;
    let offset = if app.global_search_selected >= max_rows {
        app.global_search_selected + 1 - max_rows
    } else {
        0
    };

    let visible = app
        .global_search_results
        .iter()
        .enumerate()
        .skip(offset)
        .take(max_rows)
        .collect::<Vec<_>>();

    let mut items = Vec::new();
    let mut row_idx = 0u16;

    for (idx, hit) in visible {
        let selected = idx == app.global_search_selected;
        let style = if selected {
            Style::default().bg(Color::Blue).fg(Color::White)
        } else {
            Style::default()
        };

        let text = format!("{} — {}", hit.title, hit.detail);
        let item_rect = Rect {
            x: list_area.x,
            y: list_area.y + 1 + row_idx,
            width: list_area.width,
            height: 1,
        };
        app.search_result_items.push((idx, item_rect));

        items.push(ListItem::new(text).style(style));
        row_idx += 1;
    }

    let list = List::new(items)
        .block(Block::default().title("Results").borders(Borders::ALL))
        .highlight_symbol("▶ ");
    frame.render_widget(list, list_area);
}

fn draw_validation_error_popup(frame: &mut ratatui::Frame, app: &App) {
    let size = frame.size();
    let area = get_popup_area(size.width, size.height, 70, 38);

    let block = Block::default()
        .title("[!] Validation Error")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::Red).bg(Color::Black));

    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(inner);

    // Error message
    let para = Paragraph::new(app.validation_error_message.as_str())
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));
    frame.render_widget(para, chunks[0]);

    // Dismiss hint
    let hint = Paragraph::new("Press Esc to dismiss")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray).italic());
    frame.render_widget(hint, chunks[1]);
}

fn draw_success_popup(frame: &mut ratatui::Frame, app: &App) {
    let size = frame.size();
    let area = get_popup_area(size.width, size.height, 55, 28);

    let block = Block::default()
        .title("[OK] Import Complete")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::Green).bg(Color::Black));

    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(2), Constraint::Length(1)])
        .split(inner);

    let para = Paragraph::new(app.success_message.as_str())
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));
    frame.render_widget(para, chunks[0]);

    let hint = Paragraph::new("Press Esc to dismiss")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray).italic());
    frame.render_widget(hint, chunks[1]);
}


fn draw_help_overlay(frame: &mut ratatui::Frame, app: &App) {
    let size = frame.size();
    let width = size.width.saturating_mul(3) / 4;
    let height = size.height.saturating_mul(3) / 4;
    let x = size.x + (size.width.saturating_sub(width)) / 2;
    let y = size.y + (size.height.saturating_sub(height)) / 2;
    let area = Rect { x, y, width, height };

    frame.render_widget(Clear, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    let query_text = if app.help_search_query.is_empty() {
        "Type to filter tips".to_string()
    } else {
        app.help_search_query.clone()
    };

    let input_label = "Quick Help (Esc to close)";
    let input_widget = Paragraph::new(query_text)
        .block(Block::default().title(input_label).borders(Borders::ALL))
        .style(Style::default().fg(Color::White).bg(Color::DarkGray));
    frame.render_widget(input_widget, layout[0]);

    let query = app.help_search_query.to_lowercase();
    let filtered: Vec<&HelpTopic> = HELP_TOPICS
        .iter()
        .filter(|topic| {
            if query.trim().is_empty() {
                return true;
            }
            topic.title.to_lowercase().contains(&query)
                || topic.detail.to_lowercase().contains(&query)
        })
        .collect();

    let mut lines: Vec<Line> = Vec::new();
    for topic in filtered {
        lines.push(Line::from(Span::styled(
            topic.title,
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(topic.detail));
        lines.push(Line::from(""));
    }

    if lines.is_empty() {
        lines.push(Line::from(
            "No tips match that search. Try words like 'flashcards', 'mouse', or 'bulk'.",
        ));
    } else {
        lines.push(Line::from(
            "Tip: Use Shift+Arrow in flashcards or double-click items for shortcuts.",
        ));
    }

    let help_block = Paragraph::new(lines)
        .block(Block::default().title("Tips (↑↓ or mouse wheel to scroll)").borders(Borders::ALL))
        .wrap(Wrap { trim: false })
        .scroll((app.help_scroll, 0))
        .style(Style::default().fg(Color::White));
    frame.render_widget(help_block, layout[1]);
}

fn draw_spell_check_popup(frame: &mut ratatui::Frame, app: &App) {
    let size = frame.size();
    let area = get_popup_area(size.width, size.height, 70, 28);

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title("Spell Check (Esc to close, Enter/1-9 replace, 'a' add word)")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::White).bg(Color::Black));
    frame.render_widget(block.clone(), area);

    let inner = block.inner(area);
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(5)])
        .split(inner);

    // Header info
    let header = Paragraph::new(format!(
        "{} potential issues found",
        app.spell_check_results.len()
    ))
    .style(Style::default().fg(Color::Yellow))
    .alignment(Alignment::Center);
    frame.render_widget(header, layout[0]);

    // Results list
    let mut lines: Vec<Line> = Vec::new();
    for (idx, res) in app.spell_check_results.iter().enumerate() {
        let marker = if idx == app.spell_check_selected { ">" } else { " " };
        let pos = format!("Ln {}, Col {}", res.line_number, res.column + 1);
        let suggestions = if res.suggestions.is_empty() {
            "(no suggestions)".to_string()
        } else {
            res.suggestions
                .iter()
                .take(5)
                .enumerate()
                .map(|(i, s)| format!("{}:{}", i + 1, s))
                .collect::<Vec<_>>()
                .join("  ")
        };

        lines.push(Line::from(vec![
            Span::styled(marker, Style::default().fg(Color::Cyan)),
            Span::raw(" "),
            Span::styled(pos, Style::default().fg(Color::Gray)),
            Span::raw("  "),
            Span::styled(
                res.word.as_str(),
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  →  "),
            Span::styled(suggestions, Style::default().fg(Color::Green)),
        ]));
    }

    if lines.is_empty() {
        lines.push(Line::from("No spelling issues found."));
    }

    let list = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: false })
        .scroll((app.spell_check_scroll, 0));
    frame.render_widget(list, layout[1]);
}

// Removed image overlay
// fn draw_image_preview_overlay(_frame: &mut ratatui::Frame, _app: &App) {}

fn draw_calendar_picker(frame: &mut ratatui::Frame, app: &mut App) {
    let size = frame.size();
    let width = 50.min(size.width.saturating_sub(4));
    let height = 20.min(size.height.saturating_sub(4));
    let x = size.x + (size.width.saturating_sub(width)) / 2;
    let y = size.y + (size.height.saturating_sub(height)) / 2;
    let area = Rect { x, y, width, height };

    frame.render_widget(Clear, area);

    let outer_block = Block::default()
        .title("Select Date (Esc to cancel)")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan).bg(Color::Black));
    frame.render_widget(outer_block, area);

    let inner_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(10)])
        .split(inner_area);

    // Year/Month selector and help
    let month_name = match app.calendar_month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    };
    
    let header_text = vec![
        Line::from(vec![
            Span::styled("◄ ", Style::default().fg(Color::Cyan)),
            Span::styled(format!("{} {}", month_name, app.calendar_year), 
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(" ►", Style::default().fg(Color::Cyan)),
        ]),
        Line::from(
            Span::styled("←/→: month  ↑/↓: year  Click day to select", 
                Style::default().fg(Color::Gray))
        ),
    ];

    let year_month_widget = Paragraph::new(header_text)
        .alignment(Alignment::Center);
    frame.render_widget(year_month_widget, layout[0]);

    // Calendar grid
    let calendar_area = layout[1];
    draw_calendar_grid(frame, app, calendar_area);
}

fn draw_calendar_grid(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    use chrono::Datelike;

    app.calendar_day_rects.clear();

    let first_day = match NaiveDate::from_ymd_opt(app.calendar_year, app.calendar_month, 1) {
        Some(d) => d,
        None => return,
    };

    let weekday_offset = first_day.weekday().num_days_from_monday() as usize;
    let days_in_month: u32 = match app.calendar_month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if app.calendar_year % 400 == 0 || (app.calendar_year % 4 == 0 && app.calendar_year % 100 != 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    };

    let mut lines = Vec::new();
    
    // Header
    lines.push(Line::from(vec![
        Span::styled(" Mo ", Style::default().fg(Color::Cyan)),
        Span::styled(" Tu ", Style::default().fg(Color::Cyan)),
        Span::styled(" We ", Style::default().fg(Color::Cyan)),
        Span::styled(" Th ", Style::default().fg(Color::Cyan)),
        Span::styled(" Fr ", Style::default().fg(Color::Cyan)),
        Span::styled(" Sa ", Style::default().fg(Color::Yellow)),
        Span::styled(" Su ", Style::default().fg(Color::Yellow)),
    ]));
    lines.push(Line::from(""));

    // Days
    let mut day: u32 = 1;
    let total_cells = weekday_offset + days_in_month as usize;
    let rows = (total_cells + 6) / 7;

    for week in 0..rows {
        let mut week_spans = Vec::new();
        for day_of_week in 0..7 {
            let cell_idx = week * 7 + day_of_week;
            if cell_idx < weekday_offset || day > days_in_month {
                week_spans.push(Span::raw("    "));
            } else {
                let is_today = if let Some(current_date) = NaiveDate::from_ymd_opt(app.calendar_year, app.calendar_month, day) {
                    current_date == Local::now().date_naive()
                } else {
                    false
                };
                
                let style = if is_today {
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                } else if day_of_week >= 5 {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::White)
                };
                
                // Track clickable area for this day
                let day_rect = Rect {
                    x: area.x + (day_of_week * 4) as u16,
                    y: area.y + 2 + week as u16,
                    width: 4,
                    height: 1,
                };
                app.calendar_day_rects.push((day, day_rect));
                
                week_spans.push(Span::styled(format!(" {:2} ", day), style));
                day += 1;
            }
        }
        lines.push(Line::from(week_spans));
    }

    let calendar_widget = Paragraph::new(lines)
        .block(Block::default())
        .alignment(Alignment::Left);
    frame.render_widget(calendar_widget, area);
}

fn textarea_lines_with_cursor(app: &App, height: u16) -> Vec<Line<'static>> {
    let (cursor_row, cursor_col) = app.textarea.cursor();
    let mut lines = Vec::new();
    let text_lines = app.textarea.lines();

    if text_lines.is_empty() {
        lines.push(Line::from("|"));
        return lines;
    }

    for (idx, line) in text_lines.iter().enumerate() {
        if idx == cursor_row {
            let char_col = cursor_col.min(line.chars().count());
            let mut new_line = String::new();
            for (i, c) in line.chars().enumerate() {
                if i == char_col {
                    new_line.push('|');
                }
                new_line.push(c);
            }
            if char_col == line.chars().count() {
                new_line.push('|');
            }
            lines.push(Line::from(Span::styled(
                new_line,
                Style::default().fg(Color::Yellow).bg(Color::Rgb(30, 30, 40)),
            )));
        } else if app.selection_all {
            lines.push(Line::from(Span::styled(
                line.clone(),
                Style::default().bg(Color::DarkGray),
            )));
        } else {
            lines.push(Line::from(line.clone()));
        }
    }
    let view_height = height.max(1) as usize;
    if lines.len() > view_height {
        let start = cursor_row.saturating_sub(view_height.saturating_sub(1));
        let end = (start + view_height).min(lines.len());
        lines[start..end].to_vec()
    } else {
        lines
    }
}

fn render_textarea_editor(
    frame: &mut ratatui::Frame,
    app: &App,
    area: Rect,
    title: &str,
) {
    let inner_height = area.height.saturating_sub(2); // account for borders
    let lines_display = textarea_lines_with_cursor(app, inner_height);
    let panel = Paragraph::new(lines_display)
        .block(Block::default().title(title).borders(Borders::ALL))
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::Yellow));

    frame.render_widget(panel, area);
}

fn task_help_lines() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from("Tasks PLANNER - TASK MANAGEMENT"),
        Line::from(""),
        Line::from("Features:"),
        Line::from("  - Add tasks with priorities (High/Medium/Low)"),
        Line::from("  - Set due dates and reminders with times"),
        Line::from("  - Track completion status"),
        Line::from("  - Recurring tasks (daily/weekly/monthly or date ranges)"),
        Line::from(""),
        Line::from("How to use:"),
        Line::from("  1. Click 'New Task' to create a new task"),
        Line::from("  2. First line is the title"),
        Line::from("  3. Add details on following lines"),
        Line::from("  4. Middle-click task to toggle done/undone"),
        Line::from("  5. Right-click task to delete it"),
        Line::from("  6. Edit metadata inline: Title/Status/Priority/Due/Reminder/Repeat"),
        Line::from(""),
        Line::from("Special syntax in task editor:"),
        Line::from("  - Reminder: 2025-12-25 09:00 or 2025-12-25"),
        Line::from("  - Repeat: daily|weekly|monthly"),
        Line::from("  - Repeat range: range 2025-12-01 to 2025-12-31 at 08:00"),
        Line::from("  - Due: 2025-12-31 (due date)"),
        Line::from(""),
        Line::from("Middle-click toggles complete; Right-click deletes"),
    ]
}

fn recurrence_label(rec: Recurrence) -> String {
    match rec {
        Recurrence::None => "None".to_string(),
        Recurrence::Daily => "Daily".to_string(),
        Recurrence::Weekly => "Weekly".to_string(),
        Recurrence::Monthly => "Monthly".to_string(),
        Recurrence::Range { start, end, time } => {
            if let Some(t) = time {
                format!("Range {} to {} @ {}", start, end, t.format("%H:%M"))
            } else {
                format!("Range {} to {}", start, end)
            }
        }
    }
}

fn parse_recurrence(text: &str) -> Recurrence {
    let lowered = text.trim().to_lowercase();
    match lowered.as_str() {
        "daily" => Recurrence::Daily,
        "weekly" => Recurrence::Weekly,
        "monthly" => Recurrence::Monthly,
        _ => {
            // Range format examples:
            // "range 2025-01-01 to 2025-01-31"
            // "range 2025-01-01 to 2025-01-31 at 09:00"
            // "from 2025-01-01 to 2025-02-15 at 18:30"
            if lowered.starts_with("range") || lowered.starts_with("from") {
                let cleaned = lowered
                    .trim_start_matches("range")
                    .trim_start_matches("from")
                    .trim();
                let parts: Vec<&str> = cleaned.split("to").map(|s| s.trim()).collect();
                if parts.len() >= 2 {
                    let start_str = parts[0];
                    let mut end_part = parts[1];
                    let mut time: Option<NaiveTime> = None;
                    if let Some(pos) = end_part.find("at ") {
                        let time_str = end_part[pos + 3..].trim();
                        end_part = end_part[..pos].trim();
                        if let Ok(t) = NaiveTime::parse_from_str(time_str, "%H:%M") {
                            time = Some(t);
                        }
                    }

                    if let (Ok(start), Ok(end)) = (
                        NaiveDate::parse_from_str(start_str, "%Y-%m-%d"),
                        NaiveDate::parse_from_str(end_part, "%Y-%m-%d"),
                    ) {
                        return Recurrence::Range { start, end, time };
                    }
                }
            }
            Recurrence::None
        }
    }
}

fn format_task_editor_content(task: &Task) -> String {
    let status = if task.completed { "Completed" } else { "Pending" };
    let due = task
        .due_date
        .map(|d| d.to_string())
        .unwrap_or_else(|| "Not set".to_string());
    let reminder = match (task.reminder_date, task.reminder_time, task.reminder_text.as_ref()) {
        (Some(d), Some(t), _) => format!("{} {}", d, t.format("%H:%M")),
        (Some(d), None, _) => d.to_string(),
        (None, _, Some(t)) => t.clone(),
        (None, _, None) => "None".to_string(),
    };

    format!(
        "Title: {}\nStatus: {}\nPriority: {:?}\nCreated: {}\nDue: {}\nReminder: {}\nRepeat: {}\n\nDescription:\n{}",
        task.title,
        status,
        task.priority,
        task.created_at,
        due,
        reminder,
        recurrence_label(task.recurrence),
        task.description
    )
}

// ============================================================================
// TASK EDITOR - Templates, formatting, and parsing
// ============================================================================

fn new_task_editor_template() -> String {
    let today = Local::now().date_naive();
    format!(
        "Title: \nStatus: Pending (options: Pending|Completed)\nPriority: Medium (options: High|Medium|Low)\nCreated: {}\nDue: Not set\nReminder: None (e.g. 2025-12-25 09:30)\nRepeat: none (options: none|daily|weekly|monthly|range YYYY-MM-DD to YYYY-MM-DD at HH:MM)\n\nDescription:\n",
        today
    )
}

fn parse_task_editor_content(input: &str, existing: Option<&Task>, created_fallback: NaiveDate) -> Task {
    let mut task = existing.cloned().unwrap_or_else(|| Task::new(String::new(), String::new()));

    if existing.is_none() {
        task.created_at = created_fallback;
    }

    let mut title: Option<String> = None;
    let mut status: Option<bool> = None;
    let mut priority: Option<TaskPriority> = None;
    let mut created_at = task.created_at;
    let mut due: Option<NaiveDate> = None;
    let mut reminder_date: Option<NaiveDate> = None;
    let mut reminder_time: Option<NaiveTime> = task.reminder_time;
    let mut reminder_text: Option<String> = None;
    let mut recurrence = task.recurrence;

    let mut description_lines: Vec<String> = Vec::new();
    let mut in_description = false;

    for line in input.lines() {
        if in_description {
            description_lines.push(line.to_string());
            continue;
        }

        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();

        if lower.starts_with("description:") {
            let after = line.splitn(2, ':').nth(1).unwrap_or("").trim_start();
            description_lines.push(after.to_string());
            in_description = true;
            continue;
        }

        if lower.starts_with("title:") {
            let value = line.splitn(2, ':').nth(1).unwrap_or("").trim();
            // Validate title length (max 200 characters)
            if value.len() <= 200 {
                title = Some(value.to_string());
            }
            continue;
        }

        if lower.starts_with("status:") {
            let after = line.splitn(2, ':').nth(1).unwrap_or("").trim().to_lowercase();
            status = Some(after.contains("done") || after.contains("complete"));
            continue;
        }

        if lower.starts_with("priority:") {
            let after = line.splitn(2, ':').nth(1).unwrap_or("").trim().to_lowercase();
            priority = match after.as_str() {
                "high" => Some(TaskPriority::High),
                "medium" => Some(TaskPriority::Medium),
                "low" => Some(TaskPriority::Low),
                _ => None,
            };
            continue;
        }

        if lower.starts_with("created:") {
            if let Some(val) = line.splitn(2, ':').nth(1) {
                if let Ok(d) = NaiveDate::parse_from_str(val.trim(), "%Y-%m-%d") {
                    // Validate date is reasonable
                    let max_date = Local::now().date_naive() + chrono::Duration::days(3650);
                    let min_date = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
                    if d >= min_date && d <= max_date {
                        created_at = d;
                    }
                }
            }
            continue;
        }

        if lower.starts_with("due:") {
            let after = line.splitn(2, ':').nth(1).unwrap_or("").trim();
            if after.eq_ignore_ascii_case("not set") || after.is_empty() {
                due = None;
            } else if let Ok(d) = NaiveDate::parse_from_str(after, "%Y-%m-%d") {
                // Validate date is reasonable
                let max_date = Local::now().date_naive() + chrono::Duration::days(3650);
                let min_date = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
                if d >= min_date && d <= max_date {
                    due = Some(d);
                }
            }
            continue;
        }

        if lower.starts_with("reminder:") {
            let after = line.splitn(2, ':').nth(1).unwrap_or("").trim();
            if after.eq_ignore_ascii_case("none") || after.is_empty() || after.eq_ignore_ascii_case("not set") {
                reminder_date = None;
                reminder_time = None;
                reminder_text = None;
            } else {
                // Expect formats: "YYYY-MM-DD" or "YYYY-MM-DD HH:MM"; otherwise treat as text
                let mut parts = after.split_whitespace();
                let date_part = parts.next();
                let time_part = parts.next();

                if let Some(date_str) = date_part {
                    if let Ok(d) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                        let today = Local::now().date_naive();
                        // Validate date is reasonable
                        let max_date = today + chrono::Duration::days(3650);
                        let min_date = today;
                        if d >= min_date && d <= max_date {
                            reminder_date = Some(d);
                            if let Some(t_str) = time_part {
                                if let Ok(t) = NaiveTime::parse_from_str(t_str, "%H:%M") {
                                    reminder_time = Some(t);
                                }
                            }
                            reminder_text = None;
                            continue;
                        }
                    }
                }

                // Fallback to free text
                reminder_text = Some(after.to_string());
                reminder_date = None;
                reminder_time = None;
            }
            continue;
        }

        if lower.starts_with("repeat:") {
            let after = line.splitn(2, ':').nth(1).unwrap_or("").trim();
            recurrence = parse_recurrence(after);
            continue;
        }

        // Fallback: first non-empty line becomes title if not set yet
        if title.is_none() && !trimmed.is_empty() {
            // Validate title length (max 200 characters)
            if trimmed.len() <= 200 {
                title = Some(trimmed.to_string());
            }
        }
    }

    let description = description_lines.join("\n").trim_start_matches('\n').to_string();
    // Validate description length (max 10,000 characters)
    let validated_description = if description.len() <= 10_000 {
        description
    } else {
        // Truncate if too long
        description.chars().take(10_000).collect()
    };

    if let Some(t) = title {
        if !t.is_empty() {
            task.title = t;
        }
    }
    if let Some(s) = status {
        task.completed = s;
    }
    if let Some(p) = priority {
        task.priority = p;
    }
    task.created_at = created_at;
    task.due_date = due;
    task.reminder_date = reminder_date;
    task.reminder_text = reminder_text;
    task.reminder_time = reminder_time;
    task.recurrence = recurrence;
    task.description = validated_description;

    if task.title.trim().is_empty() {
        task.title = "Untitled Task".to_string();
    }

    task
}

// ============================================================================
// TASK VALIDATORS - Parameter validation with clear error messages
// ============================================================================

fn validate_task_status(text: &str) -> Result<bool, String> {
    match text.trim().to_lowercase().as_str() {
        "pending" => Ok(false),
        "completed" => Ok(true),
        _ => Err("Invalid Status. Valid options: Pending|Completed".to_string()),
    }
}

fn validate_task_priority(text: &str) -> Result<TaskPriority, String> {
    match text.trim().to_lowercase().as_str() {
        "high" => Ok(TaskPriority::High),
        "medium" => Ok(TaskPriority::Medium),
        "low" => Ok(TaskPriority::Low),
        _ => Err("Invalid Priority. Valid options: High|Medium|Low".to_string()),
    }
}

fn validate_task_recurrence(text: &str) -> Result<Recurrence, String> {
    let trimmed = text.trim().to_lowercase();
    match trimmed.as_str() {
        "none" => Ok(Recurrence::None),
        "daily" => Ok(Recurrence::Daily),
        "weekly" => Ok(Recurrence::Weekly),
        "monthly" => Ok(Recurrence::Monthly),
        _ if trimmed.starts_with("range") || trimmed.starts_with("from") => {
            let rec = parse_recurrence(text);
            if matches!(rec, Recurrence::None) {
                Err("Invalid range format. Use: range YYYY-MM-DD to YYYY-MM-DD at HH:MM".to_string())
            } else {
                Ok(rec)
            }
        }
        _ => Err("Invalid Repeat. Valid options: none|daily|weekly|monthly|range YYYY-MM-DD to YYYY-MM-DD at HH:MM".to_string()),
    }
}

fn habit_help_lines() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from("Habits - ROUTINE BUILDER"),
        Line::from(""),
        Line::from("Editor format (fill the values):"),
        Line::from("  Name: Drink Water"),
        Line::from("  Frequency: daily | weekly | monthly | range 2025-01-01 to 2025-02-01"),
        Line::from("  Status: Active | Paused"),
        Line::from("  Start Date: 2025-12-18"),
        Line::from("  Notes: (any details on following lines)"),
        Line::from(""),
        Line::from("Workflow:"),
        Line::from("  1. Click 'New Habit'"),
        Line::from("  2. Update Name/Frequency/Status/Start Date"),
        Line::from("  3. Add Notes (optional)"),
        Line::from("  4. Use 'Mark Done' by date"),
        Line::from(""),
        Line::from("Tips:"),
        Line::from("  - Frequency accepts range syntax: range 2025-01-01 to 2025-01-31"),
        Line::from("  - Start Date defaults to the selected day"),
        Line::from("  - Marking done updates streaks automatically"),
    ]
}

fn habit_status_label(status: HabitStatus) -> &'static str {
    match status {
        HabitStatus::Active => "Active",
        HabitStatus::Paused => "Paused",
    }
}

fn parse_habit_status(text: &str) -> HabitStatus {
    match text.trim().to_lowercase().as_str() {
        "paused" => HabitStatus::Paused,
        _ => HabitStatus::Active,
    }
}

// ============================================================================
// VALIDATORS - Consolidated parameter validation with clear error messages
// ============================================================================

fn validate_frequency(text: &str) -> Result<Recurrence, String> {
    let trimmed = text.trim().to_lowercase();
    match trimmed.as_str() {
        "daily" => Ok(Recurrence::Daily),
        "weekly" => Ok(Recurrence::Weekly),
        "monthly" => Ok(Recurrence::Monthly),
        _ if trimmed.starts_with("range") || trimmed.starts_with("from") => {
            let rec = parse_recurrence(text);
            if matches!(rec, Recurrence::None) {
                Err("Invalid range format. Use: range YYYY-MM-DD to YYYY-MM-DD at HH:MM".to_string())
            } else {
                Ok(rec)
            }
        }
        _ => Err(format!(
            "Invalid Frequency. Valid options: daily|weekly|monthly|range YYYY-MM-DD to YYYY-MM-DD at HH:MM"
        )),
    }
}

fn validate_habit_status(text: &str) -> Result<HabitStatus, String> {
    match text.trim().to_lowercase().as_str() {
        "active" => Ok(HabitStatus::Active),
        "paused" => Ok(HabitStatus::Paused),
        _ => Err("Invalid Status. Valid options: Active|Paused".to_string()),
    }
}

// ============================================================================
// HABIT EDITOR - Templates, formatting, and parsing
// ============================================================================

fn new_habit_editor_template(selected_date: NaiveDate) -> String {
    format!(
        "Name: \nFrequency: daily (options: daily|weekly|monthly|range YYYY-MM-DD to YYYY-MM-DD at HH:MM)\nStatus: Active (options: Active|Paused)\nStart Date: {}\nNotes:\n",
        selected_date
    )
}

fn format_habit_editor_content(habit: &Habit) -> String {
    format!(
        "Name: {}\nFrequency: {}\nStatus: {}\nStart Date: {}\nNotes:\n{}",
        habit.name,
        recurrence_label(habit.frequency),
        habit_status_label(habit.status),
        habit.start_date,
        habit.notes
    )
}

fn parse_habit_editor_content(
    input: &str,
    existing: Option<&Habit>,
    default_start_date: NaiveDate,
) -> Option<Habit> {
    let mut habit = existing.cloned().unwrap_or_else(|| Habit::new(String::new()));
    if existing.is_none() {
        habit.start_date = default_start_date;
        habit.status = HabitStatus::Active;
        habit.marks.clear();
        habit.streak = 0;
    }
    habit.notes.clear();

    let mut in_notes = false;
    let mut notes_lines: Vec<String> = Vec::new();

    for line in input.lines() {
        if in_notes {
            notes_lines.push(line.to_string());
            continue;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Name:") {
            let value = rest.trim();
            if !value.is_empty() {
                // Validate name length (max 100 characters)
                if value.len() <= 100 {
                    habit.name = value.to_string();
                } else {
                    return None;
                }
            } else if existing.is_none() {
                habit.name.clear();
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Frequency:") {
            let value = rest.trim();
            if !value.is_empty() {
                // Extract just the value part before any options hint
                let actual_value = value.split(" (options:").next().unwrap_or(value).trim();
                habit.frequency = parse_recurrence(actual_value);
            } else if existing.is_none() {
                habit.frequency = Recurrence::Daily;
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Status:") {
            let value = rest.trim();
            if !value.is_empty() {
                // Extract just the value part before any options hint
                let actual_value = value.split(" (options:").next().unwrap_or(value).trim();
                habit.status = parse_habit_status(actual_value);
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Start Date:") {
            let value = rest.trim();
            if !value.is_empty() {
                if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
                    // Validate date is reasonable
                    let max_date = Local::now().date_naive();
                    let min_date = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
                    if date >= min_date && date <= max_date {
                        habit.start_date = date;
                    } else {
                        return None;
                    }
                }
            } else if existing.is_none() {
                habit.start_date = default_start_date;
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Notes:") {
            let value = rest.trim_start();
            if !value.is_empty() {
                notes_lines.push(value.to_string());
            }
            in_notes = true;
            continue;
        }
    }

    if in_notes {
        let body = notes_lines.join("\n");
        let notes_text = body.trim_end_matches('\n').to_string();
        // Validate notes length (max 10,000 characters)
        habit.notes = if notes_text.len() <= 10_000 {
            notes_text
        } else {
            notes_text.chars().take(10_000).collect()
        };
    }

    if habit.name.trim().is_empty() {
        return None;
    }

    Some(habit)
}

fn parse_and_validate_habit(
    input: &str,
    existing: Option<&Habit>,
    default_start_date: NaiveDate,
) -> Result<Habit, String> {
    // First pass: basic parsing
    let mut temp_habit = existing.cloned().unwrap_or_else(|| Habit::new(String::new()));
    if existing.is_none() {
        temp_habit.start_date = default_start_date;
        temp_habit.status = HabitStatus::Active;
        temp_habit.marks.clear();
        temp_habit.streak = 0;
    }

    let mut frequency_value: Option<String> = None;
    let mut status_value: Option<String> = None;

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Frequency:") {
            let value = rest.trim().split(" (options:").next().unwrap_or("").trim();
            if !value.is_empty() {
                frequency_value = Some(value.to_string());
            }
        }

        if let Some(rest) = trimmed.strip_prefix("Status:") {
            let value = rest.trim().split(" (options:").next().unwrap_or("").trim();
            if !value.is_empty() {
                status_value = Some(value.to_string());
            }
        }
    }

    // Validate Frequency
    if let Some(freq) = frequency_value {
        temp_habit.frequency = validate_frequency(&freq)?;
    } else if existing.is_none() {
        temp_habit.frequency = Recurrence::Daily;
    }

    // Validate Status
    if let Some(stat) = status_value {
        temp_habit.status = validate_habit_status(&stat)?;
    } else if existing.is_none() {
        temp_habit.status = HabitStatus::Active;
    }

    // Parse the rest normally
    let parsed = parse_habit_editor_content(input, existing, default_start_date).ok_or(
        "Invalid habit: missing required fields".to_string(),
    )?;

    Ok(parsed)
}

fn parse_and_validate_task(input: &str, existing: Option<&Task>) -> Result<Task, String> {
    // First pass: extract Status, Priority, and Recurrence values
    let mut status_value: Option<String> = None;
    let mut priority_value: Option<String> = None;
    let mut repeat_value: Option<String> = None;

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Status:") {
            let value = rest.trim().split(" (options:").next().unwrap_or("").trim();
            if !value.is_empty() {
                status_value = Some(value.to_string());
            }
        }

        if let Some(rest) = trimmed.strip_prefix("Priority:") {
            let value = rest.trim().split(" (options:").next().unwrap_or("").trim();
            if !value.is_empty() {
                priority_value = Some(value.to_string());
            }
        }

        if let Some(rest) = trimmed.strip_prefix("Repeat:") {
            let value = rest.trim().split(" (options:").next().unwrap_or("").trim();
            if !value.is_empty() {
                repeat_value = Some(value.to_string());
            }
        }
    }

    // Validate Status (Pending/Completed)
    let completed = if let Some(stat) = status_value {
        validate_task_status(&stat)?
    } else if existing.is_none() {
        false
    } else {
        existing.map(|t| t.completed).unwrap_or(false)
    };

    // Validate Priority
    let priority = if let Some(prio) = priority_value {
        validate_task_priority(&prio)?
    } else if existing.is_none() {
        TaskPriority::Medium
    } else {
        existing.map(|t| t.priority.clone()).unwrap_or(TaskPriority::Medium)
    };

    // Validate Recurrence
    let recurrence = if let Some(rep) = repeat_value {
        validate_task_recurrence(&rep)?
    } else if existing.is_none() {
        Recurrence::None
    } else {
        existing.map(|t| t.recurrence.clone()).unwrap_or(Recurrence::None)
    };

    // Parse the rest normally
    let created_date = existing.map(|t| t.created_at).unwrap_or_else(|| chrono::Local::now().date_naive());
    let mut parsed = parse_task_editor_content(input, existing, created_date);

    // Override with validated values
    parsed.completed = completed;
    parsed.priority = priority;
    parsed.recurrence = recurrence;

    Ok(parsed)
}

fn new_finance_editor_template(selected_date: NaiveDate) -> String {
    format!(
        "Category: \nAmount: \nDate: {}\nNotes:\n",
        selected_date
    )
}

fn format_finance_editor_content(entry: &FinanceEntry) -> String {
    format!(
        "Category: {}\nAmount: {:.2}\nDate: {}\nNotes:\n{}",
        entry.category, entry.amount, entry.date, entry.note
    )
}

fn parse_finance_editor_content(
    input: &str,
    existing: Option<&FinanceEntry>,
    default_date: NaiveDate,
) -> Option<FinanceEntry> {
    let mut entry = existing.cloned().unwrap_or_else(|| FinanceEntry::new(
        default_date,
        String::new(),
        String::new(),
        0.0,
    ));
    if existing.is_none() {
        entry.date = default_date;
    }
    entry.note.clear();

    let mut category: Option<String> = None;
    let mut amount: Option<f64> = None;
    let mut in_notes = false;
    let mut notes_lines: Vec<String> = Vec::new();

    for line in input.lines() {
        if in_notes {
            notes_lines.push(line.to_string());
            continue;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Category:") {
            let value = rest.trim();
            if !value.is_empty() {
                // Validate category name length (max 100 characters)
                if value.len() <= 100 {
                    category = Some(value.to_string());
                } else {
                    return None;
                }
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Amount:") {
            let value = rest.trim();
            if !value.is_empty() {
                if let Ok(amt) = value.parse::<f64>() {
                    // Validate amount: must be finite and within reasonable bounds
                    if amt.is_finite() && amt >= 0.0 && amt <= 999_999_999.99 {
                        amount = Some(amt);
                    } else {
                        // Invalid amount - too large or not a valid number
                        return None;
                    }
                }
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Date:") {
            let value = rest.trim();
            if !value.is_empty() {
                if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
                    // Validate date is reasonable
                    let max_date = Local::now().date_naive() + chrono::Duration::days(3650);
                    let min_date = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
                    if date >= min_date && date <= max_date {
                        entry.date = date;
                    } else {
                        return None;
                    }
                }
            } else if existing.is_none() {
                entry.date = default_date;
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Notes:") {
            let value = rest.trim_start();
            if !value.is_empty() {
                notes_lines.push(value.to_string());
            }
            in_notes = true;
            continue;
        }
    }

    if in_notes {
        let body = notes_lines.join("\n");
        let notes_text = body.trim_end_matches('\n').to_string();
        // Validate notes length (max 10,000 characters)
        entry.note = if notes_text.len() <= 10_000 {
            notes_text
        } else {
            notes_text.chars().take(10_000).collect()
        };
    }

    if let Some(cat) = category {
        entry.category = cat;
    } else if existing.is_none() {
        return None;
    }

    if let Some(amt) = amount {
        entry.amount = amt;
    } else if existing.is_none() {
        return None;
    }

    Some(entry)
}

fn new_calorie_editor_template(selected_date: NaiveDate) -> String {
    format!(
        "Meal: \nCalories: \nDate: {}\nNotes:\n",
        selected_date
    )
}

fn format_calorie_editor_content(entry: &CalorieEntry) -> String {
    format!(
        "Meal: {}\nCalories: {}\nDate: {}\nNotes:\n{}",
        entry.meal, entry.calories, entry.date, entry.note
    )
}

fn parse_calorie_editor_content(
    input: &str,
    existing: Option<&CalorieEntry>,
    default_date: NaiveDate,
) -> Option<CalorieEntry> {
    let mut entry = existing.cloned().unwrap_or_else(|| CalorieEntry::new(
        default_date,
        String::new(),
        String::new(),
        0,
    ));
    if existing.is_none() {
        entry.date = default_date;
    }
    entry.note.clear();

    let mut meal: Option<String> = None;
    let mut calories: Option<u32> = None;
    let mut in_notes = false;
    let mut notes_lines: Vec<String> = Vec::new();

    for line in input.lines() {
        if in_notes {
            notes_lines.push(line.to_string());
            continue;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Meal:") {
            let value = rest.trim();
            if !value.is_empty() {
                // Validate meal name length (max 200 characters)
                if value.len() <= 200 {
                    meal = Some(value.to_string());
                } else {
                    return None;
                }
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Calories:") {
            let value = rest.trim();
            if !value.is_empty() {
                if let Ok(cal) = value.parse::<u32>() {
                    // Validate calories: must be reasonable (max 50,000 per meal)
                    if cal <= 50_000 {
                        calories = Some(cal);
                    } else {
                        // Invalid calorie count - too high
                        return None;
                    }
                }
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Date:") {
            let value = rest.trim();
            if !value.is_empty() {
                if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
                    // Validate date is reasonable
                    let max_date = Local::now().date_naive() + chrono::Duration::days(3650);
                    let min_date = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
                    if date >= min_date && date <= max_date {
                        entry.date = date;
                    } else {
                        return None;
                    }
                }
            } else if existing.is_none() {
                entry.date = default_date;
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Notes:") {
            let value = rest.trim_start();
            if !value.is_empty() {
                notes_lines.push(value.to_string());
            }
            in_notes = true;
            continue;
        }
    }

    if in_notes {
        let body = notes_lines.join("\n");
        let notes_text = body.trim_end_matches('\n').to_string();
        // Validate notes length (max 10,000 characters)
        entry.note = if notes_text.len() <= 10_000 {
            notes_text
        } else {
            notes_text.chars().take(10_000).collect()
        };
    }

    if let Some(m) = meal {
        entry.meal = m;
    } else if existing.is_none() {
        return None;
    }

    if let Some(c) = calories {
        entry.calories = c;
    } else if existing.is_none() {
        return None;
    }

    Some(entry)
}

fn new_kanban_editor_template() -> String {
    "Title: \nNote:\n".to_string()
}

fn format_kanban_editor_content(card: &KanbanCard) -> String {
    format!("Title: {}\nNote:\n{}", card.title, card.note)
}

fn parse_kanban_editor_content(input: &str, existing: Option<&KanbanCard>) -> Option<KanbanCard> {
    let mut card = existing.cloned().unwrap_or_else(|| {
        KanbanCard::new(String::new(), String::new())
    });
    card.note.clear();

    let mut title: Option<String> = None;
    let mut in_note = false;
    let mut note_lines: Vec<String> = Vec::new();

    for line in input.lines() {
        if in_note {
            note_lines.push(line.to_string());
            continue;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Title:") {
            let value = rest.trim();
            if !value.is_empty() {
                // Validate title length (max 200 characters)
                if value.len() <= 200 {
                    title = Some(value.to_string());
                } else {
                    return None;
                }
            }
            continue;
        }

        if trimmed.strip_prefix("Note:").is_some() {
            in_note = true;
            continue;
        }
    }

    if in_note {
        let body = note_lines.join("\n");
        let notes_text = body.trim_end_matches('\n').to_string();
        // Validate note length (max 10,000 characters)
        card.note = if notes_text.len() <= 10_000 {
            notes_text
        } else {
            notes_text.chars().take(10_000).collect()
        };
    }

    if let Some(t) = title {
        card.title = t;
    } else if existing.is_none() {
        return None;
    }

    Some(card)
}

fn new_card_editor_template() -> String {
    "Front: \nBack: \nCollection: \n".to_string()
}

fn format_card_editor_content(card: &Card) -> String {
    let collection_str = card.collection.as_ref().map(|c| c.as_str()).unwrap_or("");
    format!("Front: {}\nBack: {}\nCollection: {}", card.front, card.back, collection_str)
}

fn parse_card_editor_content_structured(input: &str, existing: Option<&Card>) -> Option<Card> {
    let mut card = existing.cloned().unwrap_or_else(|| Card::new(
        String::new(),
        String::new(),
        CardType::Basic,
    ));

    let mut front: Option<String> = None;
    let mut back: Option<String> = None;
    let mut collection: Option<String> = None;

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Front:") {
            let value = rest.trim();
            if !value.is_empty() {
                // Validate front text length (max 1000 characters)
                if value.len() <= 1000 {
                    front = Some(value.to_string());
                } else {
                    return None;
                }
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Back:") {
            let value = rest.trim();
            if !value.is_empty() {
                // Validate back text length (max 1000 characters)
                if value.len() <= 1000 {
                    back = Some(value.to_string());
                } else {
                    return None;
                }
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Collection:") {
            let value = rest.trim();
            if !value.is_empty() {
                // Validate collection name length (max 100 characters)
                if value.len() <= 100 {
                    collection = Some(value.to_string());
                } else {
                    return None;
                }
            }
            continue;
        }
    }

    if let Some(f) = front {
        card.front = f;
    } else if existing.is_none() {
        return None;
    }

    if let Some(b) = back {
        card.back = b;
    } else if existing.is_none() {
        return None;
    }

    card.collection = collection;

    Some(card)
}

fn finance_help_lines() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from("Finance List - EXPENSE & INCOME TRACKING"),
        Line::from(""),
        Line::from("Features:"),
        Line::from("  - Track daily expenses"),
        Line::from("  - Track income"),
        Line::from("  - Categorize transactions"),
        Line::from("  - Add notes to entries"),
        Line::from("  - View monthly/yearly totals"),
        Line::from("  - Bar graph shows spending per month"),
        Line::from(""),
        Line::from("How to use:"),
        Line::from("  1. Click 'New Entry' to record a transaction"),
        Line::from("  2. Format: <category> <amount>"),
        Line::from("  3. Add notes on following lines"),
        Line::from("  4. Use date navigation to view different months"),
        Line::from("  5. Bar graph updates automatically"),
        Line::from(""),
        Line::from("Examples:"),
        Line::from("  - Groceries 45.50"),
        Line::from("  - Salary 2000.00"),
        Line::from("  - Gas 35.00"),
        Line::from("  - Rent 1500.00"),
        Line::from(""),
        Line::from("Tips:"),
        Line::from("  - Use consistent category names"),
        Line::from("  - Positive amounts for both expenses & income"),
        Line::from("  - Add descriptions in notes"),
        Line::from("  - Current month highlighted in cyan"),
    ]
}

fn calorie_help_lines() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from("Calories HEALTH - MEAL & CALORIE TRACKING"),
        Line::from(""),
        Line::from("Features:"),
        Line::from("  - Log meals throughout the day"),
        Line::from("  - Track calorie intake"),
        Line::from("  - Add meal notes"),
        Line::from("  - Daily total calculation"),
        Line::from(""),
        Line::from("How to use:"),
        Line::from("  1. Click 'New Meal' to log a meal"),
        Line::from("  2. Format: <meal name> <calories>"),
        Line::from("  3. Add notes on following lines"),
        Line::from(""),
        Line::from("Examples:"),
        Line::from("  - Breakfast 350"),
        Line::from("  - Snack 150"),
        Line::from("  - Lunch 650"),
        Line::from("  - Dinner 800"),
        Line::from(""),
        Line::from("Tips Tips:"),
        Line::from("  - Log meals as soon as you eat them"),
        Line::from("  - Use descriptive meal names"),
        Line::from("  - Typical daily goal: 2000-2500 kcal"),
    ]
}

fn draw_planner_view(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // Task list panel
    draw_task_list(frame, app, chunks[0]);

    // Task details / add panel
    draw_task_details(frame, app, chunks[1]);
}

fn draw_task_list(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    app.task_items.clear();

    let editing_tasks =
        app.is_editing() && matches!(app.edit_target, EditTarget::TaskTitle | EditTarget::TaskDetails);

    // Show help message if no tasks and not currently editing a task
    if app.tasks.is_empty() && !editing_tasks {
        let help_block = Block::default().title("Tasks").borders(Borders::ALL);
        let help_para = Paragraph::new(task_help_lines())
            .block(help_block)
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(help_para, chunks[0]);
    } else {
        // Build list items using helper
        let list_data = app.tasks.iter().enumerate().map(|(idx, task)| {
            let checkbox = if task.completed { "[x]" } else { "[ ]" };
            let priority_icon = match task.priority {
                TaskPriority::High => "(High)",
                TaskPriority::Medium => "(Med)",
                TaskPriority::Low => "(Low)",
            };
            let title_first_line = task.title.lines().next().unwrap_or(&task.title);
            let due_str = if let Some(due) = task.due_date {
                format!(" ({})", due)
            } else {
                String::new()
            };
            let reminder_icon = if task.reminder_date.is_some() || task.reminder_text.is_some() {
                " Reminder"
            } else {
                ""
            };
            let text = format!(
                "{} {} {}{}{}",
                checkbox, priority_icon, title_first_line, due_str, reminder_icon
            );
            (idx, text, task.completed)
        });

        let items = build_list_items(
            list_data.collect(),
            app.current_task_idx,
            chunks[0],
            &mut app.task_items,
        );

        let task_list = List::new(items).block(
            Block::default()
                .title("Tasks (Middle-click: toggle [check], Right-click: delete)")
                .borders(Borders::ALL),
        );
        frame.render_widget(task_list, chunks[0]);
    }

    // Add task button
    render_button(frame, "New Task", chunks[1], Color::Green);
    app.add_task_btn = chunks[1];
}

fn draw_task_details(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let editing_tasks =
        app.is_editing() && matches!(app.edit_target, EditTarget::TaskTitle | EditTarget::TaskDetails);

    if editing_tasks {
        let title = if matches!(app.edit_target, EditTarget::TaskTitle) {
            "New Task - First line: title, rest: details (Ctrl+S to save, Esc to cancel)"
        } else {
            "Edit Task - First line: title, rest: details (Ctrl+S to save, Esc to cancel)"
        };

        let show_help = app.editing_input.trim().is_empty();
        if show_help {
            let help_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
                .split(chunks[0]);

            let help_panel = Paragraph::new(task_help_lines())
                .block(Block::default().title("How to use").borders(Borders::ALL))
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(Color::Gray));
            frame.render_widget(help_panel, help_layout[0]);

            app.content_edit_area = help_layout[1];
            render_textarea_editor(frame, app, help_layout[1], title);
        } else {
            app.content_edit_area = chunks[0];
            render_textarea_editor(frame, app, chunks[0], title);
        }
    } else if let Some(task) = app.tasks.get(app.current_task_idx) {
        let reminder_line = match (task.reminder_date, task.reminder_time, task.reminder_text.clone()) {
            (Some(d), Some(t), _) => format!("\nReminder: {} {}", d, t.format("%H:%M")),
            (Some(d), None, _) => format!("\nReminder: {}", d),
            (None, Some(t), None) => format!("\nReminder: {}", t.format("%H:%M")),
            (None, _, Some(t)) => format!("\nReminder: {}", t),
            (None, None, None) => String::new(),
        };
        let rec_label = recurrence_label(task.recurrence);
        let recurrence_line = if rec_label == "None" {
            String::new()
        } else {
            format!("\nRepeat: {}", rec_label)
        };

        let description_text = if !task.description.is_empty() {
            format!("\n\nDescription:\n{}", task.description)
        } else {
            String::new()
        };

        let details = format!(
            "Task: {}\n\nStatus: {}\nPriority: {:?}\nCreated: {}\nDue Date: {}{}{}{}\n\nEdit inline examples:\n- Status: Pending | Completed\n- Priority: High | Medium | Low\n- Reminder: 2025-12-25 09:00 | none | 'text'\n- Repeat: none | daily | weekly | monthly | range 2025-12-01 to 2025-12-31 at 08:00",
            task.title,
            if task.completed {
                "Completed [check]"
            } else {
                "Pending"
            },
            task.priority,
            task.created_at,
            task.due_date
                .map(|d| d.to_string())
                .unwrap_or("Not set".to_string()),
            reminder_line,
            recurrence_line,
            description_text
        );

        let details_panel = Paragraph::new(details)
            .block(Block::default().title("Task Details").borders(Borders::ALL))
            .wrap(Wrap { trim: false });
        frame.render_widget(details_panel, chunks[0]);
    } else {
        let empty_panel = Paragraph::new("No tasks yet. Click 'New Task' to create one.")
            .block(Block::default().title("Task Details").borders(Borders::ALL))
            .wrap(Wrap { trim: false });
        frame.render_widget(empty_panel, chunks[0]);
    }

    // Edit and Delete buttons
    let btn_chunks = split_equal_horizontal(chunks[1], 2);

    app.edit_task_btn = btn_chunks[0];
    render_button(frame, "Edit Task", btn_chunks[0], Color::Yellow);

    app.delete_task_btn = btn_chunks[1];
    render_button(frame, "Delete Task", btn_chunks[1], Color::Red);
}


fn draw_habits_view(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let outer = if app.show_habits_summary {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10),
                Constraint::Min(5),
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5)])
            .split(area)
    };

    let main_area = if app.show_habits_summary {
        draw_habits_summary(frame, app, outer[0]);
        outer[1]
    } else {
        outer[0]
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(main_area);

    // Left: habit list
    app.habit_items.clear();

    let editing_habit =
        app.is_editing() && matches!(app.edit_target, EditTarget::HabitNew | EditTarget::Habit);

    if app.habits.is_empty() && !editing_habit {
        // Show help when no habits
        let list = Paragraph::new(habit_help_lines())
            .block(Block::default().title("Habits").borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(list, chunks[0]);
    } else {
        let mut items = Vec::new();
        let inner_y = chunks[0].y + 1;
        for (idx, h) in app.habits.iter().enumerate() {
            let streak = h.streak;
            let style = if idx == app.current_habit_idx {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default()
            };
            let freq_label = recurrence_label(h.frequency);

            let item_rect = Rect {
                x: chunks[0].x,
                y: inner_y + idx as u16,
                width: chunks[0].width,
                height: 1,
            };
            app.habit_items.push((idx, item_rect));

            let text = format!("{} • {} • streak {}", h.name, freq_label, streak);
            items.push(ListItem::new(text).style(style));
        }
        let list = List::new(items).block(Block::default().title("Habits").borders(Borders::ALL));
        frame.render_widget(list, chunks[0]);
    }

    // Right: date navigation + actions
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(chunks[1]);

    draw_date_navigation(frame, app, right_chunks[0]);

    // Show editing panel if adding/editing habit
    if app.is_editing() && matches!(app.edit_target, EditTarget::HabitNew | EditTarget::Habit) {
        let title = if matches!(app.edit_target, EditTarget::HabitNew) {
            "New Habit - Fill Name/Frequency/Status (Ctrl+S to save, Esc to cancel)"
        } else {
            "Edit Habit - Update Name/Frequency/Status (Ctrl+S to save, Esc to cancel)"
        };

        let show_help = app.editing_input.trim().is_empty();
        if show_help {
            let help_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
                .split(right_chunks[1]);

            let help_panel = Paragraph::new(habit_help_lines())
                .block(Block::default().title("How to use").borders(Borders::ALL))
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(Color::Gray));
            frame.render_widget(help_panel, help_layout[0]);

            app.content_edit_area = help_layout[1];
            render_textarea_editor(frame, app, help_layout[1], title);
        } else {
            app.content_edit_area = right_chunks[1];
            render_textarea_editor(frame, app, right_chunks[1], title);
        }
    } else {
        let status = if let Some(h) = app.habits.get(app.current_habit_idx) {
            let marked = h.marks.contains(&app.current_journal_date);
            let freq_label = recurrence_label(h.frequency);
            let habit_state = habit_status_label(h.status);
            let notes = if h.notes.trim().is_empty() {
                "(none)".to_string()
            } else {
                h.notes.clone()
            };
            format!(
                "Habit: {}\nHabit Status: {}\nTracking Since: {}\nFrequency: {}\nSelected Date: {}\nSelected Date Status: {}\nStreak: {}\n\nNotes:\n{}",
                h.name,
                habit_state,
                h.start_date,
                freq_label,
                app.current_journal_date,
                if marked { "Done [check]" } else { "Pending" },
                h.streak,
                notes
            )
        } else {
            "No habits yet. Use 'New Habit' to create one.".to_string()
        };

        let details = Paragraph::new(status)
            .block(
                Block::default()
                    .title("Habit Details")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(details, right_chunks[1]);
    }

    // Action buttons
    let btns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ])
        .split(right_chunks[2]);

    let add_btn = Paragraph::new("New")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green));
    app.add_habit_btn = btns[0];
    frame.render_widget(add_btn, btns[0]);

    let mark_btn = Paragraph::new("Mark")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan));
    app.mark_done_btn = btns[1];
    frame.render_widget(mark_btn, btns[1]);

    let edit_btn = Paragraph::new("Edit")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow));
    app.edit_habit_btn = btns[2];
    frame.render_widget(edit_btn, btns[2]);

    let del_btn = Paragraph::new("Delete")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Red));
    app.delete_habit_btn = btns[3];
    frame.render_widget(del_btn, btns[3]);

    let summary_style = if app.show_habits_summary {
        Style::default().bg(Color::Magenta).fg(Color::White).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Magenta)
    };
    let summary_btn = Paragraph::new("Summary")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(summary_style);
    app.summary_btn = btns[4];
    frame.render_widget(summary_btn, btns[4]);
}

fn draw_finance_view(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let outer = if app.show_finance_summary {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(10),
                Constraint::Min(5),
                Constraint::Length(3),
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(3),
            ])
            .split(area)
    };

    draw_date_navigation(frame, app, outer[0]);
    
    let (main_area, btn_area) = if app.show_finance_summary {
        draw_finance_summary(frame, app, outer[1]);
        (outer[2], outer[3])
    } else {
        (outer[1], outer[2])
    };

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_area);

    draw_finance_list(frame, app, main[0]);
    draw_finance_details(frame, app, main[1]);

    let btns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(btn_area);

    let add_btn = Paragraph::new("New Entry")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green));
    app.add_fin_btn = btns[0];
    frame.render_widget(add_btn, btns[0]);

    let edit_btn = Paragraph::new("Edit Entry")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow));
    app.edit_fin_btn = btns[1];
    frame.render_widget(edit_btn, btns[1]);

    let del_btn = Paragraph::new("Delete Entry")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Red));
    app.delete_fin_btn = btns[2];
    frame.render_widget(del_btn, btns[2]);
}

fn draw_finance_summary(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let current_date = app.current_journal_date;
    let current_year = current_date.year();
    let current_month = current_date.month();

    // Collect unique categories and always include "All" for full totals
    let categories: Vec<String> = std::iter::once("All".to_string())
        .chain(
            app.finances
                .iter()
                .map(|e| e.category.clone())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter(),
        )
        .collect();

    // Get selected category (safe wrap around)
    let selected_idx = app.selected_finance_category_idx.min(categories.len().saturating_sub(1));
    let selected_category = categories.get(selected_idx).cloned().unwrap_or_default();

    // Filter entries by selected category
    let filtered_entries: Vec<&FinanceEntry> = if selected_category == "All" {
        app.finances.iter().collect()
    } else {
        app.finances
            .iter()
            .filter(|e| e.category == selected_category)
            .collect()
    };

    // Calculate monthly total for selected category
    let monthly_total: f64 = filtered_entries
        .iter()
        .filter(|e| e.date.year() == current_year && e.date.month() == current_month)
        .map(|e| e.amount)
        .sum();

    // Calculate yearly total for selected category
    let yearly_total: f64 = filtered_entries
        .iter()
        .filter(|e| e.date.year() == current_year)
        .map(|e| e.amount)
        .sum();

    // Calculate monthly totals for the current year (for bar graph)
    let mut month_totals = vec![0.0; 12];
    for entry in &filtered_entries {
        if entry.date.year() == current_year {
            let month_idx = (entry.date.month() - 1) as usize;
            month_totals[month_idx] += entry.amount;
        }
    }

    // Find max for scaling
    let max_month = month_totals.iter().cloned().fold(0.0, f64::max);
    let scale_factor = if max_month > 0.0 { 30.0 / max_month } else { 1.0 };

    // Build bar graph
    let mut graph_lines = Vec::new();
    
    // Helper function to format currency
    let format_currency = |amount: f64| -> String {
        if amount >= 1_000_000.0 {
            format!("${:.2}M", amount / 1_000_000.0)
        } else if amount >= 1_000.0 {
            format!("${:.1}K", amount / 1_000.0)
        } else {
            format!("${:.2}", amount)
        }
    };
    
    // Category selector and totals header
    let category_nav = if categories.len() > 1 {
        format!(
            "Category: {} (← {} →) | Monthly: {} | Yearly: {}",
            selected_category, 
            format!("{}/{}", selected_idx + 1, categories.len()),
            format_currency(monthly_total),
            format_currency(yearly_total)
        )
    } else {
        format!(
            "Category: {} | Monthly: {} | Yearly: {}",
            selected_category,
            format_currency(monthly_total),
            format_currency(yearly_total)
        )
    };
    
    graph_lines.push(Line::from(Span::styled(
        category_nav,
        Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
    )));
    graph_lines.push(Line::from(""));
    graph_lines.push(Line::from(Span::styled(
        format!("{}:{} Bar = Monthly Spending", current_month, current_year),
        Style::default().fg(Color::Cyan),
    )));
    graph_lines.push(Line::from(""));

    // Monthly bar graph
    let month_names = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    for (i, &total) in month_totals.iter().enumerate() {
        let bar_length = (total * scale_factor) as usize;
        let bar = "█".repeat(bar_length.min(30));
        let is_current = (i + 1) as u32 == current_month;
        
        let color = if is_current { Color::Cyan } else { Color::Blue };
        let month_style = if is_current {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        // Format number with proper separators for readability
        let formatted_total = if total >= 1_000_000.0 {
            format!("${:.2}M", total / 1_000_000.0)
        } else if total >= 1_000.0 {
            format!("${:.1}K", total / 1_000.0)
        } else {
            format!("${:.0}", total)
        };

        graph_lines.push(Line::from(vec![
            Span::styled(format!("{:>3} ", month_names[i]), month_style),
            Span::styled(bar, Style::default().fg(color)),
            Span::raw(format!(" {}", formatted_total)),
        ]));
    }

    let summary_widget = Paragraph::new(graph_lines)
        .block(
            Block::default()
                .title(format!("Expenditure Summary {} (← → to change category, ↑ ↓ to scroll)", current_year))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.finance_summary_scroll, 0));

    frame.render_widget(summary_widget, area);
}

fn draw_habits_summary(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let current_date = app.current_journal_date;
    let current_year = current_date.year();
    let current_month = current_date.month();

    let mut graph_lines = Vec::new();
    
    // Calculate overall stats
    let total_habits = app.habits.len();
    let active_habits = app.habits.iter().filter(|h| h.status == HabitStatus::Active).count();
    let paused_habits = app.habits.iter().filter(|h| h.status == HabitStatus::Paused).count();
    
    // Calculate completion counts per month
    let mut month_completed = vec![0usize; 12];
    let mut month_possible = vec![0usize; 12];
    
    for habit in app.habits.iter().filter(|h| h.status == HabitStatus::Active) {
        for month in 1..=12 {
            // Count days in this month
            let days_in_month = if let Some(first_day) = NaiveDate::from_ymd_opt(current_year, month, 1) {
                let next_month = if month == 12 {
                    NaiveDate::from_ymd_opt(current_year + 1, 1, 1)
                } else {
                    NaiveDate::from_ymd_opt(current_year, month + 1, 1)
                };
                next_month.map(|d| (d - first_day).num_days()).unwrap_or(30)
            } else {
                30
            };
            
            month_possible[(month - 1) as usize] += days_in_month as usize;
            
            // Count completed days for this habit in this month
            let completed = habit.marks.iter()
                .filter(|d| d.year() == current_year && d.month() == month)
                .count();
            month_completed[(month - 1) as usize] += completed;
        }
    }
    
    // Calculate completion percentages
    let month_percentages: Vec<f64> = month_completed.iter()
        .zip(month_possible.iter())
        .map(|(completed, possible)| {
            if *possible > 0 {
                (*completed as f64 / *possible as f64) * 100.0
            } else {
                0.0
            }
        })
        .collect();
    
    // Current month stats (for display if needed)
    let _monthly_completed = month_completed[(current_month - 1) as usize];
    let _monthly_possible = month_possible[(current_month - 1) as usize];
    let monthly_rate = month_percentages[(current_month - 1) as usize];
    
    // Yearly totals
    let yearly_completed: usize = month_completed.iter().sum();
    let yearly_possible: usize = month_possible.iter().sum();
    let yearly_rate = if yearly_possible > 0 {
        (yearly_completed as f64 / yearly_possible as f64) * 100.0
    } else {
        0.0
    };

    // Header with stats
    graph_lines.push(Line::from(Span::styled(
        format!("Total: {} | Active: {} | Paused: {} | Monthly: {:.1}% | Yearly: {:.1}%", 
            total_habits, active_habits, paused_habits, monthly_rate, yearly_rate),
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )));
    graph_lines.push(Line::from(""));
    graph_lines.push(Line::from(Span::styled(
        format!("{}:{} Bar = Completion Rate", current_month, current_year),
        Style::default().fg(Color::Cyan),
    )));
    graph_lines.push(Line::from(""));

    // Monthly completion bar graph
    let month_names = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    for (i, &percentage) in month_percentages.iter().enumerate() {
        let bar_length = (percentage * 0.3) as usize; // Scale to 30 chars max
        let bar = "█".repeat(bar_length.min(30));
        let is_current = (i + 1) as u32 == current_month;
        
        let color = if percentage >= 80.0 {
            Color::Green
        } else if percentage >= 50.0 {
            Color::Yellow
        } else {
            Color::Red
        };
        
        let month_style = if is_current {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        graph_lines.push(Line::from(vec![
            Span::styled(format!("{:>3} ", month_names[i]), month_style),
            Span::styled(bar, Style::default().fg(color)),
            Span::raw(format!(" {:.1}%", percentage)),
        ]));
    }

    let summary_widget = Paragraph::new(graph_lines)
        .block(
            Block::default()
                .title(format!("Habits Completion Summary {} (↑ ↓ to scroll)", current_year))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.habits_summary_scroll, 0));

    frame.render_widget(summary_widget, area);
}

fn draw_finance_list(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    app.finance_items.clear();

    let entries: Vec<(usize, &FinanceEntry)> = app
        .finances
        .iter()
        .enumerate()
        .filter(|(_, e)| e.date == app.current_journal_date)
        .collect();

    let editing_finance =
        app.is_editing() && matches!(app.edit_target, EditTarget::FinanceNew | EditTarget::Finance);

    if entries.is_empty() && !editing_finance {
        // Show help when no entries
        let list = Paragraph::new(finance_help_lines())
            .block(
                Block::default()
                    .title("Finance Finance (by selected date)")
                    .borders(Borders::ALL),
            )
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(list, area);
    } else {
        let list_data = entries
            .iter()
            .map(|(idx, entry)| {
                let preview_note = entry
                    .note
                    .lines()
                    .next()
                    .map(|l| format!(" - {}", l))
                    .unwrap_or_default();
                let text = format!("{} | {:.2}{}", entry.category, entry.amount, preview_note);
                (*idx, text, false)
            })
            .collect();

        let items = build_list_items(
            list_data,
            app.current_finance_idx,
            area,
            &mut app.finance_items,
        );

        let list = List::new(items).block(
            Block::default()
                .title("Finance Finance (by selected date)")
                .borders(Borders::ALL),
        );
        frame.render_widget(list, area);
    }
}

fn draw_finance_details(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    if app.is_editing()
        && matches!(
            app.edit_target,
            EditTarget::FinanceNew | EditTarget::Finance
        )
    {
        let title = if matches!(app.edit_target, EditTarget::FinanceNew) {
            "New Finance Entry - Fill Category/Amount/Notes (Ctrl + s to save)"
        } else {
            "Edit Finance Entry - Update Category/Amount/Notes (Ctrl + s to save)"
        };

        app.content_edit_area = area;
        render_textarea_editor(frame, app, area, title);
        return;
    }

    if let Some(entry) = app.finances.get(app.current_finance_idx) {
        let body = format!(
            "Date: {}\nCategory: {}\nAmount: {:.2}\n\nNote:\n{}",
            entry.date,
            entry.category,
            entry.amount,
            if entry.note.is_empty() {
                "(none)".to_string()
            } else {
                entry.note.clone()
            }
        );

        let panel = Paragraph::new(body)
            .block(
                Block::default()
                    .title("Entry Details")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(panel, area);
    } else {
        let empty = Paragraph::new("No entries for this date. Use 'New Entry' to create one.")
            .block(
                Block::default()
                    .title("Entry Details")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(empty, area);
    }
}

fn draw_calories_view(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(area);

    draw_date_navigation(frame, app, outer[0]);

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(outer[1]);

    draw_calorie_list(frame, app, main[0]);
    draw_calorie_details(frame, app, main[1]);

    let btns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(outer[2]);

    let add_btn = Paragraph::new("New Meal")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green));
    app.add_cal_btn = btns[0];
    frame.render_widget(add_btn, btns[0]);

    let edit_btn = Paragraph::new("Edit Meal")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow));
    app.edit_cal_btn = btns[1];
    frame.render_widget(edit_btn, btns[1]);

    let del_btn = Paragraph::new("Delete Meal")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Red));
    app.delete_cal_btn = btns[2];
    frame.render_widget(del_btn, btns[2]);
}

fn draw_calorie_list(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    app.calorie_items.clear();

    let entries: Vec<(usize, &CalorieEntry)> = app
        .calories
        .iter()
        .enumerate()
        .filter(|(_, e)| e.date == app.current_journal_date)
        .collect();

    let editing_calories =
        app.is_editing() && matches!(app.edit_target, EditTarget::CaloriesNew | EditTarget::Calories);

    if entries.is_empty() && !editing_calories {
        // Show help when no entries
        let list = Paragraph::new(calorie_help_lines())
            .block(
                Block::default()
                    .title("Calories Calories (by selected date)")
                    .borders(Borders::ALL),
            )
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(list, area);
    } else {
        let list_data = entries
            .iter()
            .map(|(idx, entry)| {
                let preview_note = entry
                    .note
                    .lines()
                    .next()
                    .map(|l| format!(" - {}", l))
                    .unwrap_or_default();
                let text = format!("{} | {} kcal{}", entry.meal, entry.calories, preview_note);
                (*idx, text, false)
            })
            .collect();

        let items = build_list_items(
            list_data,
            app.current_calorie_idx,
            area,
            &mut app.calorie_items,
        );

        let list = List::new(items).block(
            Block::default()
                .title("Calories Calories (by selected date)")
                .borders(Borders::ALL),
        );
        frame.render_widget(list, area);
    }
}

fn draw_calorie_details(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    if app.is_editing()
        && matches!(
            app.edit_target,
            EditTarget::CaloriesNew | EditTarget::Calories
        )
    {
        let title = if matches!(app.edit_target, EditTarget::CaloriesNew) {
            "New Meal - Fill Meal/Calories/Notes (Ctrl+S to save, Esc to cancel)"
        } else {
            "Edit Meal - Update Meal/Calories/Notes (Ctrl+S to save, Esc to cancel)"
        };

        app.content_edit_area = area;
        render_textarea_editor(frame, app, area, title);
        return;
    }

    if let Some(entry) = app.calories.get(app.current_calorie_idx) {
        let body = format!(
            "Date: {}\nMeal: {}\nCalories: {}\n\nNote:\n{}",
            entry.date,
            entry.meal,
            entry.calories,
            if entry.note.is_empty() {
                "(none)".to_string()
            } else {
                entry.note.clone()
            }
        );

        let panel = Paragraph::new(body)
            .block(Block::default().title("Meal Details").borders(Borders::ALL))
            .wrap(Wrap { trim: false });
        frame.render_widget(panel, area);
    } else {
        let empty = Paragraph::new("No meals for this date. Use 'New Meal' to create one.")
            .block(Block::default().title("Meal Details").borders(Borders::ALL))
            .wrap(Wrap { trim: false });
        frame.render_widget(empty, area);
    }
}

fn draw_kanban_view(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let editing = app.is_editing()
        && matches!(app.edit_target, EditTarget::KanbanNew | EditTarget::KanbanEdit);

    let layout: Rc<[Rect]> = if editing {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(area)
    } else {
        Rc::from([area])
    };

    let main_area = layout[0];
    let main_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(main_area);

    draw_kanban_board(frame, app, main_split[0]);
    draw_kanban_controls(frame, app, main_split[1]);

    if editing {
        let side = layout[1];
        let title = if matches!(app.edit_target, EditTarget::KanbanNew) {
            "New Card - Fill Title/Note (Ctrl+S to save, Esc to cancel)"
        } else {
            "Edit Card - Update Title/Note (Ctrl+S to save, Esc to cancel)"
        };

        app.content_edit_area = side;
        render_textarea_editor(frame, app, side, title);
    }
}

fn draw_kanban_board(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(area);

    app.kanban_items.clear();

    for (stage, col_area) in [KanbanStage::Todo, KanbanStage::Doing, KanbanStage::Done]
        .iter()
        .zip(cols.iter())
    {
        let mut items = Vec::new();
        let mut row = 0u16;
        for (idx, card) in app.kanban_cards.iter().enumerate() {
            if &card.stage != stage {
                continue;
            }

            let is_selected = idx == app.current_kanban_card_idx;
            let mut preview = card
                .note
                .lines()
                .next()
                .map(|l| format!(" · {}", l))
                .unwrap_or_default();
            if preview.len() > 32 {
                preview.truncate(32);
                preview.push('…');
            }
            let text = format!("{}{}", card.title, preview);
            let style = if is_selected {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(stage.color())
            };
            items.push(ListItem::new(text).style(style));

            let item_rect = Rect {
                x: col_area.x + 1,
                y: col_area.y + 1 + row,
                width: col_area.width.saturating_sub(2),
                height: 1,
            };
            app.kanban_items.push((idx, item_rect));
            row += 1;
        }

        let title = format!("{} ({})", stage.label(), items.len());
        let list = List::new(items).block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(stage.color())),
        );
        frame.render_widget(list, *col_area);
    }
}

fn draw_kanban_controls(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let controls = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    let new_btn = Paragraph::new("New Flashcard")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green));
    app.add_kanban_btn = controls[0];
    frame.render_widget(new_btn, controls[0]);

    let left_btn = Paragraph::new("Move Left")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow));
    app.move_left_kanban_btn = controls[1];
    frame.render_widget(left_btn, controls[1]);

    let right_btn = Paragraph::new("Move Right")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan));
    app.move_right_kanban_btn = controls[2];
    frame.render_widget(right_btn, controls[2]);

    let del_btn = Paragraph::new("Delete Card")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Red));
    app.delete_kanban_btn = controls[3];
    frame.render_widget(del_btn, controls[3]);
}

// ===== FLASHCARDS (SPACED REPETITION) VIEW =====

fn draw_flashcards_view(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let editing = app.is_editing()
        && matches!(app.edit_target, EditTarget::CardNew | EditTarget::CardEdit | EditTarget::CardImport);

    let layout: Rc<[Rect]> = if editing {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area)
    } else {
        Rc::from([area])
    };

    let main_area = layout[0];
    let vertical_constraints: Vec<Constraint> = if app.card_review_mode {
        vec![Constraint::Length(3), Constraint::Min(10)]
    } else {
        vec![
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ]
    };
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vertical_constraints)
        .split(main_area);

    draw_card_controls(frame, app, main_chunks[0]);

    if app.card_review_mode && !app.cards.is_empty() {
        draw_card_review(frame, app, main_chunks[1]);
    } else {
        draw_card_list(frame, app, main_chunks[1]);
        if main_chunks.len() > 2 {
            draw_bulk_card_actions(frame, app, main_chunks[2]);
        }
    }

    if editing {
        let side = layout[1];
        // Card import help view before editing
        if matches!(app.edit_target, EditTarget::CardImport) && app.show_card_import_help {
            draw_card_import_help(frame, app, side);
        } else if matches!(app.edit_target, EditTarget::CardImport) {
            // Editing the import path: show editor plus the same buttons row
            let edit_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(6), Constraint::Length(3)])
                .split(side);

            let title = "Import Flashcards - Enter file path, then click 'Start Import'";
            app.content_edit_area = edit_layout[0];
            render_textarea_editor(frame, app, edit_layout[0], title);

            // Buttons row reused from help layout
            let btn_row = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(edit_layout[1]);

            let btn_import = Paragraph::new("Start Import")
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Green));
            app.card_import_help_btn = btn_row[0];
            frame.render_widget(btn_import, btn_row[0]);

            let btn_edit = Paragraph::new("Edit Path")
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Cyan));
            app.card_import_edit_btn = btn_row[1];
            frame.render_widget(btn_edit, btn_row[1]);

            app.content_edit_area = side;
        } else {
            let title = match app.edit_target {
                EditTarget::CardNew => "New Flashcard - Fill Front/Back/Collection (Ctrl+S to save, Esc to cancel)",
                EditTarget::CardEdit => "Edit Flashcard - Update Front/Back/Collection (Ctrl+S to save, Esc to cancel)",
                EditTarget::CardImport => "Import Flashcards - Enter file path (Ctrl+S to save, Esc to cancel)",
                _ => "Flashcard Editor",
            };

            app.content_edit_area = side;
            render_textarea_editor(frame, app, side, title);
        }
    }
}

// Helper: Check if card matches current filter
fn matches_filter(app: &App, card: &Card) -> bool {
    let today = Local::now().date_naive();
    match &app.card_filter {
        CardFilter::All => true,
        CardFilter::New => card.last_reviewed.is_none(),
        CardFilter::Due => card.next_review <= today,
        CardFilter::Blackout => card.ease_factor < 1.3, // Complete failure, very low ease
        CardFilter::Hard => card.ease_factor >= 1.3 && card.ease_factor < 1.8, // Difficult
        CardFilter::Medium => card.ease_factor >= 1.8 && card.ease_factor < 2.3, // Average
        CardFilter::Easy => card.ease_factor >= 2.3 && card.ease_factor < 2.8, // Good
        CardFilter::Perfect => card.ease_factor >= 2.8, // Excellent
        CardFilter::Mastered => card.repetitions >= 5 && card.ease_factor >= 2.5,
        CardFilter::Collection(name) => card.collection.as_ref() == Some(name),
    }
}

fn unique_collections(app: &App) -> Vec<String> {
    let mut set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for c in &app.cards {
        if let Some(name) = &c.collection {
            if !name.is_empty() {
                set.insert(name.clone());
            }
        }
    }
    set.into_iter().collect()
}

fn next_card_in_filter(app: &App, current: usize) -> usize {
    if app.cards.is_empty() {
        return 0;
    }
    let total = app.cards.len();
    for step in 1..=total {
        let idx = (current + step) % total;
        if matches_filter(app, &app.cards[idx]) {
            return idx;
        }
    }
    current
}

fn prev_card_in_filter(app: &App, current: usize) -> usize {
    if app.cards.is_empty() {
        return 0;
    }
    let total = app.cards.len();
    for step in 1..=total {
        let idx = (current + total - (step % total)) % total;
        if matches_filter(app, &app.cards[idx]) {
            return idx;
        }
    }
    current
}

fn draw_card_controls(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let controls = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(14), // New
            Constraint::Percentage(14), // Review/List
            Constraint::Percentage(14), // Edit
            Constraint::Percentage(14), // Delete
            Constraint::Percentage(14), // Filter
            Constraint::Percentage(14), // Import
            Constraint::Percentage(14), // Stats
        ])
        .split(area);

    let new_btn = Paragraph::new("New Card")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green));
    app.add_card_btn = controls[0];
    frame.render_widget(new_btn, controls[0]);

    let review_label = if app.card_review_mode { "List View" } else { "Review Mode" };
    let review_btn = Paragraph::new(review_label)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan));
    app.review_card_btn = controls[1];
    app.bulk_delete_btn = Rect::default();
    app.bulk_unassign_btn = Rect::default();
    frame.render_widget(review_btn, controls[1]);

    let edit_btn = Paragraph::new("Edit Flashcard")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow));
    app.edit_card_btn = controls[2];
    frame.render_widget(edit_btn, controls[2]);

    let delete_btn = Paragraph::new("Delete Flashcard")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Red));
    app.delete_card_btn = controls[3];
    frame.render_widget(delete_btn, controls[3]);

    // Filter button
    let filter_label = match &app.card_filter {
        CardFilter::All => "Filter: All".to_string(),
        CardFilter::New => "Filter: New".to_string(),
        CardFilter::Due => "Filter: Due".to_string(),
        CardFilter::Blackout => "Filter: Blackout".to_string(),
        CardFilter::Hard => "Filter: Hard".to_string(),
        CardFilter::Medium => "Filter: Medium".to_string(),
        CardFilter::Easy => "Filter: Easy".to_string(),
        CardFilter::Perfect => "Filter: Perfect".to_string(),
        CardFilter::Mastered => "Filter: Mastered".to_string(),
        CardFilter::Collection(name) => format!("Filter: {}", name),
    };
    let filter_btn = Paragraph::new(filter_label)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::LightMagenta));
    app.filter_collection_btn = controls[4];
    frame.render_widget(filter_btn, controls[4]);

    let import_btn = Paragraph::new("Import Flashcards")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::LightBlue));
    app.import_card_btn = controls[5];
    frame.render_widget(import_btn, controls[5]);

    let visible: Vec<&Card> = app
        .cards
        .iter()
        .filter(|c| matches_filter(app, c))
        .collect();
    let due_cards: usize = visible.iter().filter(|c| c.is_due()).count();
    let stats = match &app.card_filter {
        CardFilter::All => format!("Due: {} / Total: {}", due_cards, app.cards.len()),
        CardFilter::New => format!("New: {}", visible.len()),
        CardFilter::Due => format!("Due: {}", visible.len()),
        CardFilter::Blackout => format!("Blackout: {}", visible.len()),
        CardFilter::Hard => format!("Hard: {}", visible.len()),
        CardFilter::Medium => format!("Medium: {}", visible.len()),
        CardFilter::Easy => format!("Easy: {}", visible.len()),
        CardFilter::Perfect => format!("Perfect: {}", visible.len()),
        CardFilter::Mastered => format!("Mastered: {}", visible.len()),
        CardFilter::Collection(name) => format!("{}: {} cards", name, visible.len()),
    };
    let stats_widget = Paragraph::new(stats)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));
    frame.render_widget(stats_widget, controls[6]);
}

fn draw_bulk_card_actions(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    if app.card_review_mode {
        app.bulk_delete_btn = Rect::default();
        app.bulk_unassign_btn = Rect::default();
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let selected_count = app.selected_card_indices.len();
    let using_filter = matches!(app.card_filter, CardFilter::Collection(_));

    let (delete_hint, delete_style) = if selected_count > 0 {
        (format!(" ({} selected)", selected_count), Style::default().fg(Color::Red))
    } else if using_filter {
        (" (entire collection)".to_string(), Style::default().fg(Color::Red))
    } else {
        (" (select cards first)".to_string(), Style::default().fg(Color::DarkGray))
    };

    let delete_btn = Paragraph::new(format!("Bulk Delete{}", delete_hint))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(delete_style);
    app.bulk_delete_btn = chunks[0];
    frame.render_widget(delete_btn, chunks[0]);

    let (dis_hint, dis_style) = if selected_count > 0 {
        (format!(" ({} selected)", selected_count), Style::default().fg(Color::Yellow))
    } else if using_filter {
        (" (entire collection)".to_string(), Style::default().fg(Color::Yellow))
    } else {
        (" (select cards first)".to_string(), Style::default().fg(Color::DarkGray))
    };

    let dis_btn = Paragraph::new(format!("Bulk Disassociate{}", dis_hint))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(dis_style);
    app.bulk_unassign_btn = chunks[1];
    frame.render_widget(dis_btn, chunks[1]);
}

fn bulk_target_indices(app: &App) -> HashSet<usize> {
    if !app.selected_card_indices.is_empty() {
        return app.selected_card_indices.iter().copied().collect();
    }

    if let CardFilter::Collection(name) = &app.card_filter {
        return app
            .cards
            .iter()
            .enumerate()
            .filter(|(_, card)| card.collection.as_deref() == Some(name.as_str()))
            .map(|(idx, _)| idx)
            .collect();
    }

    HashSet::new()
}

fn bulk_delete_cards(app: &mut App) {
    let targets = bulk_target_indices(app);
    if targets.is_empty() {
        return;
    }

    let mut idx = 0;
    app.cards.retain(|_| {
        let keep = !targets.contains(&idx);
        idx += 1;
        keep
    });
    app.current_card_idx = app.current_card_idx.min(app.cards.len().saturating_sub(1));
    app.clear_card_selection();
    let _ = save_app_data(app);
}

fn bulk_disassociate_cards(app: &mut App) {
    let targets = bulk_target_indices(app);
    if targets.is_empty() {
        return;
    }

    let mut changed = false;
    for (idx, card) in app.cards.iter_mut().enumerate() {
        if targets.contains(&idx) && card.collection.is_some() {
            card.collection = None;
            changed = true;
        }
    }
    if changed {
        let _ = save_app_data(app);
    }
    app.clear_card_selection();
}

fn draw_card_list(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    app.card_items.clear();

    let visible: Vec<(usize, &Card)> = app
        .cards
        .iter()
        .enumerate()
        .filter(|(_, c)| matches_filter(app, c))
        .collect();

    let items: Vec<ListItem> = visible
        .iter()
        .map(|(idx, card)| {
            let status = if card.is_due() {
                "⚠ DUE"
            } else {
                "✓"
            };
            let type_label = match card.card_type {
                CardType::Basic => "Basic",
                CardType::Cloze => "Cloze",
                CardType::MultipleChoice => "MC",
            };
            let front_preview: String = card.front.chars().take(50).collect();
            let text = format!("[{}] {} | {} | Interval: {}d", status, type_label, front_preview, card.interval);
            
            let mut style = if *idx == app.current_card_idx {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else if card.is_due() {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Green)
            };

            if app.selected_card_indices.contains(idx) {
                style = style.bg(Color::DarkGray).add_modifier(Modifier::REVERSED);
            }

            ListItem::new(text).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title("Flashcards (Up/Down to navigate, Enter to review)")
            .borders(Borders::ALL),
    );

    frame.render_widget(list, area);

    // Store clickable areas
    for (idx, _) in visible.iter() {
        let item_rect = Rect {
            x: area.x + 1,
            y: area.y + 1 + (app.card_items.len() as u16),
            width: area.width.saturating_sub(2),
            height: 1,
        };
        app.card_items.push((*idx, item_rect));
    }
}

fn draw_card_review(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    if app.cards.is_empty() || app.current_card_idx >= app.cards.len() {
        let msg = Paragraph::new("No flashcards to review")
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center);
        frame.render_widget(msg, area);
        return;
    }

    if !matches_filter(app, &app.cards[app.current_card_idx]) {
        // pick first visible card in filter or show empty
        if let Some((first_idx, _)) = app
            .cards
            .iter()
            .enumerate()
            .find(|(_, c)| matches_filter(app, c))
        {
            app.current_card_idx = first_idx;
        } else {
            let msg = Paragraph::new("No flashcards match this filter")
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Center);
            frame.render_widget(msg, area);
            return;
        }
    }

    let card = &app.cards[app.current_card_idx];
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Length(3),
            Constraint::Percentage(40),
            Constraint::Length(3),
        ])
        .split(area);

    // Front
    let front_text = format!("FRONT:\n\n{}", card.front);
    let front_widget = Paragraph::new(front_text)
        .block(Block::default()
            .title(format!("Card Type: {:?}", card.card_type))
            .borders(Borders::ALL))
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(front_widget, chunks[0]);

    // Show answer button
    let show_btn_text = if app.show_card_answer { "Answer Shown ✓" } else { "Show Answer (Space)" };
    let show_btn = Paragraph::new(show_btn_text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(if app.show_card_answer {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Yellow)
        });
    app.show_answer_btn = chunks[1];
    frame.render_widget(show_btn, chunks[1]);

    // Back (only if revealed)
    if app.show_card_answer {
        let back_text = format!("BACK:\n\n{}", card.back);
        let back_widget = Paragraph::new(back_text)
            .block(Block::default()
                .title(format!("Next review: {} | Ease: {:.2}", card.next_review, card.ease_factor))
                .borders(Borders::ALL))
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(Color::Green));
        frame.render_widget(back_widget, chunks[2]);

        // Quality rating buttons
        draw_quality_buttons(frame, app, chunks[3]);
    } else {
        let placeholder = Paragraph::new("[Answer hidden - press Space to reveal]")
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(placeholder, chunks[2]);
    }
}

// Render import help with instructions and a button to start editing the path
fn draw_card_import_help(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(7), Constraint::Length(3)])
        .split(area);

    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(
        "Import Flashcards - Help",
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from("Supported formats: .json or .csv"));
    lines.push(Line::from("Paths: absolute or ~ (home)"));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("JSON format (array of objects):", Style::default().fg(Color::Cyan))));
    lines.push(Line::from("  [{"));
    lines.push(Line::from("    \"front\": \"Question\","));
    lines.push(Line::from("    \"back\": \"Answer\","));
    lines.push(Line::from("    \"card_type\": \"basic|cloze|mc\","));
    lines.push(Line::from("    \"collection\": \"optional-name\""));
    lines.push(Line::from("  }]"));
    lines.push(Line::from("card_type is case-insensitive; defaults to basic if missing."));
    lines.push(Line::from("collection is optional; other fields are ignored."));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("CSV format: front,back,type,collection", Style::default().fg(Color::Cyan))));
    lines.push(Line::from("Example lines:"));
    lines.push(Line::from("  Front text,Back text,basic,MyDeck"));
    lines.push(Line::from("  Cloze {{c1:gap}}?,Hidden text,cloze,Spanish"));
    lines.push(Line::from("type accepts basic|cloze|mc (case-insensitive). Extra columns are ignored."));
    lines.push(Line::from(""));
    lines.push(Line::from("Import steps:"));
    lines.push(Line::from("  1) Click 'Edit Path'"));
    lines.push(Line::from("  2) Enter the file path (json/csv)"));
    lines.push(Line::from("  3) Click 'Start Import' to import"));
    lines.push(Line::from("Imported cards are appended; use filters/collections as usual."));

    let help = Paragraph::new(lines)
        .block(
            Block::default()
                .title("Import Flashcards (read mode) - Click button to edit path")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: true })
        .scroll((app.card_import_help_scroll, 0));

    frame.render_widget(help, layout[0]);
       app.card_import_help_text_area = layout[0];

    let btn_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(layout[1]);

    let btn_import = Paragraph::new("Start Import")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green));
    app.card_import_help_btn = btn_row[0];
    frame.render_widget(btn_import, btn_row[0]);

    let btn_edit = Paragraph::new("Edit Path")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan));
    app.card_import_edit_btn = btn_row[1];
    frame.render_widget(btn_edit, btn_row[1]);

    app.content_edit_area = area;
}

fn draw_quality_buttons(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    app.quality_btns.clear();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(16),
            Constraint::Percentage(16),
            Constraint::Percentage(17),
            Constraint::Percentage(17),
            Constraint::Percentage(17),
            Constraint::Percentage(17),
        ])
        .split(area);

    let labels = [
        ("0: Blackout", Color::Red),
        ("1: Wrong", Color::LightRed),
        ("2: Hard", Color::Yellow),
        ("3: Good", Color::LightGreen),
        ("4: Easy", Color::Green),
        ("5: Perfect", Color::Cyan),
    ];

    for (idx, ((label, color), chunk)) in labels.iter().zip(chunks.iter()).enumerate() {
        let btn = Paragraph::new(*label)
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center)
            .style(Style::default().fg(*color));
        app.quality_btns.push((idx as u8, *chunk));
        frame.render_widget(btn, *chunk);
    }
}

fn handle_flashcards_mouse_left(app: &mut App, mouse: MouseEvent) {
    // Handle textarea mouse clicks for editing
    handle_textarea_mouse_click(app, mouse);
    
    // Only process button clicks on Down events to avoid double-triggering
    let is_click = matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left));
    
    let editing_flashcards = app.is_editing()
        && matches!(app.edit_target, EditTarget::CardNew | EditTarget::CardEdit | EditTarget::CardImport);

    // Control buttons - only respond to Down events
    if is_click && inside_rect(mouse, app.add_card_btn) {
        app.card_review_mode = false;
        let template = new_card_editor_template();
        start_editing(app, EditTarget::CardNew, template);
        // Position cursor at end of front text line
        app.textarea.move_cursor(CursorMove::Head);
        app.textarea.move_cursor(CursorMove::End);
        return;
    }
    
    if is_click && inside_rect(mouse, app.review_card_btn) {
        app.card_review_mode = !app.card_review_mode;
        app.show_card_answer = false;
        app.clear_card_selection();
        return;
    }

    if !app.card_review_mode && is_click && inside_rect(mouse, app.bulk_delete_btn) {
        bulk_delete_cards(app);
        return;
    }

    if !app.card_review_mode && is_click && inside_rect(mouse, app.bulk_unassign_btn) {
        bulk_disassociate_cards(app);
        return;
    }

    if is_click && inside_rect(mouse, app.edit_card_btn) && app.current_card_idx < app.cards.len() {
        let card = &app.cards[app.current_card_idx];
        let content = format_card_editor_content(card);
        app.card_review_mode = false;
        start_editing(app, EditTarget::CardEdit, content);
        // Position cursor at end of front text line
        app.textarea.move_cursor(CursorMove::Head);
        app.textarea.move_cursor(CursorMove::End);
        return;
    }

    if is_click && inside_rect(mouse, app.delete_card_btn) && !app.cards.is_empty() {
        delete_and_adjust_index(&mut app.cards, &mut app.current_card_idx);
        app.clear_card_selection();
        let _ = save_app_data(app);
        return;
    }

    if is_click && inside_rect(mouse, app.import_card_btn) {
        app.card_review_mode = false;
        app.show_card_import_help = true;
        // Keep edit target as CardImport to render the help panel, but do not start editing input yet
        app.edit_target = EditTarget::CardImport;
        return;
    }

    // Start Import button (visible in help/edit modes)
    if is_click && inside_rect(mouse, app.card_import_help_btn) {
        // Use pending path if saved via Ctrl+S; otherwise use live editing input
        let path = app
            .pending_card_import_path
            .clone()
            .unwrap_or_else(|| app.editing_input.trim().to_string());

        if path.trim().is_empty() {
            app.show_validation_error = true;
            app.validation_error_message = "Enter a JSON/CSV file path first (use Edit Path).".to_string();
            return;
        }

        match import_cards_from_file(app, path.trim()) {
            Ok(count) => {
                // Exit help/edit mode and show list view
                app.card_review_mode = false;
                app.show_card_import_help = false;
                app.edit_target = EditTarget::None;
                app.pending_card_import_path = None;
                app.editing_input.clear();
                if count > 0 {
                    app.current_card_idx = app.cards.len().saturating_sub(1);
                }
                app.show_success_popup = true;
                app.success_message = format!("Imported {} card(s).", count);
                let _ = save_app_data(app);
            }
            Err(err) => {
                app.show_validation_error = true;
                app.validation_error_message = format!("Import failed: {}", err);
            }
        }
        return;
    }

    // Edit Path button (visible in help/edit modes)
    if is_click && inside_rect(mouse, app.card_import_edit_btn) {
        app.show_card_import_help = false;
        let initial = app
            .pending_card_import_path
            .clone()
            .unwrap_or_else(|| app.editing_input.clone());
        start_editing(app, EditTarget::CardImport, initial);
        return;
    }

    // Click on help text area itself to transition to edit mode (only when help is shown)
    if is_click && app.show_card_import_help && inside_rect(mouse, app.card_import_help_text_area) {
        app.show_card_import_help = false;
        let initial = app
            .pending_card_import_path
            .clone()
            .unwrap_or_else(|| app.editing_input.clone());
        start_editing(app, EditTarget::CardImport, initial);
        return;
    }

    // Filter button cycles: All -> New -> Due -> Difficulty levels -> Mastered -> Collections -> All

    if is_click && inside_rect(mouse, app.filter_collection_btn) {
        app.card_filter = match &app.card_filter {
            CardFilter::All => CardFilter::New,
            CardFilter::New => CardFilter::Due,
            CardFilter::Due => CardFilter::Blackout,
            CardFilter::Blackout => CardFilter::Hard,
            CardFilter::Hard => CardFilter::Medium,
            CardFilter::Medium => CardFilter::Easy,
            CardFilter::Easy => CardFilter::Perfect,
            CardFilter::Perfect => CardFilter::Mastered,
            CardFilter::Mastered => {
                // Move to first collection if any exist
                let mut cols = unique_collections(app);
                cols.sort();
                if let Some(first) = cols.first() {
                    CardFilter::Collection(first.clone())
                } else {
                    CardFilter::All
                }
            }
            CardFilter::Collection(current) => {
                // Cycle through collections, then back to All
                let mut cols = unique_collections(app);
                cols.sort();
                if let Some(pos) = cols.iter().position(|c| c == current) {
                    if pos + 1 < cols.len() {
                        CardFilter::Collection(cols[pos + 1].clone())
                    } else {
                        CardFilter::All
                    }
                } else {
                    // Current collection no longer exists
                    CardFilter::All
                }
            }
        };
        app.clear_card_selection();
        return;
    }

    // Assign collection for current card
    // When editing flashcards, ignore the rest of the buttons to avoid unexpected state changes
    if editing_flashcards {
        return;
    }

    // Show answer button
    if is_click && app.card_review_mode && inside_rect(mouse, app.show_answer_btn) {
        app.show_card_answer = true;
        return;
    }

    // Quality buttons (only when answer is shown)
    if is_click && app.card_review_mode && app.show_card_answer {
        for (quality, rect) in app.quality_btns.clone() {
            if inside_rect(mouse, rect) {
                if let Some(card) = app.cards.get_mut(app.current_card_idx) {
                    card.review(quality);
                    app.show_card_answer = false;
                    // Move to next card within filter
                    app.current_card_idx = next_card_in_filter(app, app.current_card_idx);
                    let _ = save_app_data(app);
                }
                return;
            }
        }
    }

    // Card list items - single click to select, double click to enter review
    if is_click {
        for (idx, rect) in app.card_items.clone() {
            if inside_rect(mouse, rect) {
                // Check if this is a double-click (same card clicked twice in quick succession)
                let is_double_click = app.current_card_idx == idx && mouse.kind == MouseEventKind::Down(MouseButton::Left);
                app.clear_card_selection();
                app.current_card_idx = idx;
                if is_double_click {
                    // Double click -> enter review mode
                    app.card_review_mode = true;
                    app.show_card_answer = false;
                }
                // Single click just selects/highlights
                return;
            }
        }
    }
}

fn import_cards_from_file(app: &mut App, path: &str) -> Result<usize> {
    let path = std::path::Path::new(path);
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    match extension.to_lowercase().as_str() {
        "json" => import_cards_json(app, path),
        "csv" => import_cards_csv(app, path),
        _ => Err(anyhow::anyhow!("Unsupported file format. Use .json or .csv")),
    }
}

fn import_cards_json(app: &mut App, path: &std::path::Path) -> Result<usize> {
    #[derive(serde::Deserialize)]
    struct CardJson {
        front: String,
        back: String,
        #[serde(default)]
        card_type: Option<String>,
        #[serde(default)]
        collection: Option<String>,
        #[serde(default)]
        tags: Option<Vec<String>>,
    }

    let content = std::fs::read_to_string(path)?;
    let entries: Vec<CardJson> = serde_json::from_str(&content)?;
    let mut count = 0;

    for entry in entries {
        let ct = entry
            .card_type
            .as_deref()
            .unwrap_or("basic")
            .trim()
            .to_lowercase();
        let card_type = match ct.as_str() {
            "basic" | "frontback" | "front_back" => CardType::Basic,
            "cloze" => CardType::Cloze,
            "mc" | "multiplechoice" | "multiple choice" | "multiple_choice" =>
                CardType::MultipleChoice,
            _ => CardType::Basic,
        };

        let mut card = Card::new(entry.front, entry.back, card_type);
        if let Some(col) = entry.collection {
            if !col.trim().is_empty() {
                card.collection = Some(col.trim().to_string());
            }
        }
        if let Some(tags) = entry.tags {
            let cleaned: Vec<String> = tags
                .into_iter()
                .filter(|t| !t.trim().is_empty())
                .map(|t| t.trim().to_string())
                .collect();
            if !cleaned.is_empty() {
                card.tags = cleaned;
            }
        }
        app.cards.push(card);
        count += 1;
    }

    Ok(count)
}

fn import_cards_csv(app: &mut App, path: &std::path::Path) -> Result<usize> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(path)?;
    let mut count = 0;

    for result in reader.records() {
        let record = result?;
        if record.len() >= 2 {
            // Normal CSV: multiple fields
            let front = record.get(0).unwrap_or("").to_string();
            let back = record.get(1).unwrap_or("").to_string();
            let card_type = if record.len() > 2 {
                match record.get(2).unwrap_or("basic").to_lowercase().as_str() {
                    "cloze" => CardType::Cloze,
                    "mc" | "multiple choice" => CardType::MultipleChoice,
                    _ => CardType::Basic,
                }
            } else {
                CardType::Basic
            };
            let mut card = Card::new(front, back, card_type);
            if record.len() > 3 {
                let col = record.get(3).unwrap_or("").trim();
                if !col.is_empty() {
                    card.collection = Some(col.to_string());
                }
            }
            app.cards.push(card);
            count += 1;
        } else if record.len() == 1 {
            // Fallback: entire line provided as one quoted field, e.g. "front,back,basic,Deck"
            let raw = record.get(0).unwrap_or("");
            let s = raw.trim().trim_matches('"');
            let parts: Vec<&str> = s.split(',').collect();
            if parts.len() >= 2 {
                let front = parts.get(0).map(|p| p.trim()).unwrap_or("").to_string();
                let back = parts.get(1).map(|p| p.trim()).unwrap_or("").to_string();
                let card_type = match parts.get(2).map(|p| p.trim().to_lowercase()).as_deref() {
                    Some("cloze") => CardType::Cloze,
                    Some("mc") | Some("multiple choice") => CardType::MultipleChoice,
                    _ => CardType::Basic,
                };
                let mut card = Card::new(front, back, card_type);
                if let Some(col) = parts.get(3).map(|p| p.trim()) {
                    if !col.is_empty() {
                        card.collection = Some(col.to_string());
                    }
                }
                app.cards.push(card);
                count += 1;
            }
        }
    }

    Ok(count)
}

fn draw_journal_view(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    // Date navigation
    draw_date_navigation(frame, app, chunks[0]);

    // Journal entry
    draw_journal_entry(frame, app, chunks[1]);
}

fn draw_date_navigation(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    // Check if we're in Finance view to show Summary button
    let is_finance = matches!(app.view_mode, ViewMode::Finance);
    
    let chunks = if is_finance {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(16),
                Constraint::Percentage(16),
                Constraint::Percentage(32),
                Constraint::Percentage(18),
                Constraint::Percentage(18),
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(40),
                Constraint::Percentage(20),
            ])
            .split(area)
    };

    let prev_btn = Paragraph::new("Previous Day")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan));
    app.prev_day_btn = chunks[0];
    frame.render_widget(prev_btn, chunks[0]);

    let next_btn = Paragraph::new("Next Day")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan));
    app.next_day_btn = chunks[1];
    frame.render_widget(next_btn, chunks[1]);

    let date_display = Paragraph::new(format!("Date {}", app.current_journal_date))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    app.date_btn = chunks[2];
    frame.render_widget(date_display, chunks[2]);

    let today_btn = Paragraph::new("Jump to Today")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green));
    app.today_btn = chunks[3];
    frame.render_widget(today_btn, chunks[3]);
    
    // Add Summary button for Finance view
    if is_finance {
        let summary_text = if app.show_finance_summary {
            "Hide Summary"
        } else {
            "Show Summary"
        };
        let summary_btn = Paragraph::new(summary_text)
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Magenta));
        app.summary_btn = chunks[4];
        frame.render_widget(summary_btn, chunks[4]);
    }
}

fn draw_journal_entry(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let entry = app
        .journal_entries
        .iter()
        .find(|e| e.date == app.current_journal_date)
        .cloned();

    if app.is_editing() && matches!(app.edit_target, EditTarget::JournalEntry) {
        let title = format!("Journal Entry - {} (Ctrl+S to save, Esc to cancel)", app.current_journal_date);
        app.content_edit_area = area;
        render_textarea_editor(frame, app, area, &title);
    } else if entry.is_none() {
        // Show help when no entry exists
        let help_text = vec![
            Line::from(""),
            Line::from("Notebook JOURNAL - DAILY REFLECTIONS"),
            Line::from(""),
            Line::from("Features:"),
            Line::from("  - Write one entry per day"),
            Line::from("  - Track your mood (optional)"),
            Line::from("  - Navigate between dates"),
            Line::from("  - Search entries by date"),
            Line::from(""),
            Line::from("How to use:"),
            Line::from("  1. Click the journal area to start writing"),
            Line::from("  2. Type freely - your entry auto-saves"),
            Line::from("  3. Use Prev/Next to navigate days"),
            Line::from("  4. Click 'Today' to jump to current date"),
            Line::from(""),
            Line::from("Optional: Start with mood line:"),
            Line::from("  Mood: happy/sad/reflective/motivated/etc"),
            Line::from(""),
            Line::from("Tips Tips:"),
            Line::from("  - Write regularly for best results"),
            Line::from("  - No pressure to write long entries"),
            Line::from("  - Past entries are always there to review"),
        ];

        let journal_panel = Paragraph::new(help_text)
            .block(
                Block::default()
                    .title(format!("Notebook Journal - {}", app.current_journal_date))
                    .borders(Borders::ALL),
            )
            .style(Style::default().fg(Color::Gray));
        app.content_edit_area = area;
        frame.render_widget(journal_panel, area);
    } else {
        let content = entry
            .as_ref()
            .map(|e| {
                let mood_line = e
                    .mood
                    .as_ref()
                    .map(|m| format!("Mood: {}\n\n", m))
                    .unwrap_or_default();
                format!("{}{}", mood_line, e.content)
            })
            .unwrap_or_else(|| "(Click to write in your journal)".to_string());

        let journal_panel = Paragraph::new(content)
            .block(
                Block::default()
                    .title(format!("Notebook Journal - {}", app.current_journal_date))
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false });
        app.content_edit_area = area;
        frame.render_widget(journal_panel, area);
    }
}
