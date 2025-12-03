//! Git backend for MDBY
//!
//! Provides version control, concurrency handling, and conflict resolution.
//!
//! # Concurrency Model
//!
//! MDBY uses git as its concurrency mechanism:
//!
//! 1. **Optimistic Concurrency**: Operations work on the local copy and commit.
//! 2. **Sync**: Push to remote, pull changes, resolve conflicts.
//! 3. **Conflict Resolution**: Uses document-level merge strategies.
//!
//! # Transaction Model
//!
//! Each database operation creates a git commit. Transactions can span
//! multiple operations and are committed atomically.

use git2::{Repository as Git2Repo, Signature};
use std::path::Path;

mod conflict;
mod sync;

pub use conflict::ConflictResolution;

/// Git repository wrapper for MDBY
pub struct Repository {
    inner: Git2Repo,
}

impl Repository {
    /// Open an existing repository or initialize a new one
    pub fn open_or_init(path: &Path) -> anyhow::Result<Self> {
        let inner = match Git2Repo::open(path) {
            Ok(repo) => repo,
            Err(_) => {
                // Initialize new repository
                let repo = Git2Repo::init(path)?;

                // Create initial commit on main branch
                Self::create_initial_commit(&repo)?;

                repo
            }
        };

        Ok(Self { inner })
    }

    /// Create the initial commit for a new repository
    fn create_initial_commit(repo: &Git2Repo) -> anyhow::Result<()> {
        let sig = Signature::now("MDBY", "mdby@local")?;
        let tree_id = repo.index()?.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        repo.commit(Some("HEAD"), &sig, &sig, "Initialize MDBY database", &tree, &[])?;

        Ok(())
    }

    /// Commit current changes with a message
    pub fn commit(&self, message: &str) -> anyhow::Result<git2::Oid> {
        let sig = self.signature()?;
        let mut index = self.inner.index()?;

        // Add all changes
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;

        let tree_id = index.write_tree()?;
        let tree = self.inner.find_tree(tree_id)?;

        let head = self.inner.head()?;
        let parent = head.peel_to_commit()?;

        let oid = self.inner.commit(
            Some("HEAD"),
            &sig,
            &sig,
            message,
            &tree,
            &[&parent],
        )?;

        Ok(oid)
    }

    /// Get the current HEAD commit hash
    pub fn head_hash(&self) -> anyhow::Result<String> {
        let head = self.inner.head()?;
        let commit = head.peel_to_commit()?;
        Ok(commit.id().to_string())
    }

    /// Check if there are uncommitted changes
    pub fn has_changes(&self) -> anyhow::Result<bool> {
        let statuses = self.inner.statuses(None)?;
        Ok(!statuses.is_empty())
    }

    /// Get a signature for commits
    fn signature(&self) -> anyhow::Result<Signature<'_>> {
        // Try to get from git config, fall back to defaults
        self.inner
            .signature()
            .or_else(|_| Signature::now("MDBY", "mdby@local"))
            .map_err(Into::into)
    }

    /// Sync with remote (stub - to be implemented)
    pub async fn sync(&mut self) -> anyhow::Result<crate::SyncResult> {
        // TODO: Implement push/pull with conflict resolution
        Ok(crate::SyncResult {
            pulled: 0,
            pushed: 0,
            conflicts_resolved: vec![],
        })
    }

    /// Get the underlying git2 repository (for advanced operations)
    pub fn inner(&self) -> &Git2Repo {
        &self.inner
    }
}

/// A database transaction that will be committed atomically
pub struct Transaction<'a> {
    repo: &'a Repository,
    message: String,
    operations: Vec<String>,
}

impl<'a> Transaction<'a> {
    /// Start a new transaction
    pub fn begin(repo: &'a Repository, message: impl Into<String>) -> Self {
        Self {
            repo,
            message: message.into(),
            operations: Vec::new(),
        }
    }

    /// Record an operation in this transaction
    pub fn record(&mut self, operation: impl Into<String>) {
        self.operations.push(operation.into());
    }

    /// Commit the transaction
    pub fn commit(self) -> anyhow::Result<git2::Oid> {
        let full_message = if self.operations.is_empty() {
            self.message
        } else {
            format!("{}\n\n{}", self.message, self.operations.join("\n"))
        };

        self.repo.commit(&full_message)
    }

    /// Abort the transaction (rollback changes)
    pub fn rollback(self) -> anyhow::Result<()> {
        // Reset to HEAD
        let head = self.repo.inner.head()?.peel_to_commit()?;
        self.repo.inner.reset(
            head.as_object(),
            git2::ResetType::Hard,
            None,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_init_repository() {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::open_or_init(tmp.path()).unwrap();

        // Should have an initial commit
        let hash = repo.head_hash().unwrap();
        assert!(!hash.is_empty());
    }

    #[test]
    fn test_commit() {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::open_or_init(tmp.path()).unwrap();

        // Create a file
        std::fs::write(tmp.path().join("test.md"), "# Test").unwrap();

        // Commit
        let oid = repo.commit("Add test file").unwrap();
        assert!(!oid.is_zero());
    }
}
