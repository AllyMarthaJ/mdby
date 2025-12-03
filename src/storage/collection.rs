//! Collection - a group of documents stored in a directory
//!
//! Collections are analogous to tables in a relational database.
//! Each collection is a directory containing markdown files.
//!
//! Directory structure:
//! ```text
//! /collections/
//!   /todos/
//!     task-1.md
//!     task-2.md
//!   /notes/
//!     2024-01-15-meeting.md
//!     2024-01-16-ideas.md
//! ```

use super::document::Document;
use std::path::{Path, PathBuf};
use tokio::fs;
use walkdir::WalkDir;

/// A collection of documents
#[derive(Debug)]
pub struct Collection {
    /// Name of the collection (directory name)
    pub name: String,
    /// Path to the collection directory
    pub path: PathBuf,
}

impl Collection {
    /// Open a collection at the given path
    pub fn open(name: impl Into<String>, base_path: &Path) -> Self {
        let name = name.into();
        let path = base_path.join("collections").join(&name);
        Self { name, path }
    }

    /// Create the collection directory if it doesn't exist
    pub async fn ensure_exists(&self) -> anyhow::Result<()> {
        fs::create_dir_all(&self.path).await?;
        Ok(())
    }

    /// Check if the collection exists
    pub async fn exists(&self) -> bool {
        self.path.is_dir()
    }

    /// List all documents in the collection
    pub async fn list(&self) -> anyhow::Result<Vec<Document>> {
        let mut documents = Vec::new();

        if !self.path.exists() {
            return Ok(documents);
        }

        for entry in WalkDir::new(&self.path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().map(|e| e == "md").unwrap_or(false) {
                if let Ok(doc) = self.read_document(path).await {
                    documents.push(doc);
                }
            }
        }

        Ok(documents)
    }

    /// Read a single document by ID
    pub async fn get(&self, id: &str) -> anyhow::Result<Option<Document>> {
        let path = self.path.join(format!("{}.md", id));
        if !path.exists() {
            return Ok(None);
        }
        self.read_document(&path).await.map(Some)
    }

    /// Insert a new document
    pub async fn insert(&self, doc: &Document) -> anyhow::Result<()> {
        self.ensure_exists().await?;
        let path = self.path.join(format!("{}.md", doc.id));

        if path.exists() {
            anyhow::bail!("Document '{}' already exists in collection '{}'", doc.id, self.name);
        }

        let content = doc.render();
        fs::write(&path, content).await?;
        Ok(())
    }

    /// Update an existing document
    pub async fn update(&self, doc: &Document) -> anyhow::Result<()> {
        let path = self.path.join(format!("{}.md", doc.id));

        if !path.exists() {
            anyhow::bail!("Document '{}' not found in collection '{}'", doc.id, self.name);
        }

        let content = doc.render();
        fs::write(&path, content).await?;
        Ok(())
    }

    /// Upsert a document (insert or update)
    pub async fn upsert(&self, doc: &Document) -> anyhow::Result<()> {
        self.ensure_exists().await?;
        let path = self.path.join(format!("{}.md", doc.id));
        let content = doc.render();
        fs::write(&path, content).await?;
        Ok(())
    }

    /// Delete a document by ID
    pub async fn delete(&self, id: &str) -> anyhow::Result<bool> {
        let path = self.path.join(format!("{}.md", id));
        if path.exists() {
            fs::remove_file(&path).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Count documents in the collection
    pub async fn count(&self) -> anyhow::Result<usize> {
        let docs = self.list().await?;
        Ok(docs.len())
    }

    /// Read a document from a path
    async fn read_document(&self, path: &Path) -> anyhow::Result<Document> {
        let id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid document path"))?;

        let content = fs::read_to_string(path).await?;
        let mut doc = Document::parse(id, &content)?;

        // Set relative path within collection
        doc.path = path.strip_prefix(&self.path)?.to_path_buf();

        // Set metadata
        if let Ok(metadata) = path.metadata() {
            doc.meta.modified_at = metadata.modified().ok();
        }

        Ok(doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_collection_crud() {
        let tmp = TempDir::new().unwrap();
        let collection = Collection::open("todos", tmp.path());

        // Create
        let mut doc = Document::new("task-1");
        doc.set("title", "Buy groceries");
        doc.set("done", false);
        doc.body = "- Milk\n- Eggs\n- Bread".into();

        collection.insert(&doc).await.unwrap();

        // Read
        let fetched = collection.get("task-1").await.unwrap().unwrap();
        assert_eq!(fetched.get("title").unwrap().as_str(), Some("Buy groceries"));

        // Update
        let mut updated = fetched;
        updated.set("done", true);
        collection.update(&updated).await.unwrap();

        let refetched = collection.get("task-1").await.unwrap().unwrap();
        assert_eq!(refetched.get("done").unwrap().as_bool(), Some(true));

        // List
        let docs = collection.list().await.unwrap();
        assert_eq!(docs.len(), 1);

        // Delete
        let deleted = collection.delete("task-1").await.unwrap();
        assert!(deleted);

        let gone = collection.get("task-1").await.unwrap();
        assert!(gone.is_none());
    }
}
