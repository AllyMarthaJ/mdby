//! Filter/WHERE clause evaluation

use crate::storage::document::{Document, Value};
use mdql::{BinaryOp, Column, Expr, Literal, SpecialField, UnaryOp};

/// Evaluate an expression against a document
pub fn evaluate(expr: &Expr, doc: &Document) -> bool {
    match evaluate_expr(expr, doc) {
        ExprResult::Bool(b) => b,
        ExprResult::Value(Value::Bool(b)) => b,
        _ => false,
    }
}

/// Result of expression evaluation
#[derive(Debug, Clone)]
enum ExprResult {
    Value(Value),
    Bool(bool),
    Null,
}

impl ExprResult {
    fn as_value(&self) -> Option<&Value> {
        match self {
            ExprResult::Value(v) => Some(v),
            _ => None,
        }
    }

    fn is_truthy(&self) -> bool {
        match self {
            ExprResult::Bool(b) => *b,
            ExprResult::Value(Value::Bool(b)) => *b,
            ExprResult::Value(Value::Null) => false,
            ExprResult::Null => false,
            _ => true,
        }
    }
}

fn evaluate_expr(expr: &Expr, doc: &Document) -> ExprResult {
    match expr {
        Expr::Literal(lit) => ExprResult::Value(literal_to_value(lit)),

        Expr::Column(col) => {
            match col {
                Column::Star => ExprResult::Null, // Can't evaluate * in a filter
                Column::Field(name) => {
                    doc.get_field(name)
                        .map(ExprResult::Value)
                        .unwrap_or(ExprResult::Null)
                }
                Column::Special(sf) => match sf {
                    SpecialField::Id => ExprResult::Value(Value::String(doc.id.clone())),
                    SpecialField::Body => ExprResult::Value(Value::String(doc.body.clone())),
                    SpecialField::Path => ExprResult::Value(Value::String(doc.path.display().to_string())),
                    SpecialField::Modified | SpecialField::Created => ExprResult::Null, // TODO
                },
                Column::Expr { expr, .. } => evaluate_expr(expr, doc),
            }
        }

        Expr::BinaryOp { left, op, right } => {
            let left_val = evaluate_expr(left, doc);
            let right_val = evaluate_expr(right, doc);
            evaluate_binary_op(&left_val, *op, &right_val)
        }

        Expr::UnaryOp { op, expr } => {
            let val = evaluate_expr(expr, doc);
            match op {
                UnaryOp::Not => ExprResult::Bool(!val.is_truthy()),
                UnaryOp::Neg => {
                    match val {
                        ExprResult::Value(Value::Int(i)) => ExprResult::Value(Value::Int(-i)),
                        ExprResult::Value(Value::Float(f)) => ExprResult::Value(Value::Float(-f)),
                        _ => ExprResult::Null,
                    }
                }
            }
        }

        Expr::Contains { text } => {
            let contains = doc.body.to_lowercase().contains(&text.to_lowercase());
            ExprResult::Bool(contains)
        }

        Expr::HasTag { tag, column } => {
            let field_name = column.as_deref().unwrap_or("tags");
            let has_tag = doc.fields.get(field_name)
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().any(|v| {
                    v.as_str().map(|s| s == tag).unwrap_or(false)
                }))
                .unwrap_or(false);
            ExprResult::Bool(has_tag)
        }

        Expr::Like { expr, pattern, negated } => {
            let val = evaluate_expr(expr, doc);
            let matches = match val {
                ExprResult::Value(v) => v.matches_pattern(pattern),
                _ => false,
            };
            ExprResult::Bool(if *negated { !matches } else { matches })
        }

        Expr::In { expr, values, negated } => {
            let val = evaluate_expr(expr, doc);
            let in_list = values.iter().any(|v| {
                let v_result = evaluate_expr(v, doc);
                values_equal(&val, &v_result)
            });
            ExprResult::Bool(if *negated { !in_list } else { in_list })
        }

        Expr::IsNull { expr, negated } => {
            let val = evaluate_expr(expr, doc);
            let is_null = matches!(val, ExprResult::Null | ExprResult::Value(Value::Null));
            ExprResult::Bool(if *negated { !is_null } else { is_null })
        }

        Expr::Between { expr, low, high, negated } => {
            let val = evaluate_expr(expr, doc);
            let low_val = evaluate_expr(low, doc);
            let high_val = evaluate_expr(high, doc);

            let in_range = compare_values(&val, &low_val) >= 0 &&
                           compare_values(&val, &high_val) <= 0;
            ExprResult::Bool(if *negated { !in_range } else { in_range })
        }

        Expr::Function { name, args } => {
            // TODO: Implement built-in functions
            ExprResult::Null
        }
    }
}

fn evaluate_binary_op(left: &ExprResult, op: BinaryOp, right: &ExprResult) -> ExprResult {
    match op {
        // Logical operators
        BinaryOp::And => ExprResult::Bool(left.is_truthy() && right.is_truthy()),
        BinaryOp::Or => ExprResult::Bool(left.is_truthy() || right.is_truthy()),

        // Comparison operators
        BinaryOp::Eq => ExprResult::Bool(values_equal(left, right)),
        BinaryOp::Ne => ExprResult::Bool(!values_equal(left, right)),
        BinaryOp::Lt => ExprResult::Bool(compare_values(left, right) < 0),
        BinaryOp::Le => ExprResult::Bool(compare_values(left, right) <= 0),
        BinaryOp::Gt => ExprResult::Bool(compare_values(left, right) > 0),
        BinaryOp::Ge => ExprResult::Bool(compare_values(left, right) >= 0),

        // Arithmetic (return value, not bool)
        BinaryOp::Add => arithmetic_op(left, right, |a, b| a + b, |a, b| a + b),
        BinaryOp::Sub => arithmetic_op(left, right, |a, b| a - b, |a, b| a - b),
        BinaryOp::Mul => arithmetic_op(left, right, |a, b| a * b, |a, b| a * b),
        BinaryOp::Div => arithmetic_op(left, right, |a, b| if b != 0 { a / b } else { 0 }, |a, b| a / b),
        BinaryOp::Mod => arithmetic_op(left, right, |a, b| if b != 0 { a % b } else { 0 }, |a, b| a % b),

        // String concatenation
        BinaryOp::Concat => {
            let left_str = value_to_string(left);
            let right_str = value_to_string(right);
            ExprResult::Value(Value::String(format!("{}{}", left_str, right_str)))
        }
    }
}

fn values_equal(a: &ExprResult, b: &ExprResult) -> bool {
    match (a, b) {
        (ExprResult::Null, ExprResult::Null) => true,
        (ExprResult::Value(Value::Null), ExprResult::Null) => true,
        (ExprResult::Null, ExprResult::Value(Value::Null)) => true,
        (ExprResult::Bool(a), ExprResult::Bool(b)) => a == b,
        (ExprResult::Value(a), ExprResult::Value(b)) => a == b,
        (ExprResult::Bool(a), ExprResult::Value(Value::Bool(b))) => a == b,
        (ExprResult::Value(Value::Bool(a)), ExprResult::Bool(b)) => a == b,
        _ => false,
    }
}

fn compare_values(a: &ExprResult, b: &ExprResult) -> i32 {
    match (a, b) {
        (ExprResult::Value(Value::Int(a)), ExprResult::Value(Value::Int(b))) => {
            a.cmp(b) as i32
        }
        (ExprResult::Value(Value::Float(a)), ExprResult::Value(Value::Float(b))) => {
            a.partial_cmp(b).map(|o| o as i32).unwrap_or(0)
        }
        (ExprResult::Value(Value::String(a)), ExprResult::Value(Value::String(b))) => {
            a.cmp(b) as i32
        }
        // Cross-type comparisons
        (ExprResult::Value(Value::Int(a)), ExprResult::Value(Value::Float(b))) => {
            (*a as f64).partial_cmp(b).map(|o| o as i32).unwrap_or(0)
        }
        (ExprResult::Value(Value::Float(a)), ExprResult::Value(Value::Int(b))) => {
            a.partial_cmp(&(*b as f64)).map(|o| o as i32).unwrap_or(0)
        }
        _ => 0,
    }
}

fn arithmetic_op<F, G>(left: &ExprResult, right: &ExprResult, int_op: F, float_op: G) -> ExprResult
where
    F: Fn(i64, i64) -> i64,
    G: Fn(f64, f64) -> f64,
{
    match (left, right) {
        (ExprResult::Value(Value::Int(a)), ExprResult::Value(Value::Int(b))) => {
            ExprResult::Value(Value::Int(int_op(*a, *b)))
        }
        (ExprResult::Value(Value::Float(a)), ExprResult::Value(Value::Float(b))) => {
            ExprResult::Value(Value::Float(float_op(*a, *b)))
        }
        (ExprResult::Value(Value::Int(a)), ExprResult::Value(Value::Float(b))) => {
            ExprResult::Value(Value::Float(float_op(*a as f64, *b)))
        }
        (ExprResult::Value(Value::Float(a)), ExprResult::Value(Value::Int(b))) => {
            ExprResult::Value(Value::Float(float_op(*a, *b as f64)))
        }
        _ => ExprResult::Null,
    }
}

fn value_to_string(val: &ExprResult) -> String {
    match val {
        ExprResult::Value(Value::String(s)) => s.clone(),
        ExprResult::Value(Value::Int(i)) => i.to_string(),
        ExprResult::Value(Value::Float(f)) => f.to_string(),
        ExprResult::Value(Value::Bool(b)) => b.to_string(),
        ExprResult::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

fn literal_to_value(lit: &Literal) -> Value {
    match lit {
        Literal::Null => Value::Null,
        Literal::Bool(b) => Value::Bool(*b),
        Literal::Int(i) => Value::Int(*i),
        Literal::Float(f) => Value::Float(*f),
        Literal::String(s) => Value::String(s.clone()),
        Literal::Array(arr) => Value::Array(arr.iter().map(literal_to_value).collect()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_doc() -> Document {
        let mut doc = Document::new("test-1");
        doc.set("title", "Test Document");
        doc.set("priority", 5i64);
        doc.set("done", false);
        doc.set("tags", Value::Array(vec![
            Value::String("rust".into()),
            Value::String("database".into()),
        ]));
        doc.body = "This is the body content.".into();
        doc
    }

    #[test]
    fn test_equality() {
        let doc = make_doc();
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Column(Column::Field("title".into()))),
            op: BinaryOp::Eq,
            right: Box::new(Expr::Literal(Literal::String("Test Document".into()))),
        };
        assert!(evaluate(&expr, &doc));
    }

    #[test]
    fn test_comparison() {
        let doc = make_doc();
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Column(Column::Field("priority".into()))),
            op: BinaryOp::Gt,
            right: Box::new(Expr::Literal(Literal::Int(3))),
        };
        assert!(evaluate(&expr, &doc));
    }

    #[test]
    fn test_contains() {
        let doc = make_doc();
        let expr = Expr::Contains { text: "body content".into() };
        assert!(evaluate(&expr, &doc));
    }

    #[test]
    fn test_has_tag() {
        let doc = make_doc();
        let expr = Expr::HasTag { tag: "rust".into(), column: None };
        assert!(evaluate(&expr, &doc));

        let expr2 = Expr::HasTag { tag: "python".into(), column: None };
        assert!(!evaluate(&expr2, &doc));
    }

    #[test]
    fn test_and_or() {
        let doc = make_doc();
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::Column(Column::Field("done".into()))),
                op: BinaryOp::Eq,
                right: Box::new(Expr::Literal(Literal::Bool(false))),
            }),
            op: BinaryOp::And,
            right: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::Column(Column::Field("priority".into()))),
                op: BinaryOp::Gt,
                right: Box::new(Expr::Literal(Literal::Int(3))),
            }),
        };
        assert!(evaluate(&expr, &doc));
    }
}
