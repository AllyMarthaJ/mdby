//! MDQL - Markdown Query Language
//!
//! A SQL-like query language for MDBY markdown databases.
//!
//! # Syntax Overview
//!
//! ```mdql
//! -- Select all documents from a collection
//! SELECT * FROM todos;
//!
//! -- Select specific fields
//! SELECT title, done FROM todos WHERE done = false;
//!
//! -- Insert a new document
//! INSERT INTO todos (id, title, done) VALUES ('task-1', 'Buy milk', false);
//!
//! -- Update documents
//! UPDATE todos SET done = true WHERE id = 'task-1';
//!
//! -- Delete documents
//! DELETE FROM todos WHERE done = true;
//!
//! -- Create a view
//! CREATE VIEW active_todos AS
//!   SELECT * FROM todos WHERE done = false
//!   TEMPLATE 'todo-list.html';
//!
//! -- Create a collection with schema
//! CREATE COLLECTION todos (
//!   title STRING REQUIRED,
//!   done BOOL DEFAULT false,
//!   priority INT,
//!   tags ARRAY<STRING>
//! );
//! ```
//!
//! # Special Features
//!
//! - `@body` - Reference the markdown body content
//! - `@id` - Reference the document ID
//! - `@path` - Reference the file path
//! - `CONTAINS` - Full-text search in body
//! - `HAS TAG` - Check array membership

mod ast;
mod parser;
mod error;

pub use ast::*;
pub use error::ParseError;

/// Parse an MDQL query string into an AST
pub fn parse(input: &str) -> Result<Statement, ParseError> {
    parser::parse_statement(input)
}

/// Parse multiple MDQL statements (separated by semicolons)
pub fn parse_multi(input: &str) -> Result<Vec<Statement>, ParseError> {
    parser::parse_statements(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_select() {
        let stmt = parse("SELECT * FROM todos").unwrap();
        assert!(matches!(stmt, Statement::Select(_)));
    }

    #[test]
    fn test_parse_select_with_where() {
        let stmt = parse("SELECT title, done FROM todos WHERE done = false").unwrap();
        if let Statement::Select(select) = stmt {
            assert_eq!(select.from, "todos");
            assert!(select.where_clause.is_some());
        } else {
            panic!("Expected Select statement");
        }
    }

    #[test]
    fn test_parse_insert() {
        let stmt = parse("INSERT INTO todos (id, title) VALUES ('t1', 'Test')").unwrap();
        assert!(matches!(stmt, Statement::Insert(_)));
    }

    #[test]
    fn test_parse_update() {
        let stmt = parse("UPDATE todos SET done = true WHERE id = 'task-1'").unwrap();
        assert!(matches!(stmt, Statement::Update(_)));
    }

    #[test]
    fn test_parse_delete() {
        let stmt = parse("DELETE FROM todos WHERE done = true").unwrap();
        assert!(matches!(stmt, Statement::Delete(_)));
    }

    #[test]
    fn test_parse_create_view() {
        let stmt = parse("CREATE VIEW active AS SELECT * FROM todos WHERE done = false TEMPLATE 'list.html'").unwrap();
        assert!(matches!(stmt, Statement::CreateView(_)));
    }
}
