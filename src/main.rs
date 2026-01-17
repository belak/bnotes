mod cli;
mod git;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

// ============================================================================
// CLI Argument Parsing
// ============================================================================

#[derive(Parser)]
#[command(name = "bnotes")]
#[command(about = "A personal note-taking CLI with task management and wiki features")]
#[command(version)]
struct Cli {
    /// Path to config file (overrides $BNOTES_CONFIG and default)
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Full-text search across all notes
    Search {
        /// Search query
        query: String,
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

    /// Initialize bnotes configuration
    Init {
        /// Notes directory path
        #[arg(long)]
        notes_dir: Option<PathBuf>,
    },

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

    match cli_args.command {
        Commands::Search { query } => {
            cli::commands::search(cli_args.config, &query)?;
        }
        Commands::New { title, template } => {
            cli::commands::new(cli_args.config, title, template)?;
        }
        Commands::Edit { title } => {
            cli::commands::edit(cli_args.config, &title)?;
        }
        Commands::Tasks => {
            cli::commands::task_list(cli_args.config, &[], Some("open".to_string()))?;
        }
        Commands::Init { notes_dir } => {
            cli::commands::init(notes_dir)?;
        }
        Commands::Doctor => {
            cli::commands::doctor(cli_args.config)?;
        }
        Commands::Sync { message } => {
            cli::commands::sync(cli_args.config, message)?;
        }
        Commands::Pull => {
            cli::commands::pull(cli_args.config)?;
        }
        Commands::Note(note_cmd) => match note_cmd {
            NoteCommands::List { tags } => {
                cli::commands::note_list(cli_args.config, &tags)?;
            }
            NoteCommands::Show { title } => {
                cli::commands::note_show(cli_args.config, &title)?;
            }
            NoteCommands::Links { title } => {
                cli::commands::note_links(cli_args.config, &title)?;
            }
            NoteCommands::Graph => {
                cli::commands::note_graph(cli_args.config)?;
            }
        },
        Commands::Task(task_cmd) => match task_cmd {
            TaskCommands::List { tags, status } => {
                cli::commands::task_list(cli_args.config, &tags, status)?;
            }
            TaskCommands::Show { task_id } => {
                cli::commands::task_show(cli_args.config, &task_id)?;
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

            cli::commands::periodic::<Daily>(cli_args.config, action, template)?;
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

            cli::commands::periodic::<Weekly>(cli_args.config, action, template)?;
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

            cli::commands::periodic::<Quarterly>(cli_args.config, action, template)?;
        }
    }

    Ok(())
}
