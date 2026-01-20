# bnotes

A personal note-taking CLI built around plain markdown files.

## Why

Most note-taking apps lock you into databases or proprietary formats. This is just a CLI for working with a directory of markdown files. Search across notes, track tasks with checkboxes, link notes together with wiki-style links, and optionally sync with git.

## Installation

```bash
git clone https://github.com/belak/bnotes
cd bnotes
cargo install --path .
```

## Setup

Set where your notes live:

```bash
export BNOTES_DIR=~/notes
```

Or run `bnotes init` to create `~/.config/bnotes/config.toml`.

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

Templates live in `.templates/` and support `{{title}}`, `{{date}}`, and `{{datetime}}` variables.

Periodic notes (daily, weekly, quarterly) follow naming conventions like `2026-01-20.md`, `2026-W03.md`, `2026-Q1.md`.
