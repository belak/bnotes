use crate::periodic::PeriodType;
use crate::template;
use crate::util::CommandContext;
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
    let ctx = CommandContext::load(config_path)?;

    match action {
        PeriodicAction::Open(date_str) => {
            let period = if let Some(date) = date_str {
                P::from_date_str(&date)?
            } else {
                P::current()
            };

            open_period::<P>(&ctx, period, template_override)?;
        }
        PeriodicAction::List => {
            list_periods::<P>(&ctx)?;
        }
        PeriodicAction::Prev => {
            let period = P::current().prev();
            open_period::<P>(&ctx, period, template_override)?;
        }
        PeriodicAction::Next => {
            let period = P::current().next();
            open_period::<P>(&ctx, period, template_override)?;
        }
    }

    Ok(())
}

fn open_period<P: PeriodType>(
    ctx: &CommandContext,
    period: P,
    template_override: Option<String>,
) -> Result<()> {
    let note_path = ctx.config.notes_dir.join(period.filename());

    // If note doesn't exist, prompt to create
    if !note_path.exists() {
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

        // Create the note
        create_period_note(ctx, &period, &note_path, template_override)?;
    }

    // Open in editor
    let status = Command::new(&ctx.config.editor)
        .arg(&note_path)
        .status()
        .with_context(|| format!("Failed to open editor: {}", ctx.config.editor))?;

    if !status.success() {
        anyhow::bail!("Editor exited with status: {}", status);
    }

    Ok(())
}

fn create_period_note<P: PeriodType>(
    ctx: &CommandContext,
    period: &P,
    note_path: &PathBuf,
    template_override: Option<String>,
) -> Result<()> {
    let template_dir = ctx.config.template_dir_path();

    // Determine which template to use
    let template_name = if let Some(override_name) = template_override {
        override_name
    } else {
        // Get configured template based on period type
        match P::template_name() {
            "daily" => ctx.config.periodic.daily_template.clone(),
            "weekly" => ctx.config.periodic.weekly_template.clone(),
            "quarterly" => ctx.config.periodic.quarterly_template.clone(),
            _ => format!("{}.md", P::template_name()),
        }
    };

    let template_path = template_dir.join(&template_name);

    // Generate content
    let content = if template_path.exists() {
        template::render(&template_path, &period.identifier())?
    } else {
        // Minimal note with just title
        format!("# {}\n\n", period.identifier())
    };

    // Ensure notes directory exists
    std::fs::create_dir_all(&ctx.config.notes_dir)
        .with_context(|| format!("Failed to create notes directory: {}", ctx.config.notes_dir.display()))?;

    // Write note
    std::fs::write(note_path, content)
        .with_context(|| format!("Failed to write note: {}", note_path.display()))?;

    Ok(())
}

fn list_periods<P: PeriodType>(ctx: &CommandContext) -> Result<()> {
    let mut periods: Vec<P> = Vec::new();

    // Scan notes directory for matching files
    for entry in std::fs::read_dir(&ctx.config.notes_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                // Try to parse as this period type
                if let Ok(period) = P::from_date_str(stem) {
                    // Verify it matches the filename format
                    if period.identifier() == stem {
                        periods.push(period);
                    }
                }
            }
        }
    }

    if periods.is_empty() {
        println!("No {} notes found.", P::template_name());
        return Ok(());
    }

    // Sort by identifier (which is chronological)
    periods.sort_by(|a, b| a.identifier().cmp(&b.identifier()));

    for period in periods {
        println!("{}", period.display_string());
    }

    Ok(())
}
