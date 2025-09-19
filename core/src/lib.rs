pub mod model;
pub mod element;
pub mod relation;
pub mod error;
pub mod utils;
pub mod parser;
pub mod html_export;
pub mod export;
pub mod linting;
pub mod html;
pub mod filesystem;
pub mod diagrams;
pub mod index_generator;
pub mod reports;
pub mod git_commands;
pub mod change_impact;
pub mod subsection;
pub mod matrix_generator;
pub mod graph_registry;

// Re-export commonly used modules
pub use crate::model::ModelManager;
pub use crate::element::Element;
pub use crate::relation::Relation;
pub use crate::error::ReqvireError;
pub use crate::graph_registry::GraphRegistry;
