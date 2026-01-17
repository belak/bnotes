# Library + CLI Separation Design

**Date:** 2026-01-17
**Status:** Approved

## Overview

Refactor bnotes to separate business logic into a testable library while keeping CLI concerns (I/O, formatting, config loading) in the binary layer. This improves testability and architectural clarity.

## Goals

- Create a clean library API that handles all business logic
- Make core functionality testable without filesystem I/O
- Separate CLI concerns (stdin/stdout, editor, formatting) from domain logic
- Make notes directory self-contained with its own configuration
- Maintain all existing functionality

## Architecture

### Overall Structure

```
bnotes/
├── src/
│   ├── lib.rs              # Library entry point, exports BNotes API
│   ├── lib/
│   │   ├── storage.rs      # Storage trait + implementations
│   │   ├── config.rs       # LibraryConfig (loaded from notes dir)
│   │   └── ...             # Other library internals (repository, note, task, etc.)
│   ├── main.rs             # CLI entry point
│   ├── commands/           # CLI command handlers (thin wrappers)
│   ├── config.rs           # CLIConfig (minimal, just notes_dir path)
│   └── ...                 # Other CLI-specific modules
```

**Library responsibilities:**
- All business logic for notes, tasks, periodic notes, sync
- Returns structured data (domain types)
- No I/O operations (stdin/stdout/stderr)
- No editor launching
- Uses Storage trait for file operations
- Loads its own config from within the notes directory

**CLI responsibilities:**
- Argument parsing (clap)
- Minimal config loading (just notes directory path)
- Interactive prompting (e.g., asking for note title)
- Editor launching (using `$EDITOR` env var)
- Formatting and printing library results
- Creating BNotes instance with real storage scoped to notes directory

### Storage Abstraction

**Trait definition:**

```rust
// src/lib/storage.rs
pub trait Storage {
    fn read_to_string(&self, path: &Path) -> Result<String>;
    fn write(&self, path: &Path, contents: &str) -> Result<()>;
    fn exists(&self, path: &Path) -> bool;
    fn is_dir(&self, path: &Path) -> bool;
    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>>;
    fn create_dir_all(&self, path: &Path) -> Result<()>;
}
```

**Key design decision:** The `Storage` trait is scoped to the notes directory. All paths are relative to the notes root. This means:
- Library code uses paths like `"project.md"` or `".templates/daily.md"`
- No need to join with notes_dir throughout the library
- `Repository` doesn't need to store the notes directory path
- Tests use simple relative paths

**Implementations:**

1. **RealStorage** - Production implementation wrapping `std::fs`, scoped to notes directory

```rust
pub struct RealStorage {
    root: PathBuf,  // notes_dir
}

impl RealStorage {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl Storage for RealStorage {
    fn read_to_string(&self, path: &Path) -> Result<String> {
        std::fs::read_to_string(self.root.join(path))
            .context(format!("Failed to read {}", path.display()))
    }

    fn write(&self, path: &Path, contents: &str) -> Result<()> {
        std::fs::write(self.root.join(path), contents)
            .context(format!("Failed to write {}", path.display()))
    }

    fn exists(&self, path: &Path) -> bool {
        self.root.join(path).exists()
    }

    fn is_dir(&self, path: &Path) -> bool {
        self.root.join(path).is_dir()
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let full_path = self.root.join(path);
        let entries = std::fs::read_dir(&full_path)
            .context(format!("Failed to read directory {}", path.display()))?;

        entries
            .map(|entry| {
                entry
                    .map(|e| {
                        e.path()
                            .strip_prefix(&self.root)
                            .unwrap()
                            .to_path_buf()
                    })
                    .map_err(Into::into)
            })
            .collect()
    }

    fn create_dir_all(&self, path: &Path) -> Result<()> {
        std::fs::create_dir_all(self.root.join(path))
            .context(format!("Failed to create directory {}", path.display()))
    }
}
```

2. **MemoryStorage** - HashMap-based implementation for tests

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct MemoryStorage {
    files: Arc<Mutex<HashMap<PathBuf, String>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            files: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Storage for MemoryStorage {
    fn read_to_string(&self, path: &Path) -> Result<String> {
        let files = self.files.lock().unwrap();
        files
            .get(path)
            .cloned()
            .ok_or_else(|| anyhow!("File not found: {}", path.display()))
    }

    fn write(&self, path: &Path, contents: &str) -> Result<()> {
        let mut files = self.files.lock().unwrap();
        files.insert(path.to_path_buf(), contents.to_string());
        Ok(())
    }

    fn exists(&self, path: &Path) -> bool {
        let files = self.files.lock().unwrap();
        files.contains_key(path)
    }

    fn is_dir(&self, path: &Path) -> bool {
        let files = self.files.lock().unwrap();
        let path_str = path.to_string_lossy();
        files.keys().any(|k| {
            k.to_string_lossy().starts_with(&*path_str)
                && k != path
        })
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let files = self.files.lock().unwrap();
        let path_str = path.to_string_lossy();

        let mut entries: Vec<PathBuf> = files
            .keys()
            .filter(|k| {
                let k_str = k.to_string_lossy();
                k_str.starts_with(&*path_str) && k != &path
            })
            .cloned()
            .collect();

        entries.sort();
        Ok(entries)
    }

    fn create_dir_all(&self, _path: &Path) -> Result<()> {
        // No-op for memory storage
        Ok(())
    }
}
```

**Usage:**

```rust
// Production
let storage = Box::new(RealStorage::new(cli_config.notes_dir.clone()));
let bnotes = BNotes::new(lib_config, storage);

// Access files with relative paths
storage.read_to_string(Path::new("project.md"));           // reads notes_dir/project.md
storage.read_to_string(Path::new(".templates/daily.md")); // reads notes_dir/.templates/daily.md

// Tests
let storage = Box::new(MemoryStorage::new());
storage.write(Path::new("test.md"), "# Test").unwrap();
let lib_config = LibraryConfig::load_or_default(&*storage)?;
let bnotes = BNotes::new(lib_config, storage);
```

### Configuration Architecture

**CLIConfig (CLI layer only):**

```rust
// src/config.rs
#[derive(Debug, Serialize, Deserialize)]
pub struct CLIConfig {
    pub notes_dir: PathBuf,
}

impl CLIConfig {
    /// Load config from the specified path
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: CLIConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        Ok(config)
    }

    /// Resolve and load config file from CLI arg, env var, or default location
    pub fn resolve_and_load(config_path: Option<&Path>) -> Result<Self> {
        let path = if let Some(p) = config_path {
            // CLI argument takes precedence
            p.to_path_buf()
        } else if let Ok(env_path) = std::env::var("BNOTES_CONFIG") {
            // Environment variable
            PathBuf::from(env_path)
        } else {
            // Default location
            Self::default_config_path()?
        };

        if !path.exists() {
            anyhow::bail!(
                "No config found at: {}\nRun `bnotes init` to create one.",
                path.display()
            );
        }

        Self::load(&path)
    }

    /// Get the default config file path
    pub fn default_config_path() -> Result<PathBuf> {
        let config_dir = if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg_config)
        } else {
            let home = std::env::var("HOME")
                .context("HOME environment variable not set")?;
            PathBuf::from(home).join(".config")
        };

        Ok(config_dir.join("bnotes").join("config.toml"))
    }
}
```

**CLIConfig TOML format:**
```toml
# ~/.config/bnotes/config.toml
notes_dir = "/Users/username/notes"
```

**LibraryConfig (library layer):**

```rust
// src/lib/config.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryConfig {
    #[serde(default = "default_template_dir")]
    pub template_dir: PathBuf,
    #[serde(default)]
    pub periodic: PeriodicConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PeriodicConfig {
    #[serde(default = "default_daily_template")]
    pub daily_template: String,
    #[serde(default = "default_weekly_template")]
    pub weekly_template: String,
    #[serde(default = "default_quarterly_template")]
    pub quarterly_template: String,
}

fn default_template_dir() -> PathBuf {
    PathBuf::from(".templates")
}

fn default_daily_template() -> String {
    "daily.md".to_string()
}

fn default_weekly_template() -> String {
    "weekly.md".to_string()
}

fn default_quarterly_template() -> String {
    "quarterly.md".to_string()
}

impl Default for LibraryConfig {
    fn default() -> Self {
        Self {
            template_dir: default_template_dir(),
            periodic: PeriodicConfig::default(),
        }
    }
}

impl LibraryConfig {
    /// Load library config from storage
    /// Looks for .bnotes/config.toml or config.toml in the notes directory
    pub fn load(storage: &dyn Storage) -> Result<Self> {
        // Try .bnotes/config.toml first (preferred location)
        if storage.exists(Path::new(".bnotes/config.toml")) {
            let content = storage.read_to_string(Path::new(".bnotes/config.toml"))?;
            return toml::from_str(&content)
                .context("Failed to parse .bnotes/config.toml");
        }

        // Fall back to config.toml in root
        if storage.exists(Path::new("config.toml")) {
            let content = storage.read_to_string(Path::new("config.toml"))?;
            return toml::from_str(&content)
                .context("Failed to parse config.toml");
        }

        anyhow::bail!("No library config found. Expected .bnotes/config.toml or config.toml")
    }

    /// Load config or return defaults if not found
    pub fn load_or_default(storage: &dyn Storage) -> Self {
        Self::load(storage).unwrap_or_default()
    }

    pub fn template_dir_path(&self) -> &Path {
        &self.template_dir
    }
}
```

**LibraryConfig TOML format:**
```toml
# <notes_dir>/.bnotes/config.toml or <notes_dir>/config.toml
template_dir = ".templates"

[periodic]
daily_template = "daily.md"
weekly_template = "weekly.md"
quarterly_template = "quarterly.md"
```

**Editor handling (CLI layer):**

```rust
// No config needed - just read environment variable
fn get_editor() -> String {
    std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string())
}

// Usage in commands/edit.rs
let editor = get_editor();
Command::new(&editor)
    .arg(&note_path)
    .status()?;
```

**Key design decisions:**

1. **Separate configs for separate concerns:**
   - CLIConfig: Minimal, just points to notes directory (lives in ~/.config/bnotes/)
   - LibraryConfig: Notes-specific settings (lives inside notes directory)

2. **Self-contained notes directory:**
   - Each notes directory has its own LibraryConfig
   - Config can be version controlled with notes
   - Different notes directories can have different settings

3. **No editor config:**
   - `$EDITOR` is a standard Unix convention
   - Fallback to `vim` is reasonable
   - No need to configure in TOML

### BNotes API Struct

**Main entry point:**

```rust
// src/lib.rs
pub struct BNotes {
    config: LibraryConfig,
    repo: Repository,
}

impl BNotes {
    pub fn new(config: LibraryConfig, storage: Box<dyn Storage>) -> Self {
        let repo = Repository::new(storage);
        Self { config, repo }
    }

    /// Create BNotes by loading config from storage
    pub fn from_storage(storage: Box<dyn Storage>) -> Result<Self> {
        let config = LibraryConfig::load(&*storage)?;
        Ok(Self::new(config, storage))
    }

    /// Create BNotes with default config
    pub fn with_defaults(storage: Box<dyn Storage>) -> Self {
        let config = LibraryConfig::load_or_default(&*storage);
        Self::new(config, storage)
    }

    // Note operations
    pub fn search(&self, query: &str) -> Result<Vec<Note>>;
    pub fn create_note(&self, title: &str, template_name: Option<&str>) -> Result<PathBuf>;
    pub fn list_notes(&self, tags: &[String]) -> Result<Vec<Note>>;
    pub fn get_note(&self, title: &str) -> Result<Note>;
    pub fn find_note_by_title(&self, title: &str) -> Result<PathBuf>;
    pub fn get_note_links(&self, title: &str) -> Result<(Vec<Note>, Vec<Note>)>;
    pub fn get_link_graph(&self) -> Result<HashMap<String, Vec<String>>>;

    // Task operations
    pub fn list_tasks(&self, tags: &[String], status: Option<&str>) -> Result<Vec<Task>>;
    pub fn get_task(&self, task_id: &str) -> Result<(Task, Note)>;

    // Periodic operations
    pub fn open_periodic<P: Period>(&self, date: Option<&str>, template: Option<&str>) -> Result<PathBuf>;
    pub fn list_periodic<P: Period>(&self) -> Result<Vec<PathBuf>>;
    pub fn navigate_periodic<P: Period>(&self, direction: Direction) -> Result<PathBuf>;

    // Git operations
    pub fn sync(&self, message: Option<&str>) -> Result<()>;
    pub fn pull(&self) -> Result<()>;

    // Health checks
    pub fn check_health(&self) -> Result<HealthReport>;
}
```

**Design decision:** Logic lives directly in `BNotes` methods, not separate domain modules. Domain modules only exist if shared utility functions are needed (YAGNI principle).

### Repository Refactoring

**Current:**
```rust
impl Repository {
    pub fn new(notes_dir: &Path) -> Self {
        Self { notes_dir: notes_dir.to_path_buf() }
    }
    // Uses std::fs directly
}
```

**New:**
```rust
impl Repository {
    pub fn new(storage: Box<dyn Storage>) -> Self {
        Self { storage }
    }
    // Uses storage trait with relative paths
}
```

All `std::fs` calls replaced with `self.storage.read_to_string()`, `self.storage.write()`, etc.
All paths are relative to the storage root (notes directory).

### CLI Layer Pattern

**Before (commands/search.rs):**
```rust
pub fn run(config_path: Option<PathBuf>, query: &str) -> Result<()> {
    let ctx = CommandContext::load(config_path)?;
    let matches = ctx.repo.search(query)?;
    // formatting and printing
    Ok(())
}
```

**After:**
```rust
pub fn run(config_path: Option<PathBuf>, query: &str) -> Result<()> {
    let cli_config = CLIConfig::resolve_and_load(config_path.as_deref())?;
    let storage = Box::new(RealStorage::new(cli_config.notes_dir.clone()));
    let bnotes = BNotes::from_storage(storage)?;

    let matches = bnotes.search(query)?;

    // Formatting and printing (stays in CLI)
    if matches.is_empty() {
        println!("No notes found matching: {}", query);
        return Ok(());
    }

    for note in &matches {
        println!("{}", note.title);
        // ... snippet formatting
    }

    Ok(())
}
```

**Interactive commands (commands/new.rs):**
```rust
pub fn run(config_path: Option<PathBuf>, title: Option<String>, template: Option<String>) -> Result<()> {
    // CLI handles prompting
    let title = if let Some(t) = title {
        t
    } else {
        print!("Enter note title: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        input.trim().to_string()
    };

    if title.is_empty() {
        anyhow::bail!("Title cannot be empty");
    }

    // Call library with complete data
    let cli_config = CLIConfig::resolve_and_load(config_path.as_deref())?;
    let storage = Box::new(RealStorage::new(cli_config.notes_dir.clone()));
    let bnotes = BNotes::from_storage(storage)?;
    let note_path = bnotes.create_note(&title, template.as_deref())?;

    // note_path is relative, join with notes_dir for display
    println!("Created note: {}", cli_config.notes_dir.join(note_path).display());
    Ok(())
}
```

**Editor commands (commands/edit.rs):**
```rust
pub fn run(config_path: Option<PathBuf>, title: &str) -> Result<()> {
    let cli_config = CLIConfig::resolve_and_load(config_path.as_deref())?;
    let storage = Box::new(RealStorage::new(cli_config.notes_dir.clone()));
    let bnotes = BNotes::from_storage(storage)?;

    // Find note using library
    let note_path = bnotes.find_note_by_title(title)?;
    let full_path = cli_config.notes_dir.join(note_path);

    // Launch editor (CLI concern)
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
    let status = Command::new(&editor)
        .arg(&full_path)
        .status()
        .with_context(|| format!("Failed to launch editor: {}", editor))?;

    if !status.success() {
        anyhow::bail!("Editor exited with non-zero status");
    }

    Ok(())
}
```

### Init Command Changes

The `init` command needs to create both configs:

```rust
// commands/init.rs
pub fn run(notes_dir: Option<PathBuf>) -> Result<()> {
    // Get notes directory
    let notes_dir = if let Some(dir) = notes_dir {
        dir
    } else {
        print!("Enter notes directory path: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        PathBuf::from(input.trim())
    };

    // Expand ~ if present
    let notes_dir = shellexpand::tilde(&notes_dir.to_string_lossy()).into_owned();
    let notes_dir = PathBuf::from(notes_dir);

    // Create notes directory
    std::fs::create_dir_all(&notes_dir)?;

    // Create CLI config
    let cli_config_path = CLIConfig::default_config_path()?;
    std::fs::create_dir_all(cli_config_path.parent().unwrap())?;

    let cli_config = CLIConfig { notes_dir: notes_dir.clone() };
    let cli_config_content = toml::to_string_pretty(&cli_config)?;
    std::fs::write(&cli_config_path, cli_config_content)?;

    println!("Created CLI config: {}", cli_config_path.display());

    // Create library config in notes directory
    let lib_config_dir = notes_dir.join(".bnotes");
    std::fs::create_dir_all(&lib_config_dir)?;

    let lib_config_path = lib_config_dir.join("config.toml");
    let lib_config = LibraryConfig::default();
    let lib_config_content = toml::to_string_pretty(&lib_config)?;
    std::fs::write(&lib_config_path, lib_config_content)?;

    println!("Created library config: {}", lib_config_path.display());
    println!("\nInitialization complete! Notes directory: {}", notes_dir.display());

    Ok(())
}
```

### Error Handling

Continue using `anyhow::Result` throughout both library and CLI:
- Already used consistently in codebase
- Library is single-purpose (not a general-use library)
- Errors naturally flow from library to CLI
- Avoids premature abstraction (YAGNI)

If structured error matching is needed later, a `BNotesError` enum can be introduced.

## Testing Strategy

### Library Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_finds_matching_notes() {
        let storage = Box::new(MemoryStorage::new());
        storage.write(
            Path::new("project.md"),
            "---\ntags: [work]\n---\n# Project\n\nImplement search feature"
        ).unwrap();

        let bnotes = BNotes::with_defaults(storage);
        let results = bnotes.search("search").unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Project");
    }

    #[test]
    fn test_create_note_with_template() {
        let storage = Box::new(MemoryStorage::new());
        storage.write(
            Path::new(".templates/default.md"),
            "---\ntags: []\n---\n# {{title}}\n"
        ).unwrap();

        let bnotes = BNotes::with_defaults(storage);
        let path = bnotes.create_note("Test Note", Some("default")).unwrap();

        assert_eq!(path, PathBuf::from("test-note.md"));

        let content = storage.read_to_string(&path).unwrap();
        assert!(content.contains("# Test Note"));
    }

    #[test]
    fn test_list_notes_with_tag_filter() {
        let storage = Box::new(MemoryStorage::new());
        storage.write(
            Path::new("work.md"),
            "---\ntags: [work]\n---\n# Work Note"
        ).unwrap();
        storage.write(
            Path::new("personal.md"),
            "---\ntags: [personal]\n---\n# Personal Note"
        ).unwrap();

        let bnotes = BNotes::with_defaults(storage);
        let results = bnotes.list_notes(&["work".to_string()]).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Work Note");
    }

    #[test]
    fn test_load_custom_config() {
        let storage = Box::new(MemoryStorage::new());
        storage.write(
            Path::new(".bnotes/config.toml"),
            r#"
template_dir = "my-templates"

[periodic]
daily_template = "custom-daily.md"
"#
        ).unwrap();

        let bnotes = BNotes::from_storage(storage).unwrap();
        assert_eq!(bnotes.config.template_dir, PathBuf::from("my-templates"));
        assert_eq!(bnotes.config.periodic.daily_template, "custom-daily.md");
    }
}
```

### Benefits

- No temporary directories or filesystem cleanup
- Fast, isolated tests
- Full control over filesystem state
- Can easily test error conditions
- Verify exact domain objects, not string matching
- Simple relative paths in tests
- Easy to test with different configurations

### CLI Testing

CLI layer remains mostly untested - it's thin glue code for I/O. Integration tests can cover end-to-end behavior if needed.

## Migration Strategy

### Phase 1: Foundation

Create library structure without breaking existing code:

1. Create `src/lib.rs` and configure `Cargo.toml` for both lib and bin
2. Add `Storage` trait in `src/lib/storage.rs`
3. Implement `RealStorage` and `MemoryStorage`
4. Create `LibraryConfig` in `src/lib/config.rs`
5. Rename existing `Config` to `CLIConfig` and simplify (remove editor, keep only notes_dir)
6. Update existing code to use `CLIConfig` name

**Success criteria:** Existing binary still builds and works with renamed config.

### Phase 2: Refactor Repository

Update Repository to use Storage trait:

1. Change `Repository::new()` signature to accept `Box<dyn Storage>`
2. Remove `notes_dir` field from Repository
3. Replace all `std::fs::read_to_string()` with `self.storage.read_to_string()`
4. Replace all `std::fs::write()` with `self.storage.write()`
5. Replace `walkdir` with recursive `self.storage.read_dir()` logic
6. Update other `std::fs` calls (exists, create_dir_all, etc.)
7. Convert all absolute paths to relative paths (remove notes_dir prefix)
8. Update `CommandContext` to create Repository with `RealStorage`

**Success criteria:** All existing commands still work. Repository tests can use MemoryStorage.

### Phase 3: Create BNotes API

Build library API incrementally, one command at a time:

1. Create `BNotes` struct with constructor and `from_storage()` method
2. Migrate commands in order of complexity:
   - **search** - Simple, read-only
   - **note list/show/links/graph** - Read-only operations
   - **new** - Creates files, uses templates
   - **task list/show** - Read-only but complex parsing
   - **periodic** (daily/weekly/quarterly) - Complex date logic
   - **sync/pull** - Git operations
   - **doctor** - Health checks
3. For each command:
   - Add method to `BNotes` with business logic from command module
   - Update command module to use `BNotes::from_storage()` instead of `CommandContext`
   - Add library tests using `MemoryStorage`
   - Verify CLI still works

**Success criteria:** Each command works through library API. Library has test coverage.

### Phase 4: Update Init Command

Update the init command to create both configs:

1. Create CLIConfig at `~/.config/bnotes/config.toml`
2. Create LibraryConfig at `<notes_dir>/.bnotes/config.toml`
3. Update documentation

**Success criteria:** `bnotes init` creates both config files properly.

### Phase 5: Cleanup

1. Remove `CommandContext` struct (no longer needed)
2. Remove `src/util.rs` if only `pluralize` remains (move to CLI)
3. Update README with both config locations and formats
4. Add library documentation comments
5. Update existing users' configs (migration guide)

**Success criteria:** Clean codebase with clear library/CLI separation. Documentation updated.

## Design Decisions

### Why separate CLI and Library configs?

**Benefits:**
- Notes directory is self-contained and portable
- LibraryConfig can be version controlled with notes
- Different notes directories can have different settings
- CLI config is minimal (just the path)
- Clearer separation of concerns

**Trade-off:**
- Two config files instead of one
- Init command creates both

The benefits of having notes-specific config inside the notes directory outweigh the slight complexity.

### Why no editor config?

`$EDITOR` is a standard Unix environment variable. Benefits:
- Follows Unix conventions
- No need to duplicate environment config in files
- `vim` fallback is reasonable
- Reduces config complexity

### Why Box instead of Arc?

`Box<dyn Storage>` is sufficient because:
- Repository owns the storage
- BNotes owns the Repository
- No shared ownership or cloning needed
- Can switch to `Arc` later if thread-safety is needed

### Why Storage instead of FileSystem?

The trait is scoped to the notes directory, not the entire filesystem. "Storage" better reflects this scoping. It represents the notes storage layer, not arbitrary filesystem access.

### Why scope Storage to notes_dir?

Benefits:
- Library code uses simple relative paths
- No need to store or pass around notes_dir
- Clearer abstraction - storage IS the notes directory
- Tests are simpler (no `/notes/` prefix needed)
- Repository doesn't need notes_dir field
- LibraryConfig doesn't need notes_dir field

Trade-off:
- CLI must create RealStorage with notes_dir
- Library returns relative paths (CLI must join with notes_dir for display)

The benefits outweigh the minor CLI complexity.

### Why load LibraryConfig from storage?

Makes the notes directory self-contained:
- Config travels with notes
- Can version control config
- Different notes dirs have different settings
- Library API: `BNotes::from_storage()` handles config loading

### Why keep I/O in CLI only?

Alternatives considered:
- **I/O trait** - Too much complexity for minimal interactive commands
- **Action/instruction pattern** - Indirect and harder to understand
- **CLI-only I/O** - Simplest, library stays pure data processing ✓

The library requires complete inputs. The CLI prompts for missing optional arguments before calling the library.

### Why domain logic in BNotes methods instead of separate modules?

Separate domain modules (`lib/notes.rs`, `lib/tasks.rs`) would just be pass-through functions to BNotes methods. This adds indirection without benefit. If shared utility functions are needed later, they can be extracted (YAGNI principle).

## Migration Guide for Existing Users

### For users with existing installations:

**Old config location:** `~/.config/bnotes/config.toml`

**Old format:**
```toml
notes_dir = "/Users/username/notes"
editor = "nvim"
template_dir = ".templates"

[periodic]
daily_template = "daily.md"
```

**Migration steps:**

1. **Create new CLI config** at `~/.config/bnotes/config.toml`:
```toml
notes_dir = "/Users/username/notes"
```

2. **Create library config** at `/Users/username/notes/.bnotes/config.toml`:
```toml
template_dir = ".templates"

[periodic]
daily_template = "daily.md"
weekly_template = "weekly.md"
quarterly_template = "quarterly.md"
```

3. **Set editor** via environment variable (add to `.bashrc`, `.zshrc`, etc.):
```bash
export EDITOR=nvim
```

Or just rely on the default `vim` if you use vim.

**Automatic migration:**

We can provide a `bnotes migrate-config` command that:
1. Reads old config
2. Creates new CLI config with just notes_dir
3. Creates new library config inside notes directory
4. Prints instructions about setting `$EDITOR`

## Future Considerations

### If you add more frontends (TUI, web API)

The library API is already suitable:
- Returns structured data
- No I/O assumptions
- Different frontends can format output differently
- Each frontend loads CLIConfig and creates storage

### If you need structured errors

Create a `BNotesError` enum:
```rust
pub enum BNotesError {
    NotFound(String),
    AlreadyExists(PathBuf),
    InvalidTemplate(String),
    GitError(String),
    // ...
}
```

Can be introduced later without breaking changes if library functions already return `Result`.

### If you need async operations

Current design is synchronous. If async is needed (network operations, etc.), the library can be refactored:
- Change `Storage` trait to async
- Add `async` to `BNotes` methods
- Use `tokio` or `async-std`

### If storage needs grow beyond files

The `Storage` trait can be extended:
- Add transactions
- Add batch operations
- Add metadata queries
- Keep the same interface, just add methods

### If you need multiple notes directories

The current design already supports this! Each notes directory:
- Has its own LibraryConfig
- Can have different templates and settings
- CLI just points to different notes_dir

## Non-Goals

- **Performance optimization** - Not changing algorithms, just structure
- **Public library** - This library is for bnotes CLI, not general use
- **Plugin system** - Not adding extensibility hooks
- **API stability guarantees** - Library and CLI live in same repo, can change together
- **Backward compatibility** - This is a breaking change requiring migration

## Success Metrics

- All existing CLI commands work unchanged from user perspective (after config migration)
- Core logic testable without filesystem I/O
- Library tests can run with MemoryStorage
- Clear separation: library has no `println!`, CLI has minimal logic
- Can add new commands by implementing library method + thin CLI wrapper
- Repository and library code use simple relative paths
- Notes directory is self-contained with its own config
- Different notes directories can have different configurations
