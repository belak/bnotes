use bnotes::{BNotes, MemoryStorage, MatchLocation, Storage};
use std::path::Path;

#[test]
fn test_search_with_multiple_heading_levels() {
    let storage = Box::new(MemoryStorage::new());
    storage
        .write(
            Path::new("project.md"),
            r#"---
tags: [planning, notes]
---

# Project Ideas

## Planning

### Q1 Goals

Review the project budget and timeline.

### Q2 Goals

Discuss project milestones.

## Resources

### Team

Assign team members to the project."#,
        )
        .unwrap();

    let bnotes = BNotes::with_defaults(storage);
    let results = bnotes.search("project").unwrap();

    assert_eq!(results.len(), 1);

    let search_match = &results[0];
    assert_eq!(search_match.note.title, "Project Ideas");

    // Should have title match + tag match + 3 content matches
    assert!(search_match.locations.len() >= 4);

    // Check for title match
    let has_title_match = search_match.locations.iter().any(|loc| matches!(loc, MatchLocation::Title { .. }));
    assert!(has_title_match, "Should have title match");

    // Check for content matches with breadcrumbs
    let content_matches: Vec<_> = search_match.locations.iter().filter_map(|loc| {
        if let MatchLocation::Content { breadcrumb, .. } = loc {
            Some(breadcrumb)
        } else {
            None
        }
    }).collect();

    assert!(!content_matches.is_empty(), "Should have content matches");

    // Verify breadcrumb format
    for breadcrumb in content_matches {
        assert!(!breadcrumb.is_empty(), "Breadcrumb should not be empty");
        // Should have markdown heading markers
        assert!(breadcrumb.iter().any(|h| h.starts_with('#')));
    }
}

#[test]
fn test_search_match_before_first_heading() {
    let storage = Box::new(MemoryStorage::new());
    storage
        .write(
            Path::new("simple.md"),
            r#"---
tags: [notes]
---

# Simple Note

Some preamble text with project information before any headings.

## Section

More project details here."#,
        )
        .unwrap();

    let bnotes = BNotes::with_defaults(storage);
    let results = bnotes.search("preamble").unwrap();

    assert_eq!(results.len(), 1);

    let search_match = &results[0];

    // Find the content match for "preamble"
    let preamble_match = search_match.locations.iter().find_map(|loc| {
        if let MatchLocation::Content { breadcrumb, snippet, .. } = loc {
            if snippet.to_lowercase().contains("preamble") {
                Some((breadcrumb, snippet))
            } else {
                None
            }
        } else {
            None
        }
    });

    assert!(preamble_match.is_some(), "Should find preamble match");

    let (breadcrumb, snippet) = preamble_match.unwrap();

    // Match before first heading should have breadcrumb showing document start context
    assert!(
        breadcrumb.is_empty() || breadcrumb.iter().any(|h| h.contains("Simple Note")),
        "Should have appropriate breadcrumb for match before headings"
    );
    assert!(snippet.contains("preamble"));
}

#[test]
fn test_search_multiple_matches_same_section() {
    let storage = Box::new(MemoryStorage::new());
    storage
        .write(
            Path::new("meeting.md"),
            r#"---
tags: [planning]
---

# Meeting Notes

## Discussion

The project team reviewed project status and project timeline."#,
        )
        .unwrap();

    let bnotes = BNotes::with_defaults(storage);
    let results = bnotes.search("project").unwrap();

    assert_eq!(results.len(), 1);

    let search_match = &results[0];

    // Should have multiple matches (title + content matches)
    assert!(search_match.locations.len() >= 2);

    // Check content matches
    let content_matches: Vec<_> = search_match.locations.iter().filter_map(|loc| {
        if let MatchLocation::Content { breadcrumb, snippet, .. } = loc {
            Some((breadcrumb, snippet))
        } else {
            None
        }
    }).collect();

    assert!(!content_matches.is_empty(), "Should have content matches");

    // Verify snippet contains the match
    for (_, snippet) in content_matches {
        assert!(snippet.to_lowercase().contains("project"));
    }
}

#[test]
fn test_search_title_tag_content_variants() {
    let storage = Box::new(MemoryStorage::new());

    // Note with title match
    storage
        .write(
            Path::new("project-ideas.md"),
            r#"---
tags: [planning, notes]
---

# Project Ideas

Some content here."#,
        )
        .unwrap();

    // Note with tag match only
    storage
        .write(
            Path::new("meeting.md"),
            r#"---
tags: [planning, project, daily]
---

# Meeting Notes

Discussion about timelines."#,
        )
        .unwrap();

    // Note with content match only
    storage
        .write(
            Path::new("budget.md"),
            r#"---
tags: [finance, reporting]
---

# Budget Report

The project expenses are within budget."#,
        )
        .unwrap();

    let bnotes = BNotes::with_defaults(storage);
    let results = bnotes.search("project").unwrap();

    assert_eq!(results.len(), 3, "Should find 3 notes");

    // Verify each note has appropriate match types
    for search_match in &results {
        assert!(!search_match.locations.is_empty(), "Each note should have at least one match");

        match search_match.note.title.as_str() {
            "Project Ideas" => {
                // Should have title match
                let has_title = search_match.locations.iter().any(|loc| matches!(loc, MatchLocation::Title { .. }));
                assert!(has_title, "Project Ideas should have title match");
            }
            "Meeting Notes" => {
                // Should have tag match
                let has_tag = search_match.locations.iter().any(|loc| matches!(loc, MatchLocation::Tag { .. }));
                assert!(has_tag, "Meeting Notes should have tag match");
            }
            "Budget Report" => {
                // Should have content match
                let has_content = search_match.locations.iter().any(|loc| matches!(loc, MatchLocation::Content { .. }));
                assert!(has_content, "Budget Report should have content match");
            }
            _ => panic!("Unexpected note: {}", search_match.note.title),
        }
    }
}

#[test]
fn test_search_snippet_extraction() {
    let storage = Box::new(MemoryStorage::new());
    storage
        .write(
            Path::new("long.md"),
            r#"---
tags: [notes]
---

# Long Note

## Section

This is a very long paragraph with lots of words before the match. The project timeline is critical for success. There are also many words after the match to test snippet extraction."#,
        )
        .unwrap();

    let bnotes = BNotes::with_defaults(storage);
    let results = bnotes.search("project").unwrap();

    assert_eq!(results.len(), 1);

    let search_match = &results[0];

    // Find content match
    let content_match = search_match.locations.iter().find_map(|loc| {
        if let MatchLocation::Content { snippet, .. } = loc {
            if snippet.to_lowercase().contains("project") {
                Some(snippet)
            } else {
                None
            }
        } else {
            None
        }
    });

    assert!(content_match.is_some(), "Should find content match");

    let snippet = content_match.unwrap();

    // Snippet should contain the match
    assert!(snippet.to_lowercase().contains("project"));

    // Snippet should be shorter than full content (extraction working)
    assert!(snippet.len() < 200, "Snippet should be extracted, not full content");

    // Snippet should have context around match
    assert!(snippet.contains("timeline"));
}
