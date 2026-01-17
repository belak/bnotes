use crate::cli_config::CLIConfig;
use bnotes::{BNotes, RealStorage};
use anyhow::Result;
use std::io::{self, Write};
use std::path::PathBuf;

pub fn run(config_path: Option<PathBuf>, title: Option<String>, template_name: Option<String>) -> Result<()> {
    // CLI handles prompting for missing title
    let title = if let Some(t) = title {
        t
    } else {
        print!("Enter note title: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            anyhow::bail!("Title cannot be empty");
        }

        input.to_string()
    };

    // Call library with complete data
    let cli_config = CLIConfig::resolve_and_load(config_path.as_deref())?;
    let storage = Box::new(RealStorage::new(cli_config.notes_dir.clone()));
    let bnotes = BNotes::with_defaults(storage);

    let note_path = bnotes.create_note(&title, template_name.as_deref())?;

    // note_path is relative, join with notes_dir for display
    println!("Created note: {}", cli_config.notes_dir.join(note_path).display());

    Ok(())
}
