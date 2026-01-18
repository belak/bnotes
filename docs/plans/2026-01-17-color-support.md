# Color Support Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add minimal, purposeful colorization to bnotes commands using termcolor crate with proper terminal detection.

**Architecture:** Create a colors module with helper functions returning ColorSpec. Thread ColorChoice through CLI args to all commands. Replace raw ANSI codes and println! with StandardStream + writeln!.

**Tech Stack:** termcolor 1.4, IsTerminal trait from std::io

---

## Task 1: Create Colors Module

**Files:**
- Create: `src/cli/colors.rs`
- Modify: `src/cli/mod.rs:10`

**Step 1: Create colors module**

Create `src/cli/colors.rs`:

```rust
//! Color support for CLI output
//!
//! Provides helper functions for colorized terminal output using termcolor.
//! Respects terminal detection and NO_COLOR environment variable.

use std::io::IsTerminal;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream};

/// Create a StandardStream with appropriate color support
///
/// Follows the termcolor recommended pattern:
/// - Respects user preference from --color flag
/// - When Auto, checks IsTerminal to disable for pipes/redirects
/// - ColorChoice::Auto also respects NO_COLOR environment variable
pub fn create_stdout(preference: ColorChoice) -> StandardStream {
    let choice = if preference == ColorChoice::Auto && !std::io::stdout().is_terminal() {
        ColorChoice::Never
    } else {
        preference
    };
    StandardStream::stdout(choice)
}

/// Error color: red + bold
pub fn error() -> ColorSpec {
    let mut spec = ColorSpec::new();
    spec.set_fg(Some(Color::Red)).set_bold(true);
    spec
}

/// Warning color: yellow + bold
pub fn warning() -> ColorSpec {
    let mut spec = ColorSpec::new();
    spec.set_fg(Some(Color::Yellow)).set_bold(true);
    spec
}

/// Success color: green
pub fn success() -> ColorSpec {
    let mut spec = ColorSpec::new();
    spec.set_fg(Some(Color::Green));
    spec
}

/// Highlight color: cyan (for structure)
pub fn highlight() -> ColorSpec {
    let mut spec = ColorSpec::new();
    spec.set_fg(Some(Color::Cyan));
    spec
}

/// Dim color: gray (for secondary info)
pub fn dim() -> ColorSpec {
    let mut spec = ColorSpec::new();
    spec.set_dimmed(true);
    spec
}
```

**Step 2: Add colors module to mod.rs**

In `src/cli/mod.rs`, add after line 10:

```rust
pub mod colors;
```

**Step 3: Verify compilation**

Run: `cargo build`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/cli/colors.rs src/cli/mod.rs
git commit -m "feat(cli): add colors module with ColorSpec helpers"
```

---

## Task 2: Add --color CLI Flag

**Files:**
- Modify: `src/main.rs:15-18` (add color field to Cli struct)
- Modify: `src/main.rs:196+` (thread color parameter through all command calls)

**Step 1: Add color field to Cli struct**

In `src/main.rs`, add after line 18 (after `notes_dir` field):

```rust
    /// When to use colors
    #[arg(long, default_value = "auto", value_name = "WHEN", global = true)]
    color: termcolor::ColorChoice,
```

Add import at top of file (after line 5):

```rust
use termcolor::ColorChoice;
```

**Step 2: Update search command call**

In `src/main.rs:197-199`, change:

```rust
        Commands::Search { query } => {
            cli::commands::search(&notes_dir, &query)?;
        }
```

To:

```rust
        Commands::Search { query } => {
            cli::commands::search(&notes_dir, &query, cli_args.color)?;
        }
```

**Step 3: Update all other command calls**

Update each command call in main() to pass `cli_args.color`:
- Line 200-202: `new(&notes_dir, title, template, cli_args.color)`
- Line 203-205: `edit(&notes_dir, &title, cli_args.color)`
- Line 206-208: `task_list(&notes_dir, &[], Some("open".to_string()), cli_args.color)`
- Line 209-211: `doctor(&notes_dir, cli_args.color)`
- Line 212-214: `sync(&notes_dir, message, cli_args.color)`
- Line 215-217: `pull(&notes_dir, cli_args.color)`
- Line 220: `note_list(&notes_dir, &tags, cli_args.color)`
- Line 223: `note_show(&notes_dir, &title, cli_args.color)`
- Line 226: `note_links(&notes_dir, &title, cli_args.color)`
- Line 229: `note_graph(&notes_dir, cli_args.color)`
- Line 234: `task_list(&notes_dir, &tags, status, cli_args.color)`
- Line 237: `task_show(&notes_dir, &task_id, cli_args.color)`
- Line 263: `periodic::<Daily>(&notes_dir, action, template, cli_args.color)`
- Line 288: `periodic::<Weekly>(&notes_dir, action, template, cli_args.color)`
- Line 313: `periodic::<Quarterly>(&notes_dir, action, template, cli_args.color)`

**Step 4: Verify compilation fails (functions don't accept color yet)**

Run: `cargo build`
Expected: FAIL with errors about wrong number of arguments

**Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat(cli): add --color flag to CLI arguments

Thread ColorChoice through all command calls. Functions will be updated
in following commits to accept the parameter."
```

---

## Task 3: Update Search Command

**Files:**
- Modify: `src/cli/commands.rs:38-121` (search function)

**Step 1: Add color parameter and update function signature**

In `src/cli/commands.rs:38`, change:

```rust
pub fn search(notes_dir: &Path, query: &str) -> Result<()> {
```

To:

```rust
pub fn search(notes_dir: &Path, query: &str, color: ColorChoice) -> Result<()> {
```

Add to imports at top (after line 13):

```rust
use std::io::Write;
```

**Step 2: Create StandardStream at function start**

After line 41 (`let bnotes = BNotes::with_defaults(storage);`), add:

```rust
    let mut stdout = super::colors::create_stdout(color);
```

**Step 3: Replace ANSI codes with termcolor**

Replace lines 52-112 with:

```rust
    for note in &matches {
        // Show path (like grep/ripgrep does)
        stdout.set_color(&super::colors::highlight())?;
        write!(stdout, "{}", note.path.display())?;
        stdout.reset()?;
        writeln!(stdout)?;

        // Show title if it's different from filename
        let filename = note
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if note.title != filename {
            writeln!(stdout, "  Title: {}", note.title)?;
        }

        // Show tags if they matched or if there are any
        if !note.tags.is_empty() {
            let tags_matched = note
                .tags
                .iter()
                .any(|tag| tag.to_lowercase().contains(&query_lower));
            if tags_matched {
                write!(stdout, "  Tags: ")?;
                stdout.set_color(&super::colors::highlight())?;
                write!(stdout, "{}", note.tags.join(", "))?;
                stdout.reset()?;
                writeln!(stdout, " ← matched")?;
            } else {
                writeln!(stdout, "  Tags: {}", note.tags.join(", "))?;
            }
        }

        // Show content snippet if content was matched
        let content_lower = note.content.to_lowercase();
        let title_matched = note.title.to_lowercase().contains(&query_lower);

        if let Some(pos) = content_lower.find(&query_lower) {
            // Show context around the match
            let start = pos.saturating_sub(60);
            let end = (pos + query.len() + 60).min(note.content.len());

            let snippet = &note.content[start..end];
            let snippet = snippet
                .trim()
                .lines()
                .take(3)
                .collect::<Vec<_>>()
                .join(" ");

            let prefix = if start > 0 { "..." } else { "" };
            let suffix = if end < note.content.len() {
                "..."
            } else {
                ""
            };

            stdout.set_color(&super::colors::dim())?;
            writeln!(stdout, "  {}{}{}", prefix, snippet, suffix)?;
            stdout.reset()?;
        } else if title_matched {
            stdout.set_color(&super::colors::dim())?;
            writeln!(stdout, "  (matched in title)")?;
            stdout.reset()?;
        }

        writeln!(stdout)?;
    }
```

**Step 4: Update summary line**

Replace lines 114-119 with:

```rust
    writeln!(
        stdout,
        "Found {} {}",
        matches.len(),
        pluralize(matches.len(), "match", "matches")
    )?;
```

**Step 5: Test compilation**

Run: `cargo build`
Expected: May still have errors from other functions not accepting color parameter yet

**Step 6: Commit**

```bash
git add src/cli/commands.rs
git commit -m "feat(search): replace ANSI codes with termcolor

- Use cyan for note paths and matched tags
- Use dim for content snippets
- Proper terminal detection via StandardStream"
```

---

## Task 4: Update Remaining Command Signatures

**Files:**
- Modify: `src/cli/commands.rs` (all function signatures)

**Step 1: Add color parameter to all command functions**

Update each function signature to add `color: ColorChoice` parameter:

- Line 123: `new(notes_dir: &Path, title: Option<String>, template_name: Option<String>, color: ColorChoice)`
- Line 163: `edit(notes_dir: &Path, title: &str, color: ColorChoice)`
- Line 200: `doctor(notes_dir: &Path, color: ColorChoice)`
- Line 287: `sync(notes_dir: &Path, message: Option<String>, color: ColorChoice)`
- Line 339: `pull(notes_dir: &Path, color: ColorChoice)`
- Line 374: `note_list(notes_dir: &Path, tags: &[String], color: ColorChoice)`
- Line 411: `note_show(notes_dir: &Path, title: &str, color: ColorChoice)`
- Line 435: `note_links(notes_dir: &Path, title: &str, color: ColorChoice)`
- Line 488: `note_graph(notes_dir: &Path, color: ColorChoice)`
- Line 563: `task_list(notes_dir: &Path, tags: &[String], status: Option<String>, color: ColorChoice)`
- Line 600: `task_show(notes_dir: &Path, task_id: &str, color: ColorChoice)`
- Line 635: `periodic<P: bnotes::PeriodType>(notes_dir: &Path, action: PeriodicAction, template_override: Option<String>, color: ColorChoice)`

**Step 2: Test compilation**

Run: `cargo build`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add src/cli/commands.rs
git commit -m "feat(commands): add color parameter to all functions

All commands now accept ColorChoice but don't use it yet.
This allows compilation to succeed."
```

---

## Task 5: Update Doctor Command

**Files:**
- Modify: `src/cli/commands.rs:200-281` (doctor function)

**Step 1: Create StandardStream**

After line 205 (`let notes = bnotes.list_notes(&[])?;`), add:

```rust
    let mut stdout = super::colors::create_stdout(color);
```

**Step 2: Replace println! with writeln! for plain text**

Replace all `println!` calls with `writeln!(stdout, ...)`:
- Line 208: `writeln!(stdout, "No notes found to check.")?;`
- Line 213: `writeln!(stdout, "Running health checks on {} notes...\n", notes.len())?;`

**Step 3: Add colors to ERROR labels**

Replace lines 219-228 with:

```rust
    if !report.broken_links.is_empty() {
        stdout.set_color(&super::colors::error())?;
        write!(stdout, "ERROR:")?;
        stdout.reset()?;
        writeln!(stdout, " Broken wiki links:")?;
        for (note_title, broken) in &report.broken_links {
            writeln!(stdout, "  {} has broken links:", note_title)?;
            for link in broken {
                writeln!(stdout, "    - [[{}]]", link)?;
            }
        }
        writeln!(stdout)?;
    }
```

**Step 4: Add colors to WARNING labels**

Replace lines 231-237 with:

```rust
    if !report.notes_without_tags.is_empty() {
        stdout.set_color(&super::colors::warning())?;
        write!(stdout, "WARNING:")?;
        stdout.reset()?;
        writeln!(stdout, " Notes without tags:")?;
        for title in &report.notes_without_tags {
            writeln!(stdout, "  - {}", title)?;
        }
        writeln!(stdout)?;
    }
```

Replace lines 240-246 with:

```rust
    if !report.notes_without_frontmatter.is_empty() {
        stdout.set_color(&super::colors::warning())?;
        write!(stdout, "WARNING:")?;
        stdout.reset()?;
        writeln!(stdout, " Notes missing frontmatter:")?;
        for title in &report.notes_without_frontmatter {
            writeln!(stdout, "  - {}", title)?;
        }
        writeln!(stdout)?;
    }
```

**Step 5: Add colors to duplicate ERROR**

Replace lines 249-258 with:

```rust
    if !report.duplicate_titles.is_empty() {
        stdout.set_color(&super::colors::error())?;
        write!(stdout, "ERROR:")?;
        stdout.reset()?;
        writeln!(stdout, " Multiple notes with the same title:")?;
        for (title, paths) in &report.duplicate_titles {
            writeln!(stdout, "  Title: {}", title)?;
            for path in paths {
                writeln!(stdout, "    - {}", path)?;
            }
        }
        writeln!(stdout)?;
    }
```

**Step 6: Add colors to orphaned WARNING**

Replace lines 261-267 with:

```rust
    if !report.orphaned_notes.is_empty() {
        stdout.set_color(&super::colors::warning())?;
        write!(stdout, "WARNING:")?;
        stdout.reset()?;
        writeln!(stdout, " Orphaned notes (no links, no tags):")?;
        for title in &report.orphaned_notes {
            writeln!(stdout, "  - {}", title)?;
        }
        writeln!(stdout)?;
    }
```

**Step 7: Add color to success message**

Replace lines 270-278 with:

```rust
    if !report.has_issues() {
        stdout.set_color(&super::colors::success())?;
        writeln!(stdout, "All checks passed! Your notes are healthy.")?;
        stdout.reset()?;
    } else {
        writeln!(
            stdout,
            "Found {} {} that may need attention.",
            report.issue_count(),
            pluralize(report.issue_count(), "issue", "issues")
        )?;
    }
```

**Step 8: Test manually**

Run: `cargo build && cargo run -- --notes-dir=$HOME/tmp/bnotes-test doctor`
Expected: Compiles and runs, shows colored output

**Step 9: Commit**

```bash
git add src/cli/commands.rs
git commit -m "feat(doctor): add color support

- Red ERROR labels for broken links and duplicates
- Yellow WARNING labels for missing tags/frontmatter/orphans
- Green success message when all checks pass"
```

---

## Task 6: Update Task Commands

**Files:**
- Modify: `src/cli/commands.rs:563-621` (task_list and task_show)

**Step 1: Update task_list function**

After line 571 (`let tasks = bnotes.list_tasks(tags, status.as_deref())?;`), add:

```rust
    let mut stdout = super::colors::create_stdout(color);
```

Replace lines 574-576 with:

```rust
    if tasks.is_empty() {
        writeln!(stdout, "No tasks found.")?;
        return Ok(());
    }
```

Replace lines 579-589 with:

```rust
    for task in &tasks {
        let checkbox = if task.completed { "[x]" } else { "[ ]" };

        // Task ID in cyan
        stdout.set_color(&super::colors::highlight())?;
        write!(stdout, "{}", task.id())?;
        stdout.reset()?;

        // Checkbox in green if completed
        if task.completed {
            stdout.set_color(&super::colors::success())?;
            write!(stdout, "  {}", checkbox)?;
            stdout.reset()?;
        } else {
            write!(stdout, "  {}", checkbox)?;
        }

        // Task text in default
        write!(stdout, "  {}", task.text)?;

        // Source note in dim
        stdout.set_color(&super::colors::dim())?;
        writeln!(stdout, " (from {})", task.note_title)?;
        stdout.reset()?;
    }

    writeln!(
        stdout,
        "\nTotal: {} {}",
        tasks.len(),
        pluralize(tasks.len(), "task", "tasks")
    )?;
```

**Step 2: Update task_show function**

After line 605 (`let (task, note) = bnotes.get_task(task_id)?;`), add:

```rust
    let mut stdout = super::colors::create_stdout(color);
```

Replace lines 608-619 with:

```rust
    write!(stdout, "Task: ")?;
    stdout.set_color(&super::colors::highlight())?;
    writeln!(stdout, "{}", task.id())?;
    stdout.reset()?;

    writeln!(stdout, "Note: {}", task.note_title)?;
    writeln!(
        stdout,
        "Status: {}",
        if task.completed { "Done" } else { "Open" }
    )?;
    writeln!(stdout, "\n{}", task.text)?;

    writeln!(stdout, "\n--- Context from note ---")?;
    writeln!(stdout, "{}", note.content)?;
```

**Step 3: Test compilation**

Run: `cargo build`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/cli/commands.rs
git commit -m "feat(tasks): add color support

- Cyan task IDs for easy reference
- Green checkboxes for completed tasks
- Dim source note attribution"
```

---

## Task 7: Update Git Commands

**Files:**
- Modify: `src/cli/commands.rs:287-368` (sync and pull)

**Step 1: Update sync function**

After line 289 (`let repo = GitRepo::new(notes_dir.to_path_buf())?;`), add:

```rust
    let mut stdout = super::colors::create_stdout(color);
```

Replace lines 324-327 with:

```rust
        stdout.set_color(&super::colors::success())?;
        writeln!(
            stdout,
            "Synced successfully: committed {} changes, pulled, and pushed",
            num_changes
        )?;
        stdout.reset()?;
```

Replace lines 329-333 with:

```rust
        stdout.set_color(&super::colors::success())?;
        writeln!(stdout, "Synced successfully: pulled and pushed")?;
        stdout.reset()?;
```

**Step 2: Update pull function**

After line 341 (`let repo = GitRepo::new(notes_dir.to_path_buf())?;`), add:

```rust
    let mut stdout = super::colors::create_stdout(color);
```

Replace line 365 with:

```rust
    stdout.set_color(&super::colors::success())?;
    writeln!(stdout, "Pulled successfully")?;
    stdout.reset()?;
```

**Step 3: Test compilation**

Run: `cargo build`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/cli/commands.rs
git commit -m "feat(git): add color support

- Green success messages for sync and pull completion"
```

---

## Task 8: Update Note Commands

**Files:**
- Modify: `src/cli/commands.rs:374-557` (note_list, note_show, note_links, note_graph)

**Step 1: Update note_list**

After line 379 (`let notes = bnotes.list_notes(tags)?;`), add:

```rust
    let mut stdout = super::colors::create_stdout(color);
```

Replace lines 381-388 with:

```rust
    if notes.is_empty() {
        if tags.is_empty() {
            writeln!(stdout, "No notes found.")?;
        } else {
            writeln!(stdout, "No notes found with tags: {}", tags.join(", "))?;
        }
        return Ok(());
    }
```

Replace lines 396-404 with:

```rust
    for note in notes {
        let tag_str = if note.tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", note.tags.join(", "))
        };

        writeln!(stdout, "{}{}", note.title, tag_str)?;
    }

    write!(stdout, "\nTotal: ")?;
    stdout.set_color(&super::colors::highlight())?;
    write!(stdout, "{}", count)?;
    stdout.reset()?;
    writeln!(stdout, " {}", pluralize(count, "note", "notes"))?;
```

**Step 2: Update note_show**

After line 416 (`let matches = bnotes.find_note_by_title(title)?;`), add:

```rust
    let mut stdout = super::colors::create_stdout(color);
```

Replace lines 420-422 with:

```rust
            writeln!(stdout, "{}", note.content)?;
```

Replace lines 425-429 with:

```rust
            writeln!(stdout, "Multiple notes found with title '{}':", title)?;
            for note in matches {
                writeln!(stdout, "  - {}", note.path.display())?;
            }
```

**Step 3: Update note_links**

After line 453 (`let (outbound, inbound) = bnotes.get_note_links(&note.title)?;`), add:

```rust
    let mut stdout = super::colors::create_stdout(color);
```

Replace all println! with writeln!(stdout, ...) in lines 456-484. Add cyan color to arrows:

```rust
    writeln!(stdout, "Links for: {}\n", note.title)?;

    if !outbound.is_empty() {
        write!(stdout, "Outbound links (")?;
        stdout.set_color(&super::colors::highlight())?;
        write!(stdout, "{}", outbound.len())?;
        stdout.reset()?;
        writeln!(stdout, "):")?;

        let mut sorted_links: Vec<_> = outbound.iter().collect();
        sorted_links.sort();
        for link in sorted_links {
            stdout.set_color(&super::colors::highlight())?;
            write!(stdout, "  ->")?;
            stdout.reset()?;
            writeln!(stdout, " {}", link)?;
        }
        writeln!(stdout)?;
    }

    if !inbound.is_empty() {
        write!(stdout, "Inbound links (")?;
        stdout.set_color(&super::colors::highlight())?;
        write!(stdout, "{}", inbound.len())?;
        stdout.reset()?;
        writeln!(stdout, "):")?;

        let mut sorted_links: Vec<_> = inbound.iter().collect();
        sorted_links.sort();
        for link in sorted_links {
            stdout.set_color(&super::colors::highlight())?;
            write!(stdout, "  <-")?;
            stdout.reset()?;
            writeln!(stdout, " {}", link)?;
        }
        writeln!(stdout)?;
    }

    if outbound.is_empty() && inbound.is_empty() {
        writeln!(stdout, "No links found for this note.")?;
    }
```

**Step 4: Update note_graph**

After line 493 (`let notes = bnotes.list_notes(&[])?;`), add:

```rust
    let mut stdout = super::colors::create_stdout(color);
```

Replace lines 495-497 with:

```rust
    if notes.is_empty() {
        writeln!(stdout, "No notes found.")?;
        return Ok(());
    }
```

Replace line 502 with:

```rust
    writeln!(stdout, "Link Graph ({} notes):\n", notes.len())?;
```

Replace line 520-522 with:

```rust
    if connected_notes.is_empty() {
        writeln!(stdout, "No links found between notes.")?;
        return Ok(());
    }
```

Replace lines 530-548 with:

```rust
    for note in sorted_notes {
        let outbound = graph.outbound.get(note);
        let inbound = graph.inbound.get(note);

        let out_count = outbound.map(|s| s.len()).unwrap_or(0);
        let in_count = inbound.map(|s| s.len()).unwrap_or(0);

        write!(stdout, "- {} (")?;
        stdout.set_color(&super::colors::highlight())?;
        write!(stdout, "->{}", out_count)?;
        stdout.reset()?;
        write!(stdout, " ")?;
        stdout.set_color(&super::colors::highlight())?;
        write!(stdout, "<-{}", in_count)?;
        stdout.reset()?;
        writeln!(stdout, ")")?;

        if let Some(links) = outbound
            && !links.is_empty()
        {
            let mut sorted_links: Vec<_> = links.iter().collect();
            sorted_links.sort();
            for link in sorted_links {
                stdout.set_color(&super::colors::highlight())?;
                write!(stdout, "  ->")?;
                stdout.reset()?;
                writeln!(stdout, " {}", link)?;
            }
        }
    }
```

Replace lines 550-554 with:

```rust
    write!(stdout, "\nTotal: ")?;
    stdout.set_color(&super::colors::highlight())?;
    write!(stdout, "{}", connected_notes.len())?;
    stdout.reset()?;
    writeln!(
        stdout,
        " connected {}",
        pluralize(connected_notes.len(), "note", "notes")
    )?;
```

**Step 5: Test compilation**

Run: `cargo build`
Expected: Compiles successfully

**Step 6: Commit**

```bash
git add src/cli/commands.rs
git commit -m "feat(notes): add color support

- Cyan for arrows (-> and <-) in links/graph
- Cyan for counts in summaries
- Note titles remain default for readability"
```

---

## Task 9: Update Remaining Functions

**Files:**
- Modify: `src/cli/commands.rs:123-162,635-738` (new, edit, periodic functions)

**Step 1: Update new function**

After line 128 (`validate_notes_dir(notes_dir)?;`), add:

```rust
    let mut stdout = super::colors::create_stdout(color);
```

Replace line 134 with:

```rust
        write!(stdout, "Enter note title: ")?;
        stdout.flush()?;
```

Replace line 155-158 with:

```rust
    write!(stdout, "Created note: ")?;
    stdout.set_color(&super::colors::success())?;
    writeln!(stdout, "{}", notes_dir.join(note_path).display())?;
    stdout.reset()?;
```

**Step 2: Update edit function**

After line 164 (`validate_notes_dir(notes_dir)?;`), add:

```rust
    let mut stdout = super::colors::create_stdout(color);
```

Replace lines 174-178 with:

```rust
        _ => {
            writeln!(stdout, "Multiple notes found with title '{}':", title)?;
            for note in &matches {
                writeln!(stdout, "  - {}", notes_dir.join(&note.path).display())?;
            }
            anyhow::bail!("Please be more specific.");
        }
```

**Step 3: Update periodic_open function**

After line 676 (`let full_path = notes_dir.join(&note_path);`), add:

```rust
    let mut stdout = super::colors::create_stdout(color);
```

Replace lines 680-691 with:

```rust
    if !full_path.exists() {
        write!(
            stdout,
            "{} {} doesn't exist. Create it? [Y/n] ",
            match P::template_name() {
                "daily" => "Day",
                "weekly" => "Week",
                "quarterly" => "Quarter",
                _ => "Period",
            },
            period.identifier()
        )?;
        stdout.flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input == "n" || input == "no" {
            return Ok(());
        }

        bnotes.open_periodic(period, template_override.as_deref())?;
    }
```

**Step 4: Update periodic_list function**

After line 710 (`let periods = bnotes.list_periodic::<P>()?;`), add:

```rust
    let mut stdout = super::colors::create_stdout(color);
```

Replace lines 712-719 with:

```rust
    if periods.is_empty() {
        writeln!(stdout, "No {} notes found.", P::template_name())?;
        return Ok(());
    }

    for period in periods {
        writeln!(stdout, "{}", period.display_string())?;
    }
```

**Step 5: Update periodic signature to accept and pass color**

At line 670, change:

```rust
fn periodic_open<P: bnotes::PeriodType>(
    notes_dir: &Path,
    bnotes: &bnotes::BNotes,
    period: P,
    template_override: Option<String>,
    color: ColorChoice,
) -> Result<()> {
```

At line 709, change:

```rust
fn periodic_list<P: bnotes::PeriodType>(bnotes: &bnotes::BNotes, color: ColorChoice) -> Result<()> {
```

Update calls in periodic function:
- Line 652: `periodic_open::<P>(notes_dir, &bnotes, period, template_override, color)?;`
- Line 655: `periodic_list::<P>(&bnotes, color)?;`

**Step 6: Test compilation**

Run: `cargo build`
Expected: Compiles successfully

**Step 7: Commit**

```bash
git add src/cli/commands.rs
git commit -m "feat(misc): add color support to remaining commands

- Green success message for note creation
- Consistent StandardStream usage across all commands"
```

---

## Task 10: Manual Testing

**Files:**
- Test with: Sample notes in test directory

**Step 1: Set up test environment**

```bash
export BNOTES_TEST_DIR="$HOME/tmp/bnotes-test"
mkdir -p "$BNOTES_TEST_DIR"
cd "$BNOTES_TEST_DIR"

# Create some test notes
echo "---
tags: [test, sample]
---
# Test Note
This is a test note with [[Another Note]] link." > "test-note.md"

echo "---
tags: [sample]
---
# Another Note
- [ ] Open task
- [x] Completed task" > "another-note.md"

echo "# Broken Note
Has [[NonExistent]] link." > "broken.md"
```

**Step 2: Test with colors (default)**

Run each command:
```bash
cargo run -- --notes-dir="$BNOTES_TEST_DIR" search test
cargo run -- --notes-dir="$BNOTES_TEST_DIR" doctor
cargo run -- --notes-dir="$BNOTES_TEST_DIR" task list
cargo run -- --notes-dir="$BNOTES_TEST_DIR" note list
cargo run -- --notes-dir="$BNOTES_TEST_DIR" note links "Test Note"
cargo run -- --notes-dir="$BNOTES_TEST_DIR" note graph
```

Expected: Colors visible in terminal output

**Step 3: Test --color=never**

Run: `cargo run -- --notes-dir="$BNOTES_TEST_DIR" --color=never doctor`
Expected: No colors in output

**Step 4: Test piped output**

Run: `cargo run -- --notes-dir="$BNOTES_TEST_DIR" search test | cat`
Expected: No colors in output (terminal detection working)

**Step 5: Test NO_COLOR**

Run: `NO_COLOR=1 cargo run -- --notes-dir="$BNOTES_TEST_DIR" doctor`
Expected: No colors in output

**Step 6: Test --color=always with pipe**

Run: `cargo run -- --notes-dir="$BNOTES_TEST_DIR" --color=always doctor | cat`
Expected: ANSI codes visible when viewing with `cat -A` or similar

**Step 7: Visual verification checklist**

Verify each colorization:
- [ ] Search: cyan paths, dim snippets
- [ ] Doctor: red ERROR, yellow WARNING, green success
- [ ] Tasks: cyan IDs, green [x], dim sources
- [ ] Note links: cyan arrows and counts
- [ ] Note graph: cyan arrows and counts
- [ ] Git sync: green success (if git repo available)

**Step 8: Clean up test directory**

```bash
rm -rf "$BNOTES_TEST_DIR"
```

**Step 9: Document test results**

If all tests pass, mark complete. If issues found, document and fix before proceeding.

---

## Task 11: Final Build and Test

**Files:**
- All modified files

**Step 1: Clean build**

Run: `cargo clean && cargo build --release`
Expected: Compiles successfully

**Step 2: Run all tests**

Run: `cargo test`
Expected: All tests pass (40 tests)

**Step 3: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings or errors

**Step 4: Test with actual notes directory (if available)**

Run: `cargo run -- --notes-dir="$HOME/notes" search test` (or similar with real notes)
Expected: Works correctly with real data

**Step 5: Final commit**

```bash
git add -A
git commit -m "feat: complete color support implementation

All commands now support --color flag with proper terminal detection.
Respects NO_COLOR environment variable and handles piped output."
```

---

## Completion Checklist

- [ ] Colors module created with helper functions
- [ ] --color CLI flag added and threaded through all commands
- [ ] Search command colorized (cyan paths, dim snippets)
- [ ] Doctor command colorized (red errors, yellow warnings, green success)
- [ ] Task commands colorized (cyan IDs, green checkboxes, dim sources)
- [ ] Git commands colorized (green success messages)
- [ ] Note commands colorized (cyan arrows and counts)
- [ ] All remaining functions updated
- [ ] Manual testing completed for all scenarios
- [ ] All unit tests passing
- [ ] Clippy clean
- [ ] All changes committed

## Next Steps

After implementation is complete:
1. Test thoroughly with real notes directory
2. Consider adding integration tests for color output
3. Update README.md with --color flag documentation
4. Consider adding color examples to documentation
