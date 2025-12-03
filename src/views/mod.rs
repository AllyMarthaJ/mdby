//! Views system for MDBY
//!
//! Views are saved queries that generate static output files.
//! They can use templates to render documents in various formats.
//!
//! # View Output Structure
//!
//! ```text
//! /views/
//!   /active-todos/
//!     index.html       # Main view output
//!     index.json       # JSON export
//!   /daily-notes/
//!     index.html
//! ```
//!
//! # Templates
//!
//! Views can specify a template using Tera syntax:
//!
//! ```html
//! {% for doc in documents %}
//! <article>
//!   <h2>{{ doc.title }}</h2>
//!   <div>{{ doc.body | markdown }}</div>
//! </article>
//! {% endfor %}
//! ```

mod regenerate;
mod templates;

pub use regenerate::regenerate_all;
pub use templates::TemplateEngine;

use serde::{Deserialize, Serialize};
use mdql::SelectStmt;

/// A view definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct View {
    /// View name
    pub name: String,
    /// The query that defines this view
    pub query: SelectStmt,
    /// Template to use for rendering (optional)
    pub template: Option<String>,
    /// Output formats to generate
    #[serde(default)]
    pub formats: Vec<OutputFormat>,
}

/// Output format for a view
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Html,
    Json,
    Markdown,
    Csv,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Html
    }
}

impl View {
    pub fn new(name: impl Into<String>, query: SelectStmt) -> Self {
        Self {
            name: name.into(),
            query,
            template: None,
            formats: vec![OutputFormat::Html, OutputFormat::Json],
        }
    }

    pub fn with_template(mut self, template: impl Into<String>) -> Self {
        self.template = Some(template.into());
        self
    }
}
