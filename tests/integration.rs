//! Integration tests for MDBY
//!
//! Tests full query execution flows from parsing through to file system changes.

use mdby::{Database, QueryResult};
use tempfile::TempDir;

/// Helper to create a test database
async fn setup_test_db() -> (TempDir, Database) {
    let tmp = TempDir::new().expect("Failed to create temp dir");
    let db = Database::open(tmp.path()).await.expect("Failed to open database");
    (tmp, db)
}

/// Helper to execute a query and unwrap the result
async fn exec(db: &mut Database, query: &str) -> QueryResult {
    db.execute(query).await.expect(&format!("Query failed: {}", query))
}

// =============================================================================
// CREATE COLLECTION Tests
// =============================================================================

#[tokio::test]
async fn test_create_collection_basic() {
    let (_tmp, mut db) = setup_test_db().await;

    let result = exec(&mut db, "CREATE COLLECTION todos").await;
    assert!(matches!(result, QueryResult::CollectionCreated(name) if name == "todos"));

    // Verify directory exists
    assert!(_tmp.path().join("collections/todos").exists());
}

#[tokio::test]
async fn test_create_collection_with_schema() {
    let (_tmp, mut db) = setup_test_db().await;

    let result = exec(&mut db, "CREATE COLLECTION todos (title STRING REQUIRED, done BOOL DEFAULT false, priority INT)").await;

    assert!(matches!(result, QueryResult::CollectionCreated(_)));

    // Verify schema file exists
    assert!(_tmp.path().join(".mdby/schemas/todos.yaml").exists());
}

#[tokio::test]
async fn test_create_collection_if_not_exists() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;

    // Should not error with IF NOT EXISTS
    let result = exec(&mut db, "CREATE IF NOT EXISTS COLLECTION todos").await;
    assert!(matches!(result, QueryResult::CollectionCreated(_)));
}

#[tokio::test]
async fn test_create_collection_duplicate_fails() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;

    let result = db.execute("CREATE COLLECTION todos").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
}

// =============================================================================
// INSERT Tests
// =============================================================================

#[tokio::test]
async fn test_insert_basic() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;
    let result = exec(&mut db, "INSERT INTO todos (id, title) VALUES ('task-1', 'Buy milk')").await;

    assert!(matches!(result, QueryResult::Affected(1)));

    // Verify file exists
    assert!(_tmp.path().join("collections/todos/task-1.md").exists());
}

#[tokio::test]
async fn test_insert_with_multiple_fields() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;
    exec(&mut db, "INSERT INTO todos (id, title, done, priority) VALUES ('task-1', 'Buy milk', false, 5)").await;

    // Verify we can select it back
    let result = exec(&mut db, "SELECT * FROM todos WHERE id = 'task-1'").await;
    if let QueryResult::Documents(docs) = result {
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].id, "task-1");
    } else {
        panic!("Expected Documents");
    }
}

#[tokio::test]
async fn test_insert_duplicate_fails() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;
    exec(&mut db, "INSERT INTO todos (id, title) VALUES ('task-1', 'Buy milk')").await;

    let result = db.execute("INSERT INTO todos (id, title) VALUES ('task-1', 'Duplicate')").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_insert_requires_id() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;

    let result = db.execute("INSERT INTO todos (title) VALUES ('No ID')").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("id"));
}

#[tokio::test]
async fn test_insert_validates_schema_required_fields() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos (title STRING REQUIRED)").await;

    // Missing required field should fail
    let result = db.execute("INSERT INTO todos (id) VALUES ('task-1')").await;
    assert!(result.is_err());
}

// =============================================================================
// SELECT Tests
// =============================================================================

#[tokio::test]
async fn test_select_all() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;
    exec(&mut db, "INSERT INTO todos (id, title) VALUES ('task-1', 'First')").await;
    exec(&mut db, "INSERT INTO todos (id, title) VALUES ('task-2', 'Second')").await;

    let result = exec(&mut db, "SELECT * FROM todos").await;
    if let QueryResult::Documents(docs) = result {
        assert_eq!(docs.len(), 2);
    } else {
        panic!("Expected Documents");
    }
}

#[tokio::test]
async fn test_select_empty_collection() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;

    let result = exec(&mut db, "SELECT * FROM todos").await;
    if let QueryResult::Documents(docs) = result {
        assert_eq!(docs.len(), 0);
    } else {
        panic!("Expected Documents");
    }
}

#[tokio::test]
async fn test_select_nonexistent_collection() {
    let (_tmp, mut db) = setup_test_db().await;

    let result = db.execute("SELECT * FROM nonexistent").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not exist"));
}

#[tokio::test]
async fn test_select_with_where_equality() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;
    exec(&mut db, "INSERT INTO todos (id, title, done) VALUES ('task-1', 'First', true)").await;
    exec(&mut db, "INSERT INTO todos (id, title, done) VALUES ('task-2', 'Second', false)").await;

    let result = exec(&mut db, "SELECT * FROM todos WHERE done = true").await;
    if let QueryResult::Documents(docs) = result {
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].id, "task-1");
    } else {
        panic!("Expected Documents");
    }
}

#[tokio::test]
async fn test_select_where_by_id() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;
    exec(&mut db, "INSERT INTO todos (id, title) VALUES ('task-1', 'First')").await;
    exec(&mut db, "INSERT INTO todos (id, title) VALUES ('task-2', 'Second')").await;

    let result = exec(&mut db, "SELECT * FROM todos WHERE id = 'task-2'").await;
    if let QueryResult::Documents(docs) = result {
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].id, "task-2");
    } else {
        panic!("Expected Documents");
    }
}

#[tokio::test]
async fn test_select_with_order_by() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;
    exec(&mut db, "INSERT INTO todos (id, title, priority) VALUES ('task-1', 'Low', 1)").await;
    exec(&mut db, "INSERT INTO todos (id, title, priority) VALUES ('task-2', 'High', 10)").await;
    exec(&mut db, "INSERT INTO todos (id, title, priority) VALUES ('task-3', 'Med', 5)").await;

    let result = exec(&mut db, "SELECT * FROM todos ORDER BY priority DESC").await;
    if let QueryResult::Documents(docs) = result {
        assert_eq!(docs.len(), 3);
        assert_eq!(docs[0].id, "task-2"); // priority 10
        assert_eq!(docs[1].id, "task-3"); // priority 5
        assert_eq!(docs[2].id, "task-1"); // priority 1
    } else {
        panic!("Expected Documents");
    }
}

#[tokio::test]
async fn test_select_with_limit() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;
    for i in 1..=10 {
        exec(&mut db, &format!("INSERT INTO todos (id, title) VALUES ('task-{}', 'Task {}')", i, i)).await;
    }

    let result = exec(&mut db, "SELECT * FROM todos LIMIT 3").await;
    if let QueryResult::Documents(docs) = result {
        assert_eq!(docs.len(), 3);
    } else {
        panic!("Expected Documents");
    }
}

#[tokio::test]
async fn test_select_with_offset() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;
    exec(&mut db, "INSERT INTO todos (id, title, priority) VALUES ('task-1', 'First', 1)").await;
    exec(&mut db, "INSERT INTO todos (id, title, priority) VALUES ('task-2', 'Second', 2)").await;
    exec(&mut db, "INSERT INTO todos (id, title, priority) VALUES ('task-3', 'Third', 3)").await;

    let result = exec(&mut db, "SELECT * FROM todos ORDER BY priority LIMIT 2 OFFSET 1").await;
    if let QueryResult::Documents(docs) = result {
        assert_eq!(docs.len(), 2);
        // Should skip priority=1, get priority=2 and priority=3
    } else {
        panic!("Expected Documents");
    }
}

#[tokio::test]
async fn test_select_with_and_condition() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;
    exec(&mut db, "INSERT INTO todos (id, title, done, priority) VALUES ('task-1', 'A', true, 5)").await;
    exec(&mut db, "INSERT INTO todos (id, title, done, priority) VALUES ('task-2', 'B', false, 5)").await;
    exec(&mut db, "INSERT INTO todos (id, title, done, priority) VALUES ('task-3', 'C', true, 1)").await;

    let result = exec(&mut db, "SELECT * FROM todos WHERE done = true AND priority = 5").await;
    if let QueryResult::Documents(docs) = result {
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].id, "task-1");
    } else {
        panic!("Expected Documents");
    }
}

// =============================================================================
// UPDATE Tests
// =============================================================================

#[tokio::test]
async fn test_update_single_field() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;
    exec(&mut db, "INSERT INTO todos (id, title, done) VALUES ('task-1', 'Buy milk', false)").await;

    let result = exec(&mut db, "UPDATE todos SET done = true WHERE id = 'task-1'").await;
    assert!(matches!(result, QueryResult::Affected(1)));

    // Verify the update
    let result = exec(&mut db, "SELECT * FROM todos WHERE id = 'task-1'").await;
    if let QueryResult::Documents(docs) = result {
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].get("done").and_then(|v| v.as_bool()), Some(true));
    } else {
        panic!("Expected Documents");
    }
}

#[tokio::test]
async fn test_update_multiple_documents() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;
    exec(&mut db, "INSERT INTO todos (id, title, done) VALUES ('task-1', 'A', false)").await;
    exec(&mut db, "INSERT INTO todos (id, title, done) VALUES ('task-2', 'B', false)").await;
    exec(&mut db, "INSERT INTO todos (id, title, done) VALUES ('task-3', 'C', true)").await;

    let result = exec(&mut db, "UPDATE todos SET done = true WHERE done = false").await;
    assert!(matches!(result, QueryResult::Affected(2)));
}

#[tokio::test]
async fn test_update_no_matches() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;
    exec(&mut db, "INSERT INTO todos (id, title) VALUES ('task-1', 'Test')").await;

    let result = exec(&mut db, "UPDATE todos SET done = true WHERE id = 'nonexistent'").await;
    assert!(matches!(result, QueryResult::Affected(0)));
}

// =============================================================================
// DELETE Tests
// =============================================================================

#[tokio::test]
async fn test_delete_single() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;
    exec(&mut db, "INSERT INTO todos (id, title) VALUES ('task-1', 'Test')").await;

    let result = exec(&mut db, "DELETE FROM todos WHERE id = 'task-1'").await;
    assert!(matches!(result, QueryResult::Affected(1)));

    // Verify it's gone
    let result = exec(&mut db, "SELECT * FROM todos").await;
    if let QueryResult::Documents(docs) = result {
        assert_eq!(docs.len(), 0);
    } else {
        panic!("Expected Documents");
    }
}

#[tokio::test]
async fn test_delete_multiple() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;
    exec(&mut db, "INSERT INTO todos (id, title, done) VALUES ('task-1', 'A', true)").await;
    exec(&mut db, "INSERT INTO todos (id, title, done) VALUES ('task-2', 'B', true)").await;
    exec(&mut db, "INSERT INTO todos (id, title, done) VALUES ('task-3', 'C', false)").await;

    let result = exec(&mut db, "DELETE FROM todos WHERE done = true").await;
    assert!(matches!(result, QueryResult::Affected(2)));

    // Verify only one remains
    let result = exec(&mut db, "SELECT * FROM todos").await;
    if let QueryResult::Documents(docs) = result {
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].id, "task-3");
    } else {
        panic!("Expected Documents");
    }
}

// =============================================================================
// DROP Tests
// =============================================================================

#[tokio::test]
async fn test_drop_collection() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;
    exec(&mut db, "INSERT INTO todos (id, title) VALUES ('task-1', 'Test')").await;

    let result = exec(&mut db, "DROP COLLECTION todos").await;
    assert!(matches!(result, QueryResult::Affected(1)));

    // Verify it's gone
    assert!(!_tmp.path().join("collections/todos").exists());

    // Selecting should fail
    let result = db.execute("SELECT * FROM todos").await;
    assert!(result.is_err());
}

// =============================================================================
// VIEW Tests
// =============================================================================

#[tokio::test]
async fn test_create_view() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;

    let result = exec(&mut db, "CREATE VIEW active AS SELECT * FROM todos WHERE done = false").await;
    assert!(matches!(result, QueryResult::ViewCreated(name) if name == "active"));

    // Verify view definition exists
    assert!(_tmp.path().join(".mdby/views/active.yaml").exists());
}

#[tokio::test]
async fn test_create_view_with_template() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;

    let result = exec(&mut db, "CREATE VIEW active AS SELECT * FROM todos WHERE done = false TEMPLATE 'list.html'").await;
    assert!(matches!(result, QueryResult::ViewCreated(_)));
}

#[tokio::test]
async fn test_drop_view() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;
    exec(&mut db, "CREATE VIEW active AS SELECT * FROM todos").await;

    let result = exec(&mut db, "DROP VIEW active").await;
    assert!(matches!(result, QueryResult::Affected(1)));

    assert!(!_tmp.path().join(".mdby/views/active.yaml").exists());
}

// =============================================================================
// Security Tests
// =============================================================================

#[tokio::test]
async fn test_path_traversal_in_document_id_blocked() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;

    // These should all fail validation
    let attacks = vec![
        "INSERT INTO todos (id, title) VALUES ('../evil', 'test')",
        "INSERT INTO todos (id, title) VALUES ('foo/bar', 'test')",
        "INSERT INTO todos (id, title) VALUES ('foo\\bar', 'test')",
        "INSERT INTO todos (id, title) VALUES ('.hidden', 'test')",
    ];

    for attack in attacks {
        let result = db.execute(attack).await;
        assert!(result.is_err(), "Should have blocked: {}", attack);
    }
}

#[tokio::test]
async fn test_path_traversal_in_collection_name_blocked() {
    let (_tmp, mut db) = setup_test_db().await;

    // Valid names should parse
    let result = db.execute("CREATE COLLECTION my-collection").await;
    assert!(result.is_ok());

    // Invalid names should fail
    let result = db.execute("CREATE COLLECTION _hidden").await;
    assert!(result.is_err());
}

// =============================================================================
// Git Integration Tests
// =============================================================================

#[tokio::test]
async fn test_operations_create_git_commits() {
    let (_tmp, mut db) = setup_test_db().await;

    // Initial commit exists from database init
    let initial_hash = db.git.head_hash().unwrap();

    exec(&mut db, "CREATE COLLECTION todos").await;
    let after_create = db.git.head_hash().unwrap();
    assert_ne!(initial_hash, after_create);

    exec(&mut db, "INSERT INTO todos (id, title) VALUES ('task-1', 'Test')").await;
    let after_insert = db.git.head_hash().unwrap();
    assert_ne!(after_create, after_insert);
}

// =============================================================================
// Edge Cases
// =============================================================================

#[tokio::test]
async fn test_special_characters_in_string_values() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;

    // Test various special characters in values (not IDs)
    exec(&mut db, r#"INSERT INTO todos (id, title) VALUES ('task-1', 'Test with "quotes"')"#).await;

    let result = exec(&mut db, "SELECT * FROM todos").await;
    if let QueryResult::Documents(docs) = result {
        assert_eq!(docs.len(), 1);
    } else {
        panic!("Expected Documents");
    }
}

#[tokio::test]
async fn test_numeric_string_id() {
    let (_tmp, mut db) = setup_test_db().await;

    exec(&mut db, "CREATE COLLECTION todos").await;
    exec(&mut db, "INSERT INTO todos (id, title) VALUES ('123', 'Numeric ID')").await;

    let result = exec(&mut db, "SELECT * FROM todos WHERE id = '123'").await;
    if let QueryResult::Documents(docs) = result {
        assert_eq!(docs.len(), 1);
    } else {
        panic!("Expected Documents");
    }
}

// =============================================================================
// Schema Type Validation Tests
// =============================================================================

#[tokio::test]
async fn test_schema_type_validation_int_field() {
    let (_tmp, mut db) = setup_test_db().await;

    // Create collection with INT field
    exec(&mut db, "CREATE COLLECTION items (name STRING, count INT)").await;

    // Valid: insert with correct types
    let result = db.execute("INSERT INTO items (id, name, count) VALUES ('item-1', 'Widget', 42)").await;
    assert!(result.is_ok());

    // Invalid: string value for INT field
    let result = db.execute("INSERT INTO items (id, name, count) VALUES ('item-2', 'Gadget', 'not-a-number')").await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("TypeMismatch") || err_msg.contains("type") || err_msg.contains("expected"));
}

#[tokio::test]
async fn test_schema_type_validation_bool_field() {
    let (_tmp, mut db) = setup_test_db().await;

    // Create collection with BOOL field
    exec(&mut db, "CREATE COLLECTION flags (name STRING, enabled BOOL)").await;

    // Valid: insert with correct types
    let result = db.execute("INSERT INTO flags (id, name, enabled) VALUES ('flag-1', 'Feature', true)").await;
    assert!(result.is_ok());

    // Invalid: string value for BOOL field
    let result = db.execute("INSERT INTO flags (id, name, enabled) VALUES ('flag-2', 'Other', 'yes')").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_schema_type_validation_date_field() {
    let (_tmp, mut db) = setup_test_db().await;

    // Create collection with DATE field
    exec(&mut db, "CREATE COLLECTION events (title STRING, event_date DATE)").await;

    // Valid: ISO 8601 date
    let result = db.execute("INSERT INTO events (id, title, event_date) VALUES ('event-1', 'Meeting', '2024-01-15')").await;
    assert!(result.is_ok());

    // Invalid: non-date string
    let result = db.execute("INSERT INTO events (id, title, event_date) VALUES ('event-2', 'Party', 'next tuesday')").await;
    assert!(result.is_err());
}
