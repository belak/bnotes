# Weekly Task Migration Design

**Date:** 2026-01-26

## Overview

When creating the current week's weekly note, automatically migrate uncompleted tasks from the most recent previous weekly note. Mark migrated tasks as `- [>]` in the source note to create an audit trail.

This feature requires refactoring task parsing to support custom checkbox statuses beyond the standard GFM `[ ]` and `[x]`.

## Prerequisites: Task Status Refactoring

### Current State

Tasks currently use a `completed: bool` field and rely on pulldown_cmark's `ENABLE_TASKLISTS` option, which only recognizes:
- `- [ ]` → uncompleted task
- `- [x]` or `- [X]` → completed task

### Required Changes

#### 1. Task Status Enum

Replace `completed: bool` with a `status` field:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Uncompleted,   // - [ ]
    Completed,     // - [x] or [X]
    Migrated,      // - [>]
}

pub struct Task {
    pub note_path: PathBuf,
    pub note_title: String,
    pub index: usize,
    pub status: TaskStatus,  // Changed from completed: bool
    pub text: String,
    pub priority: Option<String>,
    pub urgency: Option<String>,
    pub tags: Vec<String>,
}
```

#### 2. Custom Checkbox Parsing

Parse list items manually instead of relying on `TaskListMarker`:

```rust
// Current approach (limited to GFM):
Event::TaskListMarker(checked) => { ... }

// New approach (flexible):
// When we see a list item, check if the text starts with [X]
// Extract the status character and parse accordingly
```

**Implementation strategy:**
- Parse list items as before
- When collecting text from a list item, check if it matches pattern: `^\[(.)\] (.*)$`
- Extract the status character and remaining text
- Map status character to `TaskStatus` enum:
  - ` ` (space) → `Uncompleted`
  - `x` or `X` → `Completed`
  - `>` → `Migrated`
  - Unknown characters → skip (not a task)

#### 3. Update Existing Code

Update all code that references `task.completed`:

**In `src/lib.rs`:**
- `list_tasks()` - filter by status instead of completed bool
- Task filtering logic

**In `src/cli/commands.rs`:**
- `task_list()` - handle `--status` flag with new values
- Display logic for task status indicators

**CLI status flag values:**
- `open` → filters to `TaskStatus::Uncompleted`
- `completed` → filters to `TaskStatus::Completed`
- `migrated` → filters to `TaskStatus::Migrated`
- `all` → no filtering

#### 4. Testing

Add tests for custom status parsing:
- `test_parse_uncompleted_task()`
- `test_parse_completed_task()`
- `test_parse_migrated_task()`
- `test_parse_non_task_list_item()` - list items without checkbox should be skipped
- `test_ignore_checkbox_in_code_block()`

### Future Extensibility

This refactoring enables additional statuses in the future:
- `- [-]` → Cancelled
- `- [/]` → In Progress
- `- [?]` → Blocked/Waiting

## Weekly Task Migration Feature

### When Migration Triggers

Migration only happens when:
- Creating a weekly note for the current week (e.g., `bnotes weekly` during week 4)
- The target weekly note doesn't exist yet (e.g., 2026-W04.md)

Creating past or future weekly notes skips migration entirely.

### What Gets Migrated

The system finds the most recent weekly note before the current week and collects every task with `TaskStatus::Uncompleted`:
- If 2026-W03.md doesn't exist, it looks for 2026-W02.md, then 2026-W01.md, etc.
- Only tasks with `status: TaskStatus::Uncompleted` are migrated
- Tasks with `Completed` or `Migrated` status are skipped

### Migration Flow

1. User runs `bnotes weekly` (or equivalent) during week 4
2. System detects 2026-W04.md doesn't exist
3. System finds most recent weekly note (e.g., 2026-W03.md)
4. System parses tasks and filters to `TaskStatus::Uncompleted`
5. Displays: `Found 12 uncompleted tasks from 2026-W03. Migrate to 2026-W04? [Y/n]`
6. If yes/Enter: creates new note with migrated tasks, updates source note
7. If no: creates new note from template without migration

### Edge Cases

- **No previous weekly notes:** If no previous weekly notes exist at all, migration is skipped silently (first weekly note ever)
- **Gaps in weekly notes:** Finds the most recent existing weekly note, even if weeks were skipped
- **Print-path mode:** When `--print-path` flag is used (scripting mode), migration is skipped entirely to avoid interactive prompts
- **No uncompleted tasks:** If previous note has no uncompleted tasks, migration is skipped silently

## Task Structure

### In the New Weekly Note

Migrated tasks appear at the top under a "## Migrated Tasks" heading, before any template content:

```markdown
# 2026-W04

## Migrated Tasks

- [ ] Fix authentication bug @backend
- [ ] !!! Deploy hotfix to production
- [ ] (A) Review PR #234
- [ ] Write Q1 planning doc @planning

## Goals
[rest of template...]
```

### Task Reconstruction

Tasks are reconstructed from the parsed `Task` struct:

```rust
let line = format!("- [ ] {}{}{}{}",
    task.urgency.as_ref().map(|u| format!("{} ", u)).unwrap_or_default(),
    task.priority.as_ref().map(|p| format!("({}) ", p)).unwrap_or_default(),
    task.text,
    if task.tags.is_empty() {
        String::new()
    } else {
        format!(" {}", task.tags.iter().map(|t| format!("@{}", t)).collect::<Vec<_>>().join(" "))
    }
);
```

### Task Formatting Preserved

- Priority markers: `(A)`, `(B)`, etc. are kept
- Urgency markers: `!!!`, `!!`, `!` are kept
- Task-level tags: `@backend`, `@planning` are kept
- Original task text is preserved

### Task Order

Tasks maintain their original order from the source note (top-to-bottom). No automatic reordering by priority/urgency happens during migration.

## Updating Source Notes

When tasks are migrated, the system updates the source note by changing `- [ ]` to `- [>]` for each migrated task.

**Before migration:**
```markdown
# 2026-W03

## Goals
- [x] Complete feature X
- [ ] Fix authentication bug @backend
- [ ] !!! Deploy hotfix to production
- [ ] (A) Review PR #234
```

**After migration:**
```markdown
# 2026-W03

## Goals
- [x] Complete feature X
- [>] Fix authentication bug @backend
- [>] !!! Deploy hotfix to production
- [>] (A) Review PR #234
```

### Implementation Detail

Simple string replacement on raw file content: `content.replace("- [ ]", "- [>]")`

**Edge case handling:**
- May replace `- [ ]` in code blocks (rare in weekly notes, acceptable risk)
- Replacement only happens when user confirms migration
- Both files updated atomically

### Atomic Operation

Both file updates (create new weekly note + update source note) happen together. If either fails, neither change is committed.

## Implementation Architecture

### Location in Codebase

The migration logic lives in the library (`src/lib.rs`) as a new method on `BNotes`:

```rust
pub fn create_weekly_with_migration(&self, period: Weekly) -> Result<PathBuf>
```

This keeps business logic testable and separate from CLI concerns.

### Integration with Existing Flow

The CLI command handler (`src/cli/commands.rs`) calls this method when:
1. Creating a weekly note for the current week
2. The target note doesn't exist yet

For past/future weeks or when the note already exists, it falls back to the existing `create_periodic_note()` method (no migration).

### Key Functions Needed

1. `find_previous_weekly_note(&self, period: Weekly) -> Option<PathBuf>` - Scans for most recent weekly note
2. `extract_uncompleted_tasks(&self, note_path: &Path) -> Result<Vec<Task>>` - Parses note and filters to `TaskStatus::Uncompleted`
3. `mark_tasks_migrated(&self, note_path: &Path) -> Result<()>` - Replaces `- [ ]` with `- [>]`
4. `build_migrated_section(tasks: &[Task]) -> String` - Generates "## Migrated Tasks\n\n..." content
5. `reconstruct_task_line(task: &Task) -> String` - Rebuilds markdown line from Task struct

### Error Handling

- If source note can't be read: show warning, skip migration, create note normally
- If source note can't be updated: rollback new note creation, show error
- If user declines migration: create note from template only
- If no uncompleted tasks found: skip migration silently

## Testing Strategy

All tests use `MemoryStorage` following the existing pattern.

### Unit Tests for Task Status Refactoring

**In `src/note.rs`:**
- `test_parse_uncompleted_task()` - Parse `- [ ]`
- `test_parse_completed_task()` - Parse `- [x]` and `- [X]`
- `test_parse_migrated_task()` - Parse `- [>]`
- `test_parse_task_with_priority_urgency()` - Status parsing with markers
- `test_non_task_list_item_ignored()` - `- Regular list item` is not a task
- `test_reconstruct_task_line()` - Rebuild markdown from Task struct

### Unit Tests for Migration Logic

**In `src/lib.rs`:**
- `test_find_previous_weekly_note()` - Finds immediate previous week
- `test_find_previous_weekly_note_with_gap()` - Finds 2026-W02 when W03 missing
- `test_extract_uncompleted_tasks()` - Only gets `Uncompleted`, skips `Completed` and `Migrated`
- `test_mark_tasks_migrated()` - Converts `- [ ]` to `- [>]` in raw content
- `test_migration_preserves_formatting()` - Priority, urgency, tags preserved
- `test_build_migrated_section()` - Generates correct markdown section

### Integration Tests

**In `tests/`:**
- `test_weekly_migration_full_flow()` - End-to-end migration scenario
- `test_weekly_migration_declined()` - User says no, gets template only
- `test_weekly_no_migration_for_past_weeks()` - Creating old weeks skips migration
- `test_weekly_no_previous_note()` - First weekly note creates without error
- `test_weekly_no_uncompleted_tasks()` - Previous week has no tasks to migrate
- `test_weekly_skips_migrated_tasks()` - Tasks already marked `[>]` not re-migrated

## CLI Details

### Prompting

Uses standard input for the Y/n prompt with Y as default (pressing Enter accepts migration).

### Task Status Filtering

Extend existing `--status` flag to support new values:
- `bnotes tasks --status open` → shows only `Uncompleted` tasks (default)
- `bnotes tasks --status completed` → shows only `Completed` tasks
- `bnotes tasks --status migrated` → shows only `Migrated` tasks
- `bnotes tasks --status all` → shows all tasks regardless of status

### Git Sync Consideration

Migration updates two files (new + old weekly note). The existing `bnotes sync` command will commit both changes together, maintaining the migration as an atomic operation in version control.

## Implementation Order

1. **Task Status Refactoring** (prerequisite)
   - Add `TaskStatus` enum
   - Update `Task` struct to use `status` field
   - Implement custom checkbox parsing
   - Update all existing code that uses `task.completed`
   - Add unit tests for status parsing
   - Verify existing tests still pass

2. **Migration Feature**
   - Implement `find_previous_weekly_note()`
   - Implement `extract_uncompleted_tasks()`
   - Implement `mark_tasks_migrated()`
   - Implement `build_migrated_section()`
   - Implement `create_weekly_with_migration()`
   - Wire up CLI command handler
   - Add integration tests
