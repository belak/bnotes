# Enhanced Search Design

## Overview

Improve search results with heading context breadcrumbs and highlighted matched words, making it easier to understand where matches occur in document structure.

## Current Behavior

```
Project Ideas
  ... review the project budget ...

Found 1 match
```

**Limitations:**
- No heading context for where match occurs in document structure
- Matched words not highlighted in output
- Shows only first match per note
- Can cut snippets mid-word

## New Behavior

```
Project Ideas
  [# Planning > ## Q1 Goals]
  ... review the **project** budget ...

  [# Resources > ## Team]
  ... the **project** team meets weekly ...

  (2 matches shown, 1 more in this note)
```

**Improvements:**
- Heading breadcrumb trail shows document structure context
- Matched words highlighted in bold (stands out from dim snippet)
- Multiple matches per note (default 3, configurable)
- Smart snippet extraction at word boundaries

## Match Types

### Title Match

Highlight matched word in title, no snippet:

```
**Project** Ideas [planning, notes]
```

### Tag Match

Highlight matched tag, no snippet:

```
Meeting Notes [planning, **project**, daily]
```

### Content Match

Show breadcrumb + snippet with highlighted matches:

```
Meeting Notes
  [# Planning > ## Q1 Goals]
  ... review the **project** budget ...
```

## Design Details

### Breadcrumb Format

**Format:** `[# Heading > ## Subheading > ### Details]`

- Keep heading markers (`#`, `##`, etc.) to show document structure
- Use `>` separator for hierarchy
- Wrap in brackets `[ ]` for visual separation from content
- Show full path regardless of depth (no truncation)

**Special case - no heading context:**

```
[Document Start]
... preamble text with **match** ...
```

### Color Scheme

- **Note title:** cyan (existing highlight color)
- **Matched words in title/tags:** default + bold
- **Breadcrumbs:** dim
- **Snippet text:** dim
- **Matched words in snippet:** default + bold (pops from dim background)
- **Truncation message:** dim

### Match Display Limits

**Default:** Show first 3 matches per note

**Flag:** `--limit N` to override default

**Truncation message:**
```
(3 matches shown, 2 more in this note)
```

**Counting:**
- Each occurrence counts as separate match
- Multiple matches in same snippet count individually

### Snippet Extraction

**Context size:** ~60 characters before and after match

**Smart boundaries:**
- Trim to word boundaries (don't cut mid-word)
- Show `...` prefix/suffix only when actually truncated
- Collapse multiple whitespace to single space

**Algorithm:**
```rust
fn extract_snippet(content: &str, match_pos: usize, query_len: usize) -> String {
    const CONTEXT_CHARS: usize = 60;

    // Find boundaries
    let start = match_pos.saturating_sub(CONTEXT_CHARS);
    let end = (match_pos + query_len + CONTEXT_CHARS).min(content.len());

    let mut snippet = &content[start..end];

    // Trim to word boundaries
    if start > 0 {
        if let Some(space_pos) = snippet.find(char::is_whitespace) {
            snippet = &snippet[space_pos..].trim_start();
        }
    }
    if end < content.len() {
        if let Some(space_pos) = snippet.rfind(char::is_whitespace) {
            snippet = &snippet[..space_pos].trim_end();
        }
    }

    // Add ellipsis indicators
    let prefix = if start > 0 { "..." } else { "" };
    let suffix = if end < content.len() { "..." } else { "" };

    format!("{}{}{}", prefix, snippet, suffix)
}
```

### Heading Breadcrumb Tracking

**Parse markdown once per note to build:**
1. Match locations (character positions where query appears)
2. Heading map (which headings are active at each position)

**Algorithm:**
```rust
// Pseudocode
for each note:
  1. Parse markdown with pulldown-cmark
  2. Track heading stack as we encounter Start(Heading)/End(Heading)
  3. Track character position as we process Text events
  4. When Text contains match:
     - Record: position, heading_breadcrumb, surrounding_text
  5. Also check title/tags for matches
  6. Return structured matches (title/tag/content variants)
```

**Heading stack example:**
```
# Main          → stack: ["# Main"]
## Section      → stack: ["# Main", "## Section"]
### Subsection  → stack: ["# Main", "## Section", "### Subsection"]
## Other        → stack: ["# Main", "## Other"]
# Another       → stack: ["# Another"]
```

## Implementation

### Data Structures

```rust
pub struct SearchMatch {
    pub note: Note,
    pub matches: Vec<MatchLocation>,
}

pub enum MatchLocation {
    Title {
        position: usize
    },
    Tag {
        tag: String
    },
    Content {
        breadcrumb: Vec<String>,  // ["# Main", "## Section"]
        snippet: String,
        match_positions: Vec<(usize, usize)>  // relative to snippet
    },
}
```

### Files to Modify

**`src/repository.rs`:**
- Update `search()` method to return `Vec<SearchMatch>` instead of `Vec<Note>`
- Parse markdown to build heading breadcrumbs
- Track all match locations (title, tags, content)
- Extract snippets with proper boundaries

**`src/cli/commands.rs`:**
- Add `limit: usize` parameter to `search()` function
- Render matches with breadcrumbs
- Highlight matched words in output
- Handle title/tag/content match variants
- Display truncation message when applicable

**`src/main.rs`:**
- Add `--limit` flag to Search command:
  ```rust
  Search {
      query: String,
      #[arg(long, default_value = "3")]
      limit: usize,
  }
  ```

### Edge Cases

**Multiple matches in same snippet:**
```
... the project team reviewed project status ...
    ^^^^^^^                    ^^^^^^^
```
- Highlight both occurrences
- Count as 2 separate matches for limit

**Match before first heading:**
```
[Document Start]
... preamble text with **match** ...
```

**Deep nesting:**
```
[# Main > ## Section > ### Subsection > #### Details > ##### Deep]
```
- Show full path (no depth limit)
- Let terminal wrapping handle long breadcrumbs

**Case sensitivity:**
- Search remains case-insensitive
- Highlight preserves original case in snippet

**Empty notes or only frontmatter:**
- Title/tag matches work normally
- No content matches (no snippets shown)

### Performance Considerations

**Impact:**
- Parsing markdown adds overhead vs simple string search
- Acceptable: search is already I/O bound (reading all notes from disk)

**Optimization (if needed):**
- Cache parsed markdown structure per note
- Parse lazily (only when match found)

### Testing

**Unit tests:**
- Snippet extraction at word boundaries
- Breadcrumb building from heading events
- Match position tracking

**Integration tests:**
- Search note with multiple heading levels
- Match before first heading
- Multiple matches in same section
- Title/tag/content match variants

**Manual verification:**
- Search with `--limit` flag variations
- Search with `--color=never` (brackets provide separation)
- Long breadcrumbs with deep nesting
- Notes without headings

## Example Output

**Multiple content matches:**
```
$ bnotes search project --limit 2

Project Ideas
  [# Planning > ## Q1 Goals]
  ... review the **project** budget and timeline ...

  [# Planning > ## Resources]
  ... allocate team members to the **project** ...

  (2 matches shown, 3 more in this note)

Meeting Notes
  [Document Start]
  ... discuss **project** kickoff meeting ...

  [# Action Items]
  ... finalize **project** charter ...

  (2 matches shown)

Found 2 notes with matches
```

**Title and tag matches:**
```
$ bnotes search planning

Planning Workshop [**planning**, team]

Project Ideas
  [# **Planning** > ## Q1 Goals]
  ... quarterly planning session ...

Found 2 notes with matches
```

## Trade-offs

**Chosen approach:**
- Parse markdown for accurate heading context
- Default 3 matches balances detail vs clutter
- Bold highlighting (vs color) keeps content readable
- Full breadcrumb paths (vs depth limit) provide complete context

**Alternatives considered:**
- **Show all matches:** Rejected as too verbose for notes with many matches
- **Cyan highlighting for matches:** Rejected as overuses structural color
- **Limit breadcrumb depth:** Rejected as context loss outweighs screen space savings
- **Strip heading markers from breadcrumbs:** Rejected as loses structural information
