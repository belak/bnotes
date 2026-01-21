# Auto-Update Timestamp Design

**Date**: 2026-01-21
**Status**: Approved

## Overview

Automatically update the `updated` field in note frontmatter when the note content is modified via the `edit` or periodic note commands.

## Requirements

- Detect when note content changes during editing
- Update the `updated` frontmatter field with current UTC timestamp
- Preserve all existing frontmatter fields (including unknown/custom fields)
- Make behavior configurable (enabled by default)
- Graceful error handling - don't fail edit if timestamp update fails
- Abstract change detection to allow future implementation changes

## Design

### 1. Change Detection

**Abstraction:**
Simple function that returns comparable state for change detection:

```rust
// In src/lib.rs

/// Capture the current state of a note file for change detection
/// Returns modification time that can be compared to detect changes
pub fn capture_note_state(path: &Path) -> Result<SystemTime> {
    let metadata = std::fs::metadata(path)?;
    metadata.modified().context("Failed to get modification time")
}
```

**Implementation:**
- Uses file modification time (mtime) for fast detection
- Returns `SystemTime` which implements `Eq` for comparison
- To change detection method later, just change return type and implementation

**Trade-offs:**
- ✅ Fast - no need to read file content
- ✅ Simple - one function, no traits needed
- ✅ Flexible - easy to swap to content hash or other method
- ⚠️ Updates timestamp if user saves without changes (acceptable - intentional save is an update)

### 2. Frontmatter Preservation

**Update Frontmatter struct in `src/note.rs`:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frontmatter {
    pub title: Option<String>,
    #[serde(default, deserialize_with = "deserialize_tags")]
    pub tags: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_datetime")]
    pub created: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "deserialize_datetime")]
    pub updated: Option<DateTime<Utc>>,

    // Preserve any unknown fields
    #[serde(flatten)]
    pub extra: serde_yaml::Value,
}
```

**Benefits:**
- ✅ Preserves all unknown frontmatter fields (author, status, custom metadata, etc.)
- ✅ Updates only the `updated` field
- ❌ Loses comments in frontmatter (YAML parser limitation - acceptable)
- ❌ May reformat spacing/field order (YAML serializer behavior - acceptable)

### 3. BNotes Method

**Add to `src/lib.rs`:**

```rust
impl BNotes {
    /// Update the 'updated' timestamp in a note's frontmatter
    pub fn update_note_timestamp(&self, note_path: &Path) -> Result<()> {
        // Read the note file
        let content = self.storage.read_to_string(note_path)?;

        // Parse to extract frontmatter and body
        let (mut frontmatter, body) = self.parse_frontmatter(&content)?;

        // Update the 'updated' field with current UTC timestamp
        frontmatter.updated = Some(Utc::now());

        // Serialize frontmatter back to YAML
        let yaml = serde_yaml::to_string(&frontmatter)?;

        // Reconstruct file: frontmatter + body
        let new_content = format!("---\n{}---\n{}", yaml, body);

        // Write back to file
        self.storage.write(note_path, &new_content)?;

        Ok(())
    }

    /// Parse frontmatter from note content
    /// Returns (frontmatter, body_content)
    fn parse_frontmatter(&self, content: &str) -> Result<(Frontmatter, String)> {
        // Similar to existing Note::extract_frontmatter logic
        // Extract YAML between --- markers
        // Return parsed frontmatter and remaining body
    }
}
```

**Key behaviors:**
- Updates only the `updated` field
- Preserves all other frontmatter fields (including unknown ones)
- Maintains body content exactly as-is
- Uses existing YAML serialization

### 4. Configuration

**Add field to `LibraryConfig` in `src/config.rs`:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryConfig {
    #[serde(default = "default_template_dir")]
    pub template_dir: PathBuf,
    #[serde(default)]
    pub periodic: PeriodicConfig,
    #[serde(default = "default_auto_update_timestamp")]
    pub auto_update_timestamp: bool,
}

fn default_auto_update_timestamp() -> bool {
    true  // Enabled by default
}
```

**Update Default implementation:**

```rust
impl Default for LibraryConfig {
    fn default() -> Self {
        Self {
            template_dir: default_template_dir(),
            periodic: PeriodicConfig::default(),
            auto_update_timestamp: default_auto_update_timestamp(),
        }
    }
}
```

**Config file example:**

```toml
# .bnotes/config.toml
auto_update_timestamp = false  # Disable automatic timestamp updates

[periodic]
daily_template = "daily.md"
```

**Config access:**

BNotes needs access to config. Update BNotes struct to store config:

```rust
pub struct BNotes {
    storage: Box<dyn Storage>,
    repo: Repository,
    config: LibraryConfig,
}

impl BNotes {
    pub fn config(&self) -> &LibraryConfig {
        &self.config
    }
}
```

### 5. CLI Integration

**Pattern for both `edit` and `periodic` commands:**

```rust
pub fn edit(notes_dir: &Path, title: &str, template_name: Option<String>) -> Result<()> {
    validate_notes_dir(notes_dir)?;
    let storage = Box::new(RealStorage::new(notes_dir.to_path_buf()));
    let bnotes = BNotes::with_defaults(storage);

    // ... existing code to determine note_path ...

    // Capture state before editing (if possible)
    let before_state = bnotes::capture_note_state(&note_path).ok();

    // Open editor
    let editor = std::env::var("EDITOR")
        .unwrap_or_else(|_| "vim".to_string());
    let status = Command::new(&editor)
        .arg(&note_path)
        .status()
        .with_context(|| format!("Failed to open editor: {}", editor))?;

    if !status.success() {
        anyhow::bail!("Editor exited with status: {}", status);
    }

    // Update timestamp if enabled and file changed
    if bnotes.config().auto_update_timestamp {
        if let Some(before) = before_state {
            if let Ok(after) = bnotes::capture_note_state(&note_path) {
                if before != after {
                    if let Err(e) = bnotes.update_note_timestamp(&note_path) {
                        eprintln!("Warning: Failed to update timestamp: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}
```

**Apply same pattern to:**
- `edit()` function in `src/cli/commands.rs`
- `periodic()` function in `src/cli/commands.rs`

### 6. Error Handling

**Strategy: Fail gracefully, prioritize user edits**

1. **If state capture fails before editing:**
   - Proceed with editing anyway
   - Skip timestamp update (can't detect changes)
   - No error message (editing still works)

2. **If timestamp update fails:**
   - Print warning to stderr: `"Warning: Failed to update timestamp: {error}"`
   - Don't fail the edit command
   - User's edits are preserved

3. **If note doesn't have frontmatter:**
   - Skip timestamp update silently
   - Don't force frontmatter on notes without it

4. **If note has frontmatter but no `updated` field:**
   - Add the `updated` field with current timestamp
   - Preserve all other fields via `#[serde(flatten)]`

**Error flow:**

```
User edits note
    ├─ State capture fails? → Continue editing, skip timestamp
    ├─ Editor fails? → Abort (existing behavior)
    ├─ Config disabled? → Skip timestamp
    ├─ No changes detected? → Skip timestamp
    └─ Timestamp update fails? → Warn user, continue
```

**Principle:** Never fail an edit command due to timestamp issues.

## Edge Cases

| Case | Behavior |
|------|----------|
| Note created during edit | State comparison detects change, timestamp added |
| Multiple rapid edits | Each edit updates timestamp (correct) |
| Editor crashes/killed | No state change detected, no timestamp update (correct) |
| Note without frontmatter | Skip timestamp update silently |
| Config disabled | Skip all timestamp logic |
| File permissions error | Warning printed, edit succeeds |

## Implementation Plan

### Phase 1: Core Infrastructure
1. Add `extra` field to `Frontmatter` struct with `#[serde(flatten)]`
2. Implement `capture_note_state()` function
3. Implement `BNotes::update_note_timestamp()` method
4. Add unit tests for frontmatter preservation

### Phase 2: Configuration
1. Add `auto_update_timestamp` field to `LibraryConfig`
2. Update `LibraryConfig::default()`
3. Store config in `BNotes` struct
4. Add `config()` accessor method

### Phase 3: CLI Integration
1. Update `edit()` command with timestamp logic
2. Update `periodic()` command with timestamp logic
3. Test with various scenarios (new notes, existing notes, no frontmatter)

### Phase 4: Testing
1. Manual testing:
   - Edit note, verify `updated` field changes
   - Edit without saving, verify no change
   - Edit note without frontmatter, verify no error
   - Disable config, verify no updates
2. Integration tests if feasible

## Testing

### Manual Verification

```bash
# Test basic functionality
echo "---
title: Test Note
created: 2026-01-20T00:00:00Z
---
Content" > test-note.md

bnotes edit "Test Note"
# Make changes, save
# Verify 'updated' field appears with current timestamp

# Test config disable
echo "auto_update_timestamp = false" >> .bnotes/config.toml
bnotes edit "Test Note"
# Make changes, save
# Verify 'updated' field does NOT change

# Test note without frontmatter
echo "Just content" > no-frontmatter.md
bnotes edit "no-frontmatter"
# Make changes, save
# Verify no error, no frontmatter added

# Test unknown field preservation
echo "---
title: Test Note
custom_field: custom_value
---
Content" > custom-note.md

bnotes edit "Test Note"
# Make changes, save
# Verify custom_field is preserved
```

### Edge Case Testing

- Edit file, don't save → no timestamp update
- Edit file, save with no changes → timestamp updates (acceptable)
- Permission denied on write → warning printed, edit succeeds
- Multiple rapid edits → timestamp updates each time
- Periodic notes → timestamp updates work

## Future Enhancements

- Add `created` timestamp on note creation (if not set)
- Track `last_accessed` timestamp for note views
- Support content-based change detection (hash comparison)
- Add config option for timestamp format
- Batch update timestamps: `bnotes note update-timestamps --all`

## Design Rationale

**Why modification time over content comparison:**
- Much faster (no need to read entire file)
- Simple implementation
- Acceptable trade-off (intentional save = update)
- Easy to swap to content-based later

**Why preserve unknown fields:**
- Users may add custom frontmatter (author, status, tags, etc.)
- Forward compatibility with future frontmatter fields
- Safe upgrades - won't lose user data

**Why enabled by default:**
- Most users expect automatic timestamp tracking
- Common pattern in note-taking tools
- Easy to disable via config

**Why graceful error handling:**
- User's edits are most important
- Timestamp is metadata - shouldn't block editing
- Better UX than failing edit on timestamp errors

**Why config in library layer:**
- Each notes directory can have different settings
- Portable - config travels with notes
- Consistent with existing config structure
