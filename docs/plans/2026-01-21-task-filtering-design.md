# Task Filtering Design

**Date**: 2026-01-21
**Status**: Approved

## Overview

Add ability to filter tasks by note name using wildcard patterns, and expose existing status filter in the CLI. Consolidate `bnotes tasks` as a wrapper around `bnotes task list`.

## CLI Interface

**Updated commands:**

```bash
bnotes tasks [OPTIONS]           # Shortcut for "task list --status open"
bnotes task list [OPTIONS]       # Full command with all options

Options:
  --note <PATTERN>        Filter tasks by note name (supports * wildcard)
  --status <STATUS>       Filter by status: open, done, all [default: open for tasks, none for task list]
  --sort-order <ORDER>    Sort order [default: urgency,priority,id]
```

**Example usage:**

```bash
# Tasks from weekly notes
bnotes tasks --note '2026-W*'
bnotes task list --note '2026-W*'

# All done tasks
bnotes tasks --status done
bnotes task list --status done

# Combine filters: open tasks from this week's note
bnotes tasks --note '2026-W04' --status open

# All tasks (done and open) from daily notes
bnotes task list --note '2026-01-*' --status all
```

## Pattern Matching

**Wildcard matching:**
- Simple wildcard: `*` matches any characters
- Case-insensitive matching ('weekly' matches 'Weekly', 'WEEKLY', etc.)
- Matches against note title (not filename)
- No pattern or `--note` flag = no filtering by note

**Implementation:**
- Use `wildmatch` crate (v2.0)
- Lightweight, battle-tested (used by git)
- Built-in case-insensitive support
- Simple API: `WildMatch::new(pattern).case_insensitive(true).matches(text)`

**Examples:**
- `"2026-w*"` matches `"2026-W04"` ✓
- `"*daily*"` matches `"My Daily Notes"` ✓
- `"2026-W04"` matches `"2026-w04"` ✓ (case-insensitive)
- `"W*4"` matches `"2026-W04"` ✗ (doesn't start with W)

## Implementation

### 1. Add Dependency

**File**: `Cargo.toml`

Add:
```toml
wildmatch = "2.0"
```

### 2. CLI Argument Parsing

**File**: `src/main.rs`

**Changes to `TaskCommands::List` (line ~168):**
```rust
TaskCommands::List {
    /// Filter by note name (supports * wildcard)
    #[arg(long)]
    note: Option<String>,

    /// Filter by tags
    #[arg(long = "tag")]
    tags: Vec<String>,

    /// Filter by status (open, done, all)
    #[arg(long)]
    status: Option<String>,

    // Sort order: comma-separated fields (urgency, priority, id)
    #[arg(long, default_value = "urgency,priority,id")]
    sort_order: String,
}
```

**Changes to `Commands::Tasks` handler (line ~214):**

Replace current implementation with delegation to `TaskCommands::List`:
```rust
Commands::Tasks { sort_order } => {
    // Delegate to task list with open status default
    let task_cmd = TaskCommands::List {
        note: None,
        tags: vec![],
        status: Some("open".to_string()),
        sort_order,
    };
    // Handle as TaskCommands::List
    let sort_order = bnotes::TaskSortOrder::parse(&task_cmd.sort_order)
        .context("Invalid sort order")?;
    cli::commands::task_list(
        &notes_dir,
        &task_cmd.tags,
        task_cmd.status,
        task_cmd.note.as_deref(),
        sort_order,
        cli_args.color
    )?;
}
```

**Changes to `TaskCommands::List` handler (line ~243):**
```rust
TaskCommands::List { note, tags, status, sort_order } => {
    let sort_order = bnotes::TaskSortOrder::parse(&sort_order)
        .context("Invalid sort order")?;
    cli::commands::task_list(
        &notes_dir,
        &tags,
        status,
        note.as_deref(),
        sort_order,
        cli_args.color
    )?;
}
```

### 3. CLI Command Handler

**File**: `src/cli/commands.rs`

**Update `task_list` signature (line ~708):**
```rust
pub fn task_list(
    notes_dir: &Path,
    tags: &[String],
    status: Option<String>,
    note_pattern: Option<&str>,
    sort_order: bnotes::TaskSortOrder,
    color: ColorChoice,
) -> Result<()>
```

**Add wildmatch import:**
```rust
use wildmatch::WildMatch;
```

**Add note filtering after line ~719:**
```rust
let mut tasks = bnotes.list_tasks(tags, status.as_deref(), sort_order)?;

// Filter by note pattern if provided
if let Some(pattern) = note_pattern {
    let matcher = WildMatch::new(pattern).case_insensitive(true);
    tasks.retain(|task| matcher.matches(&task.note_title));
}
```

### 4. Library Layer

**File**: `src/lib.rs`

No changes needed - `list_tasks()` already returns all necessary data. Filtering happens in CLI layer since pattern matching is a presentation concern.

## Design Rationale

**Why filter in CLI layer:**
- Pattern matching (wildcards) is a UI concern
- Library returns structured data; CLI decides what to display
- Keeps library simple and testable
- Wildcard syntax might differ between UIs (CLI, TUI, web, etc.)

**Why consolidate `tasks` command:**
- Eliminates code duplication
- Single source of truth for task listing logic
- Makes all options available to both commands
- `tasks` becomes a convenient shortcut with sensible defaults

**Why use `wildmatch` crate:**
- Lightweight (~10KB)
- Battle-tested (used by git)
- Built-in case-insensitive matching
- Handles edge cases we might miss in custom implementation
- Simple, focused API for string matching

**Why case-insensitive matching:**
- More user-friendly (no need to remember exact casing)
- Matches common CLI tool behavior
- Note titles often have varied casing

**Why not support tag filtering (for now):**
- Future plan to add task-level tags (separate from note tags)
- Avoid confusion between note tags and task tags
- Can add later when task tags are implemented

## Testing

### Manual Verification

```bash
# Build and test
cargo build
cargo test

# Test note filtering
bnotes tasks --note '2026-W*'
bnotes tasks --note '*daily*'
bnotes tasks --note '2026-w04'  # case-insensitive

# Test status filtering
bnotes tasks --status done
bnotes tasks --status all
bnotes task list --status open

# Test combinations
bnotes tasks --note '2026-W*' --status done
bnotes task list --note '*daily*' --status all

# Test edge cases
bnotes tasks --note 'nonexistent'  # Should show 0 tasks
bnotes tasks  # Should work as before (open tasks)
```

### Edge Cases

- No tasks match pattern → Show "No tasks found"
- No `--note` flag → Show all tasks (existing behavior)
- Pattern with no `*` → Exact match (case-insensitive)
- Empty pattern → Should be handled gracefully
- Both commands have same filtering capabilities

## Future Enhancements

- Add `--tag` support when task-level tags are implemented
- Consider adding `--not-note` for exclusion patterns
- Multiple note patterns with OR logic
- Date-based filtering for periodic notes
