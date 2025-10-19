// GitHub provider implementation - bridges API client with SearchProvider trait
use async_trait::async_trait;
use reposcout_api::{GitHubClient, GitHubRepo};

use crate::{
    models::{Platform, Repository},
    search::SearchProvider,
    Error, Result,
};

/// Wrapper around GitHubClient that implements SearchProvider
pub struct GitHubProvider {
    client: GitHubClient,
}

impl GitHubProvider {
    pub fn new(token: Option<String>) -> Self {
        Self {
            client: GitHubClient::new(token),
        }
    }
}

#[async_trait]
impl SearchProvider for GitHubProvider {
    async fn search(&self, query: &str) -> Result<Vec<Repository>> {
        let repos = self
            .client
            .search_repositories(query, 30)
            .await
            .map_err(|e| Error::ApiError(e.to_string()))?;

        Ok(repos.into_iter().map(github_to_repo).collect())
    }

    async fn get_repository(&self, owner: &str, name: &str) -> Result<Repository> {
        let repo = self
            .client
            .get_repository(owner, name)
            .await
            .map_err(|e| Error::ApiError(e.to_string()))?;

        Ok(github_to_repo(repo))
    }
}

/// Convert GitHub API repo to our internal Repository model
fn github_to_repo(gh: GitHubRepo) -> Repository {
    Repository {
        platform: Platform::GitHub,
        full_name: gh.full_name,
        description: gh.description,
        url: gh.html_url,
        homepage_url: gh.homepage,
        stars: gh.stargazers_count,
        forks: gh.forks_count,
        watchers: gh.watchers_count,
        open_issues: gh.open_issues_count,
        language: gh.language,
        topics: gh.topics,
        license: gh.license.map(|l| l.name),
        created_at: gh.created_at,
        updated_at: gh.updated_at,
        pushed_at: gh.pushed_at,
        size: gh.size,
        default_branch: gh.default_branch,
        is_archived: gh.archived,
        is_private: gh.private,
    }
}
