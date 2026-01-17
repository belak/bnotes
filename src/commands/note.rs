use crate::cli_config::CLIConfig;
use crate::util::pluralize;
use bnotes::{BNotes, RealStorage};
use anyhow::Result;
use std::path::PathBuf;

pub fn list(config_path: Option<PathBuf>, tags: &[String]) -> Result<()> {
    let cli_config = CLIConfig::resolve_and_load(config_path.as_deref())?;
    let storage = Box::new(RealStorage::new(cli_config.notes_dir.clone()));
    let bnotes = BNotes::with_defaults(storage);

    let notes = bnotes.list_notes(tags)?;

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

    println!("\nTotal: {} {}", count, pluralize(count, "note", "notes"));

    Ok(())
}

pub fn show(config_path: Option<PathBuf>, title: &str) -> Result<()> {
    let cli_config = CLIConfig::resolve_and_load(config_path.as_deref())?;
    let storage = Box::new(RealStorage::new(cli_config.notes_dir.clone()));
    let bnotes = BNotes::with_defaults(storage);

    let matches = bnotes.find_note_by_title(title)?;

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

pub fn links(config_path: Option<PathBuf>, title: &str) -> Result<()> {
    let cli_config = CLIConfig::resolve_and_load(config_path.as_deref())?;
    let storage = Box::new(RealStorage::new(cli_config.notes_dir.clone()));
    let bnotes = BNotes::with_defaults(storage);

    let matches = bnotes.find_note_by_title(title)?;

    let note = match matches.len() {
        0 => anyhow::bail!("Note not found: {}", title),
        1 => &matches[0],
        _ => {
            println!("Multiple notes found with title '{}':", title);
            for note in matches {
                println!("  - {}", note.path.display());
            }
            anyhow::bail!("Please be more specific.");
        }
    };

    let (outbound, inbound) = bnotes.get_note_links(&note.title)?;

    println!("Links for: {}\n", note.title);

    // Show outbound links (what this note links to)
    if !outbound.is_empty() {
        println!("Outbound links ({}):", outbound.len());
        let mut sorted_links: Vec<_> = outbound.iter().collect();
        sorted_links.sort();
        for link in sorted_links {
            println!("  -> {}", link);
        }
        println!();
    }

    // Show inbound links (what links to this note)
    if !inbound.is_empty() {
        println!("Inbound links ({}):", inbound.len());
        let mut sorted_links: Vec<_> = inbound.iter().collect();
        sorted_links.sort();
        for link in sorted_links {
            println!("  <- {}", link);
        }
        println!();
    }

    // If no links at all
    if outbound.is_empty() && inbound.is_empty() {
        println!("No links found for this note.");
    }

    Ok(())
}

pub fn graph(config_path: Option<PathBuf>) -> Result<()> {
    let cli_config = CLIConfig::resolve_and_load(config_path.as_deref())?;
    let storage = Box::new(RealStorage::new(cli_config.notes_dir.clone()));
    let bnotes = BNotes::with_defaults(storage);

    let notes = bnotes.list_notes(&[])?;

    if notes.is_empty() {
        println!("No notes found.");
        return Ok(());
    }

    let graph = bnotes.get_link_graph()?;

    println!("Link Graph ({} notes):\n", notes.len());

    // Collect all notes that have links (either inbound or outbound)
    let mut connected_notes: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (note, links) in &graph.outbound {
        if !links.is_empty() {
            connected_notes.insert(note.clone());
        }
    }

    for (note, links) in &graph.inbound {
        if !links.is_empty() {
            connected_notes.insert(note.clone());
        }
    }

    if connected_notes.is_empty() {
        println!("No links found between notes.");
        return Ok(());
    }

    // Sort for consistent output
    let mut sorted_notes: Vec<_> = connected_notes.iter().collect();
    sorted_notes.sort();

    // Simple ASCII representation
    for note in sorted_notes {
        let outbound = graph.outbound.get(note);
        let inbound = graph.inbound.get(note);

        let out_count = outbound.map(|s| s.len()).unwrap_or(0);
        let in_count = inbound.map(|s| s.len()).unwrap_or(0);

        println!("- {} (->{} <-{})", note, out_count, in_count);

        if let Some(links) = outbound
            && !links.is_empty() {
                let mut sorted_links: Vec<_> = links.iter().collect();
                sorted_links.sort();
                for link in sorted_links {
                    println!("  -> {}", link);
                }
            }
    }

    println!("\nTotal: {} connected {}",
        connected_notes.len(),
        pluralize(connected_notes.len(), "note", "notes")
    );

    Ok(())
}
