# Unified Edit Command Design

**Date:** 2026-01-20
**Status:** Approved

## Summary

Combine the `edit` and `new` commands into a single `edit` command that handles both creating new notes and editing existing ones, simplifying the workflow.

## Current Behavior

- **`new` command**: Creates a note but doesn't open it in an editor
- **`edit` command**: Opens an existing note but fails if the note doesn't exist

To create and start editing a new note requires two separate commands.

## Proposed Behavior

The `edit` command becomes the single entry point for both creating and editing notes:

### For Existing Notes
- `bnotes edit "My Note"` - Opens the note in your editor (unchanged)
- Handles multiple matches by showing all options and asking for clarification (unchanged)

### For Non-Existent Notes
- `bnotes edit "New Note"` - Prompts: "Note doesn't exist. Create it? [Y/n]"
- If yes: Creates the note (with optional template) and opens it in editor
- If no: Exits without doing anything

### Template Support
- `bnotes edit "New Note" --template meeting` - Creates note from template if it doesn't exist
- Template is only used during creation; ignored if note already exists

## Implementation

### 1. Modify `edit` Function
Location: `src/cli/commands.rs:262`

Enhanced logic:
1. Try to find existing note (current behavior)
2. If not found, prompt user to create it
3. If user confirms, call `bnotes.create_note()` with the title and optional template
4. Open in editor (reuse `launch_editor()` helper from line 923)

### 2. Update CLI Arguments
Location: `src/main.rs`

Changes:
- Remove `Commands::New` variant (lines 62-70)
- Add optional `--template` flag to `Commands::Edit`:
  ```rust
  Edit {
      /// Note title
      title: String,

      /// Template to use if creating a new note
      #[arg(long)]
      template: Option<String>,
  }
  ```

### 3. Cleanup
- Remove `new()` function from `src/cli/commands.rs:222`
- Remove `Commands::New` handler from main (lines 209-211)

## Edge Cases

### Multiple Matches
Current behavior preserved - show all paths and error if multiple notes match the title.

### Error Cases
- **Invalid template**: Show error before creating the note
- **Editor fails to launch**: Note is created but editor error is shown
- **User cancels creation**: Clean exit, no note created
- **Permission issues**: Standard file system errors bubble up

## Workflow Comparison

**Before:**
```bash
bnotes new "Meeting Notes"
# Created note: /path/to/meeting-notes.md
bnotes edit "Meeting Notes"
# Opens editor
```

**After:**
```bash
bnotes edit "Meeting Notes"
# Note doesn't exist. Create it? [Y/n] y
# Opens editor immediately
```

## Benefits

1. Simpler mental model - one command for all note creation/editing
2. Fewer keystrokes for the common create-and-edit workflow
3. Matches user expectations from editors like vim/nano
4. Template support integrated naturally into edit workflow
