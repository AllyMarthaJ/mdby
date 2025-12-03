//! Schema definitions and validation for MDBY
//!
//! Schemas define the structure of collections:
//! - Field definitions with types
//! - Required vs optional fields
//! - Default values
//! - Validation rules
//!
//! Schemas are stored in `/.mdby/schemas/{collection}.yaml`

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A field type in the schema
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    String,
    Int,
    Float,
    Bool,
    Date,
    DateTime,
    Array(Box<FieldType>),
    Object,
    /// Reference to another document: ref:collection_name
    Ref(String),
}

impl Default for FieldType {
    fn default() -> Self {
        Self::String
    }
}

/// Definition of a single field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    /// Field type
    #[serde(rename = "type", default)]
    pub field_type: FieldType,
    /// Whether the field is required
    #[serde(default)]
    pub required: bool,
    /// Default value (as YAML)
    #[serde(default)]
    pub default: Option<serde_yaml::Value>,
    /// Human-readable description
    #[serde(default)]
    pub description: Option<String>,
    /// Index this field for faster queries
    #[serde(default)]
    pub indexed: bool,
    /// Unique constraint
    #[serde(default)]
    pub unique: bool,
}

/// Schema for a collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    /// Collection name
    pub name: String,
    /// Human-readable description
    #[serde(default)]
    pub description: Option<String>,
    /// Field definitions
    #[serde(default)]
    pub fields: HashMap<String, FieldDef>,
    /// ID generation strategy
    #[serde(default)]
    pub id_strategy: IdStrategy,
}

/// Strategy for generating document IDs
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IdStrategy {
    /// User provides the ID
    #[default]
    Manual,
    /// Auto-incrementing integer
    AutoIncrement,
    /// UUID v4
    Uuid,
    /// Derived from a field (e.g., slug from title)
    Derived { from: String, transform: String },
}

impl Schema {
    /// Create a new schema for a collection
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            fields: HashMap::new(),
            id_strategy: IdStrategy::default(),
        }
    }

    /// Add a field definition
    pub fn field(mut self, name: impl Into<String>, def: FieldDef) -> Self {
        self.fields.insert(name.into(), def);
        self
    }

    /// Validate a document against this schema
    pub fn validate(&self, doc: &crate::Document) -> Result<(), ValidationError> {
        // Check required fields
        for (field_name, field_def) in &self.fields {
            if field_def.required && !doc.fields.contains_key(field_name) {
                return Err(ValidationError::MissingRequired(field_name.clone()));
            }
        }

        // Type checking would go here
        // For now, we're lenient

        Ok(())
    }
}

/// Validation error
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Missing required field: {0}")]
    MissingRequired(String),
    #[error("Invalid type for field {field}: expected {expected}, got {actual}")]
    TypeMismatch {
        field: String,
        expected: String,
        actual: String,
    },
    #[error("Unique constraint violated for field: {0}")]
    UniqueViolation(String),
}

/// Registry of all schemas in the database
#[derive(Debug, Default)]
pub struct SchemaRegistry {
    schemas: HashMap<String, Schema>,
    path: PathBuf,
}

impl SchemaRegistry {
    /// Load schemas from the database directory
    pub fn load(db_path: &Path) -> anyhow::Result<Self> {
        let schema_path = db_path.join(".mdby").join("schemas");
        let mut registry = Self {
            schemas: HashMap::new(),
            path: schema_path.clone(),
        };

        if schema_path.exists() {
            for entry in std::fs::read_dir(&schema_path)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map(|e| e == "yaml").unwrap_or(false) {
                    let content = std::fs::read_to_string(&path)?;
                    let schema: Schema = serde_yaml::from_str(&content)?;
                    registry.schemas.insert(schema.name.clone(), schema);
                }
            }
        }

        Ok(registry)
    }

    /// Get a schema by collection name
    pub fn get(&self, name: &str) -> Option<&Schema> {
        self.schemas.get(name)
    }

    /// Register a new schema
    pub fn register(&mut self, schema: Schema) -> anyhow::Result<()> {
        // Save to disk
        std::fs::create_dir_all(&self.path)?;
        let file_path = self.path.join(format!("{}.yaml", schema.name));
        let content = serde_yaml::to_string(&schema)?;
        std::fs::write(file_path, content)?;

        self.schemas.insert(schema.name.clone(), schema);
        Ok(())
    }

    /// List all registered schemas
    pub fn list(&self) -> impl Iterator<Item = &Schema> {
        self.schemas.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_validation() {
        let schema = Schema::new("todos")
            .field("title", FieldDef {
                field_type: FieldType::String,
                required: true,
                ..Default::default()
            })
            .field("done", FieldDef {
                field_type: FieldType::Bool,
                required: false,
                default: Some(serde_yaml::Value::Bool(false)),
                ..Default::default()
            });

        // Valid document
        let mut doc = crate::Document::new("task-1");
        doc.set("title", "Buy groceries");
        assert!(schema.validate(&doc).is_ok());

        // Missing required field
        let empty_doc = crate::Document::new("task-2");
        assert!(matches!(
            schema.validate(&empty_doc),
            Err(ValidationError::MissingRequired(_))
        ));
    }
}

impl Default for FieldDef {
    fn default() -> Self {
        Self {
            field_type: FieldType::String,
            required: false,
            default: None,
            description: None,
            indexed: false,
            unique: false,
        }
    }
}
