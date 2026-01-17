# bnotes

A personal note-taking CLI with task management, wiki-style linking, and git synchronization.

## What is bnotes?

bnotes is belak's personal command-line tool for managing notes and finding what you need. It's designed for people who want:

- **Plain text notes** - Your notes are just markdown files you can edit with any tool
- **Fast CLI access** - Create, search, and manage notes from your terminal
- **Task tracking** - Use GitHub-flavored markdown checkboxes for todos across all your notes
- **Knowledge linking** - Connect notes with wiki-style `[[links]]`
- **Git sync** - Keep notes synced across devices using git

Unlike heavy note-taking apps, bnotes is simple, fast, and keeps your data in plain markdown files you control.

## Features

- Search across all notes
- Basic task discovery with GFM task lists
- Wiki-style linking between notes (`[[Note Title]]`)
- Note templates
- Git synchronization

## Installation

Build from source with Rust:

```bash
git clone https://github.com/belak/bnotes
cd bnotes
cargo build --release
cargo install --path .
```

## Setup

Initialize bnotes configuration:

```bash
bnotes init
```

This creates `~/.config/bnotes/config.toml` and prompts for your notes directory location. The notes directory will be created if it doesn't exist.

Optionally, initialize git for syncing:

```bash
cd ~/notes  # or wherever your notes directory is
git init
git remote add origin <your-repo-url>
git push -u origin main
```

## Configuration

The config file is located at `~/.config/bnotes/config.toml` (or `$BNOTES_CONFIG` if set):

| section  | option             | default     | description                                                               |
| -------- | ------------------ | ----------- | ------------------------------------------------------------------------- |
| default  | notes_dir          | none        | Path to your notes directory                                              |
| default  | editor             | $EDITOR/vim | Which editor to open with bnotes edit                                     |
| default  | template_dir       | .templates  | Path to the templates directory, either absolute or relative to notes_dir |
| periodic | daily_template     | daily.md    | Template to use for daily notes                                           |
| periodic | weekly_template    | weekly.md   | Template to use for weekly notes                                          |
| periodic | quarterly_template | quarterly.md | Template to use for quarterly notes                                      |


## Note Format

Notes are markdown files with optional YAML frontmatter.

## Templates

Templates are markdown files in your configured template directory (`.templates/` by default). They support the following variables:

- `{{title}}` - The note title
- `{{date}}` - Current date (ISO format)
- `{{datetime}}` - Current datetime (ISO format)

Example template (`.templates/daily.md`):

```markdown
---
tags: [daily]
created: {{datetime}}
updated: {{datetime}}
---

# {{title}}

## What I did today

## Tasks
- [ ]

## Notes
```

## Periodic Notes

Create and manage daily, weekly, and quarterly notes:

```bash
# Open today's daily note (creates if needed)
bnotes daily

# Open specific date's note
bnotes daily 2026-01-16

# Open current week's note
bnotes weekly

# Open week containing a specific date
bnotes weekly 2026-01-15

# Open current quarter
bnotes quarterly

# Use quarter shortcuts
bnotes quarterly q1

# Navigate to previous/next periods
bnotes weekly prev
bnotes weekly next

# List all periodic notes
bnotes daily list
bnotes weekly list
bnotes quarterly list

# Override template
bnotes weekly --template custom-weekly
```

Configure templates in `config.toml`:

```toml
[periodic]
daily_template = "daily.md"
weekly_template = "weekly.md"
quarterly_template = "quarterly.md"
```

Periodic notes are regular markdown files with special naming:
- Daily: `2026-01-16.md`
- Weekly: `2026-W03.md`
- Quarterly: `2026-Q1.md`

## Health Checks

Check for issues in your notes:
```bash
bnotes doctor
```

This finds:
- Broken wiki links
- Notes without tags
- Notes with duplicate titles
- Orphaned notes (no links or tags)

## Commands

- `bnotes search <query>` - Full-text search across all notes
- `bnotes new [title]` - Create a new note
- `bnotes edit <title>` - Open a note in your editor
- `bnotes tasks` - List open tasks (shortcut)
- `bnotes init` - Initialize bnotes configuration
- `bnotes doctor` - Check for issues in notes
- `bnotes sync` - Sync with git remote (commit, pull, push)
- `bnotes pull` - Pull changes from git remote
- `bnotes daily [date|prev|next|list]` - Manage daily notes
- `bnotes weekly [date|prev|next|list]` - Manage weekly notes
- `bnotes quarterly [date|prev|next|list]` - Manage quarterly notes
- `bnotes note list` - List all notes
- `bnotes note show <title>` - Display a note
- `bnotes note links <title>` - Show links to/from a note
- `bnotes note graph` - Show the entire link graph
- `bnotes task list` - List tasks with filtering
- `bnotes task show <task-id>` - Show a task with context

Run `bnotes --help` or `bnotes <command> --help` for more details.
