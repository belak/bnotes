# Task List Alignment Design

**Date**: 2026-01-20
**Status**: Approved

## Overview

Improve task list display alignment to make scanning easier at a glance by aligning all columns with fixed widths.

## Current Format

```
2026-W04#1  !!! (P0) [ ] PIV signing CLI PR (from 2026-W04)
2026-W04#2  !!! (P1) [ ] Gov Migration Cohorts (from 2026-W04)
2026-W04#3  !! (P1) [ ] IL5 Distributor work (from 2026-W04)
```

Problems:
- Task IDs vary in length (2026-W04#1 vs 2026-W04#10)
- Urgency symbols vary (!!!, !!, !, none)
- Priority values vary (P0, P1, P3, none)
- Hard to scan vertically - columns don't align

## New Format

```
2026-W04#1   [ ]  !!!  (P0)  PIV signing CLI PR (from 2026-W04)
2026-W04#2   [ ]  !!!  (P1)  Gov Migration Cohorts (from 2026-W04)
2026-W04#3   [ ]   !!  (P1)  IL5 Distributor work (from 2026-W04)
2026-W04#10  [ ]    !  (P3)  Wheel of Misfortune planning (from 2026-W04)
2026-01-20#5 [ ]       (P2)  Task with no urgency (from Daily Note)
2026-01-20#6 [x]       (P1)  Completed task (from Daily Note)
```

## Column Specifications

1. **Task ID**: Left-aligned, dynamically padded to longest ID in list
2. **Checkbox**: Fixed 3 chars - `[ ]` or `[x]`
3. **Urgency**: Right-aligned in 3-char field (handles `!!!`, `!!`, `!`, or empty)
4. **Priority**: Fixed 5-char field for `(P0)` format or empty
5. **Task text and source**: Variable width, flows naturally

**Spacing**: 2 spaces between each column

## Implementation

### Location
`src/cli/commands.rs` - `task_list()` function (lines 708-773)

### Two-Pass Approach

**Pass 1: Calculate widths**
```rust
// Find longest task ID
let max_id_width = tasks.iter()
    .map(|t| t.id().len())
    .max()
    .unwrap_or(0);
```

**Pass 2: Display with alignment**
```rust
for task in &tasks {
    // ID: left-aligned, padded
    write!(stdout, "{:<width$}", task.id(), width = max_id_width)?;

    // Checkbox: fixed 3 chars
    write!(stdout, "  [{}]", if task.completed { "x" } else { " " })?;

    // Urgency: right-aligned in 3 chars
    let urgency_str = task.urgency.as_ref().map(|u| u.to_string()).unwrap_or_default();
    write!(stdout, "  {:>3}", urgency_str)?;

    // Priority: fixed 5 chars
    let priority_str = task.priority.as_ref()
        .map(|p| format!("({})", p))
        .unwrap_or_else(|| "     ".to_string());
    write!(stdout, "  {}", priority_str)?;

    // Task text and source
    writeln!(stdout, "  {} (from {})", task.text, task.note_title)?;
}
```

### Color Handling
Apply colors within the aligned columns:
- Task ID: cyan (highlight)
- Checkbox: green if completed
- Task text/source: default with dim source

### No Changes Needed
- Task data structures (`src/note.rs`)
- Task extraction logic (`src/lib.rs`)
- Task sorting (already working)

## Testing

### Manual Verification
Run `cargo run -- tasks` and verify:
- Tasks with varying ID lengths
- All urgency combinations: `!!!`, `!!`, `!`, none
- All priority values: `(P0)` through `(P9)`, none
- Mix of completed and incomplete tasks

### Edge Cases
- Empty task list
- Single task
- Tasks with partial metadata
- Very long task IDs

### Verification Checklist
- [ ] All checkboxes start at same column
- [ ] Urgency symbols align vertically
- [ ] Priority values align vertically
- [ ] Colors work correctly
- [ ] Easy to scan at a glance

## Rationale

**Column order**: ID, checkbox, urgency, priority, text
- Checkbox first after ID shows completion status immediately
- Urgency and priority are planning metadata
- Task text is most important content, gets remaining space

**Alignment approach**: Fixed-width columns with spacing
- Clean look without visual clutter
- Pipe separators would be too busy
- Dynamic ID width handles varying task ID lengths

**No separators**: Just spacing between columns
- Maintains clean CLI aesthetic
- Alignment provides sufficient visual separation
