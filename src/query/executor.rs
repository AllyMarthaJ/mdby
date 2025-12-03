//! Query execution engine

use crate::storage::collection::Collection;
use crate::storage::document::{Document, Value};
use crate::validation::{validate_collection_name, validate_document_id, validate_view_name, validate_template_name};
use crate::{Database, QueryResult};
use mdql::{
    Column, CreateCollectionStmt, CreateViewStmt, DeleteStmt, InsertStmt,
    Literal, OrderDirection, SelectStmt, Statement, UpdateStmt,
};

use super::filter;

/// Execute an MDQL statement
pub async fn execute(db: &mut Database, stmt: Statement) -> anyhow::Result<QueryResult> {
    match stmt {
        Statement::Select(select) => execute_select(db, select).await,
        Statement::Insert(insert) => execute_insert(db, insert).await,
        Statement::Update(update) => execute_update(db, update).await,
        Statement::Delete(delete) => execute_delete(db, delete).await,
        Statement::CreateCollection(create) => execute_create_collection(db, create).await,
        Statement::CreateView(create) => execute_create_view(db, create).await,
        Statement::DropCollection(name) => execute_drop_collection(db, &name).await,
        Statement::DropView(name) => execute_drop_view(db, &name).await,
    }
}

async fn execute_select(db: &Database, stmt: SelectStmt) -> anyhow::Result<QueryResult> {
    validate_collection_name(&stmt.from)?;
    let collection = Collection::open(&stmt.from, &db.root);

    if !collection.exists().await {
        anyhow::bail!("Collection '{}' does not exist", stmt.from);
    }

    let mut docs = collection.list().await?;

    // Apply WHERE filter
    if let Some(ref where_clause) = stmt.where_clause {
        docs.retain(|doc| filter::evaluate(where_clause, doc));
    }

    // Apply ORDER BY
    if !stmt.order_by.is_empty() {
        docs.sort_by(|a, b| {
            for order in &stmt.order_by {
                let a_val = a.fields.get(&order.column);
                let b_val = b.fields.get(&order.column);

                let cmp = compare_values(a_val, b_val);
                if cmp != std::cmp::Ordering::Equal {
                    return match order.direction {
                        OrderDirection::Asc => cmp,
                        OrderDirection::Desc => cmp.reverse(),
                    };
                }
            }
            std::cmp::Ordering::Equal
        });
    }

    // Apply OFFSET
    if let Some(offset) = stmt.offset {
        if offset < docs.len() {
            docs = docs.into_iter().skip(offset).collect();
        } else {
            docs.clear();
        }
    }

    // Apply LIMIT
    if let Some(limit) = stmt.limit {
        docs.truncate(limit);
    }

    // Project columns (if not *)
    if !stmt.columns.iter().any(|c| matches!(c, Column::Star)) {
        docs = docs.into_iter().map(|doc| project_columns(&doc, &stmt.columns)).collect();
    }

    Ok(QueryResult::Documents(docs))
}

async fn execute_insert(db: &Database, stmt: InsertStmt) -> anyhow::Result<QueryResult> {
    validate_collection_name(&stmt.into)?;
    let collection = Collection::open(&stmt.into, &db.root);
    collection.ensure_exists().await?;

    // Build document from columns and values
    let id_idx = stmt.columns.iter().position(|c| c == "id");
    let id = id_idx
        .and_then(|i| stmt.values.get(i))
        .and_then(|v| match v {
            Literal::String(s) => Some(s.clone()),
            _ => None,
        })
        .ok_or_else(|| anyhow::anyhow!("INSERT requires an 'id' column"))?;

    validate_document_id(&id)?;
    let mut doc = Document::new(id);

    for (i, col) in stmt.columns.iter().enumerate() {
        if col != "id" {
            if let Some(val) = stmt.values.get(i) {
                doc.fields.insert(col.clone(), literal_to_value(val));
            }
        }
    }

    if let Some(body) = stmt.body {
        doc.body = body;
    }

    // Validate against schema if exists
    if let Some(schema) = db.schema.get(&stmt.into) {
        schema.validate(&doc)?;
    }

    collection.insert(&doc).await?;

    // Commit the change
    db.git.commit(&format!("INSERT into {}: {}", stmt.into, doc.id))?;

    Ok(QueryResult::Affected(1))
}

async fn execute_update(db: &Database, stmt: UpdateStmt) -> anyhow::Result<QueryResult> {
    validate_collection_name(&stmt.collection)?;
    let collection = Collection::open(&stmt.collection, &db.root);

    if !collection.exists().await {
        anyhow::bail!("Collection '{}' does not exist", stmt.collection);
    }

    let mut docs = collection.list().await?;

    // Filter documents to update
    if let Some(ref where_clause) = stmt.where_clause {
        docs.retain(|doc| filter::evaluate(where_clause, doc));
    }

    let count = docs.len();

    // Apply SET clauses
    for mut doc in docs {
        for set_clause in &stmt.set {
            let value = evaluate_set_value(&set_clause.value, &doc);
            doc.fields.insert(set_clause.column.clone(), value);
        }
        collection.upsert(&doc).await?;
    }

    if count > 0 {
        db.git.commit(&format!("UPDATE {}: {} document(s)", stmt.collection, count))?;
    }

    Ok(QueryResult::Affected(count))
}

async fn execute_delete(db: &Database, stmt: DeleteStmt) -> anyhow::Result<QueryResult> {
    validate_collection_name(&stmt.from)?;
    let collection = Collection::open(&stmt.from, &db.root);

    if !collection.exists().await {
        anyhow::bail!("Collection '{}' does not exist", stmt.from);
    }

    let mut docs = collection.list().await?;

    // Filter documents to delete
    if let Some(ref where_clause) = stmt.where_clause {
        docs.retain(|doc| filter::evaluate(where_clause, doc));
    }

    let count = docs.len();
    let ids: Vec<_> = docs.iter().map(|d| d.id.clone()).collect();

    for id in &ids {
        collection.delete(id).await?;
    }

    if count > 0 {
        db.git.commit(&format!("DELETE from {}: {} document(s)", stmt.from, count))?;
    }

    Ok(QueryResult::Affected(count))
}

async fn execute_create_collection(db: &mut Database, stmt: CreateCollectionStmt) -> anyhow::Result<QueryResult> {
    validate_collection_name(&stmt.name)?;
    let collection = Collection::open(&stmt.name, &db.root);

    if collection.exists().await {
        if stmt.if_not_exists {
            return Ok(QueryResult::CollectionCreated(stmt.name));
        }
        anyhow::bail!("Collection '{}' already exists", stmt.name);
    }

    collection.ensure_exists().await?;

    // Create schema from column definitions
    if !stmt.columns.is_empty() {
        let mut schema = crate::schema::Schema::new(&stmt.name);
        for col in stmt.columns {
            let field_def = crate::schema::FieldDef {
                field_type: datatype_to_fieldtype(&col.data_type),
                required: col.constraints.iter().any(|c| matches!(c, mdql::Constraint::Required)),
                unique: col.constraints.iter().any(|c| matches!(c, mdql::Constraint::Unique)),
                indexed: col.constraints.iter().any(|c| matches!(c, mdql::Constraint::Indexed)),
                default: col.constraints.iter().find_map(|c| {
                    if let mdql::Constraint::Default(lit) = c {
                        Some(literal_to_yaml(&lit))
                    } else {
                        None
                    }
                }),
                description: None,
            };
            schema.fields.insert(col.name, field_def);
        }
        db.schema.register(schema)?;
    }

    db.git.commit(&format!("CREATE COLLECTION {}", stmt.name))?;

    Ok(QueryResult::CollectionCreated(stmt.name))
}

async fn execute_create_view(db: &Database, stmt: CreateViewStmt) -> anyhow::Result<QueryResult> {
    validate_view_name(&stmt.name)?;
    // Also validate the source collection
    validate_collection_name(&stmt.query.from)?;
    // Validate template if provided
    if let Some(ref template) = stmt.template {
        validate_template_name(template)?;
    }

    // Views are stored in .mdby/views/{name}.yaml
    let view_path = db.root.join(".mdby").join("views");
    tokio::fs::create_dir_all(&view_path).await?;

    let view_file = view_path.join(format!("{}.yaml", stmt.name));

    if view_file.exists() && !stmt.if_not_exists {
        anyhow::bail!("View '{}' already exists", stmt.name);
    }

    // Serialize view definition
    let view_def = serde_yaml::to_string(&ViewDefinition {
        name: stmt.name.clone(),
        query: serde_json::to_value(&stmt.query)?,
        template: stmt.template,
    })?;

    tokio::fs::write(&view_file, view_def).await?;

    db.git.commit(&format!("CREATE VIEW {}", stmt.name))?;

    Ok(QueryResult::ViewCreated(stmt.name))
}

async fn execute_drop_collection(db: &Database, name: &str) -> anyhow::Result<QueryResult> {
    validate_collection_name(name)?;
    let collection_path = db.root.join("collections").join(name);

    if !collection_path.exists() {
        anyhow::bail!("Collection '{}' does not exist", name);
    }

    tokio::fs::remove_dir_all(&collection_path).await?;

    db.git.commit(&format!("DROP COLLECTION {}", name))?;

    Ok(QueryResult::Affected(1))
}

async fn execute_drop_view(db: &Database, name: &str) -> anyhow::Result<QueryResult> {
    validate_view_name(name)?;
    let view_file = db.root.join(".mdby").join("views").join(format!("{}.yaml", name));

    if !view_file.exists() {
        anyhow::bail!("View '{}' does not exist", name);
    }

    tokio::fs::remove_file(&view_file).await?;

    // Also remove generated view output
    let output_path = db.root.join("views").join(name);
    if output_path.exists() {
        tokio::fs::remove_dir_all(&output_path).await?;
    }

    db.git.commit(&format!("DROP VIEW {}", name))?;

    Ok(QueryResult::Affected(1))
}

// Helper functions

fn project_columns(doc: &Document, columns: &[Column]) -> Document {
    let mut result = Document::new(&doc.id);
    result.body = doc.body.clone();
    result.path = doc.path.clone();
    result.meta = doc.meta.clone();

    for col in columns {
        match col {
            Column::Star => {
                result.fields = doc.fields.clone();
            }
            Column::Field(name) => {
                if let Some(val) = doc.fields.get(name) {
                    result.fields.insert(name.clone(), val.clone());
                }
            }
            Column::Special(_) => {
                // Special fields are always available via the doc structure
            }
            Column::Expr { alias: _, .. } => {
                // TODO: Evaluate expression and add as alias
            }
        }
    }

    result
}

fn compare_values(a: Option<&Value>, b: Option<&Value>) -> std::cmp::Ordering {
    match (a, b) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (Some(_), None) => std::cmp::Ordering::Greater,
        (Some(Value::Int(a)), Some(Value::Int(b))) => a.cmp(b),
        (Some(Value::Float(a)), Some(Value::Float(b))) => {
            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
        }
        (Some(Value::String(a)), Some(Value::String(b))) => a.cmp(b),
        (Some(Value::Bool(a)), Some(Value::Bool(b))) => a.cmp(b),
        _ => std::cmp::Ordering::Equal,
    }
}

fn literal_to_value(lit: &Literal) -> Value {
    match lit {
        Literal::Null => Value::Null,
        Literal::Bool(b) => Value::Bool(*b),
        Literal::Int(i) => Value::Int(*i),
        Literal::Float(f) => Value::Float(*f),
        Literal::String(s) => Value::String(s.clone()),
        Literal::Array(arr) => Value::Array(arr.iter().map(literal_to_value).collect()),
    }
}

fn literal_to_yaml(lit: &Literal) -> serde_yaml::Value {
    match lit {
        Literal::Null => serde_yaml::Value::Null,
        Literal::Bool(b) => serde_yaml::Value::Bool(*b),
        Literal::Int(i) => serde_yaml::Value::Number((*i).into()),
        Literal::Float(f) => serde_yaml::Value::Number(serde_yaml::Number::from(*f)),
        Literal::String(s) => serde_yaml::Value::String(s.clone()),
        Literal::Array(arr) => serde_yaml::Value::Sequence(arr.iter().map(literal_to_yaml).collect()),
    }
}

fn datatype_to_fieldtype(dt: &mdql::DataType) -> crate::schema::FieldType {
    match dt {
        mdql::DataType::String => crate::schema::FieldType::String,
        mdql::DataType::Int => crate::schema::FieldType::Int,
        mdql::DataType::Float => crate::schema::FieldType::Float,
        mdql::DataType::Bool => crate::schema::FieldType::Bool,
        mdql::DataType::Date => crate::schema::FieldType::Date,
        mdql::DataType::DateTime => crate::schema::FieldType::DateTime,
        mdql::DataType::Object => crate::schema::FieldType::Object,
        mdql::DataType::Array(inner) => {
            crate::schema::FieldType::Array(Box::new(datatype_to_fieldtype(inner)))
        }
        mdql::DataType::Ref(name) => crate::schema::FieldType::Ref(name.clone()),
    }
}

fn evaluate_set_value(expr: &mdql::Expr, doc: &Document) -> Value {
    match expr {
        mdql::Expr::Literal(lit) => literal_to_value(lit),
        mdql::Expr::Column(mdql::Column::Field(name)) => {
            doc.fields.get(name).cloned().unwrap_or(Value::Null)
        }
        // TODO: Handle more complex expressions
        _ => Value::Null,
    }
}

/// View definition stored in YAML
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ViewDefinition {
    name: String,
    query: serde_json::Value,
    template: Option<String>,
}
