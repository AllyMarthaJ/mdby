//! Error types for MDBY
//!
//! Provides structured error types with context for better debugging
//! and user-friendly error messages.

use std::path::PathBuf;
use thiserror::Error;

/// The main error type for MDBY operations
#[derive(Debug, Error)]
pub enum Error {
    // ==========================================================================
    // Collection Errors
    // ==========================================================================
    #[error("Collection '{name}' does not exist")]
    CollectionNotFound { name: String },

    #[error("Collection '{name}' already exists")]
    CollectionAlreadyExists { name: String },

    #[error("Failed to create collection '{name}': {source}")]
    CollectionCreateFailed {
        name: String,
        #[source]
        source: std::io::Error,
    },

    // ==========================================================================
    // Document Errors
    // ==========================================================================
    #[error("Document '{id}' not found in collection '{collection}'")]
    DocumentNotFound { collection: String, id: String },

    #[error("Document '{id}' already exists in collection '{collection}'")]
    DocumentAlreadyExists { collection: String, id: String },

    #[error("INSERT requires an 'id' column")]
    MissingDocumentId,

    // ==========================================================================
    // View Errors
    // ==========================================================================
    #[error("View '{name}' does not exist")]
    ViewNotFound { name: String },

    #[error("View '{name}' already exists")]
    ViewAlreadyExists { name: String },

    // ==========================================================================
    // Schema Errors
    // ==========================================================================
    #[error("Schema validation failed for collection '{collection}': {message}")]
    SchemaValidation { collection: String, message: String },

    #[error("Missing required field '{field}' in collection '{collection}'")]
    MissingRequiredField { collection: String, field: String },

    #[error("Type mismatch for field '{field}': expected {expected}, got {actual}")]
    TypeMismatch {
        field: String,
        expected: String,
        actual: String,
    },

    // ==========================================================================
    // Validation Errors
    // ==========================================================================
    #[error("Invalid {kind} '{value}': {reason}")]
    InvalidIdentifier {
        kind: &'static str,
        value: String,
        reason: &'static str,
    },

    #[error("Reserved name '{name}' cannot be used")]
    ReservedName { name: String },

    // ==========================================================================
    // Query Errors
    // ==========================================================================
    #[error("Query parse error: {message}")]
    ParseError { message: String },

    #[error("Query execution error: {message}")]
    QueryError { message: String },

    // ==========================================================================
    // Git Errors
    // ==========================================================================
    #[error("Git operation failed: {message}")]
    GitError {
        message: String,
        #[source]
        source: Option<git2::Error>,
    },

    // ==========================================================================
    // IO Errors
    // ==========================================================================
    #[error("Failed to read file '{path}': {source}")]
    FileReadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to write file '{path}': {source}")]
    FileWriteError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    // ==========================================================================
    // Serialization Errors
    // ==========================================================================
    #[error("Failed to parse YAML: {message}")]
    YamlParseError { message: String },

    #[error("Failed to serialize to YAML: {message}")]
    YamlSerializeError { message: String },

    #[error("Failed to parse JSON: {message}")]
    JsonParseError { message: String },

    // ==========================================================================
    // Catch-all
    // ==========================================================================
    #[error("{0}")]
    Other(String),
}

/// Result type alias for MDBY operations
pub type Result<T> = std::result::Result<T, Error>;

// =============================================================================
// Conversions from external error types
// =============================================================================

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Other(err.to_string())
    }
}

impl From<git2::Error> for Error {
    fn from(err: git2::Error) -> Self {
        Error::GitError {
            message: err.message().to_string(),
            source: Some(err),
        }
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(err: serde_yaml::Error) -> Self {
        Error::YamlParseError {
            message: err.to_string(),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::JsonParseError {
            message: err.to_string(),
        }
    }
}

impl From<mdql::ParseError> for Error {
    fn from(err: mdql::ParseError) -> Self {
        Error::ParseError {
            message: err.to_string(),
        }
    }
}

impl From<crate::validation::ValidationError> for Error {
    fn from(err: crate::validation::ValidationError) -> Self {
        match err {
            crate::validation::ValidationError::InvalidIdentifier(value, reason) => {
                Error::InvalidIdentifier {
                    kind: "identifier",
                    value,
                    reason,
                }
            }
            crate::validation::ValidationError::TooLong(value, _max) => Error::InvalidIdentifier {
                kind: "identifier",
                value,
                reason: "exceeds maximum length",
            },
            crate::validation::ValidationError::Empty => Error::InvalidIdentifier {
                kind: "identifier",
                value: String::new(),
                reason: "cannot be empty",
            },
            crate::validation::ValidationError::Reserved(name) => Error::ReservedName { name },
        }
    }
}

impl From<crate::schema::ValidationError> for Error {
    fn from(err: crate::schema::ValidationError) -> Self {
        match err {
            crate::schema::ValidationError::MissingRequired(field) => Error::MissingRequiredField {
                collection: String::new(), // Will be set by caller
                field,
            },
            crate::schema::ValidationError::TypeMismatch {
                field,
                expected,
                actual,
            } => Error::TypeMismatch {
                field,
                expected,
                actual,
            },
            crate::schema::ValidationError::UniqueViolation(field) => Error::SchemaValidation {
                collection: String::new(),
                message: format!("Unique constraint violated for field: {}", field),
            },
        }
    }
}

// =============================================================================
// Error Display Helpers
// =============================================================================

impl Error {
    /// Returns a user-friendly suggestion for fixing the error
    pub fn suggestion(&self) -> Option<&'static str> {
        match self {
            Error::CollectionNotFound { .. } => {
                Some("Create the collection first with: CREATE COLLECTION <name>")
            }
            Error::DocumentNotFound { .. } => {
                Some("Check the document ID and collection name")
            }
            Error::MissingDocumentId => {
                Some("Add an 'id' column: INSERT INTO collection (id, ...) VALUES ('my-id', ...)")
            }
            Error::InvalidIdentifier { .. } => {
                Some("Use only letters, numbers, underscores, and hyphens")
            }
            Error::MissingRequiredField { .. } => {
                Some("Add the required field to your INSERT statement")
            }
            _ => None,
        }
    }

    /// Returns true if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Error::CollectionNotFound { .. }
                | Error::DocumentNotFound { .. }
                | Error::ViewNotFound { .. }
                | Error::InvalidIdentifier { .. }
                | Error::ParseError { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::CollectionNotFound {
            name: "todos".to_string(),
        };
        assert_eq!(err.to_string(), "Collection 'todos' does not exist");
    }

    #[test]
    fn test_error_suggestion() {
        let err = Error::CollectionNotFound {
            name: "todos".to_string(),
        };
        assert!(err.suggestion().is_some());
    }
}
