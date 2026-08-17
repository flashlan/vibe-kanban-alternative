mod detection;
pub mod gitea;
pub mod github;
mod types;

use std::path::Path;

use async_trait::async_trait;
use detection::{detect_provider_from_url, is_gitea_remote};
pub use gitea::GiteaProvider;
pub use types::{
    CreatePrRequest, GitHostError, PrComment, PrCommentAuthor, PrReviewComment, ProviderKind,
    PullRequestDetail, ReviewCommentUser, UnifiedPrComment,
};

use self::github::GitHubProvider;

#[async_trait]
pub trait GitHostProvider: Send + Sync {
    async fn create_pr(
        &self,
        repo_path: &Path,
        remote_url: &str,
        request: &CreatePrRequest,
    ) -> Result<PullRequestDetail, GitHostError>;

    async fn get_pr_status(&self, pr_url: &str) -> Result<PullRequestDetail, GitHostError>;

    async fn list_prs_for_branch(
        &self,
        repo_path: &Path,
        remote_url: &str,
        branch_name: &str,
    ) -> Result<Vec<PullRequestDetail>, GitHostError>;

    async fn get_pr_comments(
        &self,
        repo_path: &Path,
        remote_url: &str,
        pr_number: i64,
    ) -> Result<Vec<UnifiedPrComment>, GitHostError>;

    async fn list_open_prs(
        &self,
        repo_path: &Path,
        remote_url: &str,
    ) -> Result<Vec<PullRequestDetail>, GitHostError>;

    fn provider_kind(&self) -> ProviderKind;
}

/// Dispatch over the supported PR hosts: GitHub (via `gh` CLI) and configured
/// Gitea/Forgejo instances (via the REST API).
pub enum GitHostService {
    GitHub(GitHubProvider),
    Gitea(GiteaProvider),
}

impl GitHostService {
    /// Build a service for a git remote. GitHub remotes always route to the
    /// GitHub provider; Gitea remotes route to the Gitea provider only when the
    /// remote host matches the configured `gitea_base_url` and a token exists.
    ///
    /// `gitea_base_url` comes from app config; `gitea_token` comes from the
    /// secure token store. A `None` Gitea base URL disables Gitea.
    pub fn from_url(
        url: &str,
        gitea_base_url: Option<&str>,
        gitea_token: Option<&str>,
    ) -> Result<Self, GitHostError> {
        let gitea = match (gitea_base_url, gitea_token) {
            (Some(base), Some(token)) if !base.trim().is_empty() && !token.trim().is_empty() => {
                Some(GiteaProvider::new(base.trim(), token.trim()))
            }
            _ => None,
        };

        if detect_provider_from_url(url) == ProviderKind::GitHub {
            return Ok(Self::GitHub(GitHubProvider::new()?));
        }

        if gitea_base_url.is_some() && is_gitea_remote(gitea_base_url, url) {
            if let Some(provider) = gitea {
                return Ok(Self::Gitea(provider));
            }
        }

        Err(GitHostError::UnsupportedProvider)
    }

    pub fn github() -> Result<Self, GitHostError> {
        Ok(Self::GitHub(GitHubProvider::new()?))
    }

    pub fn gitea(base_url: &str, token: &str) -> Self {
        Self::Gitea(GiteaProvider::new(base_url, token))
    }

    /// Resolve the Gitea `base_url` (from non-secret app config) and token
    /// (from the secure `gitea.toml` store or `GITEA_TOKEN` env) for
    /// [`GitHostService::from_url`]. Returns `(base_url, token)` when a Gitea
    /// instance is configured; `(None, None)` otherwise.
    ///
    /// `config_base_url` is `config.gitea.base_url` from the app Settings.
    pub fn resolve_gitea_credentials(
        config_base_url: Option<&str>,
    ) -> (Option<String>, Option<String>) {
        let token = utils::gitea_config::load()
            .and_then(|cfg| utils::gitea_config::resolve_token(&cfg).map(|(t, _)| t));
        let base_url = config_base_url
            .map(|b| b.trim().to_string())
            .filter(|b| !b.is_empty());
        match (base_url, token) {
            (Some(base), Some(tok)) => (Some(base), Some(tok)),
            _ => (None, None),
        }
    }
}

#[async_trait]
impl GitHostProvider for GitHostService {
    async fn create_pr(
        &self,
        repo_path: &Path,
        remote_url: &str,
        request: &CreatePrRequest,
    ) -> Result<PullRequestDetail, GitHostError> {
        match self {
            Self::GitHub(p) => p.create_pr(repo_path, remote_url, request).await,
            Self::Gitea(p) => p.create_pr(repo_path, remote_url, request).await,
        }
    }

    async fn get_pr_status(&self, pr_url: &str) -> Result<PullRequestDetail, GitHostError> {
        match self {
            Self::GitHub(p) => p.get_pr_status(pr_url).await,
            Self::Gitea(p) => p.get_pr_status(pr_url).await,
        }
    }

    async fn list_prs_for_branch(
        &self,
        repo_path: &Path,
        remote_url: &str,
        branch_name: &str,
    ) -> Result<Vec<PullRequestDetail>, GitHostError> {
        match self {
            Self::GitHub(p) => p
                .list_prs_for_branch(repo_path, remote_url, branch_name)
                .await,
            Self::Gitea(p) => p
                .list_prs_for_branch(repo_path, remote_url, branch_name)
                .await,
        }
    }

    async fn get_pr_comments(
        &self,
        repo_path: &Path,
        remote_url: &str,
        pr_number: i64,
    ) -> Result<Vec<UnifiedPrComment>, GitHostError> {
        match self {
            Self::GitHub(p) => p.get_pr_comments(repo_path, remote_url, pr_number).await,
            Self::Gitea(p) => p.get_pr_comments(repo_path, remote_url, pr_number).await,
        }
    }

    async fn list_open_prs(
        &self,
        repo_path: &Path,
        remote_url: &str,
    ) -> Result<Vec<PullRequestDetail>, GitHostError> {
        match self {
            Self::GitHub(p) => p.list_open_prs(repo_path, remote_url).await,
            Self::Gitea(p) => p.list_open_prs(repo_path, remote_url).await,
        }
    }

    fn provider_kind(&self) -> ProviderKind {
        match self {
            Self::GitHub(p) => p.provider_kind(),
            Self::Gitea(p) => p.provider_kind(),
        }
    }
}
