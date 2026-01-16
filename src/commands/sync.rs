use crate::git::GitRepo;
use crate::util::CommandContext;
use anyhow::Result;
use std::path::PathBuf;

/// Sync notes with git remote (commit, pull, push)
pub fn sync(config_path: Option<PathBuf>, message: Option<String>) -> Result<()> {
    let ctx = CommandContext::load(config_path)?;
    let repo = GitRepo::new(ctx.config.notes_dir)?;

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
        let subject = message.unwrap_or_else(|| {
            format!("bnotes sync: {}", GitRepo::get_timestamp())
        });

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

        println!("Synced successfully: committed {} changes, pulled, and pushed", num_changes);
    } else {
        // No local changes, just pull and push
        repo.pull()?;
        repo.push()?;

        println!("Synced successfully: pulled and pushed");
    }

    Ok(())
}

/// Pull changes from git remote
pub fn pull(config_path: Option<PathBuf>) -> Result<()> {
    let ctx = CommandContext::load(config_path)?;
    let repo = GitRepo::new(ctx.config.notes_dir)?;

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
