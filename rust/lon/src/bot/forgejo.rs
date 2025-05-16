use std::env;

use anyhow::{Context, Result, bail};
use reqwest::{
    blocking::Client,
    header::{self, HeaderValue},
};
use serde::{Deserialize, Serialize};

use crate::{bot::Forge, config::required_env};

pub struct Forgejo {
    // Defined by CI
    api_url: String,
    repository: String,

    // Defined by the user
    labels: Vec<String>,

    // Internal
    client: Client,
}

#[derive(Deserialize)]
struct Repository {
    default_branch: String,
}

#[derive(Serialize)]
struct PullRequest {
    head: String,
    base: String,
    title: String,
}

#[derive(Deserialize)]
struct PullRequestResponse {
    html_url: String,
    number: i64,
}

#[derive(Serialize)]
struct Labels {
    labels: Vec<String>,
}

impl Forgejo {
    pub fn from_env() -> Result<Self> {
        let labels = env::var("LON_LABELS").unwrap_or_default();
        let token = required_env("LON_TOKEN")?;

        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            format!("token {token}")
                .parse()
                .context("Failed to parse token as header value")?,
        );
        headers.insert(header::ACCEPT, HeaderValue::from_static("application/json"));

        Ok(Self {
            api_url: required_env("GITHUB_API_URL")?,
            repository: required_env("GITHUB_REPOSITORY")?,

            labels: labels.split(',').map(ToString::to_string).collect(),

            client: Client::builder()
                .user_agent("LonBot")
                .default_headers(headers)
                .build()
                .context("Failed to build the HTTP client")?,
        })
    }

    fn repo_api_url(&self) -> String {
        format!("{}/repos/{}", self.api_url, self.repository)
    }

    fn get_repository(&self) -> Result<Repository> {
        let url = self.repo_api_url();

        let res = self
            .client
            .get(&url)
            .send()
            .with_context(|| format!("Failed to send GET request to {url}"))?;

        let status = res.status();
        if !status.is_success() {
            bail!(
                "Failed to get repository information from {url}: {status}:\n{}",
                res.text()?
            )
        }

        let repository = res.json::<Repository>()?;

        Ok(repository)
    }

    fn add_labels(&self, index: i64) -> Result<()> {
        let labels = Labels {
            labels: self.labels.clone(),
        };

        let url = format!("{}/issues/{index}/labels", self.repo_api_url());

        let res = self
            .client
            .post(&url)
            .json(&labels)
            .send()
            .with_context(|| format!("Failed to send GET request to {url}"))?;

        let status = res.status();
        if !status.is_success() {
            bail!("Failed to add labels to {url}: {status}:\n{}", res.text()?)
        }

        Ok(())
    }
}

impl Forge for Forgejo {
    fn open_pull_request(&self, branch: &str, name: &str) -> Result<String> {
        let repository = self.get_repository()?;

        let pull_request = PullRequest {
            head: branch.into(),
            base: repository.default_branch.clone(),
            title: format!("lon: update {name}"),
        };

        let url = format!("{}/pulls", self.repo_api_url());

        let res = self
            .client
            .post(&url)
            .json(&pull_request)
            .send()
            .with_context(|| format!("Failed to send POST request to {url}"))?;

        let status = res.status();
        if !status.is_success() {
            bail!(
                "Failed to open Pull Request at {url}: {status}:\n{}",
                res.text()?
            )
        }

        let pull_request_response = res.json::<PullRequestResponse>()?;

        self.add_labels(pull_request_response.number)?;

        Ok(pull_request_response.html_url)
    }
}
