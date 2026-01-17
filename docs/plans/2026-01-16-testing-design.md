# Integration Testing Design for bnotes

**Date:** 2026-01-16
**Status:** Approved

## Overview

Add comprehensive integration tests for bnotes CLI commands to validate functionality works correctly without requiring manual testing. Tests will use temporary directories and run actual CLI commands to verify end-to-end behavior.

## Goals

- Validate all core command functionality works correctly
- Avoid manual testing for each feature
- Focus on behavior verification, not exact output formatting
- Maintain fast, isolated, parallel-safe tests

## Out of Scope

- Git sync/pull operations (test manually for now)
- Wiki links and doctor commands
- Unit test expansion (focus on integration tests)

## Architecture

### Test Approach: Integration Tests via CLI Binary

Tests will spawn the actual `bnotes` binary and verify filesystem effects and command success/failure. This approach:

- Tests the complete user path including CLI parsing
- Catches integration issues between components
- Provides realistic validation of actual usage
- Uses temporary directories for isolation

### Dependencies

```toml
[dev-dependencies]
assert_cmd = "2.0"    # Run CLI commands in tests
predicates = "3.0"     # Assertions for command output
tempfile = "3.12"      # Temporary test directories
```

### Test Organization

```
tests/
├── common/
│   └── mod.rs           # Shared TestEnv and utilities
├── test_notes.rs        # Core note operations
├── test_periodic.rs     # Daily/weekly/quarterly notes
├── test_tasks.rs        # Task management
└── test_search.rs       # Search functionality
```

## Implementation Details

### Shared Test Infrastructure

**TestEnv Structure:**
```rust
pub struct TestEnv {
    pub temp_dir: TempDir,
    pub config_path: PathBuf,
    pub notes_dir: PathBuf,
}

impl TestEnv {
    pub fn new() -> Self;
    pub fn bnotes(&self) -> Command;  // Pre-configured with --config and --yes
    pub fn create_note(&self, title: &str, content: &str);
    pub fn read_note(&self, filename: &str) -> String;
    pub fn create_template(&self, name: &str, content: &str);
    pub fn note_exists(&self, filename: &str) -> bool;
}
```

Each test creates its own `TestEnv` with isolated temporary directory and config.

### Non-Interactive Mode

**Add global `--yes` flag:**
- Skip all interactive prompts
- Useful for both tests and scripting
- Commands fail fast rather than prompting when input needed

```rust
#[arg(long, short = 'y', global = true)]
yes: bool,
```

Commands needing `--yes` support:
- `bnotes new` - Skip title prompt
- `bnotes init` - Use provided/default notes_dir
- Any future confirmation prompts

## Test Coverage

### Core Note Operations (test_notes.rs)

**bnotes new:**
- Create note with title → verify file exists
- Create with template → verify template applied
- Create when file exists → verify handling

**bnotes note list:**
- List empty directory
- List multiple notes
- Filter by tags
- Verify output format

**bnotes note show:**
- Show existing note
- Show non-existent note (error case)
- Show note with frontmatter

### Search (test_search.rs)

**bnotes search:**
- Search with matches → verify results
- Search with no matches
- Search across multiple notes
- Search in content and frontmatter

### Periodic Notes (test_periodic.rs)

**bnotes daily:**
- Create today's note → verify `YYYY-MM-DD.md`
- Create for specific date
- Create with template → verify variables rendered
- Open existing note (no overwrite)
- List all daily notes
- Navigate with prev/next

**bnotes weekly:**
- Create current week → verify `YYYY-WNN.md`
- Create for specific date → verify ISO week calculation
- List and navigate

**bnotes quarterly:**
- Create current quarter → verify `YYYY-QN.md`
- Create with shortcuts (q1-q4)
- Create for specific date
- List and navigate

**Template variables:**
- `{{title}}` renders to note identifier
- `{{date}}` renders to ISO date
- `{{datetime}}` includes timestamp
- Custom vs default templates

### Task Management (test_tasks.rs)

**bnotes task list:**
- List from single note
- List from multiple notes (aggregation)
- Filter `--status open`
- Filter `--status done`
- Filter by tags
- Empty directory case
- Verify output includes note reference

**bnotes task show:**
- Show specific task by ID (e.g., "note#2")
- Invalid task ID (error case)
- Non-existent note (error case)

**bnotes tasks (shortcut):**
- Verify equivalent to `task list --status open`

**Edge cases:**
- Mix of checked/unchecked boxes
- Tasks in different sections
- Notes with no tasks
- GFM format variations (`- [ ]`, `* [ ]`)

## Testing Principles

1. **One behavior per test** - Each test verifies single functionality
2. **Isolated environments** - Each test gets own TestEnv
3. **Observable effects** - Test filesystem state and command exit codes
4. **Parallel safe** - No shared state between tests
5. **Fast feedback** - Keep setup minimal, tests focused

## Success Criteria

- All prioritized commands have integration test coverage
- Tests run successfully in parallel with `cargo test`
- Tests catch regressions in command functionality
- New features can easily add corresponding tests using TestEnv helpers
