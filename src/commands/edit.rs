use crate::util::CommandContext;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

pub fn run(config_path: Option<PathBuf>, title: &str) -> Result<()> {
    let ctx = CommandContext::load(config_path)?;
    let matches = ctx.repo.find_by_title(title)?;

    let note_path = match matches.len() {
        0 => anyhow::bail!("Note not found: {}", title),
        1 => matches[0].path.clone(),
        _ => {
            println!("Multiple notes found with title '{}':", title);
            for note in matches {
                println!("  - {}", note.path.display());
            }
            anyhow::bail!("Please be more specific.");
        }
    };

    // Open in editor
    let status = Command::new(&ctx.config.editor)
        .arg(&note_path)
        .status()
        .with_context(|| format!("Failed to open editor: {}", ctx.config.editor))?;

    if !status.success() {
        anyhow::bail!("Editor exited with status: {}", status);
    }

    Ok(())
}
