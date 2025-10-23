use crate::models::{Dependency, DependencyInfo, DependencyType};
use anyhow::Result;

/// Parse Cargo.toml for Rust dependencies
pub fn parse_cargo_toml(content: &str) -> Result<DependencyInfo> {
    let cargo: toml::Value = toml::from_str(content)?;
    let mut dependencies = Vec::new();

    // Parse [dependencies]
    if let Some(deps) = cargo.get("dependencies").and_then(|v| v.as_table()) {
        for (name, value) in deps {
            let version = extract_version(value);
            dependencies.push(Dependency {
                name: name.clone(),
                version,
                dep_type: DependencyType::Runtime,
            });
        }
    }

    // Parse [dev-dependencies]
    if let Some(deps) = cargo.get("dev-dependencies").and_then(|v| v.as_table()) {
        for (name, value) in deps {
            let version = extract_version(value);
            dependencies.push(Dependency {
                name: name.clone(),
                version,
                dep_type: DependencyType::Dev,
            });
        }
    }

    // Parse [build-dependencies]
    if let Some(deps) = cargo.get("build-dependencies").and_then(|v| v.as_table()) {
        for (name, value) in deps {
            let version = extract_version(value);
            dependencies.push(Dependency {
                name: name.clone(),
                version,
                dep_type: DependencyType::Build,
            });
        }
    }

    Ok(DependencyInfo::new("Rust".to_string(), dependencies))
}

/// Parse package.json for Node.js dependencies
pub fn parse_package_json(content: &str) -> Result<DependencyInfo> {
    let package: serde_json::Value = serde_json::from_str(content)?;
    let mut dependencies = Vec::new();

    // Parse dependencies
    if let Some(deps) = package.get("dependencies").and_then(|v| v.as_object()) {
        for (name, value) in deps {
            let version = value.as_str().unwrap_or("*").to_string();
            dependencies.push(Dependency {
                name: name.clone(),
                version,
                dep_type: DependencyType::Runtime,
            });
        }
    }

    // Parse devDependencies
    if let Some(deps) = package.get("devDependencies").and_then(|v| v.as_object()) {
        for (name, value) in deps {
            let version = value.as_str().unwrap_or("*").to_string();
            dependencies.push(Dependency {
                name: name.clone(),
                version,
                dep_type: DependencyType::Dev,
            });
        }
    }

    Ok(DependencyInfo::new("Node.js".to_string(), dependencies))
}

/// Parse requirements.txt for Python dependencies
pub fn parse_requirements_txt(content: &str) -> Result<DependencyInfo> {
    let mut dependencies = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse package==version or package>=version format
        let (name, version) = if let Some(idx) = line.find("==") {
            (line[..idx].trim().to_string(), line[idx+2..].trim().to_string())
        } else if let Some(idx) = line.find(">=") {
            (line[..idx].trim().to_string(), format!(">={}", line[idx+2..].trim()))
        } else if let Some(idx) = line.find("~=") {
            (line[..idx].trim().to_string(), format!("~={}", line[idx+2..].trim()))
        } else {
            (line.to_string(), "*".to_string())
        };

        dependencies.push(Dependency {
            name,
            version,
            dep_type: DependencyType::Runtime,
        });
    }

    Ok(DependencyInfo::new("Python".to_string(), dependencies))
}

/// Extract version from TOML value (can be string or table)
fn extract_version(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => s.clone(),
        toml::Value::Table(t) => {
            t.get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("*")
                .to_string()
        }
        _ => "*".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cargo_toml() {
        let content = r#"
[dependencies]
serde = "1.0"
tokio = { version = "1.0", features = ["full"] }

[dev-dependencies]
mockall = "0.13"
        "#;

        let info = parse_cargo_toml(content).unwrap();
        assert_eq!(info.ecosystem, "Rust");
        assert_eq!(info.total_count, 3);
        assert_eq!(info.runtime_count, 2);
        assert_eq!(info.dev_count, 1);
    }

    #[test]
    fn test_parse_package_json() {
        let content = r#"
{
  "dependencies": {
    "react": "^18.0.0",
    "express": "4.18.0"
  },
  "devDependencies": {
    "typescript": "^5.0.0"
  }
}
        "#;

        let info = parse_package_json(content).unwrap();
        assert_eq!(info.ecosystem, "Node.js");
        assert_eq!(info.total_count, 3);
        assert_eq!(info.runtime_count, 2);
        assert_eq!(info.dev_count, 1);
    }

    #[test]
    fn test_parse_requirements_txt() {
        let content = r#"
# Python dependencies
requests==2.28.0
flask>=2.0.0
pandas~=1.5.0
numpy
        "#;

        let info = parse_requirements_txt(content).unwrap();
        assert_eq!(info.ecosystem, "Python");
        assert_eq!(info.total_count, 4);
    }
}
