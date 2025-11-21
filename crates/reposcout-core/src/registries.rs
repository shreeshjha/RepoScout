// Package registry API clients for fetching metadata
// Supports crates.io, npmjs.com, PyPI, and more

use crate::packages::{PackageInfo, PackageManager};
use serde::Deserialize;

/// Crates.io API response for crate metadata
#[derive(Debug, Deserialize)]
struct CratesIoResponse {
    #[serde(rename = "crate")]
    crate_data: CrateData,
}

#[derive(Debug, Deserialize)]
struct CrateData {
    #[allow(dead_code)]
    name: String,
    max_version: String,
    downloads: u64,
    description: Option<String>,
    homepage: Option<String>,
}

/// npm registry API response
#[derive(Debug, Deserialize)]
struct NpmResponse {
    #[allow(dead_code)]
    name: String,
    description: Option<String>,
    #[serde(rename = "dist-tags")]
    dist_tags: NpmDistTags,
    homepage: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NpmDistTags {
    latest: String,
}

/// PyPI API response
#[derive(Debug, Deserialize)]
struct PyPIResponse {
    info: PyPIInfo,
}

#[derive(Debug, Deserialize)]
struct PyPIInfo {
    #[allow(dead_code)]
    name: String,
    version: String,
    summary: Option<String>,
    home_page: Option<String>,
    license: Option<String>,
}

/// Registry API client
pub struct RegistryClient {
    client: reqwest::Client,
}

impl RegistryClient {
    /// Create a new registry client
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("RepoScout/0.1.0")
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { client }
    }

    /// Fetch package metadata from appropriate registry
    pub async fn fetch_metadata(&self, package_info: &mut PackageInfo) -> Result<(), String> {
        match package_info.manager {
            PackageManager::Cargo => self.fetch_crates_io(package_info).await,
            PackageManager::Npm => self.fetch_npm(package_info).await,
            PackageManager::PyPI => self.fetch_pypi(package_info).await,
            _ => {
                // Other registries not yet implemented
                Ok(())
            }
        }
    }

    /// Fetch metadata from crates.io
    async fn fetch_crates_io(&self, package_info: &mut PackageInfo) -> Result<(), String> {
        let url = format!("https://crates.io/api/v1/crates/{}", package_info.name);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch from crates.io: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("crates.io returned status: {}", response.status()));
        }

        let data: CratesIoResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse crates.io response: {}", e))?;

        // Update package info with fetched data
        package_info.latest_version = Some(data.crate_data.max_version);
        package_info.downloads = Some(data.crate_data.downloads);
        package_info.description = data.crate_data.description;
        package_info.homepage = data.crate_data.homepage;

        // Update registry URL to actual package page
        package_info.registry_url = format!("https://crates.io/crates/{}", package_info.name);

        Ok(())
    }

    /// Fetch metadata from npm registry
    async fn fetch_npm(&self, package_info: &mut PackageInfo) -> Result<(), String> {
        let url = format!("https://registry.npmjs.org/{}", package_info.name);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch from npm: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("npm returned status: {}", response.status()));
        }

        let data: NpmResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse npm response: {}", e))?;

        // Update package info
        package_info.latest_version = Some(data.dist_tags.latest);
        package_info.description = data.description;
        package_info.homepage = data.homepage;

        // Update registry URL
        package_info.registry_url = format!("https://www.npmjs.com/package/{}", package_info.name);

        // Note: npm API doesn't easily provide download stats in the main endpoint
        // Would need to query https://api.npmjs.org/downloads/point/last-week/{package}
        // for download stats, which we can add later

        Ok(())
    }

    /// Fetch metadata from PyPI
    async fn fetch_pypi(&self, package_info: &mut PackageInfo) -> Result<(), String> {
        let url = format!("https://pypi.org/pypi/{}/json", package_info.name);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch from PyPI: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("PyPI returned status: {}", response.status()));
        }

        let data: PyPIResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse PyPI response: {}", e))?;

        // Update package info
        package_info.latest_version = Some(data.info.version);
        package_info.description = data.info.summary;
        package_info.homepage = data.info.home_page;
        package_info.license = data.info.license;

        // Update registry URL
        package_info.registry_url = format!("https://pypi.org/project/{}/", package_info.name);

        Ok(())
    }
}

impl Default for RegistryClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_crates_io() {
        let client = RegistryClient::new();
        let mut pkg = PackageInfo::new(PackageManager::Cargo, "serde".to_string());

        let result = client.fetch_metadata(&mut pkg).await;
        assert!(result.is_ok());
        assert!(pkg.latest_version.is_some());
        assert!(pkg.downloads.is_some());
    }

    #[tokio::test]
    async fn test_fetch_npm() {
        let client = RegistryClient::new();
        let mut pkg = PackageInfo::new(PackageManager::Npm, "express".to_string());

        let result = client.fetch_metadata(&mut pkg).await;
        assert!(result.is_ok());
        assert!(pkg.latest_version.is_some());
    }

    #[tokio::test]
    async fn test_fetch_pypi() {
        let client = RegistryClient::new();
        let mut pkg = PackageInfo::new(PackageManager::PyPI, "requests".to_string());

        let result = client.fetch_metadata(&mut pkg).await;
        assert!(result.is_ok());
        assert!(pkg.latest_version.is_some());
    }
}
