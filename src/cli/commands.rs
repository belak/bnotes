//! Command implementations
//!
//! All CLI command logic is implemented here as functions that are called
//! from the main entry point.

use super::colors;
use super::git::GitRepo;
use super::utils::pluralize;
use anyhow::{Context, Result};
use bnotes::{BNotes, PeriodType, RealStorage};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use termcolor::{ColorChoice, WriteColor};
use wildmatch::WildMatch;

/// Validate that notes directory exists
fn validate_notes_dir(notes_dir: &Path) -> Result<()> {
    if !notes_dir.exists() {
        anyhow::bail!(
            "Notes directory does not exist: {}\n\nSet BNOTES_DIR environment variable or use --notes-dir flag to specify a different location.",
            notes_dir.display()
        );
    }

    if !notes_dir.is_dir() {
        anyhow::bail!(
            "Notes path is not a directory: {}",
            notes_dir.display()
        );
    }

    Ok(())
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Write text with highlighted query matches using proper termcolor API
///
/// Matches are written in bold, text segments inherit the current color state
fn write_with_highlights<W: WriteColor>(
    stdout: &mut W,
    text: &str,
    query: &str,
    base_color: &termcolor::ColorSpec,
    highlight_color: &termcolor::ColorSpec,
) -> io::Result<()> {
    let query_lower = query.to_lowercase();
    let text_lower = text.to_lowercase();

    let mut last_end = 0;

    while let Some(pos) = text_lower[last_end..].find(&query_lower) {
        let start = last_end + pos;
        let end = start + query.len();

        // Write text before match with base color
        stdout.set_color(base_color)?;
        write!(stdout, "{}", &text[last_end..start])?;

        // Write match in bold (reset to normal, then bold)
        stdout.set_color(highlight_color)?;
        write!(stdout, "{}", &text[start..end])?;

        last_end = end;
    }

    // Write remaining text with base color
    stdout.set_color(base_color)?;
    write!(stdout, "{}", &text[last_end..])?;

    Ok(())
}

/// Write tags with highlighted query matches
fn write_tags_with_highlights<W: WriteColor>(
    stdout: &mut W,
    tags: &[String],
    query: &str,
) -> io::Result<()> {
    let query_lower = query.to_lowercase();
    let default_color = colors::dim();

    let mut highlight_color = colors::default();
    highlight_color.set_bold(true);

    stdout.set_color(&default_color)?;
    write!(stdout, " [")?;

    for (i, tag) in tags.iter().enumerate() {
        if i > 0 {
            write!(stdout, ", ")?;
        }

        if tag.to_lowercase().contains(&query_lower) {
            write_with_highlights(stdout, tag, query, &default_color, &highlight_color)?;
        } else {
            write!(stdout, "{}", tag)?;
        }
    }

    write!(stdout, "]")?;
    Ok(())
}

// ============================================================================
// Core Commands
// ============================================================================

pub fn search(notes_dir: &Path, query: &str, limit: usize, color: ColorChoice) -> Result<()> {
    validate_notes_dir(notes_dir)?;
    let storage = Box::new(RealStorage::new(notes_dir.to_path_buf()));
    let bnotes = BNotes::with_defaults(storage);

    let title_base_color = colors::highlight();
    let mut title_highlight_color = title_base_color.clone();
    title_highlight_color.set_bold(true);

    let text_base_color = colors::default();
    let text_highlight_color = colors::highlight();

    let matches = bnotes.search(query)?;

    let mut stdout = colors::create_stdout(color);

    if matches.is_empty() {
        writeln!(stdout, "No notes found matching: {}", query)?;
        return Ok(());
    }

    for search_match in &matches {
        // Display title with matched words in bold
        write_with_highlights(&mut stdout, &search_match.note.title, query, &title_base_color, &title_highlight_color)?;
        stdout.reset()?;

        // Show tags with potential highlighting
        if !search_match.note.tags.is_empty() {
            write_tags_with_highlights(&mut stdout, &search_match.note.tags, query)?;
            writeln!(stdout)?;
        } else {
            writeln!(stdout)?;
        }

        // Apply limit to locations
        let total_matches = search_match.locations.len();
        let limited_locations = &search_match.locations[..limit.min(total_matches)];

        // Count content matches to determine when to add spacing
        let content_match_count = limited_locations
            .iter()
            .filter(|loc| matches!(loc, bnotes::MatchLocation::Content { .. }))
            .count();

        let mut content_match_index = 0;

        // Iterate through each MatchLocation and render based on type
        for location in limited_locations {
            match location {
                bnotes::MatchLocation::Title { .. } => {
                    // Title match is already displayed in title, skip additional output
                }
                bnotes::MatchLocation::Tag { .. } => {
                    // Tag match is already displayed in tags, skip additional output
                }
                bnotes::MatchLocation::Content {
                    breadcrumb,
                    snippet,
                    ..
                } => {
                    // Display breadcrumb in dim
                    stdout.set_color(&colors::dim())?;
                    if breadcrumb.is_empty() {
                        writeln!(stdout, "  [Document Start]")?;
                    } else {
                        writeln!(stdout, "  [{}]", breadcrumb.join(" > "))?;
                    }

                    // Display snippet in dim with bold highlighted matches
                    write!(stdout, "  ")?;
                    write_with_highlights(&mut stdout, snippet, query, &text_base_color, &text_highlight_color)?;
                    writeln!(stdout)?;
                    stdout.reset()?;

                    content_match_index += 1;

                    // Add blank line between content matches (but not after the last one)
                    if content_match_index < content_match_count {
                        writeln!(stdout)?;
                    }
                }
            }
        }

        // Show truncation message if needed
        if total_matches > limit {
            let remaining = total_matches - limit;
            stdout.set_color(&colors::dim())?;
            writeln!(
                stdout,
                "  ({} {} shown, {} more in this note)",
                limit,
                pluralize(limit, "match", "matches"),
                remaining
            )?;
            stdout.reset()?;
        }

        writeln!(stdout)?;
    }

    writeln!(
        stdout,
        "Found {} {}",
        matches.len(),
        pluralize(matches.len(), "note with matches", "notes with matches")
    )?;

    Ok(())
}

pub fn edit(notes_dir: &Path, title: &str, template_name: Option<String>, print_path: bool) -> Result<()> {
    validate_notes_dir(notes_dir)?;
    let storage = Box::new(RealStorage::new(notes_dir.to_path_buf()));
    let bnotes = BNotes::with_defaults(storage);

    let matches = bnotes.find_note_by_title(title)?;

    let relative_path = match matches.len() {
        0 => {
            // Try treating it as a file path
            let potential_path = PathBuf::from(title);
            let full_path = notes_dir.join(&potential_path);

            if full_path.exists() && full_path.is_file() {
                // It's a valid file path - use it directly
                potential_path
            } else {
                if print_path {
                    anyhow::bail!("Note doesn't exist: {}", title);
                }

                // Note doesn't exist - prompt to create it
                print!("Note doesn't exist. Create it? [Y/n] ");
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let input = input.trim().to_lowercase();

                if input == "n" || input == "no" {
                    return Ok(());
                }

                // Create the note
                bnotes.create_note(title, template_name.as_deref())?
            }
        }
        1 => matches[0].path.clone(),
        _ => {
            println!("Multiple notes found with title '{}':", title);
            for note in &matches {
                println!("  - {}", notes_dir.join(&note.path).display());
            }
            anyhow::bail!("Please be more specific.");
        }
    };

    launch_editor(notes_dir, &relative_path, &bnotes, print_path)?;
    Ok(())
}

// ============================================================================
// Health & Maintenance Commands
// ============================================================================

pub fn doctor(notes_dir: &Path, color: ColorChoice) -> Result<()> {
    validate_notes_dir(notes_dir)?;
    let storage = Box::new(RealStorage::new(notes_dir.to_path_buf()));
    let bnotes = BNotes::with_defaults(storage);

    // Get note count for display
    let notes = bnotes.list_notes(&[])?;

    let mut stdout = colors::create_stdout(color);

    if notes.is_empty() {
        writeln!(stdout, "No notes found to check.")?;
        return Ok(());
    }

    writeln!(stdout, "Running health checks on {} notes...\n", notes.len())?;

    // Run health checks
    let report = bnotes.check_health()?;

    // Display broken wiki links
    if !report.broken_links.is_empty() {
        stdout.set_color(&colors::error())?;
        write!(stdout, "ERROR:")?;
        stdout.reset()?;
        writeln!(stdout, " Broken wiki links:")?;
        for (note_title, broken) in &report.broken_links {
            writeln!(stdout, "  {} has broken links:", note_title)?;
            for link in broken {
                writeln!(stdout, "    - [[{}]]", link)?;
            }
        }
        writeln!(stdout)?;
    }

    // Display notes without tags
    if !report.notes_without_tags.is_empty() {
        stdout.set_color(&colors::warning())?;
        write!(stdout, "WARNING:")?;
        stdout.reset()?;
        writeln!(stdout, " Notes without tags:")?;
        for title in &report.notes_without_tags {
            writeln!(stdout, "  - {}", title)?;
        }
        writeln!(stdout)?;
    }

    // Display notes missing frontmatter
    if !report.notes_without_frontmatter.is_empty() {
        stdout.set_color(&colors::warning())?;
        write!(stdout, "WARNING:")?;
        stdout.reset()?;
        writeln!(stdout, " Notes missing frontmatter:")?;
        for title in &report.notes_without_frontmatter {
            writeln!(stdout, "  - {}", title)?;
        }
        writeln!(stdout)?;
    }

    // Display duplicate titles
    if !report.duplicate_titles.is_empty() {
        stdout.set_color(&colors::error())?;
        write!(stdout, "ERROR:")?;
        stdout.reset()?;
        writeln!(stdout, " Multiple notes with the same title:")?;
        for (title, paths) in &report.duplicate_titles {
            writeln!(stdout, "  Title: {}", title)?;
            for path in paths {
                writeln!(stdout, "    - {}", path)?;
            }
        }
        writeln!(stdout)?;
    }

    // Display orphaned notes
    if !report.orphaned_notes.is_empty() {
        stdout.set_color(&colors::warning())?;
        write!(stdout, "WARNING:")?;
        stdout.reset()?;
        writeln!(stdout, " Orphaned notes (no links, no tags):")?;
        for title in &report.orphaned_notes {
            writeln!(stdout, "  - {}", title)?;
        }
        writeln!(stdout)?;
    }

    // Summary
    if !report.has_issues() {
        stdout.set_color(&colors::success())?;
        writeln!(stdout, "All checks passed! Your notes are healthy.")?;
        stdout.reset()?;
    } else {
        writeln!(
            stdout,
            "Found {} {} that may need attention.",
            report.issue_count(),
            pluralize(report.issue_count(), "issue", "issues")
        )?;
    }

    Ok(())
}

// ============================================================================
// Git Commands
// ============================================================================

pub fn sync(notes_dir: &Path, message: Option<String>, color: ColorChoice) -> Result<()> {
    validate_notes_dir(notes_dir)?;
    let repo = GitRepo::new(notes_dir.to_path_buf())?;

    // Verify git repository and remote
    repo.check_is_repo()?;
    repo.check_has_remote()?;

    // Check for uncommitted changes
    let has_changes = repo.has_uncommitted_changes()?;

    let mut stdout = colors::create_stdout(color);

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

        stdout.set_color(&colors::success())?;
        writeln!(
            stdout,
            "Synced successfully: committed {} changes, pulled, and pushed",
            num_changes
        )?;
        stdout.reset()?;
    } else {
        // No local changes, just pull and push
        repo.pull()?;
        repo.push()?;

        stdout.set_color(&colors::success())?;
        writeln!(stdout, "Synced successfully: pulled and pushed")?;
        stdout.reset()?;
    }

    Ok(())
}

pub fn pull(notes_dir: &Path, color: ColorChoice) -> Result<()> {
    validate_notes_dir(notes_dir)?;
    let repo = GitRepo::new(notes_dir.to_path_buf())?;

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

    let mut stdout = colors::create_stdout(color);
    stdout.set_color(&colors::success())?;
    writeln!(stdout, "Pulled successfully")?;
    stdout.reset()?;

    Ok(())
}

// ============================================================================
// Note Commands
// ============================================================================

pub fn note_list(notes_dir: &Path, tags: &[String], color: ColorChoice) -> Result<()> {
    validate_notes_dir(notes_dir)?;
    let storage = Box::new(RealStorage::new(notes_dir.to_path_buf()));
    let bnotes = BNotes::with_defaults(storage);

    let notes = bnotes.list_notes(tags)?;

    let mut stdout = colors::create_stdout(color);

    if notes.is_empty() {
        if tags.is_empty() {
            writeln!(stdout, "No notes found.")?;
        } else {
            writeln!(stdout, "No notes found with tags: {}", tags.join(", "))?;
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

        writeln!(stdout, "{}{}", note.title, tag_str)?;
    }

    write!(stdout, "\nTotal: ")?;
    stdout.set_color(&colors::highlight())?;
    write!(stdout, "{}", count)?;
    stdout.reset()?;
    writeln!(stdout, " {}", pluralize(count, "note", "notes"))?;

    Ok(())
}

pub fn note_show(notes_dir: &Path, title: &str) -> Result<()> {
    validate_notes_dir(notes_dir)?;
    let storage = Box::new(RealStorage::new(notes_dir.to_path_buf()));
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

pub fn note_links(notes_dir: &Path, title: &str, color: ColorChoice) -> Result<()> {
    validate_notes_dir(notes_dir)?;
    let storage = Box::new(RealStorage::new(notes_dir.to_path_buf()));
    let bnotes = BNotes::with_defaults(storage);

    let matches = bnotes.find_note_by_title(title)?;

    let mut stdout = colors::create_stdout(color);

    let note = match matches.len() {
        0 => anyhow::bail!("Note not found: {}", title),
        1 => &matches[0],
        _ => {
            writeln!(stdout, "Multiple notes found with title '{}':", title)?;
            for note in matches {
                writeln!(stdout, "  - {}", note.path.display())?;
            }
            anyhow::bail!("Please be more specific.");
        }
    };

    let (outbound, inbound) = bnotes.get_note_links(&note.title)?;

    writeln!(stdout, "Links for: {}\n", note.title)?;

    // Show outbound links (what this note links to)
    if !outbound.is_empty() {
        write!(stdout, "Outbound links (")?;
        stdout.set_color(&colors::highlight())?;
        write!(stdout, "{}", outbound.len())?;
        stdout.reset()?;
        writeln!(stdout, "):")?;

        let mut sorted_links: Vec<_> = outbound.iter().collect();
        sorted_links.sort();
        for link in sorted_links {
            write!(stdout, "  ")?;
            stdout.set_color(&colors::highlight())?;
            write!(stdout, "->")?;
            stdout.reset()?;
            writeln!(stdout, " {}", link)?;
        }
        writeln!(stdout)?;
    }

    // Show inbound links (what links to this note)
    if !inbound.is_empty() {
        write!(stdout, "Inbound links (")?;
        stdout.set_color(&colors::highlight())?;
        write!(stdout, "{}", inbound.len())?;
        stdout.reset()?;
        writeln!(stdout, "):")?;

        let mut sorted_links: Vec<_> = inbound.iter().collect();
        sorted_links.sort();
        for link in sorted_links {
            write!(stdout, "  ")?;
            stdout.set_color(&colors::highlight())?;
            write!(stdout, "<-")?;
            stdout.reset()?;
            writeln!(stdout, " {}", link)?;
        }
        writeln!(stdout)?;
    }

    // If no links at all
    if outbound.is_empty() && inbound.is_empty() {
        writeln!(stdout, "No links found for this note.")?;
    }

    Ok(())
}

pub fn note_graph(notes_dir: &Path, color: ColorChoice) -> Result<()> {
    validate_notes_dir(notes_dir)?;
    let storage = Box::new(RealStorage::new(notes_dir.to_path_buf()));
    let bnotes = BNotes::with_defaults(storage);

    let notes = bnotes.list_notes(&[])?;

    let mut stdout = colors::create_stdout(color);

    if notes.is_empty() {
        writeln!(stdout, "No notes found.")?;
        return Ok(());
    }

    let graph = bnotes.get_link_graph()?;

    writeln!(stdout, "Link Graph ({} notes):\n", notes.len())?;

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
        writeln!(stdout, "No links found between notes.")?;
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

        write!(stdout, "- {} (", note)?;
        stdout.set_color(&colors::highlight())?;
        write!(stdout, "->{} <-{}", out_count, in_count)?;
        stdout.reset()?;
        writeln!(stdout, ")")?;

        if let Some(links) = outbound
            && !links.is_empty()
        {
            let mut sorted_links: Vec<_> = links.iter().collect();
            sorted_links.sort();
            for link in sorted_links {
                write!(stdout, "  ")?;
                stdout.set_color(&colors::highlight())?;
                write!(stdout, "->")?;
                stdout.reset()?;
                writeln!(stdout, " {}", link)?;
            }
        }
    }

    write!(stdout, "\nTotal: ")?;
    stdout.set_color(&colors::highlight())?;
    write!(stdout, "{}", connected_notes.len())?;
    stdout.reset()?;
    writeln!(
        stdout,
        " connected {}",
        pluralize(connected_notes.len(), "note", "notes")
    )?;

    Ok(())
}

// ============================================================================
// Task Commands
// ============================================================================

pub fn task_list(
    notes_dir: &Path,
    tags: &[String],
    status: Option<String>,
    note_pattern: Option<&str>,
    sort_order: bnotes::TaskSortOrder,
    color: ColorChoice,
) -> Result<()> {
    validate_notes_dir(notes_dir)?;
    let storage = Box::new(RealStorage::new(notes_dir.to_path_buf()));
    let bnotes = BNotes::with_defaults(storage);

    let mut tasks = bnotes.list_tasks(&[], status.as_deref(), sort_order)?;

    // Filter by note pattern if provided
    if let Some(pattern) = note_pattern {
        // Use lowercase pattern and titles for case-insensitive matching
        let pattern_lower = pattern.to_lowercase();
        let matcher = WildMatch::new(&pattern_lower);
        tasks.retain(|task| matcher.matches(&task.note_title.to_lowercase()));
    }

    // Filter by tags if provided (AND logic with hierarchical matching)
    if !tags.is_empty() {
        // Normalize and deduplicate filter tags
        let mut filter_tags: Vec<String> = tags.iter()
            .map(|t| t.to_lowercase())
            .collect();
        filter_tags.sort();
        filter_tags.dedup();

        tasks.retain(|task| {
            filter_tags.iter().all(|filter_tag| {
                task.tags.iter().any(|task_tag| {
                    // Hierarchical: task_tag equals or starts with filter_tag/
                    task_tag == filter_tag || task_tag.starts_with(&format!("{}/", filter_tag))
                })
            })
        });
    }

    let mut stdout = colors::create_stdout(color);

    if tasks.is_empty() {
        writeln!(stdout, "No tasks found.")?;
        return Ok(());
    }

    // Calculate maximum column widths for alignment
    let max_note_width = tasks.iter()
        .map(|t| t.note_title.len())
        .max()
        .unwrap_or(0);

    let max_urgency_width = tasks.iter()
        .map(|t| t.urgency.as_ref().map(|u| u.to_string().len()).unwrap_or(0))
        .max()
        .unwrap_or(0);

    let max_priority_width = tasks.iter()
        .map(|t| t.priority.as_ref().map(|p| format!("({})", p).len()).unwrap_or(0))
        .max()
        .unwrap_or(0);

    // Display tasks with aligned columns
    for task in &tasks {
        // Note name in cyan, left-aligned with padding
        stdout.set_color(&colors::highlight())?;
        write!(stdout, "{:<width$}", task.note_title, width = max_note_width)?;
        stdout.reset()?;

        write!(stdout, " ")?;

        // Checkbox - [x] in green, [>] in yellow, [ ] default
        match task.status {
            bnotes::note::TaskStatus::Completed => {
                stdout.set_color(&colors::success())?;
                write!(stdout, "[x]")?;
                stdout.reset()?;
            }
            bnotes::note::TaskStatus::Migrated => {
                stdout.set_color(&colors::warning())?;
                write!(stdout, "[>]")?;
                stdout.reset()?;
            }
            bnotes::note::TaskStatus::Uncompleted => {
                write!(stdout, "[ ]")?;
            }
        }

        write!(stdout, " ")?;

        // Urgency: right-aligned with dynamic width
        let urgency_str = task.urgency.as_ref()
            .map(|u| u.to_string())
            .unwrap_or_default();
        write!(stdout, "{:>width$}", urgency_str, width = max_urgency_width)?;

        if max_urgency_width > 0 {
            write!(stdout, " ")?;
        }

        // Priority: dynamic width
        let priority_str = task.priority.as_ref()
            .map(|p| format!("({})", p))
            .unwrap_or_default();
        write!(stdout, "{:<width$}", priority_str, width = max_priority_width)?;

        if max_priority_width > 0 {
            write!(stdout, " ")?;
        }

        // Task text
        write!(stdout, "{} ", task.text)?;

        // Tags (if any)
        if !task.tags.is_empty() {
            stdout.set_color(&colors::highlight())?; // Cyan, same as note name
            for tag in &task.tags {
                write!(stdout, "@{} ", tag)?;
            }
            stdout.reset()?;
        }

        writeln!(stdout)?;
    }

    writeln!(
        stdout,
        "\nTotal: {} {}",
        tasks.len(),
        pluralize(tasks.len(), "task", "tasks")
    )?;

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
    notes_dir: &Path,
    action: PeriodicAction,
    template_override: Option<String>,
    print_path: bool,
) -> Result<()> {
    validate_notes_dir(notes_dir)?;
    let storage = Box::new(RealStorage::new(notes_dir.to_path_buf()));
    let bnotes = BNotes::with_defaults(storage);

    match action {
        PeriodicAction::Open(date_str) => {
            let period = if let Some(date) = date_str {
                P::from_date_str(&date)?
            } else {
                P::current()
            };

            periodic_open::<P>(notes_dir, &bnotes, period, template_override, print_path)?;
        }
        PeriodicAction::List => {
            periodic_list::<P>(&bnotes)?;
        }
        PeriodicAction::Prev => {
            let note_path = bnotes.navigate_periodic::<P>("prev", template_override.as_deref())?;
            launch_editor(notes_dir, &note_path, &bnotes, print_path)?;
        }
        PeriodicAction::Next => {
            let note_path = bnotes.navigate_periodic::<P>("next", template_override.as_deref())?;
            launch_editor(notes_dir, &note_path, &bnotes, print_path)?;
        }
    }

    Ok(())
}

fn periodic_open<P: bnotes::PeriodType>(
    notes_dir: &Path,
    bnotes: &bnotes::BNotes,
    period: P,
    template_override: Option<String>,
    print_path: bool,
) -> Result<()> {
    let note_path = PathBuf::from(period.filename());
    let full_path = notes_dir.join(&note_path);

    // If note doesn't exist, prompt to create
    if !full_path.exists() {
        if print_path {
            anyhow::bail!("Note doesn't exist: {}", period.identifier());
        }

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

    launch_editor(notes_dir, &note_path, bnotes, print_path)?;
    Ok(())
}

/// Specialized handler for weekly note commands with migration support
pub fn weekly(
    notes_dir: &Path,
    action: PeriodicAction,
    template_override: Option<String>,
    print_path: bool,
) -> Result<()> {
    use bnotes::Weekly;

    validate_notes_dir(notes_dir)?;
    let storage = Box::new(RealStorage::new(notes_dir.to_path_buf()));
    let bnotes = BNotes::with_defaults(storage);

    match action {
        PeriodicAction::Open(date_opt) => {
            let period = if let Some(date_str) = date_opt {
                Weekly::from_date_str(&date_str)?
            } else {
                Weekly::current()
            };

            weekly_open(notes_dir, &bnotes, period, template_override, print_path)?;
        }
        PeriodicAction::List => {
            periodic_list::<Weekly>(&bnotes)?;
        }
        PeriodicAction::Prev => {
            let note_path = bnotes.navigate_periodic::<Weekly>("prev", template_override.as_deref())?;
            launch_editor(notes_dir, &note_path, &bnotes, print_path)?;
        }
        PeriodicAction::Next => {
            let note_path = bnotes.navigate_periodic::<Weekly>("next", template_override.as_deref())?;
            launch_editor(notes_dir, &note_path, &bnotes, print_path)?;
        }
    }

    Ok(())
}

fn weekly_open(
    notes_dir: &Path,
    bnotes: &bnotes::BNotes,
    period: bnotes::Weekly,
    template_override: Option<String>,
    print_path: bool,
) -> Result<()> {
    let note_path = PathBuf::from(period.filename());
    let full_path = notes_dir.join(&note_path);

    // If note doesn't exist, prompt to create (and potentially migrate)
    if !full_path.exists() {
        if print_path {
            anyhow::bail!("Note doesn't exist: {}", period.identifier());
        }

        print!("Week {} doesn't exist. Create it? [Y/n] ", period.identifier());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input == "n" || input == "no" {
            return Ok(());
        }

        // Create the note with migration support
        let (_, migrated_count) = bnotes.create_weekly_with_migration(
            period,
            template_override.as_deref(),
            !print_path, // Only prompt for migration if not in print-path mode
        )?;

        if migrated_count > 0 {
            println!("Migrated {} tasks from previous week.", migrated_count);
        }
    }

    launch_editor(notes_dir, &note_path, bnotes, print_path)?;
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

fn launch_editor(notes_dir: &Path, note_path: &PathBuf, bnotes: &BNotes, print_path: bool) -> Result<()> {
    let full_path = notes_dir.join(note_path);

    // If print_path flag is set, print the path and exit
    if print_path {
        println!("{}", full_path.display());
        return Ok(());
    }

    // Capture state before editing (if possible)
    let before_state = bnotes::capture_note_state(&full_path).ok();

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

    let status = Command::new(&editor)
        .arg(&full_path)
        .status()
        .with_context(|| format!("Failed to open editor: {}", editor))?;

    if !status.success() {
        anyhow::bail!("Editor exited with status: {}", status);
    }

    // Update timestamp if enabled and file changed
    if bnotes.config().auto_update_timestamp
        && let Some(before) = before_state
        && let Ok(after) = bnotes::capture_note_state(&full_path)
        && before != after
        && let Err(e) = bnotes.update_note_timestamp(note_path)
    {
        eprintln!("Warning: Failed to update timestamp: {}", e);
    }

    Ok(())
}
