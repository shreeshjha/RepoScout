use crate::{models::Repository, Error, Result};
use serde_json;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Export format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Csv,
    Markdown,
}

impl ExportFormat {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "json" => Some(ExportFormat::Json),
            "csv" => Some(ExportFormat::Csv),
            "md" | "markdown" => Some(ExportFormat::Markdown),
            _ => None,
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Json => "json",
            ExportFormat::Csv => "csv",
            ExportFormat::Markdown => "md",
        }
    }
}

/// Exporter for repository data
pub struct Exporter;

impl Exporter {
    /// Export repositories to a file with automatic format detection
    pub fn export_to_file<P: AsRef<Path>>(repos: &[Repository], path: P) -> Result<()> {
        let path = path.as_ref();

        // Detect format from extension
        let format = path
            .extension()
            .and_then(|e| e.to_str())
            .and_then(ExportFormat::from_extension)
            .ok_or_else(|| {
                Error::ConfigError(
                    "Could not determine export format from extension. Use .json, .csv, or .md"
                        .to_string(),
                )
            })?;

        Self::export_to_file_with_format(repos, path, format)
    }

    /// Export repositories to a file with explicit format
    pub fn export_to_file_with_format<P: AsRef<Path>>(
        repos: &[Repository],
        path: P,
        format: ExportFormat,
    ) -> Result<()> {
        let content = match format {
            ExportFormat::Json => Self::to_json(repos)?,
            ExportFormat::Csv => Self::to_csv(repos)?,
            ExportFormat::Markdown => Self::to_markdown(repos),
        };

        let mut file = File::create(path)
            .map_err(|e| Error::ConfigError(format!("Failed to create file: {}", e)))?;

        file.write_all(content.as_bytes())
            .map_err(|e| Error::ConfigError(format!("Failed to write file: {}", e)))?;

        Ok(())
    }

    /// Export repositories to JSON format
    pub fn to_json(repos: &[Repository]) -> Result<String> {
        serde_json::to_string_pretty(repos)
            .map_err(|e| Error::ConfigError(format!("Failed to serialize JSON: {}", e)))
    }

    /// Export repositories to CSV format
    pub fn to_csv(repos: &[Repository]) -> Result<String> {
        let mut output = String::new();

        // CSV Header
        output.push_str(
            "Platform,Name,Description,Stars,Forks,Watchers,Open Issues,Language,License,\
             Created At,Updated At,Pushed At,Health Score,Health Status,Maintenance Level,URL\n",
        );

        // CSV Rows
        for repo in repos {
            let health_score = repo
                .health
                .as_ref()
                .map(|h| h.score.to_string())
                .unwrap_or_default();
            let health_status = repo.health.as_ref().map(|h| h.status.label()).unwrap_or("");
            let maintenance = repo
                .health
                .as_ref()
                .map(|h| h.maintenance.label())
                .unwrap_or("");

            output.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                repo.platform,
                Self::escape_csv(&repo.full_name),
                Self::escape_csv(repo.description.as_deref().unwrap_or("")),
                repo.stars,
                repo.forks,
                repo.watchers,
                repo.open_issues,
                repo.language.as_deref().unwrap_or(""),
                repo.license.as_deref().unwrap_or(""),
                repo.created_at.format("%Y-%m-%d"),
                repo.updated_at.format("%Y-%m-%d"),
                repo.pushed_at.format("%Y-%m-%d"),
                health_score,
                health_status,
                maintenance,
                repo.url,
            ));
        }

        Ok(output)
    }

    /// Export repositories to Markdown format
    pub fn to_markdown(repos: &[Repository]) -> String {
        let mut output = String::new();

        output.push_str("# Repository Search Results\n\n");
        output.push_str(&format!("Total repositories: {}\n\n", repos.len()));
        output.push_str("---\n\n");

        for repo in repos {
            // Repository header
            output.push_str(&format!("## [{}]({})\n\n", repo.full_name, repo.url));

            // Platform badge
            output.push_str(&format!("**Platform:** {} | ", repo.platform));

            // Health badge if available
            if let Some(health) = &repo.health {
                let health_emoji = match health.status {
                    crate::HealthStatus::Healthy => "🟢",
                    crate::HealthStatus::Moderate => "🟡",
                    crate::HealthStatus::Warning => "🟠",
                    crate::HealthStatus::Critical => "🔴",
                };
                output.push_str(&format!(
                    "**Health:** {} {} ({}/100) | ",
                    health_emoji,
                    health.status.label(),
                    health.score
                ));
                output.push_str(&format!(
                    "**Maintenance:** {} {}\n\n",
                    health.maintenance.emoji(),
                    health.maintenance.label()
                ));
            } else {
                output.push('\n');
            }

            // Description
            if let Some(desc) = &repo.description {
                output.push_str(&format!("{}\n\n", desc));
            }

            // Stats table
            output.push_str("| Metric | Value |\n");
            output.push_str("|--------|-------|\n");
            output.push_str(&format!(
                "| ⭐ Stars | {} |\n",
                Self::format_number(repo.stars)
            ));
            output.push_str(&format!(
                "| 🍴 Forks | {} |\n",
                Self::format_number(repo.forks)
            ));
            output.push_str(&format!(
                "| 👀 Watchers | {} |\n",
                Self::format_number(repo.watchers)
            ));
            output.push_str(&format!(
                "| 🐛 Open Issues | {} |\n",
                Self::format_number(repo.open_issues)
            ));

            if let Some(lang) = &repo.language {
                output.push_str(&format!("| 💻 Language | {} |\n", lang));
            }

            if let Some(license) = &repo.license {
                output.push_str(&format!("| 📜 License | {} |\n", license));
            }

            output.push_str(&format!(
                "| 📅 Created | {} |\n",
                repo.created_at.format("%Y-%m-%d")
            ));
            output.push_str(&format!(
                "| 🔄 Updated | {} |\n",
                repo.updated_at.format("%Y-%m-%d")
            ));
            output.push_str(&format!(
                "| 📌 Pushed | {} |\n",
                repo.pushed_at.format("%Y-%m-%d")
            ));

            // Topics
            if !repo.topics.is_empty() {
                output.push_str("\n**Topics:** ");
                for (i, topic) in repo.topics.iter().enumerate() {
                    if i > 0 {
                        output.push_str(", ");
                    }
                    output.push_str(&format!("`{}`", topic));
                }
                output.push('\n');
            }

            // Health details if available
            if let Some(health) = &repo.health {
                output.push_str("\n### Health Metrics\n\n");
                output.push_str("| Score Component | Value |\n");
                output.push_str("|-----------------|-------|\n");
                output.push_str(&format!(
                    "| Activity | {}/30 |\n",
                    health.metrics.activity_score
                ));
                output.push_str(&format!(
                    "| Community | {}/25 |\n",
                    health.metrics.community_score
                ));
                output.push_str(&format!(
                    "| Responsiveness | {}/20 |\n",
                    health.metrics.responsiveness_score
                ));
                output.push_str(&format!(
                    "| Maturity | {}/15 |\n",
                    health.metrics.maturity_score
                ));
                output.push_str(&format!(
                    "| Documentation | {}/10 |\n",
                    health.metrics.documentation_score
                ));
            }

            output.push_str("\n---\n\n");
        }

        // Summary statistics
        if !repos.is_empty() {
            output.push_str("## Summary Statistics\n\n");

            let total_stars: u32 = repos.iter().map(|r| r.stars).sum();
            let total_forks: u32 = repos.iter().map(|r| r.forks).sum();
            let avg_health: f64 = repos
                .iter()
                .filter_map(|r| r.health.as_ref())
                .map(|h| h.score as f64)
                .sum::<f64>()
                / repos.len() as f64;

            output.push_str(&format!(
                "- Total Stars: {}\n",
                Self::format_number(total_stars)
            ));
            output.push_str(&format!(
                "- Total Forks: {}\n",
                Self::format_number(total_forks)
            ));
            if avg_health > 0.0 {
                output.push_str(&format!("- Average Health Score: {:.1}/100\n", avg_health));
            }

            // Platform distribution
            let mut platform_counts = std::collections::HashMap::new();
            for repo in repos {
                *platform_counts
                    .entry(repo.platform.to_string())
                    .or_insert(0) += 1;
            }

            output.push_str("\n### Platform Distribution\n\n");
            for (platform, count) in platform_counts {
                output.push_str(&format!("- {}: {}\n", platform, count));
            }
        }

        output
    }

    /// Escape CSV special characters
    fn escape_csv(s: &str) -> String {
        if s.contains(',') || s.contains('"') || s.contains('\n') {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    }

    /// Format numbers with K/M suffixes
    fn format_number(num: u32) -> String {
        if num >= 1_000_000 {
            format!("{:.1}M", num as f64 / 1_000_000.0)
        } else if num >= 1_000 {
            format!("{:.1}k", num as f64 / 1_000.0)
        } else {
            num.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_repo() -> Repository {
        Repository {
            platform: crate::models::Platform::GitHub,
            full_name: "test/repo".to_string(),
            description: Some("A test repository".to_string()),
            url: "https://github.com/test/repo".to_string(),
            homepage_url: None,
            stars: 1234,
            forks: 567,
            watchers: 89,
            open_issues: 12,
            language: Some("Rust".to_string()),
            topics: vec!["test".to_string(), "rust".to_string()],
            license: Some("MIT".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            pushed_at: Utc::now(),
            size: 1024,
            default_branch: "main".to_string(),
            is_archived: false,
            is_private: false,
            health: None,
        }
    }

    #[test]
    fn test_export_format_detection() {
        assert_eq!(
            ExportFormat::from_extension("json"),
            Some(ExportFormat::Json)
        );
        assert_eq!(
            ExportFormat::from_extension("JSON"),
            Some(ExportFormat::Json)
        );
        assert_eq!(ExportFormat::from_extension("csv"), Some(ExportFormat::Csv));
        assert_eq!(
            ExportFormat::from_extension("md"),
            Some(ExportFormat::Markdown)
        );
        assert_eq!(
            ExportFormat::from_extension("markdown"),
            Some(ExportFormat::Markdown)
        );
        assert_eq!(ExportFormat::from_extension("txt"), None);
    }

    #[test]
    fn test_json_export() {
        let repos = vec![create_test_repo()];
        let json = Exporter::to_json(&repos).unwrap();
        assert!(json.contains("test/repo"));
        assert!(json.contains("A test repository"));
    }

    #[test]
    fn test_csv_export() {
        let repos = vec![create_test_repo()];
        let csv = Exporter::to_csv(&repos).unwrap();
        assert!(csv.contains("Platform,Name"));
        assert!(csv.contains("test/repo"));
        assert!(csv.contains("1234"));
    }

    #[test]
    fn test_markdown_export() {
        let repos = vec![create_test_repo()];
        let md = Exporter::to_markdown(&repos);
        assert!(md.contains("# Repository Search Results"));
        assert!(md.contains("[test/repo]"));
        assert!(md.contains("A test repository"));
        assert!(md.contains("⭐ Stars"));
    }

    #[test]
    fn test_csv_escaping() {
        assert_eq!(Exporter::escape_csv("simple"), "simple");
        assert_eq!(Exporter::escape_csv("with,comma"), "\"with,comma\"");
        assert_eq!(Exporter::escape_csv("with\"quote"), "\"with\"\"quote\"");
    }
}
