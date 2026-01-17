# Module Structure Simplification

**Date:** 2026-01-17
**Status:** Approved

## Problem

The current module structure is over-engineered for a ~3600 line codebase:
- Too many small files (util.rs is 4 lines, many command files are 30-50 lines)
- Commands split between main.rs and commands/ directory
- Library has 7 separate modules when the domain is focused enough for fewer
- Unclear boundaries between modules
- Excessive navigation required to understand related functionality

## Solution

Consolidate into a simpler, more navigable structure with clearer boundaries.

### CLI Side: From 12 files to 2 files

**Before:**
```
src/
├── main.rs (292 lines) - CLI parsing + routing
├── cli_config.rs (69 lines)
├── util.rs (4 lines)
├── git.rs (313 lines)
└── commands/
    ├── mod.rs (9 lines)
    ├── new.rs (37 lines)
    ├── edit.rs (38 lines)
    ├── search.rs (43 lines)
    ├── note.rs (187 lines)
    ├── task.rs (53 lines)
    ├── doctor.rs (88 lines)
    ├── sync.rs (88 lines)
    ├── init.rs (135 lines)
    └── periodic.rs (119 lines)
```

**After:**
```
src/
├── main.rs (~820 lines) - All CLI logic
│   ├── CLIConfig struct + impl (moved from cli_config.rs)
│   ├── CLI argument structs
│   ├── slug_from_title() (moved from util.rs)
│   ├── All command implementations (moved from commands/*)
│   └── main()
└── git.rs (313 lines) - Git operations (unchanged)
```

**Rationale:**
- Commands are thin adapters around library calls - no need for separate files
- One place to see all CLI logic without jumping between files
- CLIConfig stays as a proper type with methods, just in main.rs
- git.rs stays separate: 313 lines of focused git-specific logic

### Library Side: From 7 modules to 5 modules

**Before:**
```
src/
├── lib.rs (494 lines) - BNotes facade
├── config.rs (176 lines)
├── storage.rs (213 lines)
├── periodic.rs (335 lines)
├── repository.rs (327 lines)
├── task.rs (134 lines)
├── link.rs (246 lines)
└── health.rs (224 lines)
```

**After:**
```
src/
├── lib.rs (494 lines) - BNotes facade (unchanged)
├── config.rs (176 lines) - LibraryConfig (unchanged)
├── storage.rs (213 lines) - Storage abstraction (unchanged)
├── periodic.rs (335 lines) - Date-based notes (unchanged)
├── note.rs (~480 lines) - Core domain types
│   ├── Frontmatter struct
│   ├── Note struct + parsing
│   ├── Task struct + extraction
│   └── Template rendering
└── repository.rs (~450 lines) - Operations on note collections
    ├── Repository struct + file operations
    ├── LinkGraph + wiki link extraction
    └── HealthReport + health checks
```

**Rationale:**
- **note.rs**: Groups core domain types (Note, Task, Frontmatter) together
  - Task is extracted from Note content - natural fit
  - Template rendering operates on Note content
- **repository.rs**: Operations on note collections
  - LinkGraph analyzes relationships between Notes
  - HealthReport checks quality across Note collections
  - All tightly coupled around collections of Notes
- **Kept separate**: config, storage, periodic have clear boundaries
  - config: Configuration concern
  - storage: Infrastructure boundary (trait + impls)
  - periodic: Date-based domain logic, independent from regular notes

## Implementation Plan

### Phase 1: CLI Consolidation

1. Move CLIConfig from cli_config.rs to main.rs (keep as struct + impl)
2. Inline slug_from_title() from util.rs into main.rs
3. Move all command implementations from commands/* into main.rs
   - Pattern: `fn cmd_search(config: Option<PathBuf>, query: &str) -> Result<()>`
   - Keep as private functions
4. Update module declarations in main.rs
5. Delete: cli_config.rs, util.rs, commands/ directory
6. Verify: `cargo build && cargo test`

### Phase 2: Library Consolidation

1. Create note.rs with Note + Task + Frontmatter
   - Move Note, Frontmatter from repository.rs
   - Move Task from task.rs
   - Move template rendering helpers
2. Update repository.rs
   - Add LinkGraph from link.rs
   - Add HealthReport from health.rs
   - Update imports to use note::Note, note::Task
3. Update lib.rs module declarations and imports
4. Update all internal imports in consolidated files
5. Delete: task.rs, link.rs, health.rs
6. Verify: `cargo build && cargo test --lib`

### Safety Measures

- Use `git mv` where applicable to preserve history
- Build and test after each phase
- Keep all existing tests unchanged
- No logic changes - pure code movement
- Verify zero clippy warnings at end

## File Organization Standards

**repository.rs structure:**
```rust
// 1. Repository struct + impl
// 2. LinkGraph struct + impl + helper functions
// 3. HealthReport struct + check_health function
// 4. #[cfg(test)] mod tests
```

**note.rs structure:**
```rust
// 1. Frontmatter struct
// 2. Note struct + impl
// 3. Task struct + impl
// 4. Helper functions (parsing, template rendering)
// 5. #[cfg(test)] mod tests
```

## Expected Outcome

- **Before**: 20 source files, frequent context switching
- **After**: 7 source files, related code together
- Easier navigation: "Want to see Note logic? Open note.rs"
- Clearer boundaries: domain types vs. operations vs. infrastructure
- Reduced cognitive overhead: less jumping between files
- Same test coverage, same behavior, cleaner structure
