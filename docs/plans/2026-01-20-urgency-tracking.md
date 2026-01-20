# Urgency Tracking Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add symbol-based urgency tracking (!!!, !!, !) to tasks with flexible comma-separated sort fields.

**Architecture:** Extend Task struct with urgency field, update parsing to extract urgency symbols before priority, replace TaskSortOrder enum with flexible field-based sorting that accepts comma-separated fields like "urgency,priority,id".

**Tech Stack:** Rust, pulldown-cmark for markdown parsing, clap for CLI args

---

## Task 1: Add urgency field to Task struct

**Files:**
- Modify: `src/note.rs:221-228`

**Step 1: Add urgency field to Task struct**

In `src/note.rs`, update the Task struct:

```rust
pub struct Task {
    pub note_path: PathBuf,
    pub note_title: String,
    pub index: usize, // 1-based index within the note
    pub completed: bool,
    pub text: String,
    pub priority: Option<String>,
    pub urgency: Option<String>,  // !!!, !!, !
}
```

**Step 2: Commit**

```bash
git add src/note.rs
git commit -m "feat: add urgency field to Task struct"
```

---

## Task 2: Add urgency parsing logic

**Files:**
- Modify: `src/note.rs` (around line 233 where parse_priority is)

**Step 1: Write test for urgency parsing**

Add tests to the bottom of `src/note.rs` in the `#[cfg(test)]` section:

```rust
#[test]
fn test_parse_urgency_only() {
    let content = "- [ ] !!! Fix critical bug";
    let note_path = PathBuf::from("test.md");
    let note = Note::parse(&note_path, &format!("# Test\n\n{}", content)).unwrap();
    let tasks = Task::extract_from_note(&note);

    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].urgency, Some("!!!".to_string()));
    assert_eq!(tasks[0].priority, None);
    assert_eq!(tasks[0].text, "Fix critical bug");
}

#[test]
fn test_parse_urgency_and_priority() {
    let content = "- [ ] !! (B) Moderate task";
    let note_path = PathBuf::from("test.md");
    let note = Note::parse(&note_path, &format!("# Test\n\n{}", content)).unwrap();
    let tasks = Task::extract_from_note(&note);

    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].urgency, Some("!!".to_string()));
    assert_eq!(tasks[0].priority, Some("B".to_string()));
    assert_eq!(tasks[0].text, "Moderate task");
}

#[test]
fn test_parse_no_space_after_urgency() {
    let content = "- [ ]!!!(A) Task";
    let note_path = PathBuf::from("test.md");
    let note = Note::parse(&note_path, &format!("# Test\n\n{}", content)).unwrap();
    let tasks = Task::extract_from_note(&note);

    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].urgency, None);
    assert_eq!(tasks[0].text, "!!!(A) Task");
}

#[test]
fn test_parse_exclamation_in_text() {
    let content = "- [ ] Do this now!";
    let note_path = PathBuf::from("test.md");
    let note = Note::parse(&note_path, &format!("# Test\n\n{}", content)).unwrap();
    let tasks = Task::extract_from_note(&note);

    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].urgency, None);
    assert_eq!(tasks[0].text, "Do this now!");
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test test_parse_urgency --lib
```

Expected: Tests fail because urgency parsing not implemented

**Step 3: Replace parse_priority with parse_urgency_and_priority**

In `src/note.rs`, find the `parse_priority` function (around line 233) and replace it with:

```rust
/// Parse urgency and priority from task text
/// Format: [urgency] [(priority)] task text
/// Urgency: !!!, !!, ! (must have space after)
/// Priority: (A), (B), etc.
/// Returns (urgency, priority, remaining_text)
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
    let (priority, task_text) = if rest.starts_with('(') {
        if let Some(end_paren) = rest.find(')') {
            let priority_str = &rest[1..end_paren];
            let remaining = rest[end_paren + 1..].trim();
            (Some(priority_str.to_string()), remaining.to_string())
        } else {
            (None, rest.to_string())
        }
    } else {
        (None, rest.to_string())
    };

    (urgency, priority, task_text)
}
```

**Step 4: Update Task::extract_from_note to use new parser**

Find where tasks are created in the `extract_from_note` method and update to use the new parser:

```rust
// Replace the line that calls parse_priority with:
let (urgency, priority, text) = parse_urgency_and_priority(&task_text);

// Update the Task construction to include urgency:
tasks.push(Task {
    note_path: note.path.clone(),
    note_title: note.title.clone(),
    index,
    completed,
    text,
    priority,
    urgency,
});
```

**Step 5: Run tests to verify they pass**

```bash
cargo test test_parse_urgency --lib
```

Expected: All urgency parsing tests pass

**Step 6: Run all tests to ensure nothing broke**

```bash
cargo test --lib
```

Expected: All existing tests still pass

**Step 7: Commit**

```bash
git add src/note.rs
git commit -m "feat: implement urgency parsing from task text"
```

---

## Task 3: Replace TaskSortOrder enum with flexible field-based sorting

**Files:**
- Modify: `src/lib.rs:37-50` (TaskSortOrder enum and impl)

**Step 1: Write test for TaskSortOrder parsing**

Add test to the bottom of `src/lib.rs` in the `#[cfg(test)]` section:

```rust
#[test]
fn test_task_sort_order_parse() {
    let order = TaskSortOrder::parse("urgency,priority,id").unwrap();
    assert_eq!(order.fields.len(), 3);

    let order = TaskSortOrder::parse("priority,id").unwrap();
    assert_eq!(order.fields.len(), 2);

    let order = TaskSortOrder::parse("id").unwrap();
    assert_eq!(order.fields.len(), 1);

    let result = TaskSortOrder::parse("invalid,priority");
    assert!(result.is_err());
}

#[test]
fn test_task_sort_order_default() {
    let order = TaskSortOrder::default();
    assert_eq!(order.fields.len(), 3);
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test test_task_sort_order --lib
```

Expected: Compilation error or test failure

**Step 3: Replace TaskSortOrder enum with new implementation**

In `src/lib.rs`, replace the TaskSortOrder enum (around line 37-50) with:

```rust
/// Task sort order - comma-separated list of fields
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSortOrder {
    fields: Vec<SortField>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortField {
    Urgency,
    Priority,
    Id,
}

impl TaskSortOrder {
    /// Parse sort order from comma-separated string
    pub fn parse(s: &str) -> Result<Self> {
        let fields: Result<Vec<_>> = s
            .split(',')
            .map(|f| match f.trim() {
                "urgency" => Ok(SortField::Urgency),
                "priority" => Ok(SortField::Priority),
                "id" => Ok(SortField::Id),
                unknown => anyhow::bail!("Unknown sort field: {}. Valid fields: urgency, priority, id", unknown),
            })
            .collect();

        Ok(TaskSortOrder { fields: fields? })
    }
}

impl Default for TaskSortOrder {
    fn default() -> Self {
        Self {
            fields: vec![SortField::Urgency, SortField::Priority, SortField::Id]
        }
    }
}
```

**Step 4: Run test to verify it passes**

```bash
cargo test test_task_sort_order --lib
```

Expected: Tests pass

**Step 5: Commit**

```bash
git add src/lib.rs
git commit -m "feat: implement flexible TaskSortOrder parsing"
```

---

## Task 4: Implement urgency and priority comparison functions

**Files:**
- Modify: `src/lib.rs` (add helper functions before list_tasks method)

**Step 1: Add comparison helper functions**

In `src/lib.rs`, add these functions before the `list_tasks` method (around line 138):

```rust
/// Compare urgency levels: !!! < !! < ! < None
fn compare_urgency(a: &Option<String>, b: &Option<String>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(a_urg), Some(b_urg)) => {
            let a_val = match a_urg.as_str() {
                "!!!" => 1,
                "!!" => 2,
                "!" => 3,
                _ => 4,
            };
            let b_val = match b_urg.as_str() {
                "!!!" => 1,
                "!!" => 2,
                "!" => 3,
                _ => 4,
            };
            a_val.cmp(&b_val)
        }
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

/// Compare priority levels: A < B < C < ... < None
fn compare_priority(a: &Option<String>, b: &Option<String>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(a_pri), Some(b_pri)) => a_pri.cmp(b_pri),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}
```

**Step 2: Commit**

```bash
git add src/lib.rs
git commit -m "feat: add urgency and priority comparison functions"
```

---

## Task 5: Update list_tasks to use new sort order

**Files:**
- Modify: `src/lib.rs:138-195` (list_tasks method)

**Step 1: Write integration test for sorting**

Add test to `src/lib.rs` in tests section:

```rust
#[test]
fn test_task_sorting_by_urgency_priority_id() {
    let storage = Box::new(MemoryStorage::new());

    storage.write(Path::new("tasks.md"), r#"# Tasks

- [ ] !!! (A) Critical and important
- [ ] !! (A) Soon and important
- [ ] !!! (C) Critical but low priority
- [ ] (A) Important but not urgent
- [ ] ! Eventually do this
- [ ] Plain task
"#).unwrap();

    let bnotes = BNotes::with_defaults(storage);
    let sort_order = TaskSortOrder::parse("urgency,priority,id").unwrap();
    let tasks = bnotes.list_tasks(&[], None, sort_order).unwrap();

    assert_eq!(tasks.len(), 6);

    // First two should both have !!!, sorted by priority (A < C)
    assert_eq!(tasks[0].urgency, Some("!!!".to_string()));
    assert_eq!(tasks[0].priority, Some("A".to_string()));

    assert_eq!(tasks[1].urgency, Some("!!!".to_string()));
    assert_eq!(tasks[1].priority, Some("C".to_string()));

    // Next should have !!
    assert_eq!(tasks[2].urgency, Some("!!".to_string()));

    // Then !
    assert_eq!(tasks[3].urgency, Some("!".to_string()));

    // Then tasks without urgency, sorted by priority
    assert_eq!(tasks[4].urgency, None);
    assert_eq!(tasks[4].priority, Some("A".to_string()));

    // Finally no urgency, no priority
    assert_eq!(tasks[5].urgency, None);
    assert_eq!(tasks[5].priority, None);
}

#[test]
fn test_task_sorting_by_priority_id() {
    let storage = Box::new(MemoryStorage::new());

    storage.write(Path::new("tasks.md"), r#"# Tasks

- [ ] !!! (C) Critical C
- [ ] (A) Important A
- [ ] (B) Important B
"#).unwrap();

    let bnotes = BNotes::with_defaults(storage);
    let sort_order = TaskSortOrder::parse("priority,id").unwrap();
    let tasks = bnotes.list_tasks(&[], None, sort_order).unwrap();

    // Should sort by priority only, ignoring urgency
    assert_eq!(tasks[0].priority, Some("A".to_string()));
    assert_eq!(tasks[1].priority, Some("B".to_string()));
    assert_eq!(tasks[2].priority, Some("C".to_string()));
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test test_task_sorting_by_urgency --lib
```

Expected: Compilation error or test failure

**Step 3: Update list_tasks signature**

Change the signature (around line 138):

```rust
pub fn list_tasks(&self, tags: &[String], status: Option<&str>, sort_order: TaskSortOrder) -> Result<Vec<note::Task>> {
```

**Step 4: Replace sorting logic in list_tasks**

Replace the existing match statement on `sort_order` (around line 167-192) with:

```rust
// Sort based on provided sort order
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

**Step 5: Run test to verify it passes**

```bash
cargo test test_task_sorting_by_urgency --lib
```

Expected: Tests pass

**Step 6: Run all lib tests**

```bash
cargo test --lib
```

Expected: All tests pass

**Step 7: Commit**

```bash
git add src/lib.rs
git commit -m "feat: implement flexible task sorting with urgency support"
```

---

## Task 6: Update CLI to use new sort order format

**Files:**
- Modify: `src/main.rs:75-77` (Tasks command)
- Modify: `src/main.rs:178-179` (TaskCommands::List)
- Modify: `src/main.rs:214-215` (Tasks command handler)
- Modify: `src/main.rs:241-242` (Task list handler)

**Step 1: Update Tasks command sort_order default**

In `src/main.rs`, find the `Tasks` command (around line 72-77) and update:

```rust
/// List open tasks (alias for 'task list --status open')
Tasks {
    /// Sort order: comma-separated fields (urgency, priority, id)
    #[arg(long, default_value = "urgency,priority,id")]
    sort_order: String,
},
```

**Step 2: Update TaskCommands::List sort_order default**

In `src/main.rs`, find `TaskCommands::List` (around line 166-180) and update:

```rust
/// List tasks across all notes
List {
    /// Filter by tags
    #[arg(long = "tag")]
    tags: Vec<String>,

    /// Filter by status (open or done)
    #[arg(long)]
    status: Option<String>,

    /// Sort order: comma-separated fields (urgency, priority, id)
    #[arg(long, default_value = "urgency,priority,id")]
    sort_order: String,
},
```

**Step 3: Update Commands::Tasks handler**

In `src/main.rs`, find the `Commands::Tasks` handler (around line 214) and update:

```rust
Commands::Tasks { sort_order } => {
    let sort_order = bnotes::TaskSortOrder::parse(&sort_order)
        .context("Invalid sort order")?;
    cli::commands::task_list(&notes_dir, &[], Some("open".to_string()), sort_order, cli_args.color)?;
}
```

**Step 4: Update TaskCommands::List handler**

In `src/main.rs`, find the `TaskCommands::List` handler (around line 240-242) and update:

```rust
TaskCommands::List { tags, status, sort_order } => {
    let sort_order = bnotes::TaskSortOrder::parse(&sort_order)
        .context("Invalid sort order")?;
    cli::commands::task_list(&notes_dir, &tags, status, sort_order, cli_args.color)?;
}
```

**Step 5: Build to check for compilation errors**

```bash
cargo build
```

Expected: Build fails because cli::commands::task_list signature doesn't match

**Step 6: Commit**

```bash
git add src/main.rs
git commit -m "feat: update CLI to use new sort order format"
```

---

## Task 7: Update CLI task_list to accept TaskSortOrder

**Files:**
- Modify: `src/cli/commands.rs:708-713` (task_list signature)
- Modify: `src/cli/commands.rs:715-725` (remove sort_order parsing)

**Step 1: Update task_list signature**

In `src/cli/commands.rs`, find the `task_list` function (around line 708) and update:

```rust
pub fn task_list(
    notes_dir: &Path,
    tags: &[String],
    status: Option<String>,
    sort_order: bnotes::TaskSortOrder,
    color: ColorChoice,
) -> Result<()> {
```

**Step 2: Remove sort_order parsing logic**

Remove the match statement that parses sort_order string (around lines 719-724):

```rust
// Delete these lines:
// let sort_order = match sort_order_str {
//     "priority-id" => bnotes::TaskSortOrder::PriorityId,
//     "id" => bnotes::TaskSortOrder::Id,
//     _ => anyhow::bail!(...),
// };
```

**Step 3: Build to verify**

```bash
cargo build
```

Expected: Build succeeds

**Step 4: Commit**

```bash
git add src/cli/commands.rs
git commit -m "feat: update task_list to accept TaskSortOrder directly"
```

---

## Task 8: Update CLI display to show urgency

**Files:**
- Modify: `src/cli/commands.rs:736-760` (task display loop)

**Step 1: Update task display to show urgency**

In `src/cli/commands.rs`, find the task display loop (around line 736-759) and update the display logic after the task ID:

```rust
// Display tasks
for task in &tasks {
    // Task ID in cyan
    stdout.set_color(&colors::highlight())?;
    write!(stdout, "{}", task.id())?;
    stdout.reset()?;

    write!(stdout, "  ")?;

    // Show urgency if present
    if let Some(urgency) = &task.urgency {
        write!(stdout, "{} ", urgency)?;
    }

    // Show priority if present
    if let Some(priority) = &task.priority {
        write!(stdout, "({}) ", priority)?;
    }

    // Checkbox - [x] in green, [ ] default
    if task.completed {
        stdout.set_color(&colors::success())?;
        write!(stdout, "[x]")?;
        stdout.reset()?;
    } else {
        write!(stdout, "[ ]")?;
    }

    // Task text in default
    write!(stdout, " {} ", task.text)?;

    // "from [note]" in dim
    stdout.set_color(&colors::dim())?;
    writeln!(stdout, "(from {})", task.note_title)?;
    stdout.reset()?;
}
```

**Step 2: Build to verify**

```bash
cargo build
```

Expected: Build succeeds

**Step 3: Test manually**

Create a test note with urgency:

```bash
echo '# Test

- [ ] !!! (A) Critical task
- [ ] !! Do this soon
- [ ] (B) Important
- [ ] Normal task
' > /tmp/test-urgency.md

cargo run -- --notes-dir /tmp task list
```

Expected: Tasks display with urgency symbols

**Step 4: Commit**

```bash
git add src/cli/commands.rs
git commit -m "feat: display urgency symbols in task list"
```

---

## Task 9: Run full test suite

**Files:**
- None (verification step)

**Step 1: Run all tests**

```bash
cargo test
```

Expected: All tests pass (including new urgency tests)

**Step 2: Run integration tests**

```bash
cargo test --test '*'
```

Expected: Integration tests pass

**Step 3: If any tests fail**

Debug and fix failing tests, then commit fixes:

```bash
git add <fixed-files>
git commit -m "fix: resolve test failures"
```

---

## Task 10: Update documentation

**Files:**
- Modify: `CLAUDE.md` (Task Format and Task Sorting sections)
- Modify: `README.md` (Notes section)

**Step 1: Update CLAUDE.md Task Format section**

In `CLAUDE.md`, find the Task Format section and update:

```markdown
**Task Format**
Tasks are GitHub-flavored markdown checkboxes:
```markdown
- [ ] Task text
- [x] Completed task
- [ ] (A) High priority task
- [ ] !!! Urgent task (critical/now)
- [ ] !! Soon task
- [ ] ! Eventually task
- [ ] !! (B) Soon and medium priority
```

Urgency is optional, format: `!!!` (critical/now), `!!` (soon), `!` (eventually)
Priority is optional, format: `(A)`, `(B)`, etc.
Urgency must come before priority when both present, with space after urgency symbol.
Tasks are tracked by ID: `filename#index`.

**Task Sorting**
Sort order is comma-separated fields: `urgency,priority,id`
- `urgency`: Sort by urgency (!!!, !!, !, None last)
- `priority`: Sort by priority (A < B < C, None last)
- `id`: Sort by filename#index

Default: `urgency,priority,id`
Examples:
- `--sort-order urgency,priority,id` - Eisenhower matrix style
- `--sort-order priority,urgency,id` - Important tasks first
- `--sort-order id` - By task ID only
```

**Step 2: Update README.md Notes section**

In `README.md`, find the Notes section and update the task format description:

```markdown
## Notes

Notes are markdown files with optional YAML frontmatter. Use `[[wiki links]]` to reference other notes.

Tasks are GitHub-flavored markdown checkboxes with optional urgency and priority:
- `- [ ] todo` - Basic task
- `- [ ] !!! urgent task` - Critical/now (also `!!` for soon, `!` for eventually)
- `- [ ] (A) important task` - Priority task (A, B, C, etc.)
- `- [ ] !! (B) soon and medium priority` - Both urgency and priority

Periodic notes (daily, weekly, quarterly) follow naming conventions like `2026-01-20.md`, `2026-W03.md`, `2026-Q1.md`.
```

**Step 3: Commit documentation updates**

```bash
git add CLAUDE.md README.md
git commit -m "docs: update task format and sorting documentation"
```

---

## Task 11: Final verification and cleanup

**Files:**
- None (verification and cleanup)

**Step 1: Build release binary**

```bash
cargo build --release
```

Expected: Clean release build

**Step 2: Manual testing with real workflow**

Create a test notes directory and try the full workflow:

```bash
mkdir -p /tmp/bnotes-test
cd /tmp/bnotes-test

# Create sample notes with various urgency/priority combinations
echo '# Work Tasks

- [ ] !!! (A) Fix production bug
- [ ] !! (A) Deploy new feature
- [ ] !!! (C) Urgent but not important
- [ ] (A) Plan next sprint
- [ ] !! Review PRs
- [ ] ! Update docs someday
- [ ] Basic task
' > work.md

# Test different sort orders
cargo run --manifest-path <worktree-path>/Cargo.toml -- --notes-dir . tasks
cargo run --manifest-path <worktree-path>/Cargo.toml -- --notes-dir . task list --sort-order priority,urgency,id
cargo run --manifest-path <worktree-path>/Cargo.toml -- --notes-dir . task list --sort-order id
```

Expected: Tasks display correctly with urgency symbols in correct sort order

**Step 3: Test error handling**

```bash
# Test invalid sort field
cargo run --manifest-path <worktree-path>/Cargo.toml -- --notes-dir /tmp/bnotes-test task list --sort-order invalid,priority
```

Expected: Clear error message about invalid field

**Step 4: Review all changes**

```bash
git log --oneline feature/urgency-tracking
git diff main...feature/urgency-tracking
```

Expected: Clean commit history, no unintended changes

**Step 5: All done!**

Ready to use @superpowers:finishing-a-development-branch to merge or create PR.
