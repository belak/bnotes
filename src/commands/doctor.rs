use crate::config::Config;
use crate::link::LinkGraph;
use crate::repository::Repository;
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

pub fn run(config_path: Option<PathBuf>) -> Result<()> {
    let config = Config::resolve_and_load(config_path.as_deref())?;
    let repo = Repository::new(&config.notes_dir);

    let notes = repo.discover_notes()?;

    if notes.is_empty() {
        println!("No notes found to check.");
        return Ok(());
    }

    let mut issues_found = 0;

    println!("Running health checks on {} notes...\n", notes.len());

    // Check for broken wiki links
    let graph = LinkGraph::build(&notes);
    let broken_links = graph.broken_links(&notes);

    if !broken_links.is_empty() {
        println!("❌ Broken wiki links:");
        for (note_title, broken) in &broken_links {
            println!("  {} has broken links:", note_title);
            for link in broken {
                println!("    - [[{}]]", link);
            }
        }
        issues_found += broken_links.len();
        println!();
    }

    // Check for notes without tags
    let notes_without_tags: Vec<_> = notes
        .iter()
        .filter(|n| n.tags.is_empty())
        .collect();

    if !notes_without_tags.is_empty() {
        println!("⚠️  Notes without tags:");
        for note in &notes_without_tags {
            println!("  - {}", note.title);
        }
        issues_found += notes_without_tags.len();
        println!();
    }

    // Check for notes missing frontmatter
    let notes_without_frontmatter: Vec<_> = notes
        .iter()
        .filter(|n| {
            // A note is missing frontmatter if it has no tags and no dates
            n.tags.is_empty() && n.created.is_none() && n.updated.is_none()
        })
        .collect();

    if !notes_without_frontmatter.is_empty() {
        println!("⚠️  Notes missing frontmatter:");
        for note in &notes_without_frontmatter {
            println!("  - {}", note.title);
        }
        issues_found += notes_without_frontmatter.len();
        println!();
    }

    // Check for multiple notes with the same title
    let mut title_counts: HashMap<String, Vec<String>> = HashMap::new();
    for note in &notes {
        title_counts
            .entry(note.title.to_lowercase())
            .or_insert_with(Vec::new)
            .push(note.path.display().to_string());
    }

    let duplicate_titles: Vec<_> = title_counts
        .iter()
        .filter(|(_, paths)| paths.len() > 1)
        .collect();

    if !duplicate_titles.is_empty() {
        println!("❌ Multiple notes with the same title:");
        for (title, paths) in duplicate_titles {
            println!("  Title: {}", title);
            for path in paths {
                println!("    - {}", path);
            }
        }
        issues_found += title_counts
            .values()
            .filter(|paths| paths.len() > 1)
            .count();
        println!();
    }

    // Check for orphaned notes (no links and no tags)
    let all_titles: Vec<String> = notes.iter().map(|n| n.title.clone()).collect();
    let orphaned = graph.orphaned_notes(&all_titles);

    let truly_orphaned: Vec<_> = orphaned
        .iter()
        .filter(|title| {
            notes
                .iter()
                .find(|n| &n.title == *title)
                .map(|n| n.tags.is_empty())
                .unwrap_or(true)
        })
        .collect();

    if !truly_orphaned.is_empty() {
        println!("⚠️  Orphaned notes (no links, no tags):");
        for title in truly_orphaned {
            println!("  - {}", title);
        }
        issues_found += orphaned.len();
        println!();
    }

    // Summary
    if issues_found == 0 {
        println!("✅ All checks passed! Your notes are healthy.");
    } else {
        println!("Found {} issue{} that may need attention.",
            issues_found,
            if issues_found == 1 { "" } else { "s" }
        );
    }

    Ok(())
}
