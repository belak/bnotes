use crate::note::Note;
use anyhow::{Context, Result};
use std::path::Path;
use walkdir::WalkDir;

pub struct Repository {
    notes_dir: std::path::PathBuf,
}

impl Repository {
    pub fn new(notes_dir: impl AsRef<Path>) -> Self {
        Self {
            notes_dir: notes_dir.as_ref().to_path_buf(),
        }
    }

    /// Discover all notes in the repository
    pub fn discover_notes(&self) -> Result<Vec<Note>> {
        if !self.notes_dir.exists() {
            anyhow::bail!(
                "Notes directory not found: {}",
                self.notes_dir.display()
            );
        }

        let mut notes = Vec::new();

        for entry in WalkDir::new(&self.notes_dir)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                // Skip hidden files and directories (starting with .)
                e.file_name()
                    .to_str()
                    .map(|s| !s.starts_with('.'))
                    .unwrap_or(false)
            })
        {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();

            // Only process .md files
            if path.is_file()
                && path.extension().and_then(|s| s.to_str()) == Some("md")
            {
                match Note::from_file(path) {
                    Ok(note) => notes.push(note),
                    Err(e) => {
                        eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(notes)
    }

    /// Find a note by title (case-insensitive)
    pub fn find_by_title(&self, title: &str) -> Result<Vec<Note>> {
        let all_notes = self.discover_notes()?;
        let title_lower = title.to_lowercase();

        let matches: Vec<Note> = all_notes
            .into_iter()
            .filter(|note| note.title.to_lowercase() == title_lower)
            .collect();

        Ok(matches)
    }

    /// Search notes by query (case-insensitive substring matching)
    pub fn search(&self, query: &str) -> Result<Vec<Note>> {
        let all_notes = self.discover_notes()?;
        let query_lower = query.to_lowercase();

        let matches: Vec<Note> = all_notes
            .into_iter()
            .filter(|note| {
                note.content.to_lowercase().contains(&query_lower)
                    || note.title.to_lowercase().contains(&query_lower)
                    || note
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .collect();

        Ok(matches)
    }

    /// Filter notes by tags
    pub fn filter_by_tags(&self, tags: &[String]) -> Result<Vec<Note>> {
        let all_notes = self.discover_notes()?;

        let matches: Vec<Note> = all_notes
            .into_iter()
            .filter(|note| {
                tags.iter()
                    .all(|tag| note.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)))
            })
            .collect();

        Ok(matches)
    }
}
