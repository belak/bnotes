//! Link extraction and graph building for wiki-style links

use crate::repository_lib::Note;
use pulldown_cmark::{Event, Parser};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct LinkGraph {
    /// Map from note title to set of titles it links to (outbound)
    pub outbound: HashMap<String, HashSet<String>>,
    /// Map from note title to set of titles that link to it (inbound)
    pub inbound: HashMap<String, HashSet<String>>,
}

impl LinkGraph {
    pub fn new() -> Self {
        Self {
            outbound: HashMap::new(),
            inbound: HashMap::new(),
        }
    }

    /// Build a link graph from a collection of notes
    pub fn build(notes: &[Note]) -> Self {
        let mut graph = Self::new();

        // Create title -> note mapping for resolving links
        let title_map: HashMap<String, &Note> = notes
            .iter()
            .map(|n| (n.title.to_lowercase(), n))
            .collect();

        for note in notes {
            let links = extract_wiki_links(&note.content);
            let note_title = note.title.clone();

            // Initialize outbound set for this note
            graph
                .outbound
                .entry(note_title.clone())
                .or_default();

            for link_text in links {
                let link_lower = link_text.to_lowercase();

                // Try to resolve the link
                if title_map.contains_key(&link_lower) {
                    // Add to outbound links
                    graph
                        .outbound
                        .entry(note_title.clone())
                        .or_default()
                        .insert(link_text.clone());

                    // Add to inbound links for the target
                    graph
                        .inbound
                        .entry(link_text)
                        .or_default()
                        .insert(note_title.clone());
                }
            }
        }

        graph
    }

    /// Get notes that have no incoming or outgoing links
    pub fn orphaned_notes(&self, all_note_titles: &[String]) -> Vec<String> {
        all_note_titles
            .iter()
            .filter(|title| {
                let has_outbound = self
                    .outbound
                    .get(*title)
                    .map(|set| !set.is_empty())
                    .unwrap_or(false);

                let has_inbound = self
                    .inbound
                    .get(*title)
                    .map(|set| !set.is_empty())
                    .unwrap_or(false);

                !has_outbound && !has_inbound
            })
            .cloned()
            .collect()
    }

    /// Find broken links (links to non-existent notes)
    pub fn broken_links(&self, notes: &[Note]) -> HashMap<String, Vec<String>> {
        let title_set: HashSet<String> = notes
            .iter()
            .map(|n| n.title.to_lowercase())
            .collect();

        let mut broken = HashMap::new();

        for note in notes {
            let links = extract_wiki_links(&note.content);
            let broken_in_note: Vec<String> = links
                .into_iter()
                .filter(|link| !title_set.contains(&link.to_lowercase()))
                .collect();

            if !broken_in_note.is_empty() {
                broken.insert(note.title.clone(), broken_in_note);
            }
        }

        broken
    }
}

/// Extract wiki-style links from markdown content
///
/// Parses markdown using pulldown-cmark and extracts [[wiki link]] patterns
/// from text events. Wiki links are not standard markdown, so they appear
/// as plain text in the event stream.
pub fn extract_wiki_links(content: &str) -> Vec<String> {
    let parser = Parser::new(content);
    let mut links = Vec::new();
    let mut accumulated_text = String::new();

    for event in parser {
        match event {
            Event::Text(text) => {
                // Accumulate text to handle wiki links that might be split across events
                accumulated_text.push_str(text.as_ref());
            }
            Event::Code(text) => {
                // Also check code spans for wiki links
                accumulated_text.push_str(text.as_ref());
            }
            // When we hit a non-text event, process accumulated text and reset
            _ => {
                if !accumulated_text.is_empty() {
                    extract_wiki_links_from_text(&accumulated_text, &mut links);
                    accumulated_text.clear();
                }
            }
        }
    }

    // Process any remaining accumulated text
    if !accumulated_text.is_empty() {
        extract_wiki_links_from_text(&accumulated_text, &mut links);
    }

    links
}

/// Helper function to extract wiki links from a text string
fn extract_wiki_links_from_text(text: &str, links: &mut Vec<String>) {
    let mut start = 0;

    while let Some(begin) = text[start..].find("[[") {
        let begin = start + begin;
        if let Some(end) = text[begin + 2..].find("]]") {
            let end = begin + 2 + end;
            let link_text = &text[begin + 2..end];
            links.push(link_text.to_string());
            start = end + 2;
        } else {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_extract_wiki_links() {
        let content = r#"
# My Note

See also [[Other Note]] and [[Another Note]].

More content with [[Third Link]].
"#;

        let links = extract_wiki_links(content);
        assert_eq!(links.len(), 3);
        assert!(links.contains(&"Other Note".to_string()));
        assert!(links.contains(&"Another Note".to_string()));
        assert!(links.contains(&"Third Link".to_string()));
    }

    #[test]
    fn test_link_graph() {
        let note1 = Note::parse(
            Path::new("note1.md"),
            r#"
# Note One

Links to [[Note Two]].
"#,
        )
        .unwrap();

        let note2 = Note::parse(
            Path::new("note2.md"),
            r#"
# Note Two

Links to [[Note One]] and [[Note Three]].
"#,
        )
        .unwrap();

        let note3 = Note::parse(Path::new("note3.md"), "# Note Three\n\nNo links.").unwrap();

        let notes = vec![note1, note2, note3];
        let graph = LinkGraph::build(&notes);

        // Note One links to Note Two
        assert!(graph
            .outbound
            .get("Note One")
            .unwrap()
            .contains("Note Two"));

        // Note Two is linked from Note One
        assert!(graph.inbound.get("Note Two").unwrap().contains("Note One"));

        // Note Two links to Note One and Note Three
        assert_eq!(graph.outbound.get("Note Two").unwrap().len(), 2);

        // Note Three has no outbound links
        assert!(graph
            .outbound
            .get("Note Three")
            .unwrap_or(&HashSet::new())
            .is_empty());
    }
}
