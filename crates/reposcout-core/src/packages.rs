// Package manager integration for RepoScout
// Detects and provides metadata for packages across different ecosystems

use crate::models::Repository;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported package managers and ecosystems
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageManager {
    Cargo,      // Rust (crates.io)
    Npm,        // JavaScript/TypeScript (npmjs.com)
    PyPI,       // Python (pypi.org)
    Go,         // Go (pkg.go.dev)
    Maven,      // Java (maven.org)
    Gradle,     // Java/Kotlin (gradle.org)
    RubyGems,   // Ruby (rubygems.org)
    Composer,   // PHP (packagist.org)
    NuGet,      // .NET (nuget.org)
    Pub,        // Dart/Flutter (pub.dev)
    CocoaPods,  // iOS/macOS (cocoapods.org)
    Swift,      // Swift (swift.org)
    Hex,        // Elixir (hex.pm)
}

impl fmt::Display for PackageManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageManager::Cargo => write!(f, "Cargo"),
            PackageManager::Npm => write!(f, "npm"),
            PackageManager::PyPI => write!(f, "PyPI"),
            PackageManager::Go => write!(f, "Go"),
            PackageManager::Maven => write!(f, "Maven"),
            PackageManager::Gradle => write!(f, "Gradle"),
            PackageManager::RubyGems => write!(f, "RubyGems"),
            PackageManager::Composer => write!(f, "Composer"),
            PackageManager::NuGet => write!(f, "NuGet"),
            PackageManager::Pub => write!(f, "Pub"),
            PackageManager::CocoaPods => write!(f, "CocoaPods"),
            PackageManager::Swift => write!(f, "Swift PM"),
            PackageManager::Hex => write!(f, "Hex"),
        }
    }
}

impl PackageManager {
    /// Get the registry URL for this package manager
    pub fn registry_url(&self) -> &'static str {
        match self {
            PackageManager::Cargo => "https://crates.io",
            PackageManager::Npm => "https://www.npmjs.com",
            PackageManager::PyPI => "https://pypi.org",
            PackageManager::Go => "https://pkg.go.dev",
            PackageManager::Maven => "https://mvnrepository.com",
            PackageManager::Gradle => "https://plugins.gradle.org",
            PackageManager::RubyGems => "https://rubygems.org",
            PackageManager::Composer => "https://packagist.org",
            PackageManager::NuGet => "https://www.nuget.org",
            PackageManager::Pub => "https://pub.dev",
            PackageManager::CocoaPods => "https://cocoapods.org",
            PackageManager::Swift => "https://swiftpackageindex.com",
            PackageManager::Hex => "https://hex.pm",
        }
    }

    /// Get the file that indicates this package manager is used
    pub fn indicator_file(&self) -> &'static str {
        match self {
            PackageManager::Cargo => "Cargo.toml",
            PackageManager::Npm => "package.json",
            PackageManager::PyPI => "setup.py",
            PackageManager::Go => "go.mod",
            PackageManager::Maven => "pom.xml",
            PackageManager::Gradle => "build.gradle",
            PackageManager::RubyGems => "*.gemspec",
            PackageManager::Composer => "composer.json",
            PackageManager::NuGet => "*.csproj",
            PackageManager::Pub => "pubspec.yaml",
            PackageManager::CocoaPods => "*.podspec",
            PackageManager::Swift => "Package.swift",
            PackageManager::Hex => "mix.exs",
        }
    }

    /// Get install command template for this package manager
    pub fn install_command(&self, package_name: &str) -> String {
        match self {
            PackageManager::Cargo => format!("cargo add {}", package_name),
            PackageManager::Npm => format!("npm install {}", package_name),
            PackageManager::PyPI => format!("pip install {}", package_name),
            PackageManager::Go => format!("go get {}", package_name),
            PackageManager::Maven => format!("<dependency>\n  <groupId>...</groupId>\n  <artifactId>{}</artifactId>\n</dependency>", package_name),
            PackageManager::Gradle => format!("implementation '{}'", package_name),
            PackageManager::RubyGems => format!("gem install {}", package_name),
            PackageManager::Composer => format!("composer require {}", package_name),
            PackageManager::NuGet => format!("dotnet add package {}", package_name),
            PackageManager::Pub => format!("flutter pub add {}", package_name),
            PackageManager::CocoaPods => format!("pod '{}'", package_name),
            PackageManager::Swift => ".package(url: \"...\", from: \"...\")".to_string(),
            PackageManager::Hex => format!("{{:{}, \"~> x.x\"}}", package_name),
        }
    }

    /// Get alternative install command (e.g., yarn for npm)
    pub fn alt_install_command(&self, package_name: &str) -> Option<String> {
        match self {
            PackageManager::Npm => Some(format!("yarn add {}", package_name)),
            PackageManager::PyPI => Some(format!("poetry add {}", package_name)),
            PackageManager::RubyGems => Some(format!("bundle add {}", package_name)),
            _ => None,
        }
    }
}

/// Package information from registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub manager: PackageManager,
    pub name: String,
    pub latest_version: Option<String>,
    pub description: Option<String>,
    pub downloads: Option<u64>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub registry_url: String,
    pub install_command: String,
    pub alt_install_command: Option<String>,
}

impl PackageInfo {
    /// Create a new PackageInfo
    pub fn new(manager: PackageManager, name: String) -> Self {
        let registry_url = format!("{}/packages/{}", manager.registry_url(), name);
        let install_command = manager.install_command(&name);
        let alt_install_command = manager.alt_install_command(&name);

        Self {
            manager,
            name,
            latest_version: None,
            description: None,
            downloads: None,
            license: None,
            homepage: None,
            registry_url,
            install_command,
            alt_install_command,
        }
    }
}

/// Detect package managers from repository
pub struct PackageDetector;

impl PackageDetector {
    /// Detect package manager from repository files
    /// Uses repository topics, language, and description to infer
    pub fn detect(repo: &Repository) -> Vec<PackageManager> {
        let mut managers = Vec::new();

        // Check language first
        if let Some(lang) = &repo.language {
            match lang.to_lowercase().as_str() {
                "rust" => managers.push(PackageManager::Cargo),
                "javascript" | "typescript" => managers.push(PackageManager::Npm),
                "python" => managers.push(PackageManager::PyPI),
                "go" => managers.push(PackageManager::Go),
                "java" | "kotlin" => {
                    managers.push(PackageManager::Maven);
                    managers.push(PackageManager::Gradle);
                }
                "ruby" => managers.push(PackageManager::RubyGems),
                "php" => managers.push(PackageManager::Composer),
                "c#" | "f#" => managers.push(PackageManager::NuGet),
                "dart" => managers.push(PackageManager::Pub),
                "swift" | "objective-c" => managers.push(PackageManager::Swift),
                "elixir" => managers.push(PackageManager::Hex),
                _ => {}
            }
        }

        // Check topics for package-related keywords
        for topic in &repo.topics {
            let topic_lower = topic.to_lowercase();
            if (topic_lower.contains("cargo") || topic_lower.contains("crate"))
                && !managers.contains(&PackageManager::Cargo)
            {
                managers.push(PackageManager::Cargo);
            }
            if (topic_lower.contains("npm") || topic_lower.contains("node"))
                && !managers.contains(&PackageManager::Npm)
            {
                managers.push(PackageManager::Npm);
            }
            if (topic_lower.contains("pypi") || topic_lower.contains("pip"))
                && !managers.contains(&PackageManager::PyPI)
            {
                managers.push(PackageManager::PyPI);
            }
        }

        managers
    }

    /// Extract package name from repository
    /// This is a heuristic - we try to get the canonical package name
    pub fn extract_package_name(repo: &Repository, manager: PackageManager) -> Option<String> {
        // Extract repo name from full_name (owner/repo → repo)
        let repo_name = repo.full_name
            .split('/')
            .next_back()
            .unwrap_or(&repo.full_name)
            .to_string();

        match manager {
            PackageManager::Cargo => {
                // For Rust, typically repo name matches crate name
                // But we should fetch from Cargo.toml if available
                Some(repo_name)
            }
            PackageManager::Npm => {
                // For npm, package name is in package.json
                // Use repo name as fallback
                Some(repo_name)
            }
            PackageManager::PyPI => {
                // For Python, check if repo name looks like a package
                // Common pattern: repo-name → package_name
                Some(repo_name.replace('-', "_"))
            }
            PackageManager::Go => {
                // For Go, use the full import path
                Some(repo.full_name.clone())
            }
            _ => {
                // Default: use repository name
                Some(repo_name)
            }
        }
    }
}

/// License compatibility checker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LicenseCompatibility {
    Compatible,   // License is compatible
    Warning,      // Might have restrictions
    Incompatible, // Definitely incompatible
    Unknown,      // Can't determine
}

/// Common open source licenses
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum License {
    MIT,
    Apache2,
    GPL2,
    GPL3,
    LGPL,
    BSD2,
    BSD3,
    MPL2,
    AGPL,
    Unlicense,
    ISC,
    Proprietary,
    Unknown,
}

impl License {
    /// Parse license from string
    pub fn parse_license(s: &str) -> Self {
        let s_lower = s.to_lowercase();
        if s_lower.contains("mit") {
            License::MIT
        } else if s_lower.contains("apache") {
            License::Apache2
        } else if s_lower.contains("gpl") && s_lower.contains('3') {
            License::GPL3
        } else if s_lower.contains("gpl") && s_lower.contains('2') {
            License::GPL2
        } else if s_lower.contains("lgpl") {
            License::LGPL
        } else if s_lower.contains("bsd") && s_lower.contains('3') {
            License::BSD3
        } else if s_lower.contains("bsd") && s_lower.contains('2') {
            License::BSD2
        } else if s_lower.contains("mpl") {
            License::MPL2
        } else if s_lower.contains("agpl") {
            License::AGPL
        } else if s_lower.contains("unlicense") {
            License::Unlicense
        } else if s_lower.contains("isc") {
            License::ISC
        } else if s_lower.contains("proprietary") {
            License::Proprietary
        } else {
            License::Unknown
        }
    }

    /// Check compatibility with another license
    pub fn check_compatibility(&self, other: &License) -> LicenseCompatibility {
        use License::*;
        use LicenseCompatibility::*;

        match (self, other) {
            // Unknown licenses
            (License::Unknown, _) | (_, License::Unknown) => LicenseCompatibility::Unknown,

            // MIT is compatible with almost everything
            (MIT, _) | (_, MIT) => Compatible,
            (BSD2, _) | (_, BSD2) => Compatible,
            (BSD3, _) | (_, BSD3) => Compatible,
            (Apache2, _) | (_, Apache2) => Compatible,
            (ISC, _) | (_, ISC) => Compatible,
            (Unlicense, _) | (_, Unlicense) => Compatible,

            // GPL is not compatible with proprietary
            (GPL2 | GPL3 | AGPL, Proprietary) | (Proprietary, GPL2 | GPL3 | AGPL) => {
                Incompatible
            }

            // LGPL has some restrictions
            (LGPL, _) | (_, LGPL) => Warning,

            // GPL variants have compatibility issues
            (GPL2, GPL3) | (GPL3, GPL2) => Warning,
            (AGPL, _) | (_, AGPL) => Warning,

            // Same licenses are compatible by default
            (MPL2, _) | (_, MPL2) => Compatible,

            // Same license is always compatible with itself
            (GPL2, GPL2) | (GPL3, GPL3) | (Proprietary, Proprietary) => Compatible,
        }
    }

    /// Get a human-readable compatibility message
    pub fn compatibility_message(&self, other: &License) -> String {
        match self.check_compatibility(other) {
            LicenseCompatibility::Compatible => {
                "✓ Licenses are compatible".to_string()
            }
            LicenseCompatibility::Warning => {
                format!("⚠ {} and {} may have compatibility issues - review license terms", self, other)
            }
            LicenseCompatibility::Incompatible => {
                format!("✗ {} and {} are incompatible - cannot be used together", self, other)
            }
            LicenseCompatibility::Unknown => {
                "? License compatibility unknown - manual review required".to_string()
            }
        }
    }
}

impl fmt::Display for License {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            License::MIT => write!(f, "MIT"),
            License::Apache2 => write!(f, "Apache-2.0"),
            License::GPL2 => write!(f, "GPL-2.0"),
            License::GPL3 => write!(f, "GPL-3.0"),
            License::LGPL => write!(f, "LGPL"),
            License::BSD2 => write!(f, "BSD-2-Clause"),
            License::BSD3 => write!(f, "BSD-3-Clause"),
            License::MPL2 => write!(f, "MPL-2.0"),
            License::AGPL => write!(f, "AGPL"),
            License::Unlicense => write!(f, "Unlicense"),
            License::ISC => write!(f, "ISC"),
            License::Proprietary => write!(f, "Proprietary"),
            License::Unknown => write!(f, "Unknown"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_manager_display() {
        assert_eq!(PackageManager::Cargo.to_string(), "Cargo");
        assert_eq!(PackageManager::Npm.to_string(), "npm");
        assert_eq!(PackageManager::PyPI.to_string(), "PyPI");
    }

    #[test]
    fn test_install_command_generation() {
        assert_eq!(
            PackageManager::Cargo.install_command("serde"),
            "cargo add serde"
        );
        assert_eq!(
            PackageManager::Npm.install_command("express"),
            "npm install express"
        );
        assert_eq!(
            PackageManager::PyPI.install_command("requests"),
            "pip install requests"
        );
    }

    #[test]
    fn test_license_parsing() {
        assert_eq!(License::parse_license("MIT License"), License::MIT);
        assert_eq!(License::parse_license("Apache-2.0"), License::Apache2);
        assert_eq!(License::parse_license("GPL-3.0"), License::GPL3);
    }

    #[test]
    fn test_license_compatibility() {
        assert_eq!(
            License::MIT.check_compatibility(&License::Apache2),
            LicenseCompatibility::Compatible
        );
        assert_eq!(
            License::GPL3.check_compatibility(&License::Proprietary),
            LicenseCompatibility::Incompatible
        );
        assert_eq!(
            License::LGPL.check_compatibility(&License::MIT),
            LicenseCompatibility::Warning
        );
    }
}
