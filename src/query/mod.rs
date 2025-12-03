//! Query execution engine for MDBY
//!
//! Executes MDQL statements against the database.

mod executor;
pub mod filter;

pub use executor::execute;
