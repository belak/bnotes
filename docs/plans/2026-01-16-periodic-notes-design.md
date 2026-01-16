# Periodic Notes Design

## Overview

Add support for daily, weekly, and quarterly notes to bnotes, enabling a personal workflow around planning, tracking, and reviewing work over different time periods.

## Command Structure & Architecture

Three new top-level commands:
- `bnotes daily [date|prev|next|list] [--template <name>]`
- `bnotes weekly [date|prev|next|list] [--template <name>]`
- `bnotes quarterly [date|prev|next|list] [--template <name>]`

Each command works the same way:
- **No arguments**: Opens/creates the current period's note (with confirmation)
- **Date argument**: Opens/creates a specific period's note (`bnotes weekly 2026-01-15` opens week 2026-W03)
- **`prev`/`next`**: Navigate to previous/next period relative to current
- **`list`**: Show all notes for that period type
- **`--template <name>`**: Override configured template (references a file in `template_dir`)

**Note naming conventions:**
- Daily: `2026-01-16.md`
- Weekly: `2026-W03.md` (ISO week numbers, weeks start Monday)
- Quarterly: `2026-Q1.md`

**Date parsing for lookup:**
When you pass a date like `2026-01-15`, bnotes calculates which period it belongs to (week 2026-W03) and opens that note. This makes it natural to say "show me the week of January 15th" without needing to know it's week 3.

**Architecture approach:**
Since all three period types share the same command patterns, we'll implement a generic `PeriodicNote` abstraction with implementations for `Daily`, `Weekly`, and `Quarterly`. This avoids code duplication while keeping each type's specific logic (date calculations, naming) separate.

## Configuration

New configuration options in `config.toml`:

```toml
[default]
notes_dir = "~/notes"
editor = "vim"
template_dir = ".templates"

[periodic]
daily_template = "daily.md"      # default
weekly_template = "weekly.md"    # default
quarterly_template = "quarterly.md"  # default
```

**How it works:**
- All three options have default values as shown above
- If not specified in config, bnotes uses these defaults
- Template filenames are relative to `template_dir`
- The `--template` flag also references a filename in `template_dir`

**Fallback behavior:**
- If the template file doesn't exist (whether default or configured), bnotes creates a minimal note with just a title heading
- Example minimal weekly note:
  ```markdown
  # 2026-W03
  ```

This means you can start using periodic notes immediately without creating templates, but when you do create `.templates/weekly.md`, it will automatically be used.

## Template System

Periodic notes use the existing template system with these template variables:

**Existing variables (already supported):**
- `{{title}}` - The note title (e.g., "2026-W03")
- `{{date}}` - Current date in ISO format (e.g., "2026-01-16")
- `{{datetime}}` - Current datetime in ISO format (e.g., "2026-01-16T14:30:00")

**Example weekly template** (`.templates/weekly.md`):
```markdown
---
tags: [weekly]
created: {{datetime}}
---

# {{title}}

## Tasks this week

- [ ]

## Monday

## Tuesday

## Wednesday

## Thursday

## Friday

## Saturday

## Sunday

## Week Review

```

**Template rendering:**
- When creating a note, bnotes loads the template file and substitutes variables
- The `{{title}}` will be the period identifier (e.g., "2026-W03", "2026-01-16", "2026-Q1")
- All other variables work exactly like existing note templates

This reuses the existing template infrastructure, so no new template logic is needed.

## Note Creation & Confirmation Workflow

When you run a periodic command (e.g., `bnotes weekly`), here's the flow:

**If the note already exists:**
1. Open it directly in the configured editor
2. No confirmation needed

**If the note doesn't exist:**
1. Show confirmation prompt: `Week 2026-W03 doesn't exist. Create it? [Y/n]`
2. Wait for user input (default is Yes)
3. If yes (or just pressing Enter):
   - Load and render template (or create minimal note if template doesn't exist)
   - Write note file to disk
   - Open in editor
4. If no:
   - Exit without creating anything

**For `list`, `prev`, `next` subcommands:**
- `list`: Shows all existing periodic notes of that type (doesn't create anything)
- `prev`/`next`: Calculates the previous/next period, then follows the same creation workflow above

**Note location:**
All periodic notes are created in the root of `notes_dir`, alongside other notes. They're just regular notes with special naming conventions, so they work with all existing bnotes commands (search, tasks, edit, etc.).

## Date Calculations & Navigation

**ISO Week Calculations:**
- Weeks start on Monday (ISO 8601 standard)
- Week 1 is the first week with a Thursday in the new year
- When given a date, calculate which ISO week it belongs to
- Format: `YYYY-Wnn` (e.g., `2026-W03`)

**Daily Calculations:**
- Straightforward: `YYYY-MM-DD` format (e.g., `2026-01-16`)
- `prev`/`next` add/subtract 1 day

**Quarterly Calculations:**
- Q1: Jan-Mar, Q2: Apr-Jun, Q3: Jul-Sep, Q4: Oct-Dec
- When given a date, determine which quarter it falls in
- Format: `YYYY-Qn` (e.g., `2026-Q1`)
- `prev`/`next` add/subtract 3 months
- **Shortcuts**: Accept `q1`, `q2`, `q3`, `q4` (case-insensitive) as shortcuts for current year's quarters

**Date parsing for lookup:**
All three commands accept flexible date inputs:
- `bnotes weekly 2026-01-15` → looks up and opens `2026-W03`
- `bnotes daily "Jan 16"` → looks up `2026-01-16` (assumes current year if not specified)
- `bnotes quarterly 2026-03-15` → looks up `2026-Q1`
- `bnotes quarterly q1` → looks up `2026-Q1` (current year)
- `bnotes quarterly Q3` → looks up `2026-Q3` (current year)

Use a date parsing library (e.g., `chrono` in Rust) to handle various input formats.

**List command output:**
```
$ bnotes daily list
2026-01-13
2026-01-14
2026-01-16

$ bnotes weekly list
2026-W01    Dec 30 - Jan 05
2026-W02    Jan 06 - Jan 12
2026-W03    Jan 13 - Jan 19

$ bnotes quarterly list
2025-Q4     Oct - Dec
2026-Q1     Jan - Mar
```

For weekly, show date ranges with abbreviated months and padded days. For quarterly, show month ranges. For daily, just the date. Use consistent column alignment with single spaces around the dash separator.

## Integration with Existing Features

Since periodic notes are just regular markdown files with special naming conventions, they work seamlessly with all existing bnotes commands:

**Search:**
- `bnotes search "meeting notes"` includes periodic notes in results
- Results show the note title (e.g., "2026-W03") like any other note

**Tasks:**
- `bnotes tasks` finds all GFM checkboxes in periodic notes
- Tasks show which note they're in (e.g., "[ ] Review PR - 2026-W03")
- Your weekly tasks automatically appear in the global task list

**Edit:**
- `bnotes edit 2026-W03` opens the weekly note directly
- Works exactly like editing any other note by title

**Doctor:**
- Health checks apply to periodic notes too
- Can detect broken wiki links, missing tags, etc. in weekly/daily/quarterly notes

**Links:**
- Wiki links work normally: `[[2026-W03]]` links to your weekly note
- `bnotes note links 2026-W03` shows backlinks and outgoing links

**Graph:**
- Periodic notes appear in the link graph like any other note

**No special handling needed** - the only thing that makes periodic notes special is:
1. How they're created (via `bnotes daily/weekly/quarterly` commands)
2. Their naming convention (which makes them easy to find and navigate)
3. Optional template usage

This design keeps the codebase simple and ensures periodic notes are first-class citizens in your note system.
