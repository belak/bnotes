# bnotes

A personal note-taking CLI built around plain markdown files, with a focus on tasks and periodic notes for tracking work tasks.

As a note, this is a personal project built for my own use primarily via AI. It works for me but may have rough edges. Use at your own risk.

## Why

At it's core, bnotes is a CLI for working with a directory of markdown files. It adds some basic utilities like search, task tracking, note linking and git sync, roughly based on some of the ideas behind Obsidian. The original manual testing was even done by running bnotes against my personal Obsidian knowledge repo.

## Installation

```bash
git clone https://github.com/belak/bnotes
cd bnotes
cargo install --path .
```

I also personally recommend setting up a symlink or alias called `bn`, which makes working with bnotes even more convenient.

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

Default templates are embedded in the binary and work out of the box. Customize by copying templates to your `.bnotes/templates/` directory in your notes repo.

See [templates/README.md](templates/README.md) for details.

## Notes

Notes are markdown files with optional YAML frontmatter. Use `[[wiki links]]` to reference other notes.

Tasks are GitHub-flavored markdown checkboxes with optional urgency and priority:
- `- [ ] todo` - Basic task
- `- [ ] !!! urgent task` - Critical/now (also `!!` for soon, `!` for eventually)
- `- [ ] (A) important task` - Priority task (A, B, C, etc.)
- `- [ ] !! (B) soon and medium priority` - Both urgency and priority

Periodic notes (daily, weekly, quarterly) follow naming conventions like `2026-01-20.md`, `2026-W03.md`, `2026-Q1.md`.
