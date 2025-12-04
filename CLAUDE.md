# CLAUDE.md

MDBY is a markdown-based database with git version control. Rust workspace with `mdby` (main) and `mdql` (parser) crates.

## Commands

```bash
cargo build          # Build
cargo test           # Run tests
cargo run -- --db .  # Run CLI
```

## Guiding Principles

- **Read before writing** - Understand existing code before modifying
- **Test everything** - Every feature needs integration tests
- **Keep docs current** - Update docs alongside code changes
- **Simple over clever** - Prefer readable code over abstractions

## Documentation

| Document | Update When |
|----------|-------------|
| `ROADMAP.md` | Completing tasks, adding features |
| `design/mdql-grammar.md` | New SQL syntax |
| `design/data-model.md` | Data structure changes |
| `design/architecture.md` | New modules or components |
| `plan/phase*.md` | Working on that phase |

**Task format in ROADMAP.md and plans:**
- `- [x]` completed
- `- [ ]` pending

## Working with TODOs

The `plan/` directory contains implementation plans for each phase. When working on a feature:

1. Check the relevant `plan/phase*.md` for context and approach
2. Mark tasks in progress as you work
3. Check off completed tasks in both the plan file and `ROADMAP.md`
4. Add new tasks discovered during implementation

## Key Files

- `mdql/src/ast.rs` - Query language AST (add new statement types here)
- `mdql/src/parser.rs` - nom parsers (add syntax parsing here)
- `src/query/executor.rs` - Query execution (implement statement logic here)
- `src/query/filter.rs` - WHERE clause evaluation
- `tests/integration.rs` - Integration tests

## Adding Features

New SQL statement: AST → Parser → Executor → Tests → Docs

New expression: AST → Parser → Filter → Tests
