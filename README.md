# MDBY - Markdown Database

A human-readable database backed by markdown files with git for version control and concurrency.

## Overview

MDBY stores data as markdown files with YAML frontmatter, making your database:
- **Human-readable**: Open any document in a text editor
- **Version-controlled**: Full git history for every change
- **Portable**: Just files - no server required
- **Queryable**: SQL-like query language (MDQL)

## Installation

```bash
# From source
cargo install --path .

# Or build directly
cargo build --release
./target/release/mdby
```

## Quick Start

```bash
# Initialize a new database (creates a git repo)
cd my-project
mdby query "CREATE COLLECTION todos"

# Insert documents
mdby query "INSERT INTO todos (id, title, done) VALUES ('task-1', 'Buy groceries', false)"
mdby query "INSERT INTO todos (id, title, priority) VALUES ('task-2', 'Write docs', 3)"

# Query documents
mdby query "SELECT * FROM todos"
mdby query "SELECT * FROM todos WHERE done = false ORDER BY priority DESC"

# Update documents
mdby query "UPDATE todos SET done = true WHERE id = 'task-1'"

# Delete documents
mdby query "DELETE FROM todos WHERE done = true"
```

## Document Format

Documents are stored as markdown files with YAML frontmatter:

```markdown
---
title: Buy groceries
done: false
priority: 1
tags:
  - shopping
  - urgent
---

Remember to check the pantry first.

## Shopping List
- Milk
- Bread
- Eggs
```

## MDQL Query Language

MDQL is a SQL-like query language designed for document databases.

### CREATE COLLECTION

```sql
-- Simple collection
CREATE COLLECTION todos

-- With schema
CREATE COLLECTION todos (
    title STRING REQUIRED,
    done BOOL DEFAULT false,
    priority INT,
    due_date DATE
)

-- Idempotent creation
CREATE IF NOT EXISTS COLLECTION todos
```

### INSERT

```sql
-- Basic insert
INSERT INTO todos (id, title, done) VALUES ('task-1', 'Buy milk', false)

-- With document body
INSERT INTO todos (id, title) VALUES ('task-2', 'Write report')
BODY '## Report Outline\n\n- Introduction\n- Analysis\n- Conclusion'
```

### SELECT

```sql
-- All documents
SELECT * FROM todos

-- Specific fields
SELECT title, done FROM todos

-- With filtering
SELECT * FROM todos WHERE done = false
SELECT * FROM todos WHERE priority > 3
SELECT * FROM todos WHERE title CONTAINS 'urgent'

-- Sorting and pagination
SELECT * FROM todos ORDER BY priority DESC
SELECT * FROM todos LIMIT 10 OFFSET 20

-- Combined
SELECT title, priority FROM todos
WHERE done = false
ORDER BY priority DESC
LIMIT 5
```

### UPDATE

```sql
-- Update single field
UPDATE todos SET done = true WHERE id = 'task-1'

-- Update multiple fields
UPDATE todos SET priority = 5, done = false WHERE title CONTAINS 'urgent'
```

### DELETE

```sql
-- Delete specific document
DELETE FROM todos WHERE id = 'task-1'

-- Delete matching documents
DELETE FROM todos WHERE done = true
```

### DROP

```sql
-- Drop collection (deletes all documents)
DROP COLLECTION todos

-- Drop view
DROP VIEW completed_tasks
```

## Views

Views are saved queries that can generate static output files.

```sql
-- Create a view
CREATE VIEW completed_tasks AS
SELECT * FROM todos WHERE done = true
ORDER BY title

-- With custom template
CREATE VIEW task_report AS
SELECT * FROM todos
WITH TEMPLATE 'report.html'
```

Regenerate all views:
```bash
mdby views regenerate
```

## Schema Validation

Define schemas to enforce data types and required fields:

```sql
CREATE COLLECTION users (
    name STRING REQUIRED,
    email STRING REQUIRED UNIQUE,
    age INT,
    active BOOL DEFAULT true,
    created_at DATETIME
)
```

Supported types:
- `STRING` - Text values
- `INT` - Integer numbers
- `FLOAT` - Decimal numbers
- `BOOL` - true/false
- `DATE` - ISO 8601 date (YYYY-MM-DD)
- `DATETIME` - ISO 8601 datetime
- `ARRAY` - List of values
- `OBJECT` - Nested key-value pairs

Constraints:
- `REQUIRED` - Field must be present
- `UNIQUE` - Value must be unique across collection
- `DEFAULT value` - Default value if not provided
- `INDEXED` - Create index for faster queries

## CLI Reference

```bash
# Execute a query
mdby query "SELECT * FROM todos"

# Output formats
mdby query "SELECT * FROM todos" --format json
mdby query "SELECT * FROM todos" --format table
mdby query "SELECT * FROM todos" --format minimal

# Use custom database path
mdby --path /path/to/db query "SELECT * FROM todos"

# Regenerate views
mdby views regenerate

# Version info
mdby --version
```

## File Structure

```
my-database/
├── .mdby/
│   ├── schemas/           # Collection schemas
│   │   └── todos.yaml
│   └── views/             # View definitions
│       └── completed.yaml
├── collections/
│   └── todos/
│       ├── task-1.md
│       ├── task-2.md
│       └── task-3.md
├── views/                 # Generated view output
│   └── completed/
│       └── index.html
└── .git/                  # Git repository
```

## Git Integration

Every operation automatically creates a git commit:

```bash
# View history
cd my-database
git log --oneline

# Output:
# a1b2c3d UPDATE todos: 1 document(s)
# e4f5g6h INSERT into todos: task-3
# i7j8k9l INSERT into todos: task-2
# m0n1o2p INSERT into todos: task-1
# q3r4s5t CREATE COLLECTION todos
```

## Error Handling

MDBY provides helpful error messages with suggestions:

```
Error: Collection 'todoz' does not exist

Suggestion: Create the collection first with: CREATE COLLECTION <name>
```

## Development

```bash
# Run tests
cargo test

# Run with debug output
RUST_LOG=debug cargo run -- query "SELECT * FROM todos"

# Build release binary
cargo build --release
```

## License

MIT
