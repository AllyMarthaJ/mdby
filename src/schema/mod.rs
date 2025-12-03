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

        // Type checking for fields that exist
        for (field_name, field_def) in &self.fields {
            if let Some(value) = doc.fields.get(field_name) {
                if !check_type_match(&field_def.field_type, value) {
                    return Err(ValidationError::TypeMismatch {
                        field: field_name.clone(),
                        expected: format!("{:?}", field_def.field_type),
                        actual: describe_value_type(value),
                    });
                }
            }
        }

        Ok(())
    }
}

/// Check if a Value matches the expected FieldType
fn check_type_match(field_type: &FieldType, value: &crate::storage::document::Value) -> bool {
    use crate::storage::document::Value;

    match (field_type, value) {
        // Null matches any type (represents missing/optional)
        (_, Value::Null) => true,

        // String type
        (FieldType::String, Value::String(_)) => true,

        // Int type (also accept Float that is a whole number)
        (FieldType::Int, Value::Int(_)) => true,
        (FieldType::Int, Value::Float(f)) => f.fract() == 0.0,

        // Float type (also accept Int since integers are valid floats)
        (FieldType::Float, Value::Float(_)) => true,
        (FieldType::Float, Value::Int(_)) => true,

        // Bool type
        (FieldType::Bool, Value::Bool(_)) => true,

        // Date/DateTime stored as strings (ISO 8601 format)
        (FieldType::Date, Value::String(s)) => is_valid_date(s),
        (FieldType::DateTime, Value::String(s)) => is_valid_datetime(s),

        // Object type
        (FieldType::Object, Value::Object(_)) => true,

        // Array type with inner type checking
        (FieldType::Array(inner_type), Value::Array(items)) => {
            items.iter().all(|item| check_type_match(inner_type, item))
        }

        // Ref type - stored as string (the referenced document ID)
        (FieldType::Ref(_), Value::String(_)) => true,

        // No match
        _ => false,
    }
}

/// Describe a Value's type for error messages
fn describe_value_type(value: &crate::storage::document::Value) -> String {
    use crate::storage::document::Value;

    match value {
        Value::Null => "null".to_string(),
        Value::Bool(_) => "bool".to_string(),
        Value::Int(_) => "int".to_string(),
        Value::Float(_) => "float".to_string(),
        Value::String(_) => "string".to_string(),
        Value::Array(items) => {
            if items.is_empty() {
                "array".to_string()
            } else {
                format!("array<{}>", describe_value_type(&items[0]))
            }
        }
        Value::Object(_) => "object".to_string(),
    }
}

/// Check if a string is a valid ISO 8601 date (YYYY-MM-DD)
fn is_valid_date(s: &str) -> bool {
    // Basic format check: YYYY-MM-DD
    if s.len() != 10 {
        return false;
    }

    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return false;
    }

    let year = parts[0].parse::<u32>().ok();
    let month = parts[1].parse::<u32>().ok();
    let day = parts[2].parse::<u32>().ok();

    match (year, month, day) {
        (Some(_y), Some(m), Some(d)) => {
            m >= 1 && m <= 12 && d >= 1 && d <= 31
        }
        _ => false,
    }
}

/// Check if a string is a valid ISO 8601 datetime
fn is_valid_datetime(s: &str) -> bool {
    // Accept formats like:
    // - 2024-01-15T10:30:00
    // - 2024-01-15T10:30:00Z
    // - 2024-01-15T10:30:00+00:00
    // - 2024-01-15 10:30:00

    // Must have date portion
    if s.len() < 10 {
        return false;
    }

    // Check date portion
    if !is_valid_date(&s[..10]) {
        return false;
    }

    // If there's more, check for time separator
    if s.len() > 10 {
        let sep = s.chars().nth(10).unwrap();
        if sep != 'T' && sep != ' ' {
            return false;
        }

        // Basic check for time portion (at least HH:MM)
        if s.len() < 16 {
            return false;
        }

        let time_part = &s[11..];
        let time_base: &str = if time_part.contains('Z') || time_part.contains('+') || time_part.contains('-') {
            // Has timezone, extract time portion
            time_part.split(|c| c == 'Z' || c == '+').next().unwrap_or("")
        } else {
            time_part
        };

        // Check HH:MM format
        let time_parts: Vec<&str> = time_base.split(':').collect();
        if time_parts.len() < 2 {
            return false;
        }

        let hour = time_parts[0].parse::<u32>().ok();
        let minute = time_parts[1].parse::<u32>().ok();

        match (hour, minute) {
            (Some(h), Some(m)) => h <= 23 && m <= 59,
            _ => false,
        }
    } else {
        // Just date, but expected datetime - allow it (midnight implied)
        true
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
    use crate::storage::document::Value;

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

    #[test]
    fn test_type_validation_string() {
        let schema = Schema::new("test")
            .field("name", FieldDef {
                field_type: FieldType::String,
                required: false,
                ..Default::default()
            });

        // String value - valid
        let mut doc = crate::Document::new("doc-1");
        doc.set("name", "Alice");
        assert!(schema.validate(&doc).is_ok());

        // Int value for string field - invalid
        let mut doc = crate::Document::new("doc-2");
        doc.fields.insert("name".to_string(), Value::Int(42));
        assert!(matches!(
            schema.validate(&doc),
            Err(ValidationError::TypeMismatch { .. })
        ));
    }

    #[test]
    fn test_type_validation_int() {
        let schema = Schema::new("test")
            .field("count", FieldDef {
                field_type: FieldType::Int,
                required: false,
                ..Default::default()
            });

        // Int value - valid
        let mut doc = crate::Document::new("doc-1");
        doc.fields.insert("count".to_string(), Value::Int(42));
        assert!(schema.validate(&doc).is_ok());

        // String value for int field - invalid
        let mut doc = crate::Document::new("doc-2");
        doc.set("count", "not a number");
        assert!(matches!(
            schema.validate(&doc),
            Err(ValidationError::TypeMismatch { .. })
        ));
    }

    #[test]
    fn test_type_validation_bool() {
        let schema = Schema::new("test")
            .field("active", FieldDef {
                field_type: FieldType::Bool,
                required: false,
                ..Default::default()
            });

        // Bool value - valid
        let mut doc = crate::Document::new("doc-1");
        doc.fields.insert("active".to_string(), Value::Bool(true));
        assert!(schema.validate(&doc).is_ok());

        // String value for bool field - invalid
        let mut doc = crate::Document::new("doc-2");
        doc.set("active", "true");
        assert!(matches!(
            schema.validate(&doc),
            Err(ValidationError::TypeMismatch { .. })
        ));
    }

    #[test]
    fn test_type_validation_array() {
        let schema = Schema::new("test")
            .field("tags", FieldDef {
                field_type: FieldType::Array(Box::new(FieldType::String)),
                required: false,
                ..Default::default()
            });

        // Array of strings - valid
        let mut doc = crate::Document::new("doc-1");
        doc.fields.insert("tags".to_string(), Value::Array(vec![
            Value::String("rust".to_string()),
            Value::String("database".to_string()),
        ]));
        assert!(schema.validate(&doc).is_ok());

        // Array with wrong inner type - invalid
        let mut doc = crate::Document::new("doc-2");
        doc.fields.insert("tags".to_string(), Value::Array(vec![
            Value::Int(1),
            Value::Int(2),
        ]));
        assert!(matches!(
            schema.validate(&doc),
            Err(ValidationError::TypeMismatch { .. })
        ));
    }

    #[test]
    fn test_type_validation_date() {
        let schema = Schema::new("test")
            .field("due_date", FieldDef {
                field_type: FieldType::Date,
                required: false,
                ..Default::default()
            });

        // Valid date - ok
        let mut doc = crate::Document::new("doc-1");
        doc.set("due_date", "2024-01-15");
        assert!(schema.validate(&doc).is_ok());

        // Invalid date format - error
        let mut doc = crate::Document::new("doc-2");
        doc.set("due_date", "not-a-date");
        assert!(matches!(
            schema.validate(&doc),
            Err(ValidationError::TypeMismatch { .. })
        ));
    }

    #[test]
    fn test_type_validation_datetime() {
        let schema = Schema::new("test")
            .field("created_at", FieldDef {
                field_type: FieldType::DateTime,
                required: false,
                ..Default::default()
            });

        // Valid datetime formats
        let mut doc = crate::Document::new("doc-1");
        doc.set("created_at", "2024-01-15T10:30:00");
        assert!(schema.validate(&doc).is_ok());

        let mut doc = crate::Document::new("doc-2");
        doc.set("created_at", "2024-01-15T10:30:00Z");
        assert!(schema.validate(&doc).is_ok());

        // Invalid datetime - error
        let mut doc = crate::Document::new("doc-3");
        doc.set("created_at", "yesterday");
        assert!(matches!(
            schema.validate(&doc),
            Err(ValidationError::TypeMismatch { .. })
        ));
    }

    #[test]
    fn test_null_always_valid() {
        let schema = Schema::new("test")
            .field("optional", FieldDef {
                field_type: FieldType::String,
                required: false,
                ..Default::default()
            });

        let mut doc = crate::Document::new("doc-1");
        doc.fields.insert("optional".to_string(), Value::Null);
        assert!(schema.validate(&doc).is_ok());
    }

    #[test]
    fn test_date_validation_helpers() {
        assert!(is_valid_date("2024-01-15"));
        assert!(is_valid_date("2024-12-31"));
        assert!(!is_valid_date("2024-13-01")); // invalid month
        assert!(!is_valid_date("not-a-date"));
        assert!(!is_valid_date("2024/01/15")); // wrong separator

        assert!(is_valid_datetime("2024-01-15T10:30:00"));
        assert!(is_valid_datetime("2024-01-15T10:30:00Z"));
        assert!(is_valid_datetime("2024-01-15 10:30:00"));
        assert!(!is_valid_datetime("not-a-datetime"));
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
