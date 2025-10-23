use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub dep_type: DependencyType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyType {
    Runtime,     // Regular dependencies
    Dev,         // Development dependencies
    Build,       // Build dependencies
    Optional,    // Optional dependencies
}

impl std::fmt::Display for DependencyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DependencyType::Runtime => write!(f, "runtime"),
            DependencyType::Dev => write!(f, "dev"),
            DependencyType::Build => write!(f, "build"),
            DependencyType::Optional => write!(f, "optional"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub ecosystem: String,        // "rust", "node", "python"
    pub total_count: usize,
    pub runtime_count: usize,
    pub dev_count: usize,
    pub dependencies: Vec<Dependency>,
}

impl DependencyInfo {
    pub fn new(ecosystem: String, dependencies: Vec<Dependency>) -> Self {
        let runtime_count = dependencies.iter().filter(|d| d.dep_type == DependencyType::Runtime).count();
        let dev_count = dependencies.iter().filter(|d| d.dep_type == DependencyType::Dev).count();
        let total_count = dependencies.len();

        Self {
            ecosystem,
            total_count,
            runtime_count,
            dev_count,
            dependencies,
        }
    }
}
