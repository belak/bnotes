//! Command implementations
//!
//! All CLI command logic is implemented here as functions that are called
//! from the main entry point.

use super::config::CLIConfig;
use super::git::GitRepo;
use super::utils::{expand_home, pluralize};
use anyhow::{Context, Result};
use bnotes::{BNotes, RealStorage};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

// ============================================================================
// Core Commands
// ============================================================================

pub fn search(config_path: Option<PathBuf>, query: &str) -> Result<()> {
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

    println!(
        "Found {} {}",
        matches.len(),
        pluralize(matches.len(), "match", "matches")
    );

    Ok(())
}

pub fn new(
    config_path: Option<PathBuf>,
    title: Option<String>,
    template_name: Option<String>,
) -> Result<()> {
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
    println!(
        "Created note: {}",
        cli_config.notes_dir.join(note_path).display()
    );

    Ok(())
}

pub fn edit(config_path: Option<PathBuf>, title: &str) -> Result<()> {
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

pub fn init(notes_dir: Option<PathBuf>) -> Result<()> {
    use std::fs;

    let config_path = CLIConfig::default_config_path()?;

    // Check if config already exists
    if config_path.exists() {
        print!(
            "Config file already exists at {}. Overwrite? [y/N] ",
            config_path.display()
        );
        io::stdout().flush()?;

        let mut response = String::new();
        io::stdin().read_line(&mut response)?;

        if !response.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Get notes directory
    let notes_dir = if let Some(dir) = notes_dir {
        dir
    } else {
        print!("Enter notes directory path (default: ~/notes): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            expand_home("~/notes")?
        } else {
            expand_home(input)?
        }
    };

    // Create config directory
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("Failed to create config directory: {}", parent.display())
        })?;
    }

    // Create config
    let config = CLIConfig {
        notes_dir: notes_dir.clone(),
    };

    let config_content =
        toml::to_string_pretty(&config).context("Failed to serialize config")?;

    fs::write(&config_path, config_content)
        .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

    println!("Config created at: {}", config_path.display());

    // Create notes directory
    fs::create_dir_all(&notes_dir)
        .with_context(|| format!("Failed to create notes directory: {}", notes_dir.display()))?;

    println!("Notes directory created at: {}", notes_dir.display());

    // Create templates directory
    let templates_dir = notes_dir.join(".btools/templates");
    fs::create_dir_all(&templates_dir).with_context(|| {
        format!(
            "Failed to create templates directory: {}",
            templates_dir.display()
        )
    })?;

    // Create example template
    let example_template = templates_dir.join("daily.md");
    let template_content = r#"---
tags: [daily]
created: {{datetime}}
updated: {{datetime}}
---

# {{title}}

## Tasks
- [ ]

## Notes
"#;

    fs::write(&example_template, template_content).with_context(|| {
        format!(
            "Failed to create example template: {}",
            example_template.display()
        )
    })?;

    println!("Template directory created at: {}", templates_dir.display());
    println!("Example template created: daily.md");

    // Create library config in notes directory
    let bnotes_dir = notes_dir.join(".bnotes");
    fs::create_dir_all(&bnotes_dir).with_context(|| {
        format!(
            "Failed to create .bnotes directory: {}",
            bnotes_dir.display()
        )
    })?;

    let lib_config_path = bnotes_dir.join("config.toml");
    let lib_config_content = r#"# BNotes Library Configuration
# This file is stored in your notes directory and can be committed to version control

# Directory for note templates (relative to notes root)
template_dir = ".btools/templates"

[periodic]
# Template filenames for periodic notes (in template_dir)
daily_template = "daily.md"
weekly_template = "weekly.md"
quarterly_template = "quarterly.md"
"#;

    fs::write(&lib_config_path, lib_config_content).with_context(|| {
        format!(
            "Failed to create library config: {}",
            lib_config_path.display()
        )
    })?;

    println!("Library config created at: {}", lib_config_path.display());
    println!("\nbnotes is ready! Try:");
    println!("  bnotes new \"My First Note\"");

    Ok(())
}

pub fn doctor(config_path: Option<PathBuf>) -> Result<()> {
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

// ============================================================================
// Git Commands
// ============================================================================

pub fn sync(config_path: Option<PathBuf>, message: Option<String>) -> Result<()> {
    let cli_config = CLIConfig::resolve_and_load(config_path.as_deref())?;
    let repo = GitRepo::new(cli_config.notes_dir)?;

    // Verify git repository and remote
    repo.check_is_repo()?;
    repo.check_has_remote()?;

    // Check for uncommitted changes
    let has_changes = repo.has_uncommitted_changes()?;

    if has_changes {
        // Generate change summary before staging
        let change_summary = repo.generate_change_summary()?;

        // Stage all changes
        repo.stage_all()?;

        // Create commit message
        let subject = message.unwrap_or_else(|| format!("bnotes sync: {}", GitRepo::get_timestamp()));

        let commit_message = if change_summary.is_empty() {
            subject
        } else {
            format!("{}\n\n{}", subject, change_summary)
        };

        // Commit changes
        repo.commit(&commit_message)?;

        // Count files in summary for success message
        let num_changes = change_summary.lines().filter(|l| l.starts_with('-')).count();

        // Pull and push
        repo.pull()?;
        repo.push()?;

        println!(
            "Synced successfully: committed {} changes, pulled, and pushed",
            num_changes
        );
    } else {
        // No local changes, just pull and push
        repo.pull()?;
        repo.push()?;

        println!("Synced successfully: pulled and pushed");
    }

    Ok(())
}

pub fn pull(config_path: Option<PathBuf>) -> Result<()> {
    let cli_config = CLIConfig::resolve_and_load(config_path.as_deref())?;
    let repo = GitRepo::new(cli_config.notes_dir)?;

    // Verify git repository and remote
    repo.check_is_repo()?;
    repo.check_has_remote()?;

    // Check for uncommitted changes
    let has_changes = repo.has_uncommitted_changes()?;

    if has_changes {
        // Stash changes with timestamp
        let stash_message = format!("bnotes pull auto-stash {}", GitRepo::get_timestamp());
        repo.stash_push(&stash_message)?;

        // Pull changes
        repo.pull()?;

        // Pop stash to reapply changes
        repo.stash_pop()?;
    } else {
        // Clean working directory, just pull
        repo.pull()?;
    }

    println!("Pulled successfully");

    Ok(())
}

// ============================================================================
// Note Commands
// ============================================================================

pub fn note_list(config_path: Option<PathBuf>, tags: &[String]) -> Result<()> {
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

pub fn note_show(config_path: Option<PathBuf>, title: &str) -> Result<()> {
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

pub fn note_links(config_path: Option<PathBuf>, title: &str) -> Result<()> {
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

pub fn note_graph(config_path: Option<PathBuf>) -> Result<()> {
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
    let mut connected_notes: std::collections::HashSet<String> =
        std::collections::HashSet::new();

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
            && !links.is_empty()
        {
            let mut sorted_links: Vec<_> = links.iter().collect();
            sorted_links.sort();
            for link in sorted_links {
                println!("  -> {}", link);
            }
        }
    }

    println!(
        "\nTotal: {} connected {}",
        connected_notes.len(),
        pluralize(connected_notes.len(), "note", "notes")
    );

    Ok(())
}

// ============================================================================
// Task Commands
// ============================================================================

pub fn task_list(
    config_path: Option<PathBuf>,
    tags: &[String],
    status: Option<String>,
) -> Result<()> {
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
        println!(
            "{}  {} {} (from {})",
            task.id(),
            checkbox,
            task.text,
            task.note_title
        );
    }

    println!(
        "\nTotal: {} {}",
        tasks.len(),
        pluralize(tasks.len(), "task", "tasks")
    );

    Ok(())
}

pub fn task_show(config_path: Option<PathBuf>, task_id: &str) -> Result<()> {
    let cli_config = CLIConfig::resolve_and_load(config_path.as_deref())?;
    let storage = Box::new(RealStorage::new(cli_config.notes_dir.clone()));
    let bnotes = BNotes::with_defaults(storage);

    let (task, note) = bnotes.get_task(task_id)?;

    // Display task with context
    println!("Task: {}", task.id());
    println!("Note: {}", task.note_title);
    println!(
        "Status: {}",
        if task.completed { "Done" } else { "Open" }
    );
    println!("\n{}", task.text);

    // Show a bit more context from the note
    println!("\n--- Context from note ---");
    println!("{}", note.content);

    Ok(())
}

// ============================================================================
// Periodic Commands
// ============================================================================

pub enum PeriodicAction {
    Open(Option<String>),
    List,
    Prev,
    Next,
}

/// Generic handler for periodic note commands
pub fn periodic<P: bnotes::PeriodType>(
    config_path: Option<PathBuf>,
    action: PeriodicAction,
    template_override: Option<String>,
) -> Result<()> {
    let cli_config = CLIConfig::resolve_and_load(config_path.as_deref())?;
    let storage = Box::new(RealStorage::new(cli_config.notes_dir.clone()));
    let bnotes = BNotes::with_defaults(storage);

    match action {
        PeriodicAction::Open(date_str) => {
            let period = if let Some(date) = date_str {
                P::from_date_str(&date)?
            } else {
                P::current()
            };

            periodic_open::<P>(&cli_config, &bnotes, period, template_override)?;
        }
        PeriodicAction::List => {
            periodic_list::<P>(&bnotes)?;
        }
        PeriodicAction::Prev => {
            let note_path = bnotes.navigate_periodic::<P>("prev", template_override.as_deref())?;
            launch_editor(&cli_config, &note_path)?;
        }
        PeriodicAction::Next => {
            let note_path = bnotes.navigate_periodic::<P>("next", template_override.as_deref())?;
            launch_editor(&cli_config, &note_path)?;
        }
    }

    Ok(())
}

fn periodic_open<P: bnotes::PeriodType>(
    cli_config: &CLIConfig,
    bnotes: &bnotes::BNotes,
    period: P,
    template_override: Option<String>,
) -> Result<()> {
    let note_path = PathBuf::from(period.filename());
    let full_path = cli_config.notes_dir.join(&note_path);

    // If note doesn't exist, prompt to create
    if !full_path.exists() {
        print!(
            "{} {} doesn't exist. Create it? [Y/n] ",
            match P::template_name() {
                "daily" => "Day",
                "weekly" => "Week",
                "quarterly" => "Quarter",
                _ => "Period",
            },
            period.identifier()
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input == "n" || input == "no" {
            return Ok(());
        }

        // Create the note using library
        bnotes.open_periodic(period, template_override.as_deref())?;
    }

    launch_editor(cli_config, &note_path)?;
    Ok(())
}

fn periodic_list<P: bnotes::PeriodType>(bnotes: &bnotes::BNotes) -> Result<()> {
    let periods = bnotes.list_periodic::<P>()?;

    if periods.is_empty() {
        println!("No {} notes found.", P::template_name());
        return Ok(());
    }

    for period in periods {
        println!("{}", period.display_string());
    }

    Ok(())
}

fn launch_editor(cli_config: &CLIConfig, note_path: &PathBuf) -> Result<()> {
    let full_path = cli_config.notes_dir.join(note_path);
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

    let status = Command::new(&editor)
        .arg(&full_path)
        .status()
        .with_context(|| format!("Failed to open editor: {}", editor))?;

    if !status.success() {
        anyhow::bail!("Editor exited with status: {}", status);
    }

    Ok(())
}
