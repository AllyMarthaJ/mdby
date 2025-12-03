//! Error types for MDQL parsing

use std::fmt;

/// Error that occurred during parsing
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub position: Option<usize>,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

impl ParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            position: None,
            line: None,
            column: None,
        }
    }

    pub fn with_position(mut self, pos: usize) -> Self {
        self.position = Some(pos);
        self
    }

    pub fn with_location(mut self, line: usize, column: usize) -> Self {
        self.line = Some(line);
        self.column = Some(column);
        self
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Parse error: {}", self.message)?;
        if let (Some(line), Some(col)) = (self.line, self.column) {
            write!(f, " at line {}, column {}", line, col)?;
        } else if let Some(pos) = self.position {
            write!(f, " at position {}", pos)?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseError {}

impl From<nom::Err<nom::error::Error<&str>>> for ParseError {
    fn from(err: nom::Err<nom::error::Error<&str>>) -> Self {
        match err {
            nom::Err::Incomplete(_) => ParseError::new("Incomplete input"),
            nom::Err::Error(e) | nom::Err::Failure(e) => {
                ParseError::new(format!("Parse error near: {:?}", e.input.chars().take(20).collect::<String>()))
            }
        }
    }
}
