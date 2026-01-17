use crate::config::CLIConfig;
use bnotes::{BNotes, PeriodType, RealStorage};
use anyhow::{Context, Result};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

pub enum PeriodicAction {
    Open(Option<String>),
    List,
    Prev,
    Next,
}

/// Generic handler for periodic note commands
pub fn handle_periodic<P: PeriodType>(
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

            open_period::<P>(&cli_config, &bnotes, period, template_override)?;
        }
        PeriodicAction::List => {
            list_periods::<P>(&bnotes)?;
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

fn open_period<P: PeriodType>(
    cli_config: &CLIConfig,
    bnotes: &BNotes,
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

fn list_periods<P: PeriodType>(bnotes: &BNotes) -> Result<()> {
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
