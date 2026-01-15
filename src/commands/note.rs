use crate::config::Config;
use crate::link::LinkGraph;
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

pub fn links(config_path: Option<PathBuf>, title: &str) -> Result<()> {
    let config = Config::resolve_and_load(config_path.as_deref())?;
    let repo = Repository::new(&config.notes_dir);

    let matches = repo.find_by_title(title)?;

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

    // Build link graph
    let notes = repo.discover_notes()?;
    let graph = LinkGraph::build(&notes);

    println!("Links for: {}\n", note.title);

    // Show outbound links (what this note links to)
    let outbound = graph.outbound.get(&note.title);
    if let Some(links) = outbound {
        if !links.is_empty() {
            println!("Outbound links ({}):", links.len());
            let mut sorted_links: Vec<_> = links.iter().collect();
            sorted_links.sort();
            for link in sorted_links {
                println!("  → {}", link);
            }
            println!();
        }
    }

    // Show inbound links (what links to this note)
    let inbound = graph.inbound.get(&note.title);
    if let Some(links) = inbound {
        if !links.is_empty() {
            println!("Inbound links ({}):", links.len());
            let mut sorted_links: Vec<_> = links.iter().collect();
            sorted_links.sort();
            for link in sorted_links {
                println!("  ← {}", link);
            }
            println!();
        }
    }

    // If no links at all
    if (outbound.is_none() || outbound.unwrap().is_empty())
        && (inbound.is_none() || inbound.unwrap().is_empty())
    {
        println!("No links found for this note.");
    }

    Ok(())
}

pub fn graph(config_path: Option<PathBuf>) -> Result<()> {
    let config = Config::resolve_and_load(config_path.as_deref())?;
    let repo = Repository::new(&config.notes_dir);

    let notes = repo.discover_notes()?;

    if notes.is_empty() {
        println!("No notes found.");
        return Ok(());
    }

    let graph = LinkGraph::build(&notes);

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

        println!("• {} (→{} ←{})", note, out_count, in_count);

        if let Some(links) = outbound {
            if !links.is_empty() {
                let mut sorted_links: Vec<_> = links.iter().collect();
                sorted_links.sort();
                for link in sorted_links {
                    println!("  → {}", link);
                }
            }
        }
    }

    println!("\nTotal: {} connected note{}",
        connected_notes.len(),
        if connected_notes.len() == 1 { "" } else { "s" }
    );

    Ok(())
}
