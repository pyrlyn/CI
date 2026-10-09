//! Whether a pull request may be merged automatically.
//!
//! Merge only a ready (non-draft) pull request that is either Dependabot's own or carries the
//! `license` label on a license-sync branch (`chore/license-*`). Everything else is refused
//! with a reason.

use crate::config::{DEFAULT_BRANCH, DEFAULT_LABEL};

pub const DEPENDABOT: &str = "dependabot[bot]";
/// Head branch prefixes of license-sync pull requests.
pub const LICENSE_PREFIXES: &[&str] = &[DEFAULT_BRANCH, "chore/license-"];

#[derive(Debug, Clone, Default)]
pub struct PrFacts {
    pub author: String,
    pub labels: Vec<String>,
    pub head_branch: String,
    pub draft: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Merge(String),
    Refuse(String),
}

impl Decision {
    pub fn is_merge(&self) -> bool {
        matches!(self, Decision::Merge(_))
    }
}

/// `head` is `prefix`, below it (`prefix/...`), or continues a prefix ending in `-`.
fn branch_matches(head: &str, prefix: &str) -> bool {
    head == prefix
        || head.starts_with(&format!("{prefix}/"))
        || (prefix.ends_with('-') && head.starts_with(prefix))
}

pub fn decide(pr: &PrFacts) -> Decision {
    if pr.draft {
        return Decision::Refuse("draft pull request".into());
    }
    if pr.author == DEPENDABOT {
        return Decision::Merge("Dependabot pull request".into());
    }
    let labelled = pr.labels.iter().any(|l| l == DEFAULT_LABEL);
    let license_branch = LICENSE_PREFIXES
        .iter()
        .any(|p| branch_matches(&pr.head_branch, p));
    match (labelled, license_branch) {
        (true, true) => Decision::Merge(format!("`{DEFAULT_LABEL}` label on {}", pr.head_branch)),
        (true, false) => {
            Decision::Refuse(format!("`{}` is not a license-sync branch", pr.head_branch))
        }
        (false, true) => Decision::Refuse(format!("no `{DEFAULT_LABEL}` label")),
        (false, false) => Decision::Refuse(format!("author `{}` is not allowed", pr.author)),
    }
}
