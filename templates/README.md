# Sample Templates

Copy these to your notes directory's `.templates/` folder.

## Usage

```bash
cp templates/* ~/notes/.templates/
```

Or copy individual templates as needed.

The `default.md` template is used automatically when creating notes without specifying a template.

## Periodic Notes

- `daily.md` - Daily notes with tasks and quick capture
- `weekly.md` - Weekly reviews with goals and retrospective
- `quarterly.md` - Quarterly planning with OKRs

## Other Templates

- `meeting.md` - Meeting notes with agenda and action items
- `project.md` - Project planning and tracking
- `person.md` - Contact and relationship notes
- `book.md` - Reading notes and key ideas
- `default.md` - Minimal template for general notes

## Variables

Templates support these variables:

- `{{title}}` - Note title
- `{{date}}` - Current date (YYYY-MM-DD)
- `{{datetime}}` - Current datetime (ISO 8601)
