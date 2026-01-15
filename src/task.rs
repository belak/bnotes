use crate::note::Note;
use pulldown_cmark::{Event, Options, Parser, Tag};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Task {
    pub note_path: PathBuf,
    pub note_title: String,
    pub index: usize, // 1-based index within the note
    pub completed: bool,
    pub text: String,
    pub context: String, // Surrounding lines for context
}

impl Task {
    /// Extract all tasks from a note
    pub fn extract_from_note(note: &Note) -> Vec<Task> {
        let mut tasks = Vec::new();
        let mut task_index = 0;

        // Parse the markdown to find task list items
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TASKLISTS);
        let parser = Parser::new_ext(&note.content, options);
        let mut in_task_item = false;
        let mut task_text = String::new();
        let mut is_checked = false;

        for event in parser {
            match event {
                Event::Start(Tag::Item) => {
                    in_task_item = false;
                    task_text.clear();
                }
                Event::TaskListMarker(checked) => {
                    in_task_item = true;
                    is_checked = checked;
                }
                Event::Text(text) if in_task_item => {
                    task_text.push_str(&text);
                }
                Event::End(pulldown_cmark::TagEnd::Item) if in_task_item => {
                    task_index += 1;

                    // Extract context (simplified - just use the task text)
                    let context = task_text.trim().to_string();

                    tasks.push(Task {
                        note_path: note.path.clone(),
                        note_title: note.title.clone(),
                        index: task_index,
                        completed: is_checked,
                        text: context.clone(),
                        context,
                    });

                    in_task_item = false;
                }
                _ => {}
            }
        }

        tasks
    }

    /// Get the task ID in format "filename#index"
    pub fn id(&self) -> String {
        let filename = self
            .note_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        format!("{}#{}", filename, self.index)
    }
}

/// Extract all tasks from multiple notes
pub fn extract_tasks_from_notes(notes: &[Note]) -> Vec<Task> {
    let mut all_tasks = Vec::new();

    for note in notes {
        let tasks = Task::extract_from_note(note);
        all_tasks.extend(tasks);
    }

    all_tasks
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_extract_tasks() {
        let content = r#"---
tags: [test]
---

# My Note

Some text.

## Tasks
- [ ] First task
- [x] Completed task
- [ ] Another task

More text.
"#;

        let note = Note::parse(Path::new("test.md"), content).unwrap();
        let tasks = Task::extract_from_note(&note);

        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].text, "First task");
        assert_eq!(tasks[0].completed, false);
        assert_eq!(tasks[1].text, "Completed task");
        assert_eq!(tasks[1].completed, true);
        assert_eq!(tasks[2].text, "Another task");
        assert_eq!(tasks[2].completed, false);
    }
}
