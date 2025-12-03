//! MDQL Parser using nom
//!
//! Parses MDQL query strings into AST nodes.

use nom::{
    IResult,
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_while1},
    character::complete::{char, multispace0, multispace1, digit1, none_of},
    combinator::{map, opt, value},
    multi::{separated_list0, separated_list1, many0},
    sequence::{delimited, preceded, terminated, tuple},
};

use crate::ast::*;
use crate::error::ParseError;

/// Parse a complete statement
pub fn parse_statement(input: &str) -> Result<Statement, ParseError> {
    let input = input.trim();
    let (remaining, stmt) = statement(input)?;

    // Check for trailing content (ignoring whitespace and semicolons)
    let remaining = remaining.trim().trim_end_matches(';').trim();
    if !remaining.is_empty() {
        return Err(ParseError::new(format!("Unexpected trailing content: {}", remaining)));
    }

    Ok(stmt)
}

/// Parse multiple statements separated by semicolons
pub fn parse_statements(input: &str) -> Result<Vec<Statement>, ParseError> {
    let mut statements = Vec::new();
    let mut remaining = input.trim();

    while !remaining.is_empty() {
        // Skip leading whitespace and empty statements
        remaining = remaining.trim().trim_start_matches(';').trim();
        if remaining.is_empty() {
            break;
        }

        let (rest, stmt) = statement(remaining)?;
        statements.push(stmt);
        remaining = rest.trim().trim_start_matches(';').trim();
    }

    Ok(statements)
}

// ============================================================================
// Statement Parsers
// ============================================================================

fn statement(input: &str) -> IResult<&str, Statement> {
    alt((
        map(select_stmt, Statement::Select),
        map(insert_stmt, Statement::Insert),
        map(update_stmt, Statement::Update),
        map(delete_stmt, Statement::Delete),
        map(create_collection_stmt, Statement::CreateCollection),
        map(create_view_stmt, Statement::CreateView),
        map(drop_collection_stmt, Statement::DropCollection),
        map(drop_view_stmt, Statement::DropView),
    ))(input)
}

// ============================================================================
// SELECT
// ============================================================================

fn select_stmt(input: &str) -> IResult<&str, SelectStmt> {
    let (input, _) = tag_no_case("SELECT")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, columns) = select_columns(input)?;
    let (input, _) = multispace1(input)?;
    let (input, _) = tag_no_case("FROM")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, from) = identifier(input)?;
    let (input, where_clause) = opt(preceded(
        tuple((multispace1, tag_no_case("WHERE"), multispace1)),
        expr,
    ))(input)?;
    let (input, order_by) = opt(preceded(
        tuple((multispace1, tag_no_case("ORDER"), multispace1, tag_no_case("BY"), multispace1)),
        order_by_list,
    ))(input)?;
    let (input, limit) = opt(preceded(
        tuple((multispace1, tag_no_case("LIMIT"), multispace1)),
        map(digit1, |s: &str| s.parse::<usize>().unwrap_or(0)),
    ))(input)?;
    let (input, offset) = opt(preceded(
        tuple((multispace1, tag_no_case("OFFSET"), multispace1)),
        map(digit1, |s: &str| s.parse::<usize>().unwrap_or(0)),
    ))(input)?;

    Ok((input, SelectStmt {
        columns,
        from: from.to_string(),
        where_clause,
        order_by: order_by.unwrap_or_default(),
        limit,
        offset,
    }))
}

fn select_columns(input: &str) -> IResult<&str, Vec<Column>> {
    alt((
        map(char('*'), |_| vec![Column::Star]),
        separated_list1(
            tuple((multispace0, char(','), multispace0)),
            column,
        ),
    ))(input)
}

fn column(input: &str) -> IResult<&str, Column> {
    alt((
        map(char('*'), |_| Column::Star),
        map(special_field, Column::Special),
        map(identifier, |s| Column::Field(s.to_string())),
    ))(input)
}

fn special_field(input: &str) -> IResult<&str, SpecialField> {
    preceded(
        char('@'),
        alt((
            value(SpecialField::Id, tag_no_case("id")),
            value(SpecialField::Body, tag_no_case("body")),
            value(SpecialField::Path, tag_no_case("path")),
            value(SpecialField::Modified, tag_no_case("modified")),
            value(SpecialField::Created, tag_no_case("created")),
        )),
    )(input)
}

fn order_by_list(input: &str) -> IResult<&str, Vec<OrderBy>> {
    separated_list1(
        tuple((multispace0, char(','), multispace0)),
        order_by_item,
    )(input)
}

fn order_by_item(input: &str) -> IResult<&str, OrderBy> {
    let (input, col) = identifier(input)?;
    let (input, dir) = opt(preceded(
        multispace1,
        alt((
            value(OrderDirection::Asc, tag_no_case("ASC")),
            value(OrderDirection::Desc, tag_no_case("DESC")),
        )),
    ))(input)?;

    Ok((input, OrderBy {
        column: col.to_string(),
        direction: dir.unwrap_or_default(),
    }))
}

// ============================================================================
// INSERT
// ============================================================================

fn insert_stmt(input: &str) -> IResult<&str, InsertStmt> {
    let (input, _) = tag_no_case("INSERT")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, _) = tag_no_case("INTO")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, into) = identifier(input)?;
    let (input, _) = multispace0(input)?;
    let (input, columns) = delimited(
        char('('),
        separated_list1(tuple((multispace0, char(','), multispace0)), identifier),
        char(')'),
    )(input)?;
    let (input, _) = multispace1(input)?;
    let (input, _) = tag_no_case("VALUES")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, values) = delimited(
        char('('),
        separated_list1(tuple((multispace0, char(','), multispace0)), literal),
        char(')'),
    )(input)?;
    let (input, body) = opt(preceded(
        tuple((multispace1, tag_no_case("BODY"), multispace1)),
        string_literal,
    ))(input)?;

    Ok((input, InsertStmt {
        into: into.to_string(),
        columns: columns.into_iter().map(String::from).collect(),
        values,
        body,
    }))
}

// ============================================================================
// UPDATE
// ============================================================================

fn update_stmt(input: &str) -> IResult<&str, UpdateStmt> {
    let (input, _) = tag_no_case("UPDATE")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, collection) = identifier(input)?;
    let (input, _) = multispace1(input)?;
    let (input, _) = tag_no_case("SET")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, set) = separated_list1(
        tuple((multispace0, char(','), multispace0)),
        set_clause,
    )(input)?;
    let (input, where_clause) = opt(preceded(
        tuple((multispace1, tag_no_case("WHERE"), multispace1)),
        expr,
    ))(input)?;

    Ok((input, UpdateStmt {
        collection: collection.to_string(),
        set,
        where_clause,
    }))
}

fn set_clause(input: &str) -> IResult<&str, SetClause> {
    let (input, col) = identifier(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char('=')(input)?;
    let (input, _) = multispace0(input)?;
    let (input, val) = expr(input)?;

    Ok((input, SetClause {
        column: col.to_string(),
        value: val,
    }))
}

// ============================================================================
// DELETE
// ============================================================================

fn delete_stmt(input: &str) -> IResult<&str, DeleteStmt> {
    let (input, _) = tag_no_case("DELETE")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, _) = tag_no_case("FROM")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, from) = identifier(input)?;
    let (input, where_clause) = opt(preceded(
        tuple((multispace1, tag_no_case("WHERE"), multispace1)),
        expr,
    ))(input)?;

    Ok((input, DeleteStmt {
        from: from.to_string(),
        where_clause,
    }))
}

// ============================================================================
// CREATE COLLECTION
// ============================================================================

fn create_collection_stmt(input: &str) -> IResult<&str, CreateCollectionStmt> {
    let (input, _) = tag_no_case("CREATE")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, if_not_exists) = opt(tuple((
        tag_no_case("IF"),
        multispace1,
        tag_no_case("NOT"),
        multispace1,
        tag_no_case("EXISTS"),
        multispace1,
    )))(input)?;
    let (input, _) = tag_no_case("COLLECTION")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, name) = identifier(input)?;
    let (input, _) = multispace0(input)?;
    let (input, columns) = opt(delimited(
        char('('),
        separated_list0(tuple((multispace0, char(','), multispace0)), column_def),
        char(')'),
    ))(input)?;

    Ok((input, CreateCollectionStmt {
        name: name.to_string(),
        columns: columns.unwrap_or_default(),
        if_not_exists: if_not_exists.is_some(),
    }))
}

fn column_def(input: &str) -> IResult<&str, ColumnDef> {
    let (input, name) = identifier(input)?;
    let (input, _) = multispace1(input)?;
    let (input, data_type) = data_type(input)?;
    let (input, constraints) = many0(preceded(multispace1, constraint))(input)?;

    Ok((input, ColumnDef {
        name: name.to_string(),
        data_type,
        constraints,
    }))
}

fn data_type(input: &str) -> IResult<&str, DataType> {
    alt((
        value(DataType::String, tag_no_case("STRING")),
        value(DataType::Int, tag_no_case("INT")),
        value(DataType::Float, tag_no_case("FLOAT")),
        value(DataType::Bool, tag_no_case("BOOL")),
        value(DataType::Date, tag_no_case("DATE")),
        value(DataType::DateTime, tag_no_case("DATETIME")),
        value(DataType::Object, tag_no_case("OBJECT")),
        map(
            preceded(
                tuple((tag_no_case("ARRAY"), multispace0, char('<'))),
                terminated(data_type, char('>')),
            ),
            |inner| DataType::Array(Box::new(inner)),
        ),
        map(
            preceded(tuple((tag_no_case("REF"), multispace0, char('<'))),
                     terminated(identifier, char('>'))),
            |name| DataType::Ref(name.to_string()),
        ),
    ))(input)
}

fn constraint(input: &str) -> IResult<&str, Constraint> {
    alt((
        value(Constraint::Required, tag_no_case("REQUIRED")),
        value(Constraint::Unique, tag_no_case("UNIQUE")),
        value(Constraint::Indexed, tag_no_case("INDEXED")),
        map(
            preceded(tuple((tag_no_case("DEFAULT"), multispace1)), literal),
            Constraint::Default,
        ),
    ))(input)
}

// ============================================================================
// CREATE VIEW
// ============================================================================

fn create_view_stmt(input: &str) -> IResult<&str, CreateViewStmt> {
    let (input, _) = tag_no_case("CREATE")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, if_not_exists) = opt(tuple((
        tag_no_case("IF"),
        multispace1,
        tag_no_case("NOT"),
        multispace1,
        tag_no_case("EXISTS"),
        multispace1,
    )))(input)?;
    let (input, _) = tag_no_case("VIEW")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, name) = identifier(input)?;
    let (input, _) = multispace1(input)?;
    let (input, _) = tag_no_case("AS")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, query) = select_stmt(input)?;
    let (input, template) = opt(preceded(
        tuple((multispace1, tag_no_case("TEMPLATE"), multispace1)),
        string_literal,
    ))(input)?;

    Ok((input, CreateViewStmt {
        name: name.to_string(),
        query: Box::new(query),
        template,
        if_not_exists: if_not_exists.is_some(),
    }))
}

// ============================================================================
// DROP
// ============================================================================

fn drop_collection_stmt(input: &str) -> IResult<&str, String> {
    let (input, _) = tag_no_case("DROP")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, _) = tag_no_case("COLLECTION")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, name) = identifier(input)?;
    Ok((input, name.to_string()))
}

fn drop_view_stmt(input: &str) -> IResult<&str, String> {
    let (input, _) = tag_no_case("DROP")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, _) = tag_no_case("VIEW")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, name) = identifier(input)?;
    Ok((input, name.to_string()))
}

// ============================================================================
// Expressions
// ============================================================================

fn expr(input: &str) -> IResult<&str, Expr> {
    or_expr(input)
}

fn or_expr(input: &str) -> IResult<&str, Expr> {
    let (input, first) = and_expr(input)?;
    let (input, rest) = many0(preceded(
        tuple((multispace1, tag_no_case("OR"), multispace1)),
        and_expr,
    ))(input)?;

    Ok((input, rest.into_iter().fold(first, |acc, e| Expr::BinaryOp {
        left: Box::new(acc),
        op: BinaryOp::Or,
        right: Box::new(e),
    })))
}

fn and_expr(input: &str) -> IResult<&str, Expr> {
    let (input, first) = not_expr(input)?;
    let (input, rest) = many0(preceded(
        tuple((multispace1, tag_no_case("AND"), multispace1)),
        not_expr,
    ))(input)?;

    Ok((input, rest.into_iter().fold(first, |acc, e| Expr::BinaryOp {
        left: Box::new(acc),
        op: BinaryOp::And,
        right: Box::new(e),
    })))
}

fn not_expr(input: &str) -> IResult<&str, Expr> {
    alt((
        map(
            preceded(tuple((tag_no_case("NOT"), multispace1)), not_expr),
            |e| Expr::UnaryOp { op: UnaryOp::Not, expr: Box::new(e) },
        ),
        comparison_expr,
    ))(input)
}

fn comparison_expr(input: &str) -> IResult<&str, Expr> {
    alt((
        contains_expr,
        has_tag_expr,
        is_null_expr,
        like_expr,
        in_expr,
        between_expr,
        binary_comparison,
    ))(input)
}

fn binary_comparison(input: &str) -> IResult<&str, Expr> {
    let (input, left) = primary_expr(input)?;
    let (input, rest) = opt(tuple((
        multispace0,
        alt((
            value(BinaryOp::Eq, tag("=")),
            value(BinaryOp::Ne, alt((tag("!="), tag("<>")))),
            value(BinaryOp::Le, tag("<=")),
            value(BinaryOp::Lt, tag("<")),
            value(BinaryOp::Ge, tag(">=")),
            value(BinaryOp::Gt, tag(">")),
        )),
        multispace0,
        primary_expr,
    )))(input)?;

    match rest {
        Some((_, op, _, right)) => Ok((input, Expr::BinaryOp {
            left: Box::new(left),
            op,
            right: Box::new(right),
        })),
        None => Ok((input, left)),
    }
}

fn contains_expr(input: &str) -> IResult<&str, Expr> {
    let (input, _) = tag_no_case("CONTAINS")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char('(')(input)?;
    let (input, _) = multispace0(input)?;
    let (input, text) = string_literal(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char(')')(input)?;

    Ok((input, Expr::Contains { text }))
}

fn has_tag_expr(input: &str) -> IResult<&str, Expr> {
    let (input, _) = tag_no_case("HAS")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, _) = tag_no_case("TAG")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, tag_val) = string_literal(input)?;
    let (input, column) = opt(preceded(
        tuple((multispace1, tag_no_case("IN"), multispace1)),
        identifier,
    ))(input)?;

    Ok((input, Expr::HasTag {
        tag: tag_val,
        column: column.map(String::from),
    }))
}

fn is_null_expr(input: &str) -> IResult<&str, Expr> {
    let (input, e) = primary_expr(input)?;
    let (input, _) = multispace1(input)?;
    let (input, _) = tag_no_case("IS")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, negated) = opt(tuple((tag_no_case("NOT"), multispace1)))(input)?;
    let (input, _) = tag_no_case("NULL")(input)?;

    Ok((input, Expr::IsNull {
        expr: Box::new(e),
        negated: negated.is_some(),
    }))
}

fn like_expr(input: &str) -> IResult<&str, Expr> {
    let (input, e) = primary_expr(input)?;
    let (input, _) = multispace1(input)?;
    let (input, negated) = opt(tuple((tag_no_case("NOT"), multispace1)))(input)?;
    let (input, _) = tag_no_case("LIKE")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, pattern) = string_literal(input)?;

    Ok((input, Expr::Like {
        expr: Box::new(e),
        pattern,
        negated: negated.is_some(),
    }))
}

fn in_expr(input: &str) -> IResult<&str, Expr> {
    let (input, e) = primary_expr(input)?;
    let (input, _) = multispace1(input)?;
    let (input, negated) = opt(tuple((tag_no_case("NOT"), multispace1)))(input)?;
    let (input, _) = tag_no_case("IN")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, values) = delimited(
        char('('),
        separated_list1(
            tuple((multispace0, char(','), multispace0)),
            map(literal, Expr::Literal),
        ),
        char(')'),
    )(input)?;

    Ok((input, Expr::In {
        expr: Box::new(e),
        values,
        negated: negated.is_some(),
    }))
}

fn between_expr(input: &str) -> IResult<&str, Expr> {
    let (input, e) = primary_expr(input)?;
    let (input, _) = multispace1(input)?;
    let (input, negated) = opt(tuple((tag_no_case("NOT"), multispace1)))(input)?;
    let (input, _) = tag_no_case("BETWEEN")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, low) = primary_expr(input)?;
    let (input, _) = multispace1(input)?;
    let (input, _) = tag_no_case("AND")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, high) = primary_expr(input)?;

    Ok((input, Expr::Between {
        expr: Box::new(e),
        low: Box::new(low),
        high: Box::new(high),
        negated: negated.is_some(),
    }))
}

fn primary_expr(input: &str) -> IResult<&str, Expr> {
    alt((
        delimited(
            tuple((char('('), multispace0)),
            expr,
            tuple((multispace0, char(')'))),
        ),
        map(literal, Expr::Literal),
        map(special_field, |sf| Expr::Column(Column::Special(sf))),
        map(identifier, |s| Expr::Column(Column::Field(s.to_string()))),
    ))(input)
}

// ============================================================================
// Primitives
// ============================================================================

fn identifier(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '-')(input)
}

fn literal(input: &str) -> IResult<&str, Literal> {
    alt((
        value(Literal::Null, tag_no_case("NULL")),
        value(Literal::Bool(true), tag_no_case("true")),
        value(Literal::Bool(false), tag_no_case("false")),
        map(float_literal, Literal::Float),
        map(integer_literal, Literal::Int),
        map(string_literal, Literal::String),
        map(array_literal, Literal::Array),
    ))(input)
}

fn integer_literal(input: &str) -> IResult<&str, i64> {
    let (input, neg) = opt(char('-'))(input)?;
    let (input, digits) = digit1(input)?;
    let val: i64 = digits.parse().unwrap_or(0);
    Ok((input, if neg.is_some() { -val } else { val }))
}

fn float_literal(input: &str) -> IResult<&str, f64> {
    let (input, neg) = opt(char('-'))(input)?;
    let (input, int_part) = digit1(input)?;
    let (input, _) = char('.')(input)?;
    let (input, frac_part) = digit1(input)?;
    let val: f64 = format!("{}.{}", int_part, frac_part).parse().unwrap_or(0.0);
    Ok((input, if neg.is_some() { -val } else { val }))
}

fn string_literal(input: &str) -> IResult<&str, String> {
    alt((
        delimited(
            char('\''),
            map(
                many0(alt((
                    map(tag("''"), |_| "'".to_string()),
                    map(none_of("'"), |c| c.to_string()),
                ))),
                |v| v.join(""),
            ),
            char('\''),
        ),
        delimited(
            char('"'),
            map(
                many0(alt((
                    map(tag("\\\""), |_| "\"".to_string()),
                    map(tag("\\n"), |_| "\n".to_string()),
                    map(tag("\\t"), |_| "\t".to_string()),
                    map(tag("\\\\"), |_| "\\".to_string()),
                    map(none_of("\"\\"), |c| c.to_string()),
                ))),
                |v| v.join(""),
            ),
            char('"'),
        ),
    ))(input)
}

fn array_literal(input: &str) -> IResult<&str, Vec<Literal>> {
    delimited(
        char('['),
        separated_list0(
            tuple((multispace0, char(','), multispace0)),
            literal,
        ),
        char(']'),
    )(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_select() {
        let stmt = parse_statement("SELECT * FROM todos").unwrap();
        if let Statement::Select(s) = stmt {
            assert_eq!(s.from, "todos");
            assert!(matches!(s.columns[0], Column::Star));
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_parse_select_with_where() {
        let stmt = parse_statement("SELECT title, done FROM todos WHERE done = false").unwrap();
        if let Statement::Select(s) = stmt {
            assert_eq!(s.columns.len(), 2);
            assert!(s.where_clause.is_some());
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_parse_insert() {
        let stmt = parse_statement("INSERT INTO todos (id, title, done) VALUES ('task-1', 'Buy milk', false)").unwrap();
        if let Statement::Insert(i) = stmt {
            assert_eq!(i.into, "todos");
            assert_eq!(i.columns.len(), 3);
            assert_eq!(i.values.len(), 3);
        } else {
            panic!("Expected Insert");
        }
    }

    #[test]
    fn test_parse_create_collection() {
        let stmt = parse_statement("CREATE COLLECTION todos (title STRING REQUIRED, done BOOL DEFAULT false)").unwrap();
        if let Statement::CreateCollection(c) = stmt {
            assert_eq!(c.name, "todos");
            assert_eq!(c.columns.len(), 2);
        } else {
            panic!("Expected CreateCollection");
        }
    }

    #[test]
    fn test_parse_create_view() {
        let stmt = parse_statement("CREATE VIEW active AS SELECT * FROM todos WHERE done = false TEMPLATE 'list.html'").unwrap();
        if let Statement::CreateView(v) = stmt {
            assert_eq!(v.name, "active");
            assert_eq!(v.template, Some("list.html".to_string()));
        } else {
            panic!("Expected CreateView");
        }
    }

    #[test]
    fn test_parse_contains() {
        let stmt = parse_statement("SELECT * FROM notes WHERE CONTAINS('meeting')").unwrap();
        if let Statement::Select(s) = stmt {
            assert!(matches!(s.where_clause, Some(Expr::Contains { .. })));
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_parse_has_tag() {
        let stmt = parse_statement("SELECT * FROM todos WHERE HAS TAG 'urgent'").unwrap();
        if let Statement::Select(s) = stmt {
            assert!(matches!(s.where_clause, Some(Expr::HasTag { .. })));
        } else {
            panic!("Expected Select");
        }
    }
}
