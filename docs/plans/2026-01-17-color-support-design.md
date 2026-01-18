# Color Support Design

## Overview

Add minimal, purposeful colorization to bnotes commands using the termcolor crate. Replace existing ANSI escape codes with proper terminal detection that respects NO_COLOR and handles piped output correctly.

## Philosophy

Color highlights structure and status, not content. Note titles, task text, and note content remain in default terminal colors for maximum readability.

## Core Color Palette

**Semantic colors:**
- **Red + Bold** - Errors and broken things (ERROR labels, broken links, duplicates)
- **Yellow + Bold** - Warnings (WARNING labels, missing tags, orphans)
- **Green** - Success messages (sync complete, created, all checks passed)
- **Cyan** - Structural highlights (note paths, task IDs, arrows, link counts)
- **Dim** - Secondary context (snippets, metadata)

## Command Colorization

### Search (`bnotes search`)
- Note path: cyan
- Matched keywords in snippets: cyan
- Content snippets: dim
- Match count summary: default

### Doctor (`bnotes doctor`)
- "ERROR:" prefix: red + bold
- "WARNING:" prefix: yellow + bold
- Broken link items, duplicate paths: default
- Success message "All checks passed!": green
- Issue count summary: default

### Task Commands (`bnotes task list/show`)
- Task ID: cyan
- Checkbox `[x]`: green
- Checkbox `[ ]`: default
- Task text: default
- "from [note]" source: dim
- Count summary: default

### Git Commands (`bnotes sync/pull`)
- Success messages: green
- Error messages: red
- Change counts: default

### Note Commands (`bnotes note list/links/graph`)
- Note titles: default
- Tags in brackets: default
- Arrows (-> and <-): cyan
- Link/note counts: cyan
- "Total: X notes": count in cyan

## Implementation

### CLI Flag

Add `--color` flag to control color output:

```rust
#[arg(long, default_value = "auto", value_name = "WHEN")]
color: ColorChoice,  // ColorChoice implements FromStr
```

**Values:**
- `auto` (default): Use colors if stdout is a terminal and NO_COLOR is not set
- `always`: Force colors even when piped/redirected
- `never`: Disable colors even in terminal

### Color Module (`src/cli/colors.rs`)

```rust
use std::io::IsTerminal;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream};

/// Create a StandardStream with appropriate color support
///
/// Follows the termcolor recommended pattern:
/// - Respects user preference from --color flag
/// - When Auto, checks IsTerminal to disable for pipes/redirects
/// - ColorChoice::Auto also respects NO_COLOR environment variable
pub fn create_stdout(preference: ColorChoice) -> StandardStream {
    let choice = if preference == ColorChoice::Auto && !std::io::stdout().is_terminal() {
        ColorChoice::Never
    } else {
        preference
    };
    StandardStream::stdout(choice)
}

/// Error color: red + bold
pub fn error() -> ColorSpec {
    let mut spec = ColorSpec::new();
    spec.set_fg(Some(Color::Red)).set_bold(true);
    spec
}

/// Warning color: yellow + bold
pub fn warning() -> ColorSpec {
    let mut spec = ColorSpec::new();
    spec.set_fg(Some(Color::Yellow)).set_bold(true);
    spec
}

/// Success color: green
pub fn success() -> ColorSpec {
    let mut spec = ColorSpec::new();
    spec.set_fg(Some(Color::Green));
    spec
}

/// Highlight color: cyan (for structure)
pub fn highlight() -> ColorSpec {
    let mut spec = ColorSpec::new();
    spec.set_fg(Some(Color::Cyan));
    spec
}

/// Dim color: gray (for secondary info)
pub fn dim() -> ColorSpec {
    let mut spec = ColorSpec::new();
    spec.set_dimmed(true);
    spec
}
```

### Usage Pattern

```rust
use std::io::Write;

// In command function that receives color preference
pub fn search(notes_dir: &Path, query: &str, color: ColorChoice) -> Result<()> {
    let mut stdout = colors::create_stdout(color);

    // Normal text
    writeln!(stdout, "Normal text")?;

    // Colored text
    stdout.set_color(&colors::error())?;
    writeln!(stdout, "Error message")?;
    stdout.reset()?;

    // Back to normal
    writeln!(stdout, "More text")?;

    Ok(())
}
```

### Migration Strategy

1. Add `--color` flag to CLI args
2. Create `src/cli/colors.rs` module
3. Add `mod colors;` to `src/cli/mod.rs`
4. Update each command function to:
   - Accept `color: ColorChoice` parameter
   - Create `stdout` with `colors::create_stdout(color)`
   - Replace `println!` with `writeln!(stdout, ...)`
   - Remove raw ANSI escape codes (`\x1b[...m`)
   - Add color using `stdout.set_color(&colors::xxx())` and `stdout.reset()`
5. Thread `color` parameter from CLI args through to command functions

## Terminal Behavior

**Color output when:**
- `--color=always`: Always use colors
- `--color=auto` (default) AND stdout is a terminal AND NO_COLOR is not set

**Plain output when:**
- `--color=never`: Never use colors
- `--color=auto` AND stdout is piped: `bnotes search foo | cat`
- `--color=auto` AND stdout is redirected: `bnotes search foo > file.txt`
- `--color=auto` AND NO_COLOR is set: `NO_COLOR=1 bnotes search foo`

## Testing

**Manual verification:**
- Run each command in terminal (should show colors)
- Test `--color=always` forces colors when piped
- Test `--color=never` disables colors in terminal
- Pipe output: `bnotes search test | cat` (no colors with auto)
- Redirect output: `bnotes search test > out.txt && cat out.txt` (no colors with auto)
- Disable colors: `NO_COLOR=1 bnotes search test` (no colors with auto)

**Visual checks:**
- Search with matches
- Doctor with errors and warnings
- Task list with completed/incomplete
- Note links with arrows
- Git sync success messages

## Trade-offs

**Chosen approach:**
- Minimal color for clarity, not decoration
- Helper functions returning ColorSpec (clean API)
- Consistent use of StandardStream throughout (single output path)
- Explicit --color flag following termcolor recommendations
- IsTerminal check when ColorChoice::Auto

**Alternatives considered:**
- Two-color arrows for direction (cyan out, green in): Rejected as more complex than needed
- ColorWriter wrapper class: Rejected as clunky API
- Helper functions with automatic color/reset: Rejected as less flexible
- Automatic terminal detection without flag: Rejected, termcolor docs recommend the flag pattern
