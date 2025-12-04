# MDBY Roadmap

This document outlines the development roadmap for MDBY (Markdown Database).

## Current Status

MDBY is functional with core CRUD operations, schema validation, views, and git integration. The following phases outline remaining work to reach production readiness and future enhancements.

---

## Phase 1: Core Stability (Current)

**Goal:** Solid foundation with comprehensive testing and security.

### Completed
- [x] Document storage with YAML frontmatter
- [x] Collection management (CREATE, DROP)
- [x] CRUD operations (INSERT, SELECT, UPDATE, DELETE)
- [x] WHERE clause filtering with AND/OR/NOT
- [x] ORDER BY, LIMIT, OFFSET
- [x] Schema definitions with type validation
- [x] Input validation (path traversal prevention)
- [x] Structured error types with suggestions
- [x] Git backend for version control
- [x] CLI with multiple output formats (table, JSON, minimal)
- [x] SHOW COLLECTIONS / SHOW VIEWS commands
- [x] JOIN syntax parsing (AST support)
- [x] Views with Tera templates
- [x] Comprehensive integration tests (37+ tests)

### TODO
- [ ] Markdown
- [ ] Implement JOIN execution (multi-collection fetching and merging)
- [ ] Add DESCRIBE COLLECTION command (show schema)
- [ ] Add EXPLAIN command (show query plan)
- [ ] Improve error messages with line/column information
- [ ] Add query validation before execution

---

## Phase 2: Performance & Indexing

**Goal:** Make queries fast on larger datasets.

### TODO
- [ ] Implement basic B-tree indexes for indexed fields
- [ ] Add index file storage (`.mdby/indexes/{collection}/{field}.idx`)
- [ ] Automatic index updates on INSERT/UPDATE/DELETE
- [ ] Query planner that uses indexes when available
- [ ] Add ANALYZE command to gather statistics
- [ ] Implement query caching for repeated queries
- [ ] Lazy document loading (load frontmatter first, body on demand)
- [ ] Parallel document loading for large collections
- [ ] Benchmark suite with performance targets

---

## Phase 3: Git Sync & Collaboration

**Goal:** Enable multi-user collaboration with conflict resolution.

### TODO
- [ ] Implement `mdby sync` command (push/pull)
- [ ] Automatic conflict detection on pull
- [ ] Document-aware merge strategies:
  - [ ] Field-level merging (non-conflicting field changes)
  - [ ] Body concatenation with markers
  - [ ] Theirs-wins / Ours-wins strategies
- [ ] Conflict resolution UI in REPL
- [ ] Remote configuration management
- [ ] Branch support for isolated changes
- [ ] Merge branch command
- [ ] Sync status indicators

---

## Phase 4: Advanced Queries

**Goal:** Full SQL-like query capabilities.

### TODO
- [ ] Full JOIN execution (INNER, LEFT, RIGHT)
- [ ] Subqueries in WHERE clause
- [ ] Aggregate functions (COUNT, SUM, AVG, MIN, MAX)
- [ ] GROUP BY clause
- [ ] HAVING clause
- [ ] DISTINCT keyword
- [ ] UNION / INTERSECT / EXCEPT
- [ ] Common Table Expressions (WITH clause)
- [ ] Window functions (basic)
- [ ] Full-text search improvements (stemming, relevance)

---

## Phase 5: Transactions & Reliability

**Goal:** ACID-like guarantees for multi-document operations.

### TODO
- [ ] BEGIN / COMMIT / ROLLBACK commands
- [ ] Transaction isolation (snapshot reads)
- [ ] Atomic multi-document writes
- [ ] Write-ahead logging for crash recovery
- [ ] Automatic rollback on error
- [ ] Savepoints within transactions
- [ ] Lock management for concurrent access
- [ ] Deadlock detection

---

## Phase 6: Schema Evolution

**Goal:** Safe schema changes over time.

### TODO
- [ ] ALTER COLLECTION ADD COLUMN
- [ ] ALTER COLLECTION DROP COLUMN
- [ ] ALTER COLLECTION RENAME COLUMN
- [ ] ALTER COLLECTION MODIFY COLUMN (type changes)
- [ ] Migration scripts support
- [ ] Schema versioning
- [ ] Backward compatibility checks
- [ ] Data migration helpers

---

## Phase 7: Developer Experience

**Goal:** Make MDBY delightful to use.

### TODO
- [ ] Syntax highlighting for MDQL
- [ ] VSCode extension
- [ ] Auto-completion in REPL
- [ ] Query history in REPL
- [ ] Import from JSON/CSV
- [ ] Export to JSON/CSV
- [ ] Database dump/restore
- [ ] Web-based admin UI
- [ ] GraphQL API layer
- [ ] REST API layer
- [ ] SDK for Rust, Python, JavaScript

---

## Phase 8: Advanced Features

**Goal:** Enterprise and advanced use cases.

### TODO
- [ ] Triggers (on INSERT/UPDATE/DELETE)
- [ ] Computed/virtual fields
- [ ] Foreign key constraints with referential integrity
- [ ] Cascading deletes
- [ ] Audit logging
- [ ] Row-level security
- [ ] Encryption at rest
- [ ] Compression for large documents
- [ ] Attachments/binary file support
- [ ] Webhooks for change notifications
- [ ] Replication to multiple directories

---

## Version Milestones

| Version | Phase | Target Features |
|---------|-------|-----------------|
| 0.1.0   | 1     | Current state - core CRUD, schemas, views |
| 0.2.0   | 2     | Basic indexing, query optimization |
| 0.3.0   | 3     | Git sync with conflict resolution |
| 0.4.0   | 4     | JOINs, aggregates, GROUP BY |
| 0.5.0   | 5     | Transactions |
| 1.0.0   | 6-7   | Schema evolution, great DX |

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to contribute to MDBY development.

Priority is given to:
1. Bug fixes and security issues
2. Phase 1-3 items (core stability)
3. Performance improvements
4. Documentation
