//! Git conflict resolution for MDBY
//!
//! When concurrent edits create conflicts, MDBY resolves them using
//! document-aware merge strategies.

use crate::storage::document::Document;

/// Strategy for resolving conflicts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Keep the local version
    Ours,
    /// Keep the remote version
    Theirs,
    /// Merge fields individually, preferring newer values
    MergeFields,
    /// Concatenate body content with conflict markers
    ConcatenateBody,
    /// Fail and require manual resolution
    Manual,
}

impl Default for ConflictResolution {
    fn default() -> Self {
        Self::MergeFields
    }
}

/// Resolve a conflict between two document versions
pub fn resolve(
    base: Option<&Document>,
    ours: &Document,
    theirs: &Document,
    strategy: ConflictResolution,
) -> anyhow::Result<Document> {
    match strategy {
        ConflictResolution::Ours => Ok(ours.clone()),
        ConflictResolution::Theirs => Ok(theirs.clone()),
        ConflictResolution::MergeFields => merge_fields(base, ours, theirs),
        ConflictResolution::ConcatenateBody => concatenate_body(ours, theirs),
        ConflictResolution::Manual => {
            anyhow::bail!("Manual conflict resolution required for document '{}'", ours.id)
        }
    }
}

/// Merge documents by merging their fields individually
fn merge_fields(
    base: Option<&Document>,
    ours: &Document,
    theirs: &Document,
) -> anyhow::Result<Document> {
    let mut result = Document::new(&ours.id);

    // Get all field keys from both documents
    let mut all_keys: std::collections::HashSet<&String> = ours.fields.keys().collect();
    all_keys.extend(theirs.fields.keys());

    for key in all_keys {
        let ours_val = ours.fields.get(key);
        let theirs_val = theirs.fields.get(key);
        let base_val = base.and_then(|b| b.fields.get(key));

        // Three-way merge logic
        let merged_val = match (base_val, ours_val, theirs_val) {
            // Both changed to same value
            (_, Some(o), Some(t)) if o == t => Some(o.clone()),
            // Only ours changed from base
            (Some(b), Some(o), Some(t)) if b == t => Some(o.clone()),
            // Only theirs changed from base
            (Some(b), Some(o), Some(t)) if b == o => Some(t.clone()),
            // No base, ours exists, theirs doesn't
            (None, Some(o), None) => Some(o.clone()),
            // No base, theirs exists, ours doesn't
            (None, None, Some(t)) => Some(t.clone()),
            // Both exist, prefer theirs (last-write-wins for true conflicts)
            (_, _, Some(t)) => Some(t.clone()),
            // Only ours exists
            (_, Some(o), None) => Some(o.clone()),
            // Neither exists
            (_, None, None) => None,
        };

        if let Some(val) = merged_val {
            result.fields.insert(key.clone(), val);
        }
    }

    // For body, prefer theirs if different (last-write-wins)
    result.body = if ours.body == theirs.body {
        ours.body.clone()
    } else {
        theirs.body.clone()
    };

    Ok(result)
}

/// Concatenate bodies with conflict markers
fn concatenate_body(ours: &Document, theirs: &Document) -> anyhow::Result<Document> {
    let mut result = ours.clone();

    if ours.body != theirs.body {
        result.body = format!(
            "<<<<<<< OURS\n{}\n=======\n{}\n>>>>>>> THEIRS",
            ours.body, theirs.body
        );
    }

    // Merge fields using three-way merge
    for (key, value) in &theirs.fields {
        // Theirs wins for conflicting fields
        result.fields.insert(key.clone(), value.clone());
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::document::Value;

    #[test]
    fn test_merge_fields_no_conflict() {
        let mut ours = Document::new("test");
        ours.set("title", "Our Title");
        ours.set("ours_only", "value");

        let mut theirs = Document::new("test");
        theirs.set("title", "Our Title"); // Same
        theirs.set("theirs_only", "value");

        let result = merge_fields(None, &ours, &theirs).unwrap();

        assert_eq!(result.get("title"), Some(&Value::String("Our Title".into())));
        assert_eq!(result.get("ours_only"), Some(&Value::String("value".into())));
        assert_eq!(result.get("theirs_only"), Some(&Value::String("value".into())));
    }

    #[test]
    fn test_merge_fields_conflict_theirs_wins() {
        let mut ours = Document::new("test");
        ours.set("title", "Our Title");

        let mut theirs = Document::new("test");
        theirs.set("title", "Their Title");

        let result = merge_fields(None, &ours, &theirs).unwrap();

        // Theirs wins in true conflicts
        assert_eq!(result.get("title"), Some(&Value::String("Their Title".into())));
    }
}
