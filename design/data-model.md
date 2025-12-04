# MDBY Data Model

## Core Concepts

### Document

A document is the fundamental unit of data in MDBY. It consists of:

1. **ID** - Unique identifier within a collection (derived from filename)
2. **Fields** - Structured key-value pairs (stored as YAML frontmatter)
3. **Body** - Freeform markdown content
4. **Metadata** - System-managed properties (path, git hash, timestamps)

```rust
pub struct Document {
    pub id: String,
    pub path: PathBuf,
    pub fields: HashMap<String, Value>,
    pub body: String,
    pub meta: DocumentMeta,
}
```

### Value Types

Documents store values in a flexible type system:

```rust
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}
```

### Collection

A collection is a directory containing related documents:

```
collections/
└── todos/
    ├── task-1.md
    ├── task-2.md
    └── task-3.md
```

Collections can have an associated schema defining field types and constraints.

### Schema

A schema defines the structure of documents in a collection:

```rust
pub struct Schema {
    pub name: String,
    pub description: Option<String>,
    pub fields: HashMap<String, FieldDef>,
    pub id_strategy: IdStrategy,
}

pub struct FieldDef {
    pub field_type: FieldType,
    pub required: bool,
    pub unique: bool,
    pub indexed: bool,
    pub default: Option<Value>,
    pub description: Option<String>,
}
```

### Field Types

```rust
pub enum FieldType {
    String,
    Int,
    Float,
    Bool,
    Date,       // ISO 8601 date (YYYY-MM-DD)
    DateTime,   // ISO 8601 datetime
    Array(Box<FieldType>),
    Object,
    Ref(String), // Reference to another collection
}
```

## File Format

### Document File (.md)

```markdown
---
title: Buy groceries
done: false
priority: 1
tags:
  - shopping
  - urgent
assigned_to: user-123
due_date: 2024-01-20
---

Remember to check the pantry first.

## Shopping List
- Milk
- Bread
- Eggs
```

### Schema File (.yaml)

```yaml
name: todos
description: Task tracking collection
fields:
  title:
    type: string
    required: true
    description: Task title
  done:
    type: bool
    required: false
    default: false
  priority:
    type: int
    required: false
    indexed: true
  tags:
    type: array<string>
    required: false
  assigned_to:
    type: ref<users>
    required: false
  due_date:
    type: date
    required: false
    indexed: true
id_strategy: manual
```

### View Definition (.yaml)

```yaml
name: active_tasks
query:
  columns: ["*"]
  from: todos
  where_clause:
    BinaryOp:
      left: { Column: { Field: done } }
      op: Eq
      right: { Literal: { Bool: false } }
  order_by:
    - column: priority
      direction: Desc
template: task-list.html
```

## Relationships

### References

Documents can reference other documents using the `Ref` type:

```yaml
# In todos/task-1.md
---
title: Review PR
assigned_to: user-123  # References users/user-123.md
project: proj-456      # References projects/proj-456.md
---
```

### Future: Foreign Keys

```sql
CREATE COLLECTION todos (
    title STRING REQUIRED,
    assigned_to REF<users> REQUIRED,
    project REF<projects>
)
```

## Special Fields

Built-in fields accessible via `@` prefix:

| Field | Type | Description |
|-------|------|-------------|
| `@id` | String | Document identifier |
| `@body` | String | Markdown body content |
| `@path` | String | File path relative to collection |
| `@modified` | DateTime | Last modification time (from filesystem) |
| `@created` | DateTime | Creation time (from git history) |

## ID Strategies

### Manual (Default)

User provides the ID in INSERT statements:

```sql
INSERT INTO todos (id, title) VALUES ('task-1', 'Buy milk')
```

### UUID (Future)

Auto-generate UUIDs:

```sql
CREATE COLLECTION todos (
    title STRING REQUIRED
) WITH ID_STRATEGY = UUID
```

### Auto-Increment (Future)

Sequential integer IDs:

```sql
CREATE COLLECTION todos (
    title STRING REQUIRED
) WITH ID_STRATEGY = AUTO_INCREMENT
```

### Derived (Future)

Derive from another field (e.g., slug from title):

```sql
CREATE COLLECTION posts (
    title STRING REQUIRED
) WITH ID_STRATEGY = DERIVED(title, 'slug')
```

## Constraints

### Required

Field must be present and non-null:

```sql
title STRING REQUIRED
```

### Unique

Value must be unique across all documents:

```sql
email STRING REQUIRED UNIQUE
```

### Default

Default value if not provided:

```sql
done BOOL DEFAULT false
created_at DATETIME DEFAULT NOW()
```

### Indexed

Create an index for faster queries:

```sql
priority INT INDEXED
```

## Type Coercion

### Implicit Coercions

| From | To | Rule |
|------|-----|------|
| Int | Float | Always allowed |
| Float (whole) | Int | Allowed if no fractional part |
| Any | Null | Null matches any type |

### Date/DateTime Formats

Dates must be ISO 8601:
- Date: `YYYY-MM-DD` (e.g., `2024-01-15`)
- DateTime: `YYYY-MM-DDTHH:MM:SS[Z|±HH:MM]` (e.g., `2024-01-15T10:30:00Z`)

## Query Result Types

```rust
pub enum QueryResult {
    Documents(Vec<Document>),    // SELECT results
    Affected(usize),             // INSERT/UPDATE/DELETE count
    CollectionCreated(String),   // CREATE COLLECTION
    ViewCreated(String),         // CREATE VIEW
    Collections(Vec<String>),    // SHOW COLLECTIONS
    Views(Vec<String>),          // SHOW VIEWS
}
```

## Indexes (Future)

### Index File Format

```
.mdby/indexes/{collection}/{field}.idx
```

### Index Types

1. **B-Tree** - For range queries and ordering
2. **Hash** - For equality lookups
3. **Full-Text** - For CONTAINS queries

### Index Entry

```rust
pub struct IndexEntry {
    pub value: Value,
    pub document_ids: Vec<String>,
}
```

## Versioning

All changes are tracked in git:

```
commit abc123
Author: mdby <mdby@local>
Date:   Mon Jan 15 10:30:00 2024

    INSERT into todos: task-1
```

Document metadata includes git information:

```rust
pub struct DocumentMeta {
    pub git_hash: Option<String>,
    pub modified_at: Option<SystemTime>,
}
```
