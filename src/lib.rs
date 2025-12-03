//! MDBY - Markdown-Based Database
//!
//! A human-readable database backed by markdown files with git for concurrency control.
//!
//! # Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                         MDBY Database                           │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
//! │  │   MDQL      │  │   Views     │  │   Schema Registry       │  │
//! │  │   Parser    │  │   Engine    │  │   (Collection Defs)     │  │
//! │  └──────┬──────┘  └──────┬──────┘  └───────────┬─────────────┘  │
//! │         │                │                     │                │
//! │         ▼                ▼                     ▼                │
//! │  ┌─────────────────────────────────────────────────────────────┐│
//! │  │                    Query Engine                             ││
//! │  │  (SELECT, INSERT, UPDATE, DELETE, CREATE VIEW)              ││
//! │  └──────────────────────────┬──────────────────────────────────┘│
//! │                             │                                   │
//! │                             ▼                                   │
//! │  ┌─────────────────────────────────────────────────────────────┐│
//! │  │                   Storage Layer                             ││
//! │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  ││
//! │  │  │  Document   │  │  Index      │  │  Transaction        │  ││
//! │  │  │  Store      │  │  Manager    │  │  Manager            │  ││
//! │  │  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘  ││
//! │  └─────────┼────────────────┼────────────────────┼─────────────┘│
//! │            │                │                    │              │
//! │            ▼                ▼                    ▼              │
//! │  ┌─────────────────────────────────────────────────────────────┐│
//! │  │                    Git Backend                              ││
//! │  │  (Commits, Branches, Merge Conflict Resolution)             ││
//! │  └─────────────────────────────────────────────────────────────┘│
//! │                             │                                   │
//! │                             ▼                                   │
//! │  ┌─────────────────────────────────────────────────────────────┐│
//! │  │              File System (Markdown Files)                   ││
//! │  │  /collections/{name}/*.md   /views/{name}/*.md              ││
//! │  └─────────────────────────────────────────────────────────────┘│
//! └─────────────────────────────────────────────────────────────────┘
//! ```

pub mod error;
pub mod git;
pub mod query;
pub mod schema;
pub mod storage;
pub mod validation;
pub mod views;

pub use error::{Error, Result};

use std::path::PathBuf;

pub use storage::document::Document;
pub use storage::collection::Collection;
pub use schema::Schema;

/// The main database handle
pub struct Database {
    /// Root path of the database
    pub root: PathBuf,
    /// Git repository handle
    pub git: git::Repository,
    /// Schema registry
    pub(crate) schema: schema::SchemaRegistry,
}

impl Database {
    /// Open or create a database at the given path
    pub async fn open(path: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let root = path.into();
        let git = git::Repository::open_or_init(&root)?;
        let schema = schema::SchemaRegistry::load(&root)?;

        Ok(Self { root, git, schema })
    }

    /// Execute an MDQL query
    pub async fn execute(&mut self, query: &str) -> anyhow::Result<QueryResult> {
        let parsed = mdql::parse(query)?;
        self.execute_ast(parsed).await
    }

    /// Execute a parsed AST
    async fn execute_ast(&mut self, ast: mdql::Statement) -> anyhow::Result<QueryResult> {
        query::execute(self, ast).await
    }

    /// Regenerate all views (async)
    pub async fn regenerate_views(&self) -> anyhow::Result<()> {
        views::regenerate_all(self).await
    }

    /// Sync with remote (push/pull with conflict resolution)
    pub async fn sync(&mut self) -> anyhow::Result<SyncResult> {
        self.git.sync().await
    }
}

/// Result of a query execution
#[derive(Debug)]
pub enum QueryResult {
    /// Documents returned from a SELECT
    Documents(Vec<Document>),
    /// Number of affected documents
    Affected(usize),
    /// View created/updated
    ViewCreated(String),
    /// Collection created
    CollectionCreated(String),
}

/// Result of a sync operation
#[derive(Debug)]
pub struct SyncResult {
    pub pulled: usize,
    pub pushed: usize,
    pub conflicts_resolved: Vec<String>,
}
