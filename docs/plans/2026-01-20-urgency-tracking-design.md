# Urgency Tracking Design

**Date:** 2026-01-20
**Status:** Approved

## Overview

Add urgency tracking to tasks using an Eisenhower Matrix approach where priority represents importance (A/B/C) and urgency represents time-sensitivity (!!!/!!/!).

## Requirements

- Urgency is separate from priority
- Symbol-based notation: `!!!` (critical/now), `!!` (soon), `!` (eventually)
- Urgency appears before priority: `- [ ] !!! (A) Task text`
- Space required after urgency symbol
- Default sort order: urgency, then priority, then ID
- Flexible comma-separated sort fields

## Data Model

### Task Struct Changes

Add `urgency` field to `Task` in `src/note.rs`:

```rust
pub struct Task {
    pub note_path: PathBuf,
    pub note_title: String,
    pub index: usize,
    pub completed: bool,
    pub text: String,
    pub priority: Option<String>,     // (A), (B), (C), etc.
    pub urgency: Option<String>,      // !!!, !!, !
}
```

### Parsing Logic

```rust
fn parse_urgency_and_priority(text: &str) -> (Option<String>, Option<String>, String) {
    let trimmed = text.trim();

    // Parse urgency first (requires space after)
    let (urgency, rest) = if let Some(rest) = trimmed.strip_prefix("!!! ") {
        (Some("!!!".to_string()), rest)
    } else if let Some(rest) = trimmed.strip_prefix("!! ") {
        (Some("!!".to_string()), rest)
    } else if let Some(rest) = trimmed.strip_prefix("! ") {
        (Some("!".to_string()), rest)
    } else {
        (None, trimmed)
    };

    // Then parse priority
    let (priority, task_text) = parse_priority_from_text(rest);

    (urgency, priority, task_text.to_string())
}
```

## Sort Order

### New Structure

Replace `TaskSortOrder` enum with flexible field-based sorting:

```rust
pub struct TaskSortOrder {
    fields: Vec<SortField>,
}

enum SortField {
    Urgency,
    Priority,
    Id,
}

impl TaskSortOrder {
    pub fn parse(s: &str) -> Result<Self> {
        // Parse comma-separated fields: "urgency,priority,id"
    }
}

impl Default for TaskSortOrder {
    fn default() -> Self {
        // urgency,priority,id
    }
}
```

### Sorting Logic

```rust
tasks.sort_by(|a, b| {
    for field in &sort_order.fields {
        let cmp = match field {
            SortField::Urgency => compare_urgency(&a.urgency, &b.urgency),
            SortField::Priority => compare_priority(&a.priority, &b.priority),
            SortField::Id => a.id().cmp(&b.id()),
        };
        if cmp != std::cmp::Ordering::Equal {
            return cmp;
        }
    }
    std::cmp::Ordering::Equal
});
```

Comparison order:
- Urgency: `!!!` < `!!` < `!` < `None`
- Priority: `A` < `B` < `C` < `None`
- ID: Lexicographic

## CLI Changes

### Command Arguments

Update `src/main.rs`:

```rust
Tasks {
    #[arg(long, default_value = "urgency,priority,id")]
    sort_order: String,
}

Task(TaskCommands::List {
    #[arg(long, default_value = "urgency,priority,id")]
    sort_order: String,
})
```

### Display Format

Update `task_list()` in `src/cli/commands.rs`:

```
note#1  !!! (A) [ ] Task text (from note)
note#2  !! [ ] Another task (from note)
note#3  (B) [x] Completed (from note)
```

Display order: ID, urgency (if present), priority (if present), checkbox, text, note reference.

## Testing

### Unit Tests

- Parse urgency only: `!!! Fix bug`
- Parse priority only: `(A) Important task`
- Parse both: `!! (B) Moderate task`
- Parse neither: `Plain task`
- Missing space: `!!!(A)` should not parse urgency

### Integration Tests

- Sort by `urgency,priority,id`
- Sort by `priority,urgency,id` (different order)
- Sort by `id` only
- Invalid sort field error handling

### Edge Cases

- `!` at end of text: `Do this!` - Not urgency
- Multiple `!` in text: `!! Fix the !! bug` - Only prefix counts
- Tasks without urgency/priority still work
- Old format tasks are unaffected

## Documentation Updates

### CLAUDE.md

Update Task Format section with urgency examples and sort order documentation.

### README.md

Add urgency symbols to Notes section showing task format examples.

## Migration

### Breaking Changes

- Old sort options `priority-id` and `id` become `priority,id` and `id`
- Default changes from `priority-id` to `urgency,priority,id`
- Users must update CLI arguments/config if they use non-default sorting

### Backward Compatibility

- Tasks without urgency continue to work
- Parsing is additive (only adds urgency field)
- No database migration needed (plain text markdown)

## Implementation Order

1. Update `Task` struct and parsing in `src/note.rs`
2. Add `TaskSortOrder` parsing and comparison logic in `src/lib.rs`
3. Update CLI arguments in `src/main.rs`
4. Update display in `src/cli/commands.rs`
5. Add unit tests
6. Add integration tests
7. Update documentation
