# Phase 2: Performance & Indexing Plan

## Overview

Phase 2 focuses on making MDBY performant for larger datasets through indexing and query optimization.

## Goals

1. Sub-second queries on collections with 10,000+ documents
2. Automatic index maintenance
3. Query planner that leverages indexes
4. Baseline performance benchmarks

## Current Limitations

- Full collection scan for every query
- No index support despite `INDEXED` constraint
- Sequential document loading
- No query caching

---

## Task 1: Index File Format

**Priority:** High
**Effort:** Medium

### Description
Define the on-disk format for indexes.

### Design

**Location:** `.mdby/indexes/{collection}/{field}.idx`

**Format:** Binary file with header + entries

```
Header (32 bytes):
  - Magic: "MDIX" (4 bytes)
  - Version: u32 (4 bytes)
  - Index Type: u8 (1 byte) - 0=BTree, 1=Hash, 2=FullText
  - Field Type: u8 (1 byte)
  - Entry Count: u64 (8 bytes)
  - Reserved: 14 bytes

Entries (variable):
  For BTree:
    - Value length: u32
    - Value bytes: [u8]
    - Doc ID count: u32
    - Doc IDs: [String with length prefix]
```

### Implementation Steps

1. Create `src/index/mod.rs`
2. Define `Index` trait
3. Implement `BTreeIndex` struct
4. Add serialization/deserialization
5. Add index file I/O

---

## Task 2: Index Manager

**Priority:** High
**Effort:** Medium

### Description
Manage index lifecycle (create, update, delete).

### API

```rust
pub struct IndexManager {
    indexes: HashMap<(String, String), Box<dyn Index>>,
}

impl IndexManager {
    pub fn load_indexes(db_path: &Path) -> Result<Self>;
    pub fn get_index(&self, collection: &str, field: &str) -> Option<&dyn Index>;
    pub fn create_index(&mut self, collection: &str, field: &str, field_type: FieldType) -> Result<()>;
    pub fn drop_index(&mut self, collection: &str, field: &str) -> Result<()>;
    pub fn update_on_insert(&mut self, collection: &str, doc: &Document) -> Result<()>;
    pub fn update_on_update(&mut self, collection: &str, old: &Document, new: &Document) -> Result<()>;
    pub fn update_on_delete(&mut self, collection: &str, doc: &Document) -> Result<()>;
}
```

### Implementation Steps

1. Create `src/index/manager.rs`
2. Implement index loading on database open
3. Hook into executor for automatic updates
4. Handle index rebuilding

---

## Task 3: BTree Index Implementation

**Priority:** High
**Effort:** High

### Description
Implement B-tree index for range queries.

### Operations

```rust
pub trait Index {
    fn lookup_eq(&self, value: &Value) -> Vec<String>;
    fn lookup_range(&self, low: Option<&Value>, high: Option<&Value>) -> Vec<String>;
    fn insert(&mut self, value: &Value, doc_id: &str);
    fn remove(&mut self, value: &Value, doc_id: &str);
    fn save(&self, path: &Path) -> Result<()>;
    fn load(path: &Path) -> Result<Self>;
}
```

### Implementation Steps

1. Use `std::collections::BTreeMap` for in-memory structure
2. Implement disk persistence
3. Handle multi-value entries (same value, multiple docs)
4. Add range query support
5. Optimize for common cases

---

## Task 4: Query Planner

**Priority:** High
**Effort:** High

### Description
Analyze queries and choose optimal execution strategy.

### Plan Types

```rust
pub enum PlanNode {
    // Scan entire collection
    FullScan { collection: String },

    // Use index for lookup
    IndexLookup {
        collection: String,
        field: String,
        condition: IndexCondition,
    },

    // Filter results
    Filter {
        input: Box<PlanNode>,
        predicate: Expr,
    },

    // Sort results
    Sort {
        input: Box<PlanNode>,
        order: Vec<OrderBy>,
    },

    // Limit results
    Limit {
        input: Box<PlanNode>,
        count: usize,
        offset: usize,
    },
}

pub enum IndexCondition {
    Eq(Value),
    Range { low: Option<Value>, high: Option<Value> },
}
```

### Query Planning Algorithm

```
1. Parse WHERE clause into conditions
2. For each condition:
   a. Check if indexed field
   b. Check if condition is index-compatible (=, <, <=, >, >=, BETWEEN)
3. Choose best index (most selective)
4. Build plan:
   - If good index: IndexLookup -> Filter (remaining conditions) -> Sort -> Limit
   - If no index: FullScan -> Filter -> Sort -> Limit
```

### Implementation Steps

1. Create `src/query/planner.rs`
2. Implement condition extraction from WHERE
3. Implement index selection logic
4. Build plan tree
5. Integrate with executor

---

## Task 5: Index-Aware Executor

**Priority:** High
**Effort:** Medium

### Description
Execute queries using the query plan.

### Changes to Executor

```rust
async fn execute_select(db: &Database, stmt: SelectStmt) -> Result<QueryResult> {
    // Generate plan
    let plan = planner::plan_query(&stmt, &db.index_manager)?;

    // Execute plan
    let docs = execute_plan(db, plan).await?;

    // Project columns
    let results = project_columns(docs, &stmt.columns);

    Ok(QueryResult::Documents(results))
}

async fn execute_plan(db: &Database, plan: PlanNode) -> Result<Vec<Document>> {
    match plan {
        PlanNode::FullScan { collection } => {
            Collection::open(&collection, &db.root).list().await
        }
        PlanNode::IndexLookup { collection, field, condition } => {
            let index = db.index_manager.get_index(&collection, &field)?;
            let doc_ids = match condition {
                IndexCondition::Eq(v) => index.lookup_eq(&v),
                IndexCondition::Range { low, high } => index.lookup_range(low.as_ref(), high.as_ref()),
            };
            load_documents_by_ids(&collection, &doc_ids, &db.root).await
        }
        PlanNode::Filter { input, predicate } => {
            let docs = execute_plan(db, *input).await?;
            Ok(docs.into_iter().filter(|d| filter::evaluate(&predicate, d)).collect())
        }
        // ...
    }
}
```

---

## Task 6: CREATE INDEX Command

**Priority:** Medium
**Effort:** Low

### Description
Allow manual index creation.

### Syntax

```sql
CREATE INDEX ON todos(priority)
CREATE INDEX ON todos(due_date)
DROP INDEX ON todos(priority)
```

### Implementation Steps

1. Add `Statement::CreateIndex` and `Statement::DropIndex`
2. Add parsers
3. Implement executor functions
4. Trigger index rebuild

---

## Task 7: ANALYZE Command

**Priority:** Low
**Effort:** Medium

### Description
Gather statistics for query optimization.

### Syntax

```sql
ANALYZE todos
ANALYZE  -- All collections
```

### Statistics Gathered

- Document count
- Field cardinality (unique values)
- Value distribution (histogram)
- Null percentage

### Storage

```yaml
# .mdby/stats/todos.yaml
collection: todos
document_count: 1523
analyzed_at: 2024-01-15T10:30:00Z
fields:
  priority:
    cardinality: 5
    null_percent: 2.3
    distribution:
      1: 312
      2: 456
      3: 398
      4: 234
      5: 123
  done:
    cardinality: 2
    null_percent: 0
```

---

## Task 8: Query Caching

**Priority:** Low
**Effort:** Medium

### Description
Cache query results for repeated queries.

### Design

- LRU cache with configurable size
- Cache key: normalized query string
- Invalidate on INSERT/UPDATE/DELETE to collection
- Optional per-query cache bypass

### Implementation

```rust
pub struct QueryCache {
    cache: LruCache<String, Vec<Document>>,
    max_size: usize,
}

impl QueryCache {
    pub fn get(&self, query: &str) -> Option<&Vec<Document>>;
    pub fn put(&mut self, query: &str, results: Vec<Document>);
    pub fn invalidate_collection(&mut self, collection: &str);
}
```

---

## Task 9: Benchmarks

**Priority:** Medium
**Effort:** Medium

### Description
Create benchmark suite for performance tracking.

### Benchmarks

1. **Insert throughput** - Documents per second
2. **Select all** - Time to scan N documents
3. **Select with filter** - Indexed vs non-indexed
4. **Select with sort** - Various sizes
5. **Update single** - By ID lookup
6. **Delete batch** - Multiple documents

### Implementation

Use `criterion` crate for benchmarking:

```rust
// benches/query_benchmarks.rs
fn bench_select_all(c: &mut Criterion) {
    let db = setup_benchmark_db(1000);

    c.bench_function("select_all_1000", |b| {
        b.iter(|| {
            black_box(db.execute("SELECT * FROM bench"))
        })
    });
}
```

---

## Testing Plan

### Unit Tests
- [ ] Index serialization/deserialization
- [ ] BTree operations (insert, delete, lookup, range)
- [ ] Query plan generation
- [ ] Plan execution

### Integration Tests
- [ ] Index creation on collection with schema
- [ ] Automatic index updates on INSERT
- [ ] Automatic index updates on UPDATE
- [ ] Automatic index updates on DELETE
- [ ] Query uses index when available
- [ ] Query falls back to scan when no index

### Performance Tests
- [ ] 1,000 document collection benchmarks
- [ ] 10,000 document collection benchmarks
- [ ] 100,000 document collection benchmarks

---

## Dependencies

```
Phase 2 Tasks
├── Task 1 (File Format) - Foundation
├── Task 2 (Manager) - Depends on 1
├── Task 3 (BTree) - Depends on 1
├── Task 4 (Planner) - Depends on 2, 3
├── Task 5 (Executor) - Depends on 4
├── Task 6 (CREATE INDEX) - Depends on 2
├── Task 7 (ANALYZE) - Depends on 2
├── Task 8 (Caching) - Independent
└── Task 9 (Benchmarks) - Independent
```

## File Changes

New files:
- `src/index/mod.rs`
- `src/index/btree.rs`
- `src/index/manager.rs`
- `src/query/planner.rs`
- `src/query/cache.rs`
- `benches/query_benchmarks.rs`

Modified files:
- `src/lib.rs` - Add index module, IndexManager to Database
- `src/query/executor.rs` - Use planner and index
- `mdql/src/ast.rs` - Add CREATE INDEX, DROP INDEX, ANALYZE
- `mdql/src/parser.rs` - Parse new statements
