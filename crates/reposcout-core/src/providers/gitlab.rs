// GitLab provider implementation - bridges API client with SearchProvider trait
use async_trait::async_trait;
use reposcout_api::{GitLabClient, GitLabProject};

use crate::{
    models::{Platform, Repository},
    search::SearchProvider,
    Error, Result,
};

/// Wrapper around GitLabClient that implements SearchProvider
pub struct GitLabProvider {
    client: GitLabClient,
}

impl GitLabProvider {
    pub fn new(token: Option<String>) -> Self {
        Self {
            client: GitLabClient::new(token),
        }
    }
}

#[async_trait]
impl SearchProvider for GitLabProvider {
    async fn search(&self, query: &str) -> Result<Vec<Repository>> {
        let projects = self
            .client
            .search_projects(query, 30)
            .await
            .map_err(|e| Error::ApiError(e.to_string()))?;

        Ok(projects.into_iter().map(gitlab_to_repo).collect())
    }

    async fn get_repository(&self, owner: &str, name: &str) -> Result<Repository> {
        // GitLab uses "owner/name" format as the path
        let path = format!("{}/{}", owner, name);
        let project = self
            .client
            .get_project(&path)
            .await
            .map_err(|e| Error::ApiError(e.to_string()))?;

        Ok(gitlab_to_repo(project))
    }
}

/// Convert GitLab API project to our internal Repository model
fn gitlab_to_repo(gl: GitLabProject) -> Repository {
    // GitLab has both topics and tag_list - merge them
    let mut all_topics = gl.topics;
    all_topics.extend(gl.tag_list);
    all_topics.sort();
    all_topics.dedup();

    Repository {
        platform: Platform::GitLab,
        full_name: gl.path_with_namespace,
        description: gl.description,
        url: gl.web_url,
        homepage_url: None, // GitLab API doesn't provide homepage in basic response
        stars: gl.star_count,
        forks: gl.forks_count,
        watchers: 0, // GitLab doesn't have watchers concept
        open_issues: gl.open_issues,
        language: None, // Would need additional API call to get this
        topics: all_topics,
        license: None, // Would need additional API call to get this
        created_at: gl.created_at,
        updated_at: gl.last_activity_at,
        pushed_at: gl.last_activity_at,
        size: 0, // GitLab API doesn't provide size in basic response
        default_branch: gl.default_branch.unwrap_or_else(|| "main".to_string()),
        is_archived: false, // Would need additional API call
        is_private: gl.visibility != "public",
        health: None,
    }
}
