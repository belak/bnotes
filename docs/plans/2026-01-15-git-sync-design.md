# Git Sync Feature Design

**Date:** 2026-01-15
**Status:** Approved

## Overview

Add git synchronization commands to bnotes for backing up and syncing notes across devices. This feature provides manual sync commands that commit, pull, and push changes using git.

## Design Principles

- **Manual control** - User triggers sync operations explicitly (no auto-sync for CLI)
- **Shell out to git** - Use existing git installation rather than reimplementing git
- **Merge strategy** - Use merge commits rather than rebase (better for multi-device note editing)
- **Fail safely** - Error on conflicts and let user resolve manually

## Commands

### `bnotes sync [--message MSG]`

Full bidirectional sync: commit local changes, pull from remote, push to remote.

**Behavior when local changes exist:**
1. Stage all files (`git add .`)
2. Generate change summary from git status
3. Create commit with message (custom or timestamp) and summary in body
4. Pull with merge strategy
5. Push to remote

**Behavior when no local changes:**
1. Pull with merge strategy
2. Push to remote (ensures remote is up-to-date)

**Commit message format:**

With `--message`:
```
Custom message here

Modified:
- file1.md

Added:
- file2.md
```

Without `--message` (auto-generated):
```
bnotes sync: 2026-01-15T14:30:00Z

Modified:
- file1.md

Added:
- file2.md
```

### `bnotes pull`

Pull changes from remote without committing local changes.

**Behavior when working directory is dirty:**
1. Stash uncommitted changes with timestamped message
2. Pull with merge strategy
3. Pop stash to reapply changes
4. If stash pop has conflicts: warn user but continue (exit 0)

**Stash message format:**
```
bnotes pull auto-stash 2026-01-15T14:30:00Z
```

**Behavior when working directory is clean:**
1. Pull with merge strategy

## Detailed Command Flow

### `bnotes sync [--message MSG]`

1. Verify notes directory is a git repository (error if not)
2. Verify remote is configured (error if not)
3. Check for uncommitted changes via `git status --porcelain`
4. If changes exist:
   - Stage all files: `git add .`
   - Generate change summary from git status
   - Create commit message (custom or timestamp) with summary in body
   - Commit: `git commit -m "..."`
5. Pull from remote: `git pull --no-rebase`
   - If merge conflicts: error and list conflicted files, exit
6. Push to remote: `git push`
   - If push fails: pass through git error, exit
7. Display success summary

### `bnotes pull`

1. Verify notes directory is a git repository (error if not)
2. Verify remote is configured (error if not)
3. Check for uncommitted changes via `git status --porcelain`
4. If dirty working directory:
   - Create timestamped stash: `git stash push -m "bnotes pull auto-stash 2026-01-15T14:30:00Z"`
   - Pull: `git pull --no-rebase`
   - If merge conflicts during pull: error and list conflicted files, exit
   - Pop stash: `git stash pop`
   - If stash pop conflicts: warn and list conflicted files, continue (exit 0)
5. If clean working directory:
   - Pull: `git pull --no-rebase`
   - If merge conflicts: error and list conflicted files, exit
6. Display success summary

## Error Handling

### Not a git repository
```
Error: Not a git repository
The notes directory is not initialized with git.

Run 'git init' in your notes directory to get started.
```
Exit code: 1

### No remote configured
```
Error: No remote repository configured
Run 'git remote add origin <url>' to configure a remote.
```
Exit code: 1

### Merge conflicts during pull/sync
```
Error: Merge conflicts detected

The following files have conflicts:
  - project-notes.md
  - meeting-notes.md

Resolve conflicts manually and run 'git merge --continue'
```
Exit code: 1, repository left in conflicted state

Conflicted files obtained via: `git diff --name-only --diff-filter=U`

### Stash pop conflicts
```
Warning: Conflicts occurred while reapplying stashed changes

The following files have conflicts:
  - project-notes.md
  - meeting-notes.md

The stash has been applied but conflicts need resolution.
Run 'git status' to see details.
Your stashed changes are preserved in the stash list.
```
Exit code: 0 (warning only, not fatal)

### Network/authentication failures

Pass through git's error output directly and exit with code 1.

### Success messages

- `bnotes sync` with changes: "Synced successfully: committed N changes, pulled, and pushed"
- `bnotes sync` without changes: "Synced successfully: pulled and pushed"
- `bnotes pull`: "Pulled successfully"

## Implementation Architecture

### Dependencies

No new dependencies needed - use `std::process::Command` to shell out to git commands.

### New Module: `src/git.rs`

Encapsulates all git operations:

```rust
pub struct GitRepo {
    notes_dir: PathBuf,
}

impl GitRepo {
    pub fn new(notes_dir: PathBuf) -> Result<Self>;

    // Verification
    pub fn check_is_repo(&self) -> Result<()>;
    pub fn check_has_remote(&self) -> Result<()>;

    // Status checking
    pub fn has_uncommitted_changes(&self) -> Result<bool>;
    pub fn get_conflicted_files(&self) -> Result<Vec<String>>;

    // Operations
    pub fn stage_all(&self) -> Result<()>;
    pub fn commit(&self, message: &str) -> Result<()>;
    pub fn pull(&self) -> Result<()>;
    pub fn push(&self) -> Result<()>;
    pub fn stash_push(&self, message: &str) -> Result<()>;
    pub fn stash_pop(&self) -> Result<()>;

    // Analysis
    pub fn generate_change_summary(&self) -> Result<String>;
}
```

**Implementation details:**
- All methods use `std::process::Command` to execute git commands
- Working directory set to `notes_dir` for all git operations
- Parse command output and exit codes to detect errors
- Return Rust `Result` types with descriptive error messages

### New Module: `src/commands/sync.rs`

Command implementations that orchestrate git operations:

```rust
pub fn sync(config: Option<PathBuf>, message: Option<String>) -> Result<()>;
pub fn pull(config: Option<PathBuf>) -> Result<()>;
```

These functions:
1. Load config to get notes directory
2. Create `GitRepo` instance
3. Execute the command flow described above
4. Handle errors and display appropriate messages

### CLI Changes: `src/main.rs`

Add to `Commands` enum:
```rust
/// Sync notes with git remote (commit, pull, push)
Sync {
    /// Custom commit message
    #[arg(long, short)]
    message: Option<String>,
},

/// Pull changes from git remote
Pull,
```

Add to match statement in `main()`:
```rust
Commands::Sync { message } => {
    commands::sync::sync(cli.config, message)?;
}
Commands::Pull => {
    commands::sync::pull(cli.config)?;
}
```

### Change Summary Generation

Parse `git status --porcelain` output before staging to capture what will be committed.

**Git status format:** Each line is `XY filename` where:
- `M ` = Modified file
- `A ` = Added file
- `D ` = Deleted file
- `??` = Untracked file (treat as Added)

**Summary format:**
```
Modified:
- file1.md
- file2.md

Added:
- file3.md

Deleted:
- file4.md
```

Only include sections that have files. Empty sections are omitted.

## Why Merge Over Rebase

For code repositories, rebase creates clean linear history. For notes edited across multiple devices:

- **Merge is more forgiving**: Handles parallel edits on different devices better with a single conflict resolution point
- **Rebase is fragile**: Replays commits one by one, potentially creating multiple conflict points for the same logical change
- **History matches reality**: Merge commits show "I synced at this point" which accurately reflects multi-device workflow
- **Notes don't need clean history**: Unlike code, notes don't benefit from linear commit history

The merge strategy (`git pull --no-rebase`) is therefore the recommended approach for note synchronization.

## Future Considerations

Features explicitly deferred:
- Auto-sync after operations like `new` or `edit`
- Background daemon for periodic syncing
- Git repository initialization command (`bnotes sync init`)
- Push-only command (`bnotes push`)
- Configuration options for sync behavior
- Custom merge strategies

These can be added later if needed, especially if a TUI mode is implemented.
