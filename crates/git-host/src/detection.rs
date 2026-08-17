//! Git hosting provider detection from repository URLs.

use crate::types::ProviderKind;

/// Detect the git hosting provider from a remote URL.
///
/// Supports:
/// - GitHub.com: `https://github.com/owner/repo` or `git@github.com:owner/repo.git`
/// - GitHub Enterprise: URLs containing `github.` (e.g., `https://github.company.com/owner/repo`)
///
/// Anything else (GitLab, Bitbucket, custom hosts, etc.) falls through to
/// `ProviderKind::Unknown` and `GitHostService::from_url` will return
/// `GitHostError::UnsupportedProvider`.
pub(crate) fn detect_provider_from_url(url: &str) -> ProviderKind {
    let url_lower = url.to_lowercase();
    if url_lower.contains("github.com") || url_lower.contains("github.") {
        ProviderKind::GitHub
    } else {
        ProviderKind::Unknown
    }
}

/// Returns true when the configured Gitea `base_url` matches the host of a git
/// remote, indicating the remote is served by the same Gitea instance.
///
/// Gitea hosts are arbitrary (not detectable by hostname alone), so this
/// config-based check is what lets `GitHostService::from_url` decide to route a
/// non-GitHub remote to the Gitea provider.
pub(crate) fn is_gitea_remote(base_url: Option<&str>, remote_url: &str) -> bool {
    let Some(base) = base_url else {
        return false;
    };
    let base = base.trim();
    if base.is_empty() {
        return false;
    };
    let base_host = host_of(base);
    let remote_host = host_of(remote_url);
    !base_host.is_empty() && base_host.eq_ignore_ascii_case(&remote_host)
}

fn host_of(url: &str) -> String {
    let parsed = match url::Url::parse(url) {
        Ok(u) => u,
        Err(_) => match url::Url::parse(&format!("https://{url}")) {
            Ok(u) => u,
            Err(_) => return String::new(),
        },
    };
    parsed.host_str().unwrap_or_default().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_com_https() {
        assert_eq!(
            detect_provider_from_url("https://github.com/owner/repo"),
            ProviderKind::GitHub
        );
        assert_eq!(
            detect_provider_from_url("https://github.com/owner/repo.git"),
            ProviderKind::GitHub
        );
    }

    #[test]
    fn test_github_com_ssh() {
        assert_eq!(
            detect_provider_from_url("git@github.com:owner/repo.git"),
            ProviderKind::GitHub
        );
    }

    #[test]
    fn test_github_enterprise() {
        assert_eq!(
            detect_provider_from_url("https://github.company.com/owner/repo"),
            ProviderKind::GitHub
        );
        assert_eq!(
            detect_provider_from_url("https://github.acme.corp/team/project"),
            ProviderKind::GitHub
        );
        assert_eq!(
            detect_provider_from_url("git@github.internal.io:org/repo.git"),
            ProviderKind::GitHub
        );
    }

    #[test]
    fn test_unknown_provider() {
        assert_eq!(
            detect_provider_from_url("https://gitlab.com/owner/repo"),
            ProviderKind::Unknown
        );
        assert_eq!(
            detect_provider_from_url("https://bitbucket.org/owner/repo"),
            ProviderKind::Unknown
        );
    }

    #[test]
    fn test_is_gitea_remote_match() {
        assert!(is_gitea_remote(
            Some("https://gitea.example.com"),
            "https://gitea.example.com/owner/repo.git"
        ));
        assert!(is_gitea_remote(
            Some("https://gitea.example.com/"),
            "https://GITEA.example.com/org/project"
        ));
        assert!(is_gitea_remote(
            Some("http://localhost:3000"),
            "http://localhost:3000/team/repo.git"
        ));
    }

    #[test]
    fn test_is_gitea_remote_no_match() {
        assert!(!is_gitea_remote(
            None,
            "https://gitea.example.com/owner/repo.git"
        ));
        assert!(!is_gitea_remote(
            Some("https://other.example.com"),
            "https://gitea.example.com/owner/repo.git"
        ));
        assert!(!is_gitea_remote(
            Some("https://gitea.example.com"),
            "https://github.com/owner/repo"
        ));
    }
}
