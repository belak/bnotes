use crate::config::CLIConfig;
use crate::util::pluralize;
use anyhow::Result;
use bnotes::{BNotes, RealStorage};
use std::path::PathBuf;

pub fn run(config_path: Option<PathBuf>) -> Result<()> {
    let cli_config = CLIConfig::resolve_and_load(config_path.as_deref())?;
    let storage = Box::new(RealStorage::new(cli_config.notes_dir.clone()));
    let bnotes = BNotes::with_defaults(storage);

    // Get note count for display
    let notes = bnotes.list_notes(&[])?;

    if notes.is_empty() {
        println!("No notes found to check.");
        return Ok(());
    }

    println!("Running health checks on {} notes...\n", notes.len());

    // Run health checks
    let report = bnotes.check_health()?;

    // Display broken wiki links
    if !report.broken_links.is_empty() {
        println!("ERROR: Broken wiki links:");
        for (note_title, broken) in &report.broken_links {
            println!("  {} has broken links:", note_title);
            for link in broken {
                println!("    - [[{}]]", link);
            }
        }
        println!();
    }

    // Display notes without tags
    if !report.notes_without_tags.is_empty() {
        println!("WARNING: Notes without tags:");
        for title in &report.notes_without_tags {
            println!("  - {}", title);
        }
        println!();
    }

    // Display notes missing frontmatter
    if !report.notes_without_frontmatter.is_empty() {
        println!("WARNING: Notes missing frontmatter:");
        for title in &report.notes_without_frontmatter {
            println!("  - {}", title);
        }
        println!();
    }

    // Display duplicate titles
    if !report.duplicate_titles.is_empty() {
        println!("ERROR: Multiple notes with the same title:");
        for (title, paths) in &report.duplicate_titles {
            println!("  Title: {}", title);
            for path in paths {
                println!("    - {}", path);
            }
        }
        println!();
    }

    // Display orphaned notes
    if !report.orphaned_notes.is_empty() {
        println!("WARNING: Orphaned notes (no links, no tags):");
        for title in &report.orphaned_notes {
            println!("  - {}", title);
        }
        println!();
    }

    // Summary
    if !report.has_issues() {
        println!("All checks passed! Your notes are healthy.");
    } else {
        println!(
            "Found {} {} that may need attention.",
            report.issue_count(),
            pluralize(report.issue_count(), "issue", "issues")
        );
    }

    Ok(())
}
