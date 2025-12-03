//! View regeneration

use std::path::Path;
use tokio::fs;

use super::TemplateEngine;
use crate::storage::collection::Collection;
use crate::storage::document::Document;
use crate::Database;
use crate::query::filter;

/// Regenerate all views in the database
pub async fn regenerate_all(db: &Database) -> anyhow::Result<()> {
    let views_def_path = db.root.join(".mdby").join("views");

    if !views_def_path.exists() {
        return Ok(());
    }

    let mut entries = fs::read_dir(&views_def_path).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().map(|e| e == "yaml").unwrap_or(false) {
            if let Err(e) = regenerate_view(db, &path).await {
                tracing::error!("Failed to regenerate view {:?}: {}", path, e);
            }
        }
    }

    Ok(())
}

/// Regenerate a single view
pub async fn regenerate_view(db: &Database, view_def_path: &Path) -> anyhow::Result<()> {
    let content = fs::read_to_string(view_def_path).await?;
    let view_def: ViewDefinition = serde_yaml::from_str(&content)?;

    // Parse the stored query
    let query: mdql::SelectStmt = serde_json::from_value(view_def.query.clone())?;

    // Execute the query
    let collection = Collection::open(&query.from, &db.root);
    let mut docs = collection.list().await?;

    // Apply WHERE filter
    if let Some(ref where_clause) = query.where_clause {
        docs.retain(|doc| filter::evaluate(where_clause, doc));
    }

    // Apply ORDER BY
    if !query.order_by.is_empty() {
        docs.sort_by(|a, b| {
            for order in &query.order_by {
                let a_val = a.fields.get(&order.column);
                let b_val = b.fields.get(&order.column);
                let cmp = compare_opt_values(a_val, b_val);
                if cmp != std::cmp::Ordering::Equal {
                    return match order.direction {
                        mdql::OrderDirection::Asc => cmp,
                        mdql::OrderDirection::Desc => cmp.reverse(),
                    };
                }
            }
            std::cmp::Ordering::Equal
        });
    }

    // Apply pagination
    if let Some(offset) = query.offset {
        docs = docs.into_iter().skip(offset).collect();
    }
    if let Some(limit) = query.limit {
        docs.truncate(limit);
    }

    // Create output directory
    let output_dir = db.root.join("views").join(&view_def.name);
    fs::create_dir_all(&output_dir).await?;

    // Generate HTML output
    let html = generate_html(&view_def, &docs, db).await?;
    fs::write(output_dir.join("index.html"), html).await?;

    // Generate JSON output
    let json = generate_json(&docs)?;
    fs::write(output_dir.join("index.json"), json).await?;

    tracing::info!("Regenerated view: {}", view_def.name);

    Ok(())
}

async fn generate_html(view_def: &ViewDefinition, docs: &[Document], db: &Database) -> anyhow::Result<String> {
    let mut engine = if let Some(ref template_name) = view_def.template {
        // Load from templates directory
        let templates_dir = db.root.join(".mdby").join("templates");
        let mut engine = TemplateEngine::new(&templates_dir)?;

        // Also try to load the specific template file
        let template_path = templates_dir.join(template_name);
        if template_path.exists() {
            let content = fs::read_to_string(&template_path).await?;
            engine.add_template(template_name, &content)?;
        }

        engine
    } else {
        TemplateEngine::empty()
    };

    let template = if let Some(ref name) = view_def.template {
        name.as_str()
    } else {
        // Use default template based on collection/view name
        engine.add_template("default", TemplateEngine::default_list_template())?;
        "default"
    };

    engine.render(template, docs)
}

fn generate_json(docs: &[Document]) -> anyhow::Result<String> {
    let items: Vec<serde_json::Value> = docs.iter().map(|doc| {
        let mut obj = serde_json::Map::new();
        obj.insert("id".to_string(), serde_json::Value::String(doc.id.clone()));
        obj.insert("body".to_string(), serde_json::Value::String(doc.body.clone()));

        for (key, value) in &doc.fields {
            obj.insert(key.clone(), value_to_json(value));
        }

        serde_json::Value::Object(obj)
    }).collect();

    Ok(serde_json::to_string_pretty(&items)?)
}

fn value_to_json(value: &crate::storage::document::Value) -> serde_json::Value {
    use crate::storage::document::Value;
    match value {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(i) => serde_json::Value::Number((*i).into()),
        Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Array(arr) => serde_json::Value::Array(arr.iter().map(value_to_json).collect()),
        Value::Object(obj) => {
            let map: serde_json::Map<String, serde_json::Value> = obj
                .iter()
                .map(|(k, v)| (k.clone(), value_to_json(v)))
                .collect();
            serde_json::Value::Object(map)
        }
    }
}

fn compare_opt_values(
    a: Option<&crate::storage::document::Value>,
    b: Option<&crate::storage::document::Value>,
) -> std::cmp::Ordering {
    use crate::storage::document::Value;
    match (a, b) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (Some(_), None) => std::cmp::Ordering::Greater,
        (Some(Value::Int(a)), Some(Value::Int(b))) => a.cmp(b),
        (Some(Value::Float(a)), Some(Value::Float(b))) => {
            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
        }
        (Some(Value::String(a)), Some(Value::String(b))) => a.cmp(b),
        _ => std::cmp::Ordering::Equal,
    }
}

/// View definition stored in YAML
#[derive(Debug, serde::Deserialize)]
struct ViewDefinition {
    name: String,
    query: serde_json::Value,
    template: Option<String>,
}
