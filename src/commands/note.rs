use crate::config::Config;
use crate::repository::Repository;
use anyhow::Result;
use std::path::PathBuf;

pub fn list(config_path: Option<PathBuf>, tags: &[String]) -> Result<()> {
    let config = Config::resolve_and_load(config_path.as_deref())?;
    let repo = Repository::new(&config.notes_dir);

    let notes = if tags.is_empty() {
        repo.discover_notes()?
    } else {
        repo.filter_by_tags(tags)?
    };

    if notes.is_empty() {
        if tags.is_empty() {
            println!("No notes found.");
        } else {
            println!("No notes found with tags: {}", tags.join(", "));
        }
        return Ok(());
    }

    // Sort by title
    let mut notes = notes;
    notes.sort_by(|a, b| a.title.cmp(&b.title));

    let count = notes.len();

    for note in notes {
        let tag_str = if note.tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", note.tags.join(", "))
        };

        println!("{}{}", note.title, tag_str);
    }

    println!("\nTotal: {} note{}", count, if count == 1 { "" } else { "s" });

    Ok(())
}

pub fn show(config_path: Option<PathBuf>, title: &str) -> Result<()> {
    let config = Config::resolve_and_load(config_path.as_deref())?;
    let repo = Repository::new(&config.notes_dir);

    let matches = repo.find_by_title(title)?;

    match matches.len() {
        0 => anyhow::bail!("Note not found: {}", title),
        1 => {
            let note = &matches[0];
            println!("{}", note.content);
            Ok(())
        }
        _ => {
            println!("Multiple notes found with title '{}':", title);
            for note in matches {
                println!("  - {}", note.path.display());
            }
            anyhow::bail!("Please be more specific or use the full path.");
        }
    }
}
