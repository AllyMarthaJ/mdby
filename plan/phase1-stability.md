# Phase 1: Core Stability Plan

## Overview

Phase 1 focuses on completing core functionality and ensuring stability before adding advanced features.

## Current Status

### Completed
- [x] Basic CRUD operations (INSERT, SELECT, UPDATE, DELETE)
- [x] Collection management (CREATE, DROP)
- [x] WHERE clause with AND/OR/NOT
- [x] Comparison operators (=, !=, <, <=, >, >=)
- [x] CONTAINS for body search
- [x] HAS TAG for array membership
- [x] ORDER BY, LIMIT, OFFSET
- [x] Schema definitions and type validation
- [x] Input validation (security)
- [x] Error types with suggestions
- [x] Git backend (init, commit)
- [x] CLI with output formats
- [x] SHOW COLLECTIONS / SHOW VIEWS
- [x] JOIN parsing (AST only)
- [x] Views with templates
- [x] Integration tests (37+)

### Remaining Work

## Task 1: JOIN Execution

**Priority:** High
**Effort:** Medium
**Files:** `src/query/executor.rs`

### Description
The JOIN syntax is parsed but not executed. Need to implement multi-collection document fetching and merging.

### Implementation Steps

1. Update `execute_select` to detect JOINs
2. Load documents from all joined collections
3. Build a document map keyed by join fields
4. Implement nested loop join algorithm
5. Merge fields from matched documents
6. Handle LEFT/RIGHT join semantics (include non-matches with nulls)
7. Support qualified column projection

### Code Sketch

```rust
async fn execute_select_with_joins(
    db: &Database,
    stmt: &SelectStmt,
) -> anyhow::Result<Vec<Document>> {
    // Load primary collection
    let primary_docs = load_collection(&stmt.from, db).await?;

    // For each join, load and merge
    let mut result = primary_docs;
    for join in &stmt.joins {
        let join_docs = load_collection(&join.collection, db).await?;
        result = merge_documents(result, join_docs, &join.on, join.join_type)?;
    }

    Ok(result)
}
```

---

## Task 2: DESCRIBE COLLECTION Command

**Priority:** Medium
**Effort:** Low
**Files:** `mdql/src/ast.rs`, `mdql/src/parser.rs`, `src/query/executor.rs`

### Description
Add command to show collection schema information.

### Syntax
```sql
DESCRIBE COLLECTION todos
-- or
DESC todos
```

### Output
```
Collection: todos
Documents: 15

Fields:
  title     STRING   REQUIRED
  done      BOOL     DEFAULT false
  priority  INT      INDEXED
  tags      ARRAY<STRING>
```

### Implementation Steps

1. Add `Statement::DescribeCollection(String)` to AST
2. Add parser for `DESCRIBE COLLECTION <name>`
3. Add `QueryResult::SchemaInfo { ... }` variant
4. Implement `execute_describe_collection`
5. Format output in CLI

---

## Task 3: EXPLAIN Command

**Priority:** Low
**Effort:** Medium
**Files:** `mdql/src/ast.rs`, `mdql/src/parser.rs`, `src/query/executor.rs`

### Description
Show query execution plan for debugging and optimization.

### Syntax
```sql
EXPLAIN SELECT * FROM todos WHERE priority > 3
```

### Output
```
Query Plan:
  1. Scan collection 'todos' (15 documents)
  2. Filter: priority > 3
  3. Project: * (all fields)

Estimated rows: ~5
Index used: none
```

### Implementation Steps

1. Add `Statement::Explain(Box<Statement>)` to AST
2. Add parser for `EXPLAIN <statement>`
3. Create `QueryPlan` struct with plan nodes
4. Add `QueryResult::Plan(QueryPlan)` variant
5. Implement plan generation (without execution)
6. Format plan output

---

## Task 4: Improved Error Messages

**Priority:** Medium
**Effort:** Medium
**Files:** `mdql/src/error.rs`, `mdql/src/parser.rs`

### Description
Add line/column information to parse errors for better debugging.

### Current
```
Error: Parse error: Unexpected trailing content: foo
```

### Target
```
Error: Parse error at line 1, column 25:
  SELECT * FROM todos WHER done = false
                          ^^^^
  Expected: WHERE, ORDER, LIMIT, OFFSET, or end of query
```

### Implementation Steps

1. Track position during parsing
2. Update `ParseError` to include position
3. Add context window (surrounding text)
4. Implement error formatting with caret

---

## Task 5: Query Validation

**Priority:** Medium
**Effort:** Medium
**Files:** `src/query/validator.rs` (new)

### Description
Validate queries before execution to catch errors early.

### Validations

1. **Collection exists** - Check before SELECT/UPDATE/DELETE
2. **Fields exist** - Validate against schema if present
3. **Type compatibility** - Check WHERE clause types
4. **Join fields** - Verify join conditions reference valid fields

### Implementation Steps

1. Create `src/query/validator.rs`
2. Add `validate_query(db, &stmt) -> Result<(), ValidationError>`
3. Call from `execute()` before execution
4. Return specific validation errors

---

## Testing Plan

### Unit Tests
- [ ] JOIN document merging logic
- [ ] DESCRIBE output formatting
- [ ] EXPLAIN plan generation
- [ ] Error message formatting
- [ ] Query validation rules

### Integration Tests
- [ ] JOIN with matching documents
- [ ] LEFT JOIN with non-matches
- [ ] DESCRIBE existing collection
- [ ] DESCRIBE non-existent collection
- [ ] EXPLAIN various query types
- [ ] Validation error cases

---

## Dependencies

```
Phase 1 Tasks
├── Task 1 (JOIN) - Independent
├── Task 2 (DESCRIBE) - Independent
├── Task 3 (EXPLAIN) - Depends on query plan structure
├── Task 4 (Errors) - Independent
└── Task 5 (Validation) - Should come after schema work
```

## Estimated Timeline

| Task | Effort |
|------|--------|
| JOIN Execution | 4-6 hours |
| DESCRIBE | 1-2 hours |
| EXPLAIN | 2-4 hours |
| Error Messages | 2-3 hours |
| Query Validation | 3-4 hours |
| Testing | 2-3 hours |
| **Total** | **14-22 hours** |
