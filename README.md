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

## Notes

Notes are markdown files with optional YAML frontmatter. Use `[[wiki links]]` to reference other notes. Tasks are GitHub-flavored markdown checkboxes (`- [ ] todo`).

Templates live in `.templates/` and support `{{title}}`, `{{date}}`, and `{{datetime}}` variables. Sample templates are in the `templates/` directory.

Periodic notes (daily, weekly, quarterly) follow naming conventions like `2026-01-20.md`, `2026-W03.md`, `2026-Q1.md`.
