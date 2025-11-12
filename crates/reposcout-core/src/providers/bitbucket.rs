// Bitbucket provider implementation - bridges API client with SearchProvider trait
use async_trait::async_trait;
use reposcout_api::{BitbucketClient, BitbucketRepository};

use crate::{
    models::{Platform, Repository},
    search::SearchProvider,
    Error, Result,
};

/// Wrapper around BitbucketClient that implements SearchProvider
pub struct BitbucketProvider {
    client: BitbucketClient,
}

impl BitbucketProvider {
    pub fn new(username: Option<String>, app_password: Option<String>) -> Self {
        Self {
            client: BitbucketClient::new(username, app_password),
        }
    }
}

#[async_trait]
impl SearchProvider for BitbucketProvider {
    async fn search(&self, query: &str) -> Result<Vec<Repository>> {
        let repos = self
            .client
            .search_repositories(query, 30)
            .await
            .map_err(|e| Error::ApiError(e.to_string()))?;

        Ok(repos.into_iter().map(bitbucket_to_repo).collect())
    }

    async fn get_repository(&self, owner: &str, name: &str) -> Result<Repository> {
        let repo = self
            .client
            .get_repository(owner, name)
            .await
            .map_err(|e| Error::ApiError(e.to_string()))?;

        Ok(bitbucket_to_repo(repo))
    }
}

/// Convert Bitbucket API repository to our internal Repository model
fn bitbucket_to_repo(bb: BitbucketRepository) -> Repository {
    // Bitbucket doesn't have stars/forks/watchers in the same way as GitHub/GitLab
    // We use defaults for these fields
    Repository {
        platform: Platform::Bitbucket,
        full_name: bb.full_name.clone(),
        description: bb.description,
        url: bb.links.html.href,
        homepage_url: None, // Bitbucket API doesn't provide homepage
        stars: 0,           // Bitbucket doesn't have stars
        forks: 0,           // Would need additional API call
        watchers: 0,        // Would need additional API call
        open_issues: 0,     // Bitbucket has issues but count requires additional API call
        language: bb.language,
        topics: Vec::new(), // Bitbucket doesn't have topics/tags in API v2.0
        license: None,      // Would need to parse from repository files
        created_at: bb.created_on,
        updated_at: bb.updated_on,
        pushed_at: bb.updated_on, // Bitbucket doesn't track pushed_at separately
        size: bb.size.unwrap_or(0),
        default_branch: bb
            .mainbranch
            .map(|b| b.name)
            .unwrap_or_else(|| "main".to_string()),
        is_archived: false, // Would need additional API call
        is_private: bb.is_private,
        health: None,
    }
}
