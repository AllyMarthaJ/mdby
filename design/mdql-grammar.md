# MDQL Grammar Specification

## Overview

MDQL (Markdown Query Language) is a SQL-like query language designed for document databases with markdown storage.

## Lexical Elements

### Keywords (case-insensitive)

```
SELECT, FROM, WHERE, ORDER, BY, ASC, DESC, LIMIT, OFFSET
INSERT, INTO, VALUES, BODY
UPDATE, SET
DELETE
CREATE, DROP, COLLECTION, VIEW, AS, IF, NOT, EXISTS
SHOW, COLLECTIONS, VIEWS
JOIN, INNER, LEFT, RIGHT, OUTER, ON
AND, OR, NOT, IN, LIKE, BETWEEN, IS, NULL, CONTAINS, HAS, TAG
STRING, INT, FLOAT, BOOL, DATE, DATETIME, ARRAY, OBJECT, REF
REQUIRED, UNIQUE, DEFAULT, INDEXED
TRUE, FALSE
```

### Identifiers

```
identifier = letter (letter | digit | '_' | '-')*
letter     = 'a'..'z' | 'A'..'Z'
digit      = '0'..'9'
```

### Literals

```
string_literal  = "'" (char | "''")* "'"
                | '"' (char | escape)* '"'
integer_literal = ['-'] digit+
float_literal   = ['-'] digit+ '.' digit+
bool_literal    = 'true' | 'false'
null_literal    = 'NULL'
array_literal   = '[' [literal (',' literal)*] ']'
```

### Special Fields

```
special_field = '@' ('id' | 'body' | 'path' | 'modified' | 'created')
```

### Qualified Names

```
qualified_name = identifier '.' identifier
```

## Statement Grammar

### SELECT Statement

```ebnf
select_stmt = 'SELECT' select_list
              'FROM' table_ref
              [join_clause*]
              ['WHERE' expr]
              ['ORDER' 'BY' order_list]
              ['LIMIT' integer]
              ['OFFSET' integer]

select_list = '*' | column (',' column)*

column = '*'
       | identifier
       | qualified_name
       | special_field

table_ref = identifier ['AS' identifier]

join_clause = join_type 'JOIN' identifier ['AS' identifier] 'ON' expr

join_type = ['INNER']
          | 'LEFT' ['OUTER']
          | 'RIGHT' ['OUTER']

order_list = order_item (',' order_item)*

order_item = identifier ['ASC' | 'DESC']
```

### INSERT Statement

```ebnf
insert_stmt = 'INSERT' 'INTO' identifier
              '(' column_list ')'
              'VALUES' '(' value_list ')'
              ['BODY' string_literal]

column_list = identifier (',' identifier)*

value_list = literal (',' literal)*
```

### UPDATE Statement

```ebnf
update_stmt = 'UPDATE' identifier
              'SET' set_list
              ['WHERE' expr]

set_list = set_clause (',' set_clause)*

set_clause = identifier '=' expr
```

### DELETE Statement

```ebnf
delete_stmt = 'DELETE' 'FROM' identifier
              ['WHERE' expr]
```

### CREATE COLLECTION Statement

```ebnf
create_collection = 'CREATE' ['IF' 'NOT' 'EXISTS'] 'COLLECTION' identifier
                    ['(' column_def_list ')']

column_def_list = column_def (',' column_def)*

column_def = identifier data_type constraint*

data_type = 'STRING' | 'INT' | 'FLOAT' | 'BOOL'
          | 'DATE' | 'DATETIME' | 'OBJECT'
          | 'ARRAY' '<' data_type '>'
          | 'REF' '<' identifier '>'

constraint = 'REQUIRED' | 'UNIQUE' | 'INDEXED'
           | 'DEFAULT' literal
```

### CREATE VIEW Statement

```ebnf
create_view = 'CREATE' ['IF' 'NOT' 'EXISTS'] 'VIEW' identifier
              'AS' select_stmt
              ['TEMPLATE' string_literal]
```

### DROP Statements

```ebnf
drop_collection = 'DROP' 'COLLECTION' identifier

drop_view = 'DROP' 'VIEW' identifier
```

### SHOW Statements

```ebnf
show_stmt = 'SHOW' ('COLLECTIONS' | 'VIEWS')
```

## Expression Grammar

```ebnf
expr = or_expr

or_expr = and_expr ('OR' and_expr)*

and_expr = not_expr ('AND' not_expr)*

not_expr = 'NOT' not_expr
         | comparison_expr

comparison_expr = contains_expr
                | has_tag_expr
                | is_null_expr
                | like_expr
                | in_expr
                | between_expr
                | binary_comparison

binary_comparison = primary_expr [comp_op primary_expr]

comp_op = '=' | '!=' | '<>' | '<' | '<=' | '>' | '>='

contains_expr = 'CONTAINS' '(' string_literal ')'

has_tag_expr = 'HAS' 'TAG' string_literal ['IN' identifier]

is_null_expr = primary_expr 'IS' ['NOT'] 'NULL'

like_expr = primary_expr ['NOT'] 'LIKE' string_literal

in_expr = primary_expr ['NOT'] 'IN' '(' value_list ')'

between_expr = primary_expr ['NOT'] 'BETWEEN' primary_expr 'AND' primary_expr

primary_expr = '(' expr ')'
             | literal
             | special_field
             | qualified_name
             | identifier
```

## Examples

### Basic Queries

```sql
-- Select all
SELECT * FROM todos

-- Select specific columns
SELECT title, done FROM todos

-- With WHERE clause
SELECT * FROM todos WHERE done = false AND priority > 3

-- With ORDER BY and LIMIT
SELECT * FROM todos ORDER BY priority DESC LIMIT 10
```

### Document-Specific Features

```sql
-- Full-text search in body
SELECT * FROM notes WHERE CONTAINS('meeting')

-- Array membership
SELECT * FROM todos WHERE HAS TAG 'urgent'
SELECT * FROM todos WHERE HAS TAG 'work' IN tags

-- Special fields
SELECT @id, @body FROM todos WHERE @path LIKE '%.md'
```

### Joins

```sql
-- Basic join
SELECT todos.title, users.name
FROM todos
JOIN users ON todos.user_id = users.id

-- Left join with alias
SELECT t.title, u.name
FROM todos AS t
LEFT JOIN users AS u ON t.user_id = u.id
```

### Schema Definition

```sql
CREATE COLLECTION users (
    name STRING REQUIRED,
    email STRING REQUIRED UNIQUE,
    age INT,
    active BOOL DEFAULT true,
    roles ARRAY<STRING>,
    created_at DATETIME
)
```

### Views

```sql
CREATE VIEW active_tasks AS
SELECT * FROM todos
WHERE done = false
ORDER BY priority DESC
TEMPLATE 'task-list.html'
```

## Differences from SQL

| Feature | SQL | MDQL |
|---------|-----|------|
| Tables | CREATE TABLE | CREATE COLLECTION |
| Primary Key | AUTO_INCREMENT, SERIAL | Manual 'id' column |
| Body Content | N/A | BODY clause, @body field |
| Full-text | MATCH AGAINST | CONTAINS() |
| Array Membership | JSON functions | HAS TAG |
| File Path | N/A | @path field |
| Aliases | Optional AS | Required AS |

## Reserved Words

The following words cannot be used as unquoted identifiers:

```
SELECT, FROM, WHERE, ORDER, BY, ASC, DESC, LIMIT, OFFSET,
INSERT, INTO, VALUES, UPDATE, SET, DELETE, CREATE, DROP,
COLLECTION, VIEW, AS, IF, NOT, EXISTS, JOIN, INNER, LEFT,
RIGHT, OUTER, ON, AND, OR, IN, LIKE, BETWEEN, IS, NULL,
CONTAINS, HAS, TAG, SHOW, COLLECTIONS, VIEWS, STRING, INT,
FLOAT, BOOL, DATE, DATETIME, ARRAY, OBJECT, REF, REQUIRED,
UNIQUE, DEFAULT, INDEXED, TRUE, FALSE, BODY, TEMPLATE
```
