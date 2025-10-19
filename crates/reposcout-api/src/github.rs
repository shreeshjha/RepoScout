// GitHub API client - stub for now

pub struct GitHubClient {
    client: reqwest::Client,
    token: Option<String>,
}

impl GitHubClient {
    pub fn new(token: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            token,
        }
    }

    // TODO: Implement actual GitHub API calls
    // Will need: search, get_repo, star/unstar, etc.
}
