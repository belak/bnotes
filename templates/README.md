# Sample Templates

Default templates (`default.md`, `daily.md`, `weekly.md`, `quarterly.md`) are embedded in the binary and work out of the box. Copy them to your `.bnotes/templates/` directory to customize.

## Usage

```bash
mkdir -p ~/notes/.bnotes/templates
cp templates/* ~/notes/.bnotes/templates/
```

Or copy individual templates as needed.

## Templates Used Automatically

These templates are embedded in bnotes and used automatically:

- `default.md` - Used by `bnotes edit` when creating regular notes
- `daily.md` - Used by `bnotes daily`
- `weekly.md` - Used by `bnotes weekly`
- `quarterly.md` - Used by `bnotes quarterly`

Copy them to `.bnotes/templates/` to customize. Your versions will override the embedded defaults.

## Other Templates

Use these with the `--template` flag:

- `meeting.md` - Meeting notes with agenda and action items
- `project.md` - Project planning and tracking
- `person.md` - Contact and relationship notes
- `book.md` - Reading notes and key ideas

## Variables

Templates support these variables:

- `{{title}}` - Note title
- `{{date}}` - Current date (YYYY-MM-DD)
- `{{datetime}}` - Current datetime (ISO 8601)
- `{{migrated_tasks}}` - Migrated tasks from previous period (weekly notes only)

### Task Migration (Weekly Notes)

When creating a new weekly note for the current week, bnotes will prompt to migrate uncompleted tasks from the most recent previous weekly note. Use the `{{migrated_tasks}}` variable in your weekly template to control where migrated tasks appear:

```markdown
---
tags: [weekly]
created: {{datetime}}
---

# {{title}}

{{migrated_tasks}}

## Goals
```

If you don't include `{{migrated_tasks}}`, the variable will be replaced with an empty string and no tasks will be inserted. Tasks in the previous note will be marked with `[>]` to indicate they've been migrated.
