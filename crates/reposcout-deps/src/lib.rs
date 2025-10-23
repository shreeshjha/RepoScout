// Dependency analysis module
// Parses and analyzes dependencies from various package managers

pub mod models;
pub mod parsers;

pub use models::{Dependency, DependencyInfo, DependencyType};
pub use parsers::{parse_cargo_toml, parse_package_json, parse_requirements_txt};
