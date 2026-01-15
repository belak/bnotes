use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use std::path::PathBuf;
use std::process::Command;

pub struct GitRepo {
    notes_dir: PathBuf,
}

impl GitRepo {
    pub fn new(notes_dir: PathBuf) -> Result<Self> {
        Ok(Self { notes_dir })
    }

    /// Check if the notes directory is a git repository
    pub fn check_is_repo(&self) -> Result<()> {
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("--git-dir")
            .current_dir(&self.notes_dir)
            .output()
            .context("Failed to execute git command")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Error: Not a git repository\n\
                The notes directory is not initialized with git.\n\n\
                Run 'git init' in your notes directory to get started."
            ));
        }

        Ok(())
    }

    /// Check if a remote repository is configured
    pub fn check_has_remote(&self) -> Result<()> {
        let output = Command::new("git")
            .arg("remote")
            .current_dir(&self.notes_dir)
            .output()
            .context("Failed to execute git command")?;

        if !output.status.success() {
            return Err(anyhow!("Failed to check git remote"));
        }

        let remotes = String::from_utf8_lossy(&output.stdout);
        if remotes.trim().is_empty() {
            return Err(anyhow!(
                "Error: No remote repository configured\n\
                Run 'git remote add origin <url>' to configure a remote."
            ));
        }

        Ok(())
    }

    /// Check if there are uncommitted changes
    pub fn has_uncommitted_changes(&self) -> Result<bool> {
        let output = Command::new("git")
            .arg("status")
            .arg("--porcelain")
            .current_dir(&self.notes_dir)
            .output()
            .context("Failed to execute git status")?;

        if !output.status.success() {
            return Err(anyhow!("Failed to check git status"));
        }

        Ok(!output.stdout.is_empty())
    }

    /// Get list of files with conflicts
    pub fn get_conflicted_files(&self) -> Result<Vec<String>> {
        let output = Command::new("git")
            .arg("diff")
            .arg("--name-only")
            .arg("--diff-filter=U")
            .current_dir(&self.notes_dir)
            .output()
            .context("Failed to get conflicted files")?;

        if !output.status.success() {
            return Err(anyhow!("Failed to get conflicted files"));
        }

        let files: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect();

        Ok(files)
    }

    /// Stage all changes
    pub fn stage_all(&self) -> Result<()> {
        let output = Command::new("git")
            .arg("add")
            .arg(".")
            .current_dir(&self.notes_dir)
            .output()
            .context("Failed to stage changes")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to stage changes: {}", stderr));
        }

        Ok(())
    }

    /// Create a commit with the given message
    pub fn commit(&self, message: &str) -> Result<()> {
        let output = Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg(message)
            .current_dir(&self.notes_dir)
            .output()
            .context("Failed to commit changes")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to commit changes: {}", stderr));
        }

        Ok(())
    }

    /// Pull changes from remote with merge strategy
    pub fn pull(&self) -> Result<()> {
        let output = Command::new("git")
            .arg("pull")
            .arg("--no-rebase")
            .current_dir(&self.notes_dir)
            .output()
            .context("Failed to pull changes")?;

        if !output.status.success() {
            // Check if there are merge conflicts
            let conflicted_files = self.get_conflicted_files()?;
            if !conflicted_files.is_empty() {
                let files_list = conflicted_files
                    .iter()
                    .map(|f| format!("  - {}", f))
                    .collect::<Vec<_>>()
                    .join("\n");

                return Err(anyhow!(
                    "Error: Merge conflicts detected\n\n\
                    The following files have conflicts:\n\
                    {}\n\n\
                    Resolve conflicts manually and run 'git merge --continue'",
                    files_list
                ));
            }

            // Some other error
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to pull changes: {}", stderr));
        }

        Ok(())
    }

    /// Push changes to remote
    pub fn push(&self) -> Result<()> {
        let output = Command::new("git")
            .arg("push")
            .current_dir(&self.notes_dir)
            .output()
            .context("Failed to push changes")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to push changes: {}", stderr));
        }

        Ok(())
    }

    /// Stash uncommitted changes with a timestamped message
    pub fn stash_push(&self, message: &str) -> Result<()> {
        let output = Command::new("git")
            .arg("stash")
            .arg("push")
            .arg("-m")
            .arg(message)
            .current_dir(&self.notes_dir)
            .output()
            .context("Failed to stash changes")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to stash changes: {}", stderr));
        }

        Ok(())
    }

    /// Pop the most recent stash
    pub fn stash_pop(&self) -> Result<()> {
        let output = Command::new("git")
            .arg("stash")
            .arg("pop")
            .current_dir(&self.notes_dir)
            .output()
            .context("Failed to pop stash")?;

        if !output.status.success() {
            // Check if there are conflicts
            let conflicted_files = self.get_conflicted_files()?;
            if !conflicted_files.is_empty() {
                let files_list = conflicted_files
                    .iter()
                    .map(|f| format!("  - {}", f))
                    .collect::<Vec<_>>()
                    .join("\n");

                eprintln!(
                    "Warning: Conflicts occurred while reapplying stashed changes\n\n\
                    The following files have conflicts:\n\
                    {}\n\n\
                    The stash has been applied but conflicts need resolution.\n\
                    Run 'git status' to see details.\n\
                    Your stashed changes are preserved in the stash list.",
                    files_list
                );

                // Return Ok because this is a warning, not a fatal error
                return Ok(());
            }

            // Some other error
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to pop stash: {}", stderr));
        }

        Ok(())
    }

    /// Generate a summary of changes from git status
    pub fn generate_change_summary(&self) -> Result<String> {
        let output = Command::new("git")
            .arg("status")
            .arg("--porcelain")
            .current_dir(&self.notes_dir)
            .output()
            .context("Failed to get git status")?;

        if !output.status.success() {
            return Err(anyhow!("Failed to get git status"));
        }

        let status_output = String::from_utf8_lossy(&output.stdout);
        let mut modified = Vec::new();
        let mut added = Vec::new();
        let mut deleted = Vec::new();

        for line in status_output.lines() {
            if line.len() < 3 {
                continue;
            }

            let status = &line[..2];
            let filename = &line[3..];

            match status {
                "M " | " M" | "MM" => modified.push(filename.to_string()),
                "A " | "??" => added.push(filename.to_string()),
                "D " | " D" => deleted.push(filename.to_string()),
                _ => {}
            }
        }

        let mut summary = Vec::new();

        if !modified.is_empty() {
            summary.push("Modified:".to_string());
            for file in modified {
                summary.push(format!("- {}", file));
            }
        }

        if !added.is_empty() {
            if !summary.is_empty() {
                summary.push(String::new());
            }
            summary.push("Added:".to_string());
            for file in added {
                summary.push(format!("- {}", file));
            }
        }

        if !deleted.is_empty() {
            if !summary.is_empty() {
                summary.push(String::new());
            }
            summary.push("Deleted:".to_string());
            for file in deleted {
                summary.push(format!("- {}", file));
            }
        }

        Ok(summary.join("\n"))
    }

    /// Get a timestamp for commit messages and stash names
    pub fn get_timestamp() -> String {
        Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }
}
