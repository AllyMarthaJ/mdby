//! YAML frontmatter parsing and rendering
//!
//! Markdown files use YAML frontmatter delimited by `---`:
//!
//! ```markdown
//! ---
//! title: My Document
//! tags: [rust, database]
//! priority: 1
//! ---
//!
//! # Document content here
//! ```

use super::document::{Fields, Value};
use std::collections::HashMap;

/// Parse YAML frontmatter from markdown content
pub fn parse(content: &str) -> anyhow::Result<(Fields, String)> {
    let content = content.trim_start();

    // Check for frontmatter delimiter
    if !content.starts_with("---") {
        // No frontmatter, entire content is body
        return Ok((Fields::new(), content.to_string()));
    }

    // Find the closing delimiter
    let rest = &content[3..];
    let end_pos = rest
        .find("\n---")
        .ok_or_else(|| anyhow::anyhow!("Unclosed frontmatter: missing closing ---"))?;

    let yaml_content = &rest[..end_pos].trim();
    let body_start = end_pos + 4; // Skip past "\n---"
    let body = rest[body_start..].trim_start_matches('\n').to_string();

    // Parse YAML
    let yaml_value: serde_yaml::Value = serde_yaml::from_str(yaml_content)?;
    let fields = yaml_to_fields(yaml_value)?;

    Ok((fields, body))
}

/// Convert serde_yaml::Value to our Fields type
fn yaml_to_fields(value: serde_yaml::Value) -> anyhow::Result<Fields> {
    match value {
        serde_yaml::Value::Mapping(map) => {
            let mut fields = Fields::new();
            for (k, v) in map {
                let key = k
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Non-string key in frontmatter"))?
                    .to_string();
                fields.insert(key, yaml_value_to_value(v));
            }
            Ok(fields)
        }
        serde_yaml::Value::Null => Ok(Fields::new()),
        _ => Err(anyhow::anyhow!("Frontmatter must be a YAML mapping")),
    }
}

/// Convert a serde_yaml::Value to our Value type
fn yaml_value_to_value(v: serde_yaml::Value) -> Value {
    match v {
        serde_yaml::Value::Null => Value::Null,
        serde_yaml::Value::Bool(b) => Value::Bool(b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Null
            }
        }
        serde_yaml::Value::String(s) => Value::String(s),
        serde_yaml::Value::Sequence(seq) => {
            Value::Array(seq.into_iter().map(yaml_value_to_value).collect())
        }
        serde_yaml::Value::Mapping(map) => {
            let obj: HashMap<String, Value> = map
                .into_iter()
                .filter_map(|(k, v)| {
                    k.as_str().map(|key| (key.to_string(), yaml_value_to_value(v)))
                })
                .collect();
            Value::Object(obj)
        }
        serde_yaml::Value::Tagged(tagged) => yaml_value_to_value(tagged.value),
    }
}

/// Convert our Value to serde_yaml::Value
fn value_to_yaml(v: &Value) -> serde_yaml::Value {
    match v {
        Value::Null => serde_yaml::Value::Null,
        Value::Bool(b) => serde_yaml::Value::Bool(*b),
        Value::Int(i) => serde_yaml::Value::Number((*i).into()),
        Value::Float(f) => serde_yaml::Value::Number(serde_yaml::Number::from(*f)),
        Value::String(s) => serde_yaml::Value::String(s.clone()),
        Value::Array(arr) => {
            serde_yaml::Value::Sequence(arr.iter().map(value_to_yaml).collect())
        }
        Value::Object(obj) => {
            let map: serde_yaml::Mapping = obj
                .iter()
                .map(|(k, v)| (serde_yaml::Value::String(k.clone()), value_to_yaml(v)))
                .collect();
            serde_yaml::Value::Mapping(map)
        }
    }
}

/// Render fields and body back to markdown with frontmatter
pub fn render(fields: &Fields, body: &str) -> String {
    if fields.is_empty() {
        return body.to_string();
    }

    // Convert fields to YAML mapping
    let yaml_map: serde_yaml::Mapping = fields
        .iter()
        .map(|(k, v)| (serde_yaml::Value::String(k.clone()), value_to_yaml(v)))
        .collect();

    let yaml_str = serde_yaml::to_string(&serde_yaml::Value::Mapping(yaml_map))
        .unwrap_or_default();

    format!("---\n{}---\n\n{}", yaml_str, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter() {
        let content = r#"---
title: Hello World
count: 42
tags:
  - rust
  - database
---

# My Document

Some content here.
"#;

        let (fields, body) = parse(content).unwrap();

        assert_eq!(
            fields.get("title"),
            Some(&Value::String("Hello World".into()))
        );
        assert_eq!(fields.get("count"), Some(&Value::Int(42)));
        assert!(body.contains("# My Document"));
    }

    #[test]
    fn test_no_frontmatter() {
        let content = "# Just a document\n\nWith no frontmatter.";
        let (fields, body) = parse(content).unwrap();

        assert!(fields.is_empty());
        assert!(body.contains("Just a document"));
    }

    #[test]
    fn test_render_roundtrip() {
        let mut fields = Fields::new();
        fields.insert("title".into(), Value::String("Test".into()));
        fields.insert("priority".into(), Value::Int(1));

        let body = "# Content\n\nHello!";
        let rendered = render(&fields, body);
        let (parsed_fields, parsed_body) = parse(&rendered).unwrap();

        assert_eq!(parsed_fields.get("title"), fields.get("title"));
        assert_eq!(parsed_fields.get("priority"), fields.get("priority"));
        assert!(parsed_body.contains("# Content"));
    }
}
