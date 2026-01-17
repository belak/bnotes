use crate::cli_config::CLIConfig;
use crate::util::pluralize;
use bnotes::{BNotes, RealStorage};
use anyhow::Result;
use std::path::PathBuf;

pub fn list(config_path: Option<PathBuf>, tags: &[String], status: Option<String>) -> Result<()> {
    let cli_config = CLIConfig::resolve_and_load(config_path.as_deref())?;
    let storage = Box::new(RealStorage::new(cli_config.notes_dir.clone()));
    let bnotes = BNotes::with_defaults(storage);

    let tasks = bnotes.list_tasks(tags, status.as_deref())?;

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

    println!("\nTotal: {} {}", tasks.len(), pluralize(tasks.len(), "task", "tasks"));

    Ok(())
}

pub fn show(config_path: Option<PathBuf>, task_id: &str) -> Result<()> {
    let cli_config = CLIConfig::resolve_and_load(config_path.as_deref())?;
    let storage = Box::new(RealStorage::new(cli_config.notes_dir.clone()));
    let bnotes = BNotes::with_defaults(storage);

    let (task, note) = bnotes.get_task(task_id)?;

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
