mod commands;
mod config;
mod note;
mod repository;
mod task;
mod template;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

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

    /// Note management commands
    #[command(subcommand)]
    Note(NoteCommands),

    /// Task management commands
    #[command(subcommand)]
    Task(TaskCommands),
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

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Search { query } => {
            commands::search::run(cli.config, &query)?;
        }
        Commands::New { title, template } => {
            commands::new::run(cli.config, title, template)?;
        }
        Commands::Edit { title } => {
            commands::edit::run(cli.config, &title)?;
        }
        Commands::Tasks => {
            commands::task::list(cli.config, &[], Some("open".to_string()))?;
        }
        Commands::Init { notes_dir } => {
            commands::init::run(notes_dir)?;
        }
        Commands::Doctor => {
            println!("Running doctor checks");
            // TODO: Implement doctor
        }
        Commands::Note(note_cmd) => match note_cmd {
            NoteCommands::List { tags } => {
                commands::note::list(cli.config, &tags)?;
            }
            NoteCommands::Show { title } => {
                commands::note::show(cli.config, &title)?;
            }
            NoteCommands::Links { title } => {
                println!("Showing links for: {}", title);
                // TODO: Implement note links
            }
            NoteCommands::Graph => {
                println!("Showing link graph");
                // TODO: Implement note graph
            }
        },
        Commands::Task(task_cmd) => match task_cmd {
            TaskCommands::List { tags, status } => {
                commands::task::list(cli.config, &tags, status)?;
            }
            TaskCommands::Show { task_id } => {
                commands::task::show(cli.config, &task_id)?;
            }
        },
    }

    Ok(())
}
