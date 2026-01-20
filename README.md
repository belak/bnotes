# bnotes

A personal note-taking CLI built around plain markdown files.

## Why

At it's core, bnotes is a CLI for working with a directory of markdown files. It adds some basic utilities like search, task tracking, note linking and git sync, roughly based on some of the ideas behind Obsidian.

## Installation

```bash
git clone https://github.com/belak/bnotes
cd bnotes
cargo install --path .
```

## Setup

By default your notes will live in $XDG_DATA_HOME/bnotes (or ~/.local/share/bnotes).

Set where your notes live:

```bash
export BNOTES_DIR=~/notes
```

## Usage

```bash
# Create or edit a note
bnotes edit "Project Ideas"

# Search across all notes
bnotes search "meeting notes"

# List open tasks
bnotes tasks

# Sync with git
bnotes sync
```

Run `bnotes --help` for all commands.

## Templates

Default templates for daily, weekly, quarterly, and regular notes are embedded in the binary. You can customize them by copying templates to your `.templates/` directory - your versions will override the defaults.

Sample templates are in the `templates/` directory. Includes templates for meetings, projects, books, and contacts. Templates support `{{title}}`, `{{date}}`, and `{{datetime}}` variables.

Templates used automatically:
- `default.md` - Regular notes created with `bnotes edit`
- `daily.md` - Daily notes created with `bnotes daily`
- `weekly.md` - Weekly notes created with `bnotes weekly`
- `quarterly.md` - Quarterly notes created with `bnotes quarterly`

## Notes

Notes are markdown files with optional YAML frontmatter. Use `[[wiki links]]` to reference other notes. Tasks are GitHub-flavored markdown checkboxes (`- [ ] todo`).

Periodic notes (daily, weekly, quarterly) follow naming conventions like `2026-01-20.md`, `2026-W03.md`, `2026-Q1.md`.
