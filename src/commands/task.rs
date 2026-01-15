use crate::config::Config;
use crate::repository::Repository;
use crate::task::{self, Task};
use anyhow::Result;
use std::path::PathBuf;

pub fn list(config_path: Option<PathBuf>, tags: &[String], status: Option<String>) -> Result<()> {
    let config = Config::resolve_and_load(config_path.as_deref())?;
    let repo = Repository::new(&config.notes_dir);

    // Get notes, optionally filtered by tags
    let notes = if tags.is_empty() {
        repo.discover_notes()?
    } else {
        repo.filter_by_tags(tags)?
    };

    // Extract tasks from all notes
    let mut tasks = task::extract_tasks_from_notes(&notes);

    // Filter by status if specified
    if let Some(status_filter) = status {
        let filter_open = status_filter.eq_ignore_ascii_case("open");
        let filter_done = status_filter.eq_ignore_ascii_case("done");

        if !filter_open && !filter_done {
            anyhow::bail!("Invalid status filter: {}. Use 'open' or 'done'.", status_filter);
        }

        tasks.retain(|task| {
            if filter_open {
                !task.completed
            } else {
                task.completed
            }
        });
    }

    if tasks.is_empty() {
        println!("No tasks found.");
        return Ok(());
    }

    // Display tasks
    for task in &tasks {
        let checkbox = if task.completed { "[x]" } else { "[ ]" };
        println!("{}  {} {} (from {})",
            task.id(),
            checkbox,
            task.text,
            task.note_title
        );
    }

    println!("\nTotal: {} task{}", tasks.len(), if tasks.len() == 1 { "" } else { "s" });

    Ok(())
}

pub fn show(config_path: Option<PathBuf>, task_id: &str) -> Result<()> {
    let config = Config::resolve_and_load(config_path.as_deref())?;
    let repo = Repository::new(&config.notes_dir);

    // Parse task ID (format: "filename#index")
    let parts: Vec<&str> = task_id.split('#').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid task ID format. Expected 'filename#index'");
    }

    let filename = parts[0];
    let index: usize = parts[1].parse()
        .map_err(|_| anyhow::anyhow!("Invalid task index: {}", parts[1]))?;

    // Find the note
    let notes = repo.discover_notes()?;
    let note = notes.iter()
        .find(|n| n.path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s == filename)
            .unwrap_or(false))
        .ok_or_else(|| anyhow::anyhow!("Note not found: {}", filename))?;

    // Extract tasks from the note
    let tasks = Task::extract_from_note(note);

    // Find the specific task
    let task = tasks.iter()
        .find(|t| t.index == index)
        .ok_or_else(|| anyhow::anyhow!("Task not found: {}", task_id))?;

    // Display task with context
    println!("Task: {}", task.id());
    println!("Note: {}", task.note_title);
    println!("Status: {}", if task.completed { "Done" } else { "Open" });
    println!("\n{}", task.text);

    // Show a bit more context from the note
    println!("\n--- Context from note ---");
    println!("{}", note.content);

    Ok(())
}
