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
