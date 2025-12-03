//! Document representation
//!
//! A Document is a single markdown file with YAML frontmatter.
//! The frontmatter contains structured data (fields), and the body
//! contains the markdown content.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// A document in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Unique identifier (derived from filename, without .md extension)
    pub id: String,

    /// Path relative to collection root
    pub path: PathBuf,

    /// YAML frontmatter fields
    pub fields: Fields,

    /// Markdown body content
    pub body: String,

    /// Metadata about the document
    #[serde(skip)]
    pub meta: DocumentMeta,
}

/// Field values that can be stored in frontmatter
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Check if this value matches a pattern (for LIKE queries)
    pub fn matches_pattern(&self, pattern: &str) -> bool {
        match self {
            Value::String(s) => {
                // Simple glob matching: * matches any sequence
                let regex_pattern = pattern
                    .replace('%', ".*")
                    .replace('_', ".");
                regex::Regex::new(&format!("^{}$", regex_pattern))
                    .map(|r| r.is_match(s))
                    .unwrap_or(false)
            }
            _ => false,
        }
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Int(i)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

/// A map of field names to values
pub type Fields = HashMap<String, Value>;

/// Metadata about a document (not persisted in the file)
#[derive(Debug, Clone, Default)]
pub struct DocumentMeta {
    /// Git commit hash when last read
    pub git_hash: Option<String>,
    /// File modification time
    pub modified_at: Option<std::time::SystemTime>,
}

impl Document {
    /// Create a new document with the given ID
    pub fn new(id: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            path: PathBuf::from(format!("{}.md", &id)),
            id,
            fields: Fields::new(),
            body: String::new(),
            meta: DocumentMeta::default(),
        }
    }

    /// Set a field value
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<Value>) -> &mut Self {
        self.fields.insert(key.into(), value.into());
        self
    }

    /// Get a field value
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.fields.get(key)
    }

    /// Set the body content
    pub fn with_body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    /// Parse a document from markdown content
    pub fn parse(id: impl Into<String>, content: &str) -> anyhow::Result<Self> {
        let id = id.into();
        let (fields, body) = super::frontmatter::parse(content)?;

        Ok(Self {
            path: PathBuf::from(format!("{}.md", &id)),
            id,
            fields,
            body,
            meta: DocumentMeta::default(),
        })
    }

    /// Render document back to markdown
    pub fn render(&self) -> String {
        super::frontmatter::render(&self.fields, &self.body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_creation() {
        let mut doc = Document::new("my-doc");
        doc.set("title", "Hello World")
            .set("priority", 1i64)
            .set("done", false);

        assert_eq!(doc.id, "my-doc");
        assert_eq!(doc.get("title"), Some(&Value::String("Hello World".into())));
    }

    #[test]
    fn test_roundtrip() {
        let mut doc = Document::new("test");
        doc.set("title", "Test Document");
        doc.body = "This is the body.\n\nWith multiple paragraphs.".into();

        let rendered = doc.render();
        let parsed = Document::parse("test", &rendered).unwrap();

        assert_eq!(parsed.fields, doc.fields);
        assert_eq!(parsed.body.trim(), doc.body.trim());
    }
}
