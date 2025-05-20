use std::env;

use anyhow::Result;

use crate::{bot::Forge, config::required_env, http::GitHubRepoApi};

pub struct GitHub {
    // Defined by the user
    labels: Vec<String>,

    // Internal
    github_repo_api: GitHubRepoApi,
}

impl GitHub {
    pub fn from_env() -> Result<Self> {
        let repository = required_env("GITHUB_REPOSITORY")?;
        let labels = env::var("LON_LABELS").unwrap_or_default();
        let token = required_env("LON_TOKEN")?;

        Ok(Self {
            labels: labels.split(',').map(ToString::to_string).collect(),

            github_repo_api: GitHubRepoApi::builder(&repository).token(&token).build()?,
        })
    }
}

impl Forge for GitHub {
    fn open_pull_request(&self, branch: &str, name: &str, body: Option<String>) -> Result<String> {
        let pull_request_response =
            self.github_repo_api
                .open_pull_request(branch, &format!("lon: update {name}"), body)?;

        self.github_repo_api
            .add_labels_to_issue(pull_request_response.number, &self.labels)?;

        Ok(pull_request_response.html_url)
    }
}
