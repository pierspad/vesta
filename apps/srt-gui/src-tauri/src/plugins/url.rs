use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedPluginSource {
    pub original_url: String,
    pub archive_url: Option<String>,
    pub git_clone_url: String,
    pub git_ref: String,
}

/// Resolves a user-provided plugin repository, archive URL, or local path.
pub fn resolve_plugin_url(
    url: &str,
    git_ref: Option<&str>,
) -> Result<ResolvedPluginSource, String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("URL or path cannot be empty".to_string());
    }

    let default_ref = git_ref
        .map(|r| r.trim())
        .filter(|r| !r.is_empty())
        .unwrap_or("HEAD");

    // Check if input is a local directory or file path (absolute, ~, or file://)
    let path_str = trimmed.strip_prefix("file://").unwrap_or(trimmed);
    let expanded = if let Some(stripped) = path_str.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            home.join(stripped).to_string_lossy().to_string()
        } else {
            path_str.to_string()
        }
    } else {
        path_str.to_string()
    };

    let path = std::path::Path::new(&expanded);
    if path.is_absolute()
        || path_str.starts_with("./")
        || path_str.starts_with("../")
        || path_str.starts_with("~/")
        || path.exists()
    {
        return Ok(ResolvedPluginSource {
            original_url: expanded.clone(),
            archive_url: None,
            git_clone_url: expanded,
            git_ref: "local".to_string(),
        });
    }

    // Check for direct archive URLs (.tar.gz, .tgz, .zip)
    let lower = trimmed.to_lowercase();
    if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") || lower.ends_with(".zip") {
        return Ok(ResolvedPluginSource {
            original_url: trimmed.to_string(),
            archive_url: Some(trimmed.to_string()),
            git_clone_url: trimmed.to_string(),
            git_ref: default_ref.to_string(),
        });
    }

    // Normalize URL prefixes
    let normalized = if trimmed.starts_with("git@") {
        trimmed
            .replace("git@github.com:", "https://github.com/")
            .replace("git@gitlab.com:", "https://gitlab.com/")
    } else if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        format!("https://{trimmed}")
    } else {
        trimmed.to_string()
    };

    // GitHub pattern: (https://)github.com/:owner/:repo
    if let Some(rest) = normalized
        .strip_prefix("https://github.com/")
        .or_else(|| normalized.strip_prefix("http://github.com/"))
    {
        let segments: Vec<&str> = rest
            .trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        if segments.len() >= 2 {
            let owner = segments[0];
            let repo = segments[1].trim_end_matches(".git");
            let archive_url =
                format!("https://codeload.github.com/{owner}/{repo}/tar.gz/{default_ref}");
            let git_clone_url = format!("https://github.com/{owner}/{repo}.git");

            return Ok(ResolvedPluginSource {
                original_url: trimmed.to_string(),
                archive_url: Some(archive_url),
                git_clone_url,
                git_ref: default_ref.to_string(),
            });
        }
    }

    // GitLab pattern: (https://)gitlab.com/:owner/:repo
    if let Some(rest) = normalized
        .strip_prefix("https://gitlab.com/")
        .or_else(|| normalized.strip_prefix("http://gitlab.com/"))
    {
        let segments: Vec<&str> = rest
            .trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        if segments.len() >= 2 {
            let owner = segments[0];
            let repo = segments[1].trim_end_matches(".git");
            let archive_url = format!(
                "https://gitlab.com/{owner}/{repo}/-/archive/{default_ref}/{repo}-{default_ref}.tar.gz"
            );
            let git_clone_url = format!("https://gitlab.com/{owner}/{repo}.git");

            return Ok(ResolvedPluginSource {
                original_url: trimmed.to_string(),
                archive_url: Some(archive_url),
                git_clone_url,
                git_ref: default_ref.to_string(),
            });
        }
    }

    // Generic git repository
    Ok(ResolvedPluginSource {
        original_url: trimmed.to_string(),
        archive_url: None,
        git_clone_url: normalized,
        git_ref: default_ref.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_github_url() {
        let res = resolve_plugin_url("https://github.com/example/sample-plugin", None).unwrap();
        assert_eq!(
            res.archive_url,
            Some("https://codeload.github.com/example/sample-plugin/tar.gz/HEAD".to_string())
        );
        assert_eq!(
            res.git_clone_url,
            "https://github.com/example/sample-plugin.git"
        );
        assert_eq!(res.git_ref, "HEAD");

        let res_branch = resolve_plugin_url(
            "https://github.com/example/sample-plugin.git",
            Some("v1.0.0"),
        )
        .unwrap();
        assert_eq!(
            res_branch.archive_url,
            Some("https://codeload.github.com/example/sample-plugin/tar.gz/v1.0.0".to_string())
        );
        assert_eq!(res_branch.git_ref, "v1.0.0");
    }

    #[test]
    fn test_resolve_gitlab_url() {
        let res =
            resolve_plugin_url("https://gitlab.com/my-group/my-plugin", Some("main")).unwrap();
        assert_eq!(
            res.archive_url,
            Some(
                "https://gitlab.com/my-group/my-plugin/-/archive/main/my-plugin-main.tar.gz"
                    .to_string()
            )
        );
        assert_eq!(
            res.git_clone_url,
            "https://gitlab.com/my-group/my-plugin.git"
        );
    }

    #[test]
    fn test_resolve_local_paths() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().to_string_lossy().to_string();

        let res = resolve_plugin_url(&path, None).unwrap();
        assert_eq!(res.git_ref, "local");
        assert!(res.archive_url.is_none());
        assert_eq!(res.git_clone_url, path);

        let res_file_url = resolve_plugin_url(&format!("file://{path}"), None).unwrap();
        assert_eq!(res_file_url.git_ref, "local");
        assert_eq!(res_file_url.git_clone_url, path);
    }

    #[test]
    fn test_resolve_direct_archive() {
        let res = resolve_plugin_url("https://example.com/downloads/plugin.tar.gz", None).unwrap();
        assert_eq!(
            res.archive_url,
            Some("https://example.com/downloads/plugin.tar.gz".to_string())
        );
    }

    #[test]
    fn test_resolve_empty_url() {
        assert!(resolve_plugin_url("  ", None).is_err());
    }
}
