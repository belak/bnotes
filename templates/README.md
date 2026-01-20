# Sample Templates

Copy these to your notes directory's `.templates/` folder.

## Usage

```bash
cp templates/* ~/notes/.templates/
```

Or copy individual templates as needed.

## Templates Used Automatically

These templates are used automatically by specific commands:

- `default.md` - Used by `bnotes edit` when creating regular notes
- `daily.md` - Used by `bnotes daily`
- `weekly.md` - Used by `bnotes weekly`
- `quarterly.md` - Used by `bnotes quarterly`

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
