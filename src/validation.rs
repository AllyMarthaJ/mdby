//! Input validation for MDBY
//!
//! Provides validation for identifiers (collection names, view names, document IDs)
//! to prevent path traversal attacks and ensure filesystem safety.

use thiserror::Error;

/// Validation errors
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Invalid identifier '{0}': {1}")]
    InvalidIdentifier(String, &'static str),

    #[error("Identifier '{0}' is too long (max {1} characters)")]
    TooLong(String, usize),

    #[error("Identifier cannot be empty")]
    Empty,

    #[error("Reserved name: '{0}'")]
    Reserved(String),
}

/// Maximum length for identifiers
pub const MAX_IDENTIFIER_LENGTH: usize = 255;

/// Reserved names that cannot be used
const RESERVED_NAMES: &[&str] = &[
    ".", "..", "con", "prn", "aux", "nul",
    "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8", "com9",
    "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Validate a collection or view name
///
/// Rules:
/// - Must be 1-255 characters
/// - Only alphanumeric, underscore, and hyphen allowed
/// - Cannot start with a hyphen or underscore
/// - Cannot be a reserved name
/// - Case-insensitive reserved name check
pub fn validate_collection_name(name: &str) -> Result<(), ValidationError> {
    validate_identifier(name, "collection name")
}

/// Validate a document ID
///
/// Same rules as collection names
pub fn validate_document_id(id: &str) -> Result<(), ValidationError> {
    validate_identifier(id, "document ID")
}

/// Validate a view name
pub fn validate_view_name(name: &str) -> Result<(), ValidationError> {
    validate_identifier(name, "view name")
}

/// Validate a template name
///
/// More permissive - allows `.` for file extensions
pub fn validate_template_name(name: &str) -> Result<(), ValidationError> {
    if name.is_empty() {
        return Err(ValidationError::Empty);
    }

    if name.len() > MAX_IDENTIFIER_LENGTH {
        return Err(ValidationError::TooLong(name.to_string(), MAX_IDENTIFIER_LENGTH));
    }

    // Check for path traversal
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err(ValidationError::InvalidIdentifier(
            name.to_string(),
            "contains path traversal characters",
        ));
    }

    // Must only contain safe characters
    for (i, c) in name.chars().enumerate() {
        if !c.is_ascii_alphanumeric() && c != '_' && c != '-' && c != '.' {
            return Err(ValidationError::InvalidIdentifier(
                name.to_string(),
                "contains invalid characters (only alphanumeric, underscore, hyphen, and dot allowed)",
            ));
        }
        // Cannot start with dot, hyphen, or underscore
        if i == 0 && (c == '.' || c == '-' || c == '_') {
            return Err(ValidationError::InvalidIdentifier(
                name.to_string(),
                "cannot start with dot, hyphen, or underscore",
            ));
        }
    }

    // Check reserved names (case-insensitive, without extension)
    let base_name = name.split('.').next().unwrap_or(name);
    if RESERVED_NAMES.contains(&base_name.to_lowercase().as_str()) {
        return Err(ValidationError::Reserved(name.to_string()));
    }

    Ok(())
}

/// Core identifier validation
fn validate_identifier(name: &str, _kind: &'static str) -> Result<(), ValidationError> {
    if name.is_empty() {
        return Err(ValidationError::Empty);
    }

    if name.len() > MAX_IDENTIFIER_LENGTH {
        return Err(ValidationError::TooLong(name.to_string(), MAX_IDENTIFIER_LENGTH));
    }

    // Check each character
    for (i, c) in name.chars().enumerate() {
        if !c.is_ascii_alphanumeric() && c != '_' && c != '-' {
            return Err(ValidationError::InvalidIdentifier(
                name.to_string(),
                "contains invalid characters (only alphanumeric, underscore, and hyphen allowed)",
            ));
        }
        // Cannot start with hyphen or underscore
        if i == 0 && (c == '-' || c == '_') {
            return Err(ValidationError::InvalidIdentifier(
                name.to_string(),
                "cannot start with hyphen or underscore",
            ));
        }
    }

    // Check reserved names (case-insensitive)
    if RESERVED_NAMES.contains(&name.to_lowercase().as_str()) {
        return Err(ValidationError::Reserved(name.to_string()));
    }

    Ok(())
}

/// Sanitize an identifier by replacing invalid characters
/// Returns None if the result would be empty or invalid
pub fn sanitize_identifier(input: &str) -> Option<String> {
    if input.is_empty() {
        return None;
    }

    let mut result = String::with_capacity(input.len());

    for (i, c) in input.chars().enumerate() {
        if c.is_ascii_alphanumeric() {
            result.push(c);
        } else if (c == '_' || c == '-') && i > 0 {
            result.push(c);
        } else if !result.is_empty() && result.chars().last() != Some('_') {
            // Replace invalid chars with underscore (avoiding duplicates)
            result.push('_');
        }
    }

    // Trim trailing underscores
    let result = result.trim_end_matches('_').to_string();

    if result.is_empty() || validate_identifier(&result, "").is_err() {
        None
    } else {
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_identifiers() {
        assert!(validate_collection_name("todos").is_ok());
        assert!(validate_collection_name("my-collection").is_ok());
        assert!(validate_collection_name("Collection123").is_ok());
        assert!(validate_collection_name("a").is_ok());
        assert!(validate_document_id("task-1").is_ok());
        assert!(validate_document_id("2024-01-15-notes").is_ok());
    }

    #[test]
    fn test_path_traversal_blocked() {
        assert!(validate_collection_name("..").is_err());
        assert!(validate_collection_name("../secret").is_err());
        assert!(validate_collection_name("foo/bar").is_err());
        assert!(validate_collection_name("foo\\bar").is_err());
        assert!(validate_document_id("../../../etc/passwd").is_err());
    }

    #[test]
    fn test_invalid_characters() {
        assert!(validate_collection_name("foo bar").is_err());
        assert!(validate_collection_name("foo.bar").is_err());
        assert!(validate_collection_name("foo@bar").is_err());
        assert!(validate_collection_name("foo:bar").is_err());
    }

    #[test]
    fn test_invalid_start_characters() {
        assert!(validate_collection_name("-foo").is_err());
        assert!(validate_collection_name("_foo").is_err());
    }

    #[test]
    fn test_reserved_names() {
        assert!(validate_collection_name("con").is_err());
        assert!(validate_collection_name("CON").is_err());
        assert!(validate_collection_name("nul").is_err());
        assert!(validate_collection_name("..").is_err());
    }

    #[test]
    fn test_empty_and_too_long() {
        assert!(validate_collection_name("").is_err());
        let long_name = "a".repeat(256);
        assert!(validate_collection_name(&long_name).is_err());
    }

    #[test]
    fn test_template_names() {
        assert!(validate_template_name("list.html").is_ok());
        assert!(validate_template_name("todo-view.html").is_ok());
        assert!(validate_template_name("../secret.html").is_err());
        assert!(validate_template_name(".hidden").is_err());
    }

    #[test]
    fn test_sanitize() {
        assert_eq!(sanitize_identifier("hello world"), Some("hello_world".to_string()));
        assert_eq!(sanitize_identifier("foo/bar"), Some("foo_bar".to_string()));
        assert_eq!(sanitize_identifier("___"), None);
        assert_eq!(sanitize_identifier("123-test"), Some("123-test".to_string()));
        assert_eq!(sanitize_identifier("-foo"), Some("foo".to_string()));
    }
}
