use crate::config::CLIConfig;
use anyhow::{Context, Result};
use bnotes::{BNotes, RealStorage};
use std::path::PathBuf;
use std::process::Command;

pub fn run(config_path: Option<PathBuf>, title: &str) -> Result<()> {
    let cli_config = CLIConfig::resolve_and_load(config_path.as_deref())?;
    let storage = Box::new(RealStorage::new(cli_config.notes_dir.clone()));
    let bnotes = BNotes::with_defaults(storage);

    let matches = bnotes.find_note_by_title(title)?;

    let note_path = match matches.len() {
        0 => anyhow::bail!("Note not found: {}", title),
        1 => cli_config.notes_dir.join(&matches[0].path),
        _ => {
            println!("Multiple notes found with title '{}':", title);
            for note in &matches {
                println!("  - {}", cli_config.notes_dir.join(&note.path).display());
            }
            anyhow::bail!("Please be more specific.");
        }
    };

    // Open in editor
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
    let status = Command::new(&editor)
        .arg(&note_path)
        .status()
        .with_context(|| format!("Failed to open editor: {}", editor))?;

    if !status.success() {
        anyhow::bail!("Editor exited with status: {}", status);
    }

    Ok(())
}
