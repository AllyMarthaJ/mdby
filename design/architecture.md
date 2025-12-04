# MDBY Architecture

## Overview

MDBY is a document database that stores data as markdown files with YAML frontmatter, using git for version control and concurrency management.

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         MDBY Database                           │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │   MDQL      │  │   Views     │  │   Schema Registry       │  │
│  │   Parser    │  │   Engine    │  │   (Collection Defs)     │  │
│  └──────┬──────┘  └──────┬──────┘  └───────────┬─────────────┘  │
│         │                │                     │                │
│         ▼                ▼                     ▼                │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    Query Engine                             ││
│  │  (SELECT, INSERT, UPDATE, DELETE, CREATE VIEW)              ││
│  └──────────────────────────┬──────────────────────────────────┘│
│                             │                                   │
│                             ▼                                   │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                   Storage Layer                             ││
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  ││
│  │  │  Document   │  │  Index      │  │  Transaction        │  ││
│  │  │  Store      │  │  Manager    │  │  Manager            │  ││
│  │  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘  ││
│  └─────────┼────────────────┼────────────────────┼─────────────┘│
│            │                │                    │              │
│            ▼                ▼                    ▼              │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    Git Backend                              ││
│  │  (Commits, Branches, Merge Conflict Resolution)             ││
│  └─────────────────────────────────────────────────────────────┘│
│                             │                                   │
│                             ▼                                   │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │              File System (Markdown Files)                   ││
│  │  /collections/{name}/*.md   /views/{name}/*.md              ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

## Component Details

### 1. MDQL Parser (`mdql` crate)

The parser converts MDQL query strings into an Abstract Syntax Tree (AST).

**Location:** `mdql/src/`

**Key Files:**
- `ast.rs` - AST node definitions
- `parser.rs` - nom-based parser implementation
- `error.rs` - Parse error types

**Responsibilities:**
- Lexical analysis and tokenization
- Syntax validation
- AST construction
- Error reporting with position information

### 2. Query Engine (`src/query/`)

Executes parsed AST nodes against the storage layer.

**Key Files:**
- `executor.rs` - Statement execution
- `filter.rs` - WHERE clause evaluation

**Responsibilities:**
- Query planning (future: optimization)
- Statement dispatch
- Filter evaluation
- Result construction

### 3. Storage Layer (`src/storage/`)

Manages document persistence and retrieval.

**Key Files:**
- `document.rs` - Document struct and Value types
- `collection.rs` - Collection operations
- `frontmatter.rs` - YAML frontmatter parsing/rendering

**Responsibilities:**
- Document serialization/deserialization
- Collection directory management
- Frontmatter parsing
- File I/O operations

### 4. Schema Registry (`src/schema/`)

Manages collection schemas and validates documents.

**Key Files:**
- `mod.rs` - Schema definitions and validation

**Responsibilities:**
- Schema storage and retrieval
- Type validation
- Required field checking
- Default value application

### 5. Git Backend (`src/git/`)

Provides version control integration.

**Key Files:**
- `mod.rs` - Repository operations
- `conflict.rs` - Merge conflict resolution
- `sync.rs` - Remote synchronization

**Responsibilities:**
- Repository initialization
- Automatic commits on changes
- Conflict detection and resolution
- Remote push/pull operations

### 6. Views Engine (`src/views/`)

Generates static output from saved queries.

**Key Files:**
- `mod.rs` - View management
- `templates.rs` - Tera template rendering
- `regenerate.rs` - Batch regeneration

**Responsibilities:**
- View definition storage
- Query execution for views
- Template rendering
- Output file generation

### 7. Validation (`src/validation.rs`)

Input validation for security.

**Responsibilities:**
- Collection name validation
- Document ID validation
- Path traversal prevention
- Reserved name checking

### 8. Error Handling (`src/error.rs`)

Structured error types with context.

**Responsibilities:**
- Error categorization
- User-friendly messages
- Suggestion generation
- Error chaining

## Data Flow

### Query Execution Flow

```
User Input (MDQL string)
        │
        ▼
┌───────────────┐
│  MDQL Parser  │ ──▶ ParseError (if invalid)
└───────┬───────┘
        │ AST
        ▼
┌───────────────┐
│ Input Validator│ ──▶ ValidationError (if invalid names)
└───────┬───────┘
        │
        ▼
┌───────────────┐
│ Query Executor│
└───────┬───────┘
        │
        ▼
┌───────────────┐
│ Storage Layer │ ──▶ Read/Write files
└───────┬───────┘
        │
        ▼
┌───────────────┐
│  Git Backend  │ ──▶ Create commit
└───────┬───────┘
        │
        ▼
    QueryResult
```

### Document Storage Format

```markdown
---
title: Buy groceries
done: false
priority: 1
tags:
  - shopping
  - urgent
created_at: 2024-01-15T10:30:00Z
---

Remember to check the pantry first.

## Shopping List
- Milk
- Bread
- Eggs
```

## Directory Structure

```
database-root/
├── .git/                    # Git repository
├── .mdby/
│   ├── schemas/            # Collection schema definitions
│   │   └── todos.yaml
│   ├── views/              # View definitions
│   │   └── active.yaml
│   ├── templates/          # Tera templates for views
│   │   └── list.html
│   └── indexes/            # Future: index files
│       └── todos/
│           └── priority.idx
├── collections/
│   ├── todos/
│   │   ├── task-1.md
│   │   ├── task-2.md
│   │   └── task-3.md
│   └── users/
│       ├── user-1.md
│       └── user-2.md
└── views/                  # Generated view output
    └── active/
        └── index.html
```

## Design Decisions

### Why Markdown + YAML Frontmatter?

1. **Human Readable** - Can be edited in any text editor
2. **Git Friendly** - Clean diffs, easy merging
3. **Flexible** - Structured data (YAML) + freeform content (Markdown)
4. **Standard Formats** - No proprietary formats

### Why Git for Concurrency?

1. **Built-in History** - Full audit trail
2. **Conflict Resolution** - Well-understood merge semantics
3. **Distribution** - Easy sync across machines
4. **Tooling** - Leverage existing git tools

### Why Custom Query Language (MDQL)?

1. **SQL Familiarity** - Low learning curve
2. **Document Extensions** - CONTAINS, HAS TAG, @body
3. **Simplicity** - No need for full SQL complexity
4. **Type Safety** - Designed for document model

## Future Considerations

### Indexing Strategy

Indexes will be stored as separate files:
- B-tree structure for range queries
- Hash index for equality lookups
- Full-text index for CONTAINS queries

### Transaction Support

Transactions will use:
- Write-ahead logging
- Snapshot isolation
- Optimistic concurrency control

### Scaling

For larger datasets:
- Lazy loading (frontmatter first)
- Parallel I/O
- Memory-mapped files
- Query result streaming
