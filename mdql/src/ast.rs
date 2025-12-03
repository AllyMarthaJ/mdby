//! Abstract Syntax Tree for MDQL

use serde::{Deserialize, Serialize};

/// A complete MDQL statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    Select(SelectStmt),
    Insert(InsertStmt),
    Update(UpdateStmt),
    Delete(DeleteStmt),
    CreateCollection(CreateCollectionStmt),
    CreateView(CreateViewStmt),
    DropCollection(String),
    DropView(String),
}

/// SELECT statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectStmt {
    /// Columns to select (empty = *)
    pub columns: Vec<Column>,
    /// Collection to select from
    pub from: String,
    /// Optional WHERE clause
    pub where_clause: Option<Expr>,
    /// ORDER BY clauses
    pub order_by: Vec<OrderBy>,
    /// LIMIT clause
    pub limit: Option<usize>,
    /// OFFSET clause
    pub offset: Option<usize>,
}

/// A column reference
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Column {
    /// All columns (*)
    Star,
    /// Named field
    Field(String),
    /// Special fields (@body, @id, @path)
    Special(SpecialField),
    /// Expression with alias
    Expr { expr: Box<Expr>, alias: Option<String> },
}

/// Special built-in fields
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SpecialField {
    /// @id - document ID
    Id,
    /// @body - markdown body content
    Body,
    /// @path - file path
    Path,
    /// @modified - last modified time
    Modified,
    /// @created - creation time (from git)
    Created,
}

/// ORDER BY clause
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderBy {
    pub column: String,
    pub direction: OrderDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderDirection {
    Asc,
    Desc,
}

impl Default for OrderDirection {
    fn default() -> Self {
        Self::Asc
    }
}

/// INSERT statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InsertStmt {
    /// Target collection
    pub into: String,
    /// Column names
    pub columns: Vec<String>,
    /// Values to insert
    pub values: Vec<Literal>,
    /// Body content (optional)
    pub body: Option<String>,
}

/// UPDATE statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateStmt {
    /// Target collection
    pub collection: String,
    /// SET clauses
    pub set: Vec<SetClause>,
    /// WHERE clause
    pub where_clause: Option<Expr>,
}

/// SET clause in UPDATE
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetClause {
    pub column: String,
    pub value: Expr,
}

/// DELETE statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeleteStmt {
    /// Target collection
    pub from: String,
    /// WHERE clause
    pub where_clause: Option<Expr>,
}

/// CREATE COLLECTION statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateCollectionStmt {
    pub name: String,
    pub columns: Vec<ColumnDef>,
    pub if_not_exists: bool,
}

/// Column definition in CREATE COLLECTION
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: DataType,
    pub constraints: Vec<Constraint>,
}

/// Data types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataType {
    String,
    Int,
    Float,
    Bool,
    Date,
    DateTime,
    Array(Box<DataType>),
    Object,
    Ref(String), // Reference to another collection
}

/// Column constraints
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Constraint {
    Required,
    Unique,
    Default(Literal),
    Indexed,
}

/// CREATE VIEW statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateViewStmt {
    pub name: String,
    pub query: Box<SelectStmt>,
    pub template: Option<String>,
    pub if_not_exists: bool,
}

/// Expression in WHERE clause or elsewhere
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    /// Literal value
    Literal(Literal),
    /// Column reference
    Column(Column),
    /// Binary operation
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    /// Unary operation
    UnaryOp {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    /// Function call
    Function {
        name: String,
        args: Vec<Expr>,
    },
    /// IN expression: column IN (values...)
    In {
        expr: Box<Expr>,
        values: Vec<Expr>,
        negated: bool,
    },
    /// LIKE expression
    Like {
        expr: Box<Expr>,
        pattern: String,
        negated: bool,
    },
    /// CONTAINS (full-text search in body)
    Contains {
        text: String,
    },
    /// HAS TAG expression (array membership)
    HasTag {
        tag: String,
        column: Option<String>,
    },
    /// IS NULL / IS NOT NULL
    IsNull {
        expr: Box<Expr>,
        negated: bool,
    },
    /// BETWEEN expression
    Between {
        expr: Box<Expr>,
        low: Box<Expr>,
        high: Box<Expr>,
        negated: bool,
    },
}

/// Literal values
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<Literal>),
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOp {
    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    // Logical
    And,
    Or,
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    // String
    Concat,
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    Not,
    Neg,
}

impl SelectStmt {
    pub fn new(from: impl Into<String>) -> Self {
        Self {
            columns: vec![Column::Star],
            from: from.into(),
            where_clause: None,
            order_by: vec![],
            limit: None,
            offset: None,
        }
    }
}
