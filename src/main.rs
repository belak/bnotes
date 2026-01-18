mod cli;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use termcolor::ColorChoice;

// ============================================================================
// CLI Argument Parsing
// ============================================================================

#[derive(Parser)]
#[command(name = "bnotes")]
#[command(about = "A personal note-taking CLI with task management and wiki features")]
#[command(version)]
struct Cli {
    /// Notes directory (overrides $BNOTES_DIR)
    #[arg(long, global = true)]
    notes_dir: Option<PathBuf>,

    /// When to use colors (auto, always, never)
    #[arg(long, global = true, default_value = "auto", value_name = "WHEN")]
    color: ColorChoice,

    #[command(subcommand)]
    command: Commands,
}

/// Resolve notes directory from CLI arg, env var, or default
fn resolve_notes_dir(cli_arg: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(dir) = cli_arg {
        return Ok(dir);
    }

    if let Ok(env_dir) = std::env::var("BNOTES_DIR") {
        return Ok(PathBuf::from(env_dir));
    }

    // Default: $XDG_DATA_HOME/bnotes or ~/.local/share/bnotes
    let data_home = if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(xdg_data)
    } else {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        PathBuf::from(home).join(".local/share")
    };

    Ok(data_home.join("bnotes"))
}

#[derive(Subcommand)]
enum Commands {
    /// Full-text search across all notes
    Search {
        /// Search query
        query: String,

        /// Maximum matches to show per note
        #[arg(long, default_value = "3")]
        limit: usize,
    },

    /// Create a new note
    New {
        /// Note title
        title: Option<String>,

        /// Template name to use
        #[arg(long)]
        template: Option<String>,
    },

    /// Open a note in the default editor
    Edit {
        /// Note title
        title: String,
    },

    /// List open tasks (alias for 'task list --status open')
    Tasks,

    /// Check for issues in the note collection
    Doctor,

    /// Sync notes with git remote (commit, pull, push)
    Sync {
        /// Custom commit message
        #[arg(long, short)]
        message: Option<String>,
    },

    /// Pull changes from git remote
    Pull,

    /// Note management commands
    #[command(subcommand)]
    Note(NoteCommands),

    /// Task management commands
    #[command(subcommand)]
    Task(TaskCommands),

    /// Daily note management
    Daily {
        /// Date (YYYY-MM-DD format) or 'prev'/'next'
        date: Option<String>,

        /// Override configured template
        #[arg(long)]
        template: Option<String>,

        #[command(subcommand)]
        subcommand: Option<PeriodicSubcommands>,
    },

    /// Weekly note management
    Weekly {
        /// Date (YYYY-MM-DD format) or 'prev'/'next'
        date: Option<String>,

        /// Override configured template
        #[arg(long)]
        template: Option<String>,

        #[command(subcommand)]
        subcommand: Option<PeriodicSubcommands>,
    },

    /// Quarterly note management
    Quarterly {
        /// Date (YYYY-MM-DD format), quarter shortcut (q1-q4), or 'prev'/'next'
        date: Option<String>,

        /// Override configured template
        #[arg(long)]
        template: Option<String>,

        #[command(subcommand)]
        subcommand: Option<PeriodicSubcommands>,
    },
}

#[derive(Subcommand)]
enum NoteCommands {
    /// List all notes
    List {
        /// Filter by tags
        #[arg(long = "tag")]
        tags: Vec<String>,
    },

    /// Display a note
    Show {
        /// Note title
        title: String,
    },

    /// Show links to and from a note
    Links {
        /// Note title
        title: String,
    },

    /// Show link graph of all notes
    Graph,
}

#[derive(Subcommand)]
enum TaskCommands {
    /// List tasks across all notes
    List {
        /// Filter by tags
        #[arg(long = "tag")]
        tags: Vec<String>,

        /// Filter by status (open or done)
        #[arg(long)]
        status: Option<String>,
    },

    /// Show a specific task with context
    Show {
        /// Task ID (e.g., "project-notes#2")
        task_id: String,
    },
}

#[derive(Subcommand)]
enum PeriodicSubcommands {
    /// List all notes of this period type
    List,
    /// Navigate to previous period
    Prev,
    /// Navigate to next period
    Next,
}

// ============================================================================
// Main Entry Point
// ============================================================================

fn main() -> Result<()> {
    let cli_args = Cli::parse();
    let notes_dir = resolve_notes_dir(cli_args.notes_dir)?;

    match cli_args.command {
        Commands::Search { query, limit } => {
            cli::commands::search(&notes_dir, &query, limit, cli_args.color)?;
        }
        Commands::New { title, template } => {
            cli::commands::new(&notes_dir, title, template)?;
        }
        Commands::Edit { title } => {
            cli::commands::edit(&notes_dir, &title)?;
        }
        Commands::Tasks => {
            cli::commands::task_list(&notes_dir, &[], Some("open".to_string()), cli_args.color)?;
        }
        Commands::Doctor => {
            cli::commands::doctor(&notes_dir, cli_args.color)?;
        }
        Commands::Sync { message } => {
            cli::commands::sync(&notes_dir, message, cli_args.color)?;
        }
        Commands::Pull => {
            cli::commands::pull(&notes_dir, cli_args.color)?;
        }
        Commands::Note(note_cmd) => match note_cmd {
            NoteCommands::List { tags } => {
                cli::commands::note_list(&notes_dir, &tags, cli_args.color)?;
            }
            NoteCommands::Show { title } => {
                cli::commands::note_show(&notes_dir, &title)?;
            }
            NoteCommands::Links { title } => {
                cli::commands::note_links(&notes_dir, &title, cli_args.color)?;
            }
            NoteCommands::Graph => {
                cli::commands::note_graph(&notes_dir, cli_args.color)?;
            }
        },
        Commands::Task(task_cmd) => match task_cmd {
            TaskCommands::List { tags, status } => {
                cli::commands::task_list(&notes_dir, &tags, status, cli_args.color)?;
            }
            TaskCommands::Show { task_id } => {
                cli::commands::task_show(&notes_dir, &task_id, cli_args.color)?;
            }
        },
        Commands::Daily {
            date,
            template,
            subcommand,
        } => {
            use bnotes::Daily;

            let action = if let Some(cmd) = subcommand {
                match cmd {
                    PeriodicSubcommands::List => cli::PeriodicAction::List,
                    PeriodicSubcommands::Prev => cli::PeriodicAction::Prev,
                    PeriodicSubcommands::Next => cli::PeriodicAction::Next,
                }
            } else if date.as_deref() == Some("prev") {
                cli::PeriodicAction::Prev
            } else if date.as_deref() == Some("next") {
                cli::PeriodicAction::Next
            } else if date.as_deref() == Some("list") {
                cli::PeriodicAction::List
            } else {
                cli::PeriodicAction::Open(date)
            };

            cli::commands::periodic::<Daily>(&notes_dir, action, template)?;
        }
        Commands::Weekly {
            date,
            template,
            subcommand,
        } => {
            use bnotes::Weekly;

            let action = if let Some(cmd) = subcommand {
                match cmd {
                    PeriodicSubcommands::List => cli::PeriodicAction::List,
                    PeriodicSubcommands::Prev => cli::PeriodicAction::Prev,
                    PeriodicSubcommands::Next => cli::PeriodicAction::Next,
                }
            } else if date.as_deref() == Some("prev") {
                cli::PeriodicAction::Prev
            } else if date.as_deref() == Some("next") {
                cli::PeriodicAction::Next
            } else if date.as_deref() == Some("list") {
                cli::PeriodicAction::List
            } else {
                cli::PeriodicAction::Open(date)
            };

            cli::commands::periodic::<Weekly>(&notes_dir, action, template)?;
        }
        Commands::Quarterly {
            date,
            template,
            subcommand,
        } => {
            use bnotes::Quarterly;

            let action = if let Some(cmd) = subcommand {
                match cmd {
                    PeriodicSubcommands::List => cli::PeriodicAction::List,
                    PeriodicSubcommands::Prev => cli::PeriodicAction::Prev,
                    PeriodicSubcommands::Next => cli::PeriodicAction::Next,
                }
            } else if date.as_deref() == Some("prev") {
                cli::PeriodicAction::Prev
            } else if date.as_deref() == Some("next") {
                cli::PeriodicAction::Next
            } else if date.as_deref() == Some("list") {
                cli::PeriodicAction::List
            } else {
                cli::PeriodicAction::Open(date)
            };

            cli::commands::periodic::<Quarterly>(&notes_dir, action, template)?;
        }
    }

    Ok(())
}
