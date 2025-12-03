//! Template engine for views

use std::collections::HashMap;
use std::path::Path;
use tera::{Context, Tera};

use crate::storage::document::{Document, Value};

/// Template engine wrapper
pub struct TemplateEngine {
    tera: Tera,
}

impl TemplateEngine {
    /// Create a new template engine loading templates from a directory
    pub fn new(templates_dir: &Path) -> anyhow::Result<Self> {
        let pattern = templates_dir.join("**/*.html").display().to_string();
        let mut tera = Tera::new(&pattern).unwrap_or_else(|_| Tera::default());

        // Register custom filters
        tera.register_filter("markdown", markdown_filter);

        Ok(Self { tera })
    }

    /// Create an empty template engine
    pub fn empty() -> Self {
        let mut tera = Tera::default();
        tera.register_filter("markdown", markdown_filter);
        Self { tera }
    }

    /// Add a template from a string
    pub fn add_template(&mut self, name: &str, content: &str) -> anyhow::Result<()> {
        self.tera.add_raw_template(name, content)?;
        Ok(())
    }

    /// Render a template with documents
    pub fn render(&self, template_name: &str, documents: &[Document]) -> anyhow::Result<String> {
        let mut context = Context::new();
        context.insert("documents", &documents_to_json(documents));
        context.insert("count", &documents.len());

        let result = self.tera.render(template_name, &context)?;
        Ok(result)
    }

    /// Render an inline template string
    pub fn render_inline(&mut self, template: &str, documents: &[Document]) -> anyhow::Result<String> {
        self.add_template("__inline__", template)?;
        self.render("__inline__", documents)
    }

    /// Get the default list template
    pub fn default_list_template() -> &'static str {
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>{{ view_name | default(value="View") }}</title>
    <style>
        body { font-family: system-ui, sans-serif; max-width: 800px; margin: 2rem auto; padding: 0 1rem; }
        article { border-bottom: 1px solid #eee; padding: 1rem 0; }
        h2 { margin: 0 0 0.5rem; }
        .meta { color: #666; font-size: 0.9rem; }
        .body { margin-top: 0.5rem; }
    </style>
</head>
<body>
    <h1>{{ view_name | default(value="View") }}</h1>
    <p>{{ count }} document(s)</p>

    {% for doc in documents %}
    <article>
        <h2>{{ doc.title | default(value=doc.id) }}</h2>
        <div class="meta">
            {% if doc.tags %}<span>Tags: {{ doc.tags | join(sep=", ") }}</span>{% endif %}
        </div>
        {% if doc.body %}
        <div class="body">{{ doc.body | markdown | safe }}</div>
        {% endif %}
    </article>
    {% endfor %}
</body>
</html>"#
    }

    /// Get the default TODO list template
    pub fn todo_list_template() -> &'static str {
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>TODO List</title>
    <style>
        body { font-family: system-ui, sans-serif; max-width: 800px; margin: 2rem auto; padding: 0 1rem; }
        .todo { display: flex; align-items: flex-start; padding: 0.5rem 0; border-bottom: 1px solid #eee; }
        .checkbox { width: 1.5rem; height: 1.5rem; margin-right: 1rem; }
        .done { text-decoration: line-through; color: #999; }
        .priority { font-size: 0.8rem; padding: 0.2rem 0.5rem; border-radius: 4px; margin-left: 0.5rem; }
        .priority-high { background: #ffebee; color: #c62828; }
        .priority-medium { background: #fff3e0; color: #e65100; }
        .priority-low { background: #e8f5e9; color: #2e7d32; }
    </style>
</head>
<body>
    <h1>TODO List</h1>
    <p>{{ count }} item(s)</p>

    {% for doc in documents %}
    <div class="todo">
        <input type="checkbox" class="checkbox" {% if doc.done %}checked{% endif %} disabled>
        <div class="content {% if doc.done %}done{% endif %}">
            <strong>{{ doc.title | default(value=doc.id) }}</strong>
            {% if doc.priority %}
            <span class="priority priority-{{ doc.priority | lower | default(value='medium') }}">
                {{ doc.priority }}
            </span>
            {% endif %}
            {% if doc.body %}
            <div>{{ doc.body | markdown | safe }}</div>
            {% endif %}
        </div>
    </div>
    {% endfor %}
</body>
</html>"#
    }
}

/// Convert documents to JSON-serializable format
fn documents_to_json(documents: &[Document]) -> Vec<serde_json::Value> {
    documents.iter().map(|doc| {
        let mut obj = serde_json::Map::new();
        obj.insert("id".to_string(), serde_json::Value::String(doc.id.clone()));
        obj.insert("body".to_string(), serde_json::Value::String(doc.body.clone()));

        for (key, value) in &doc.fields {
            obj.insert(key.clone(), value_to_json(value));
        }

        serde_json::Value::Object(obj)
    }).collect()
}

fn value_to_json(value: &Value) -> serde_json::Value {
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

/// Tera filter to convert markdown to HTML
fn markdown_filter(value: &tera::Value, _args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
    let text = value.as_str().unwrap_or("");
    let parser = pulldown_cmark::Parser::new(text);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    Ok(tera::Value::String(html))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_inline() {
        let mut engine = TemplateEngine::empty();
        let mut doc = Document::new("test");
        doc.set("title", "Hello World");

        let result = engine.render_inline(
            "{% for d in documents %}{{ d.title }}{% endfor %}",
            &[doc],
        ).unwrap();

        assert_eq!(result, "Hello World");
    }
}
