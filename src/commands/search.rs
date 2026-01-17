use crate::config::CLIConfig;
use crate::util::pluralize;
use bnotes::{BNotes, RealStorage};
use anyhow::Result;
use std::path::PathBuf;

pub fn run(config_path: Option<PathBuf>, query: &str) -> Result<()> {
    let cli_config = CLIConfig::resolve_and_load(config_path.as_deref())?;
    let storage = Box::new(RealStorage::new(cli_config.notes_dir.clone()));
    let bnotes = BNotes::with_defaults(storage);

    let matches = bnotes.search(query)?;

    if matches.is_empty() {
        println!("No notes found matching: {}", query);
        return Ok(());
    }

    for note in &matches {
        println!("{}", note.title);

        // Show a snippet of content containing the query
        let query_lower = query.to_lowercase();
        let content_lower = note.content.to_lowercase();

        if let Some(pos) = content_lower.find(&query_lower) {
            // Show context around the match
            let start = pos.saturating_sub(50);
            let end = (pos + query.len() + 50).min(note.content.len());

            let snippet = &note.content[start..end];
            let snippet = snippet.trim();

            println!("  ... {} ...", snippet);
        }

        println!();
    }

    println!("Found {} {}", matches.len(), pluralize(matches.len(), "match", "matches"));

    Ok(())
}
