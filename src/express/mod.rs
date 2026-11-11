//! A2UI 1.0 Express in Rust: compiler, response parser, prompt generator and
//! server-side macros.
//!
//! A port of the Express parts of the official Python agent SDK
//! (`a2ui-agent-sdk` 0.11.0, github.com/a2ui-project/a2ui), checked against the
//! upstream conformance suites vendored under `tests/a2ui_conformance/`.
//! Targets protocol v1.0 only.

pub mod catalog;
pub mod compiler;
pub mod macros;
pub mod normalize;
pub mod prompt;
pub mod response;
pub mod syntax;

pub use catalog::{Catalog, basic_catalog_v1_0};
pub use compiler::Compiler;
pub use response::{Part, RawPart, has_format_content, parse_response, unwrap, wrap};

/// Errors use the two categories the conformance suites test for.
#[derive(Debug, Clone, PartialEq)]
pub enum ExpressError {
    /// Payload text the format cannot read.
    Parse(String),
    /// A payload that reads but names something the catalog does not declare.
    Validation(String),
    /// A catalog that cannot be loaded.
    Catalog(String),
}

impl ExpressError {
    pub fn category(&self) -> &'static str {
        match self {
            ExpressError::Parse(_) => "ParseError",
            ExpressError::Validation(_) | ExpressError::Catalog(_) => "ValidationError",
        }
    }
    pub fn message(&self) -> &str {
        match self {
            ExpressError::Parse(m) | ExpressError::Validation(m) | ExpressError::Catalog(m) => m,
        }
    }
}

impl std::fmt::Display for ExpressError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.category(), self.message())
    }
}

impl std::error::Error for ExpressError {}
