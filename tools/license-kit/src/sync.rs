//! Dependabot-style sync: one long-lived branch and pull request per target repository.
//!
//! For each GPL target the canonical files and the README block are compared with the default
//! branch. Nothing to change: no commit, no pull request. Otherwise one commit lands on the
//! sync branch (created from the default branch, or fast-forwarded, never forced) with only
//! the files the branch still lacks, and a pull request is opened unless one is already open
//! from that branch. The pull request is a draft unless the target sets `auto-merge: true`
//! (then it is ready for review with GitHub auto-merge, which waits for required checks).

use anyhow::Result;

use crate::config::{Config, Target};
use crate::drift::{self, Status};
use crate::github::{GitHub, PrRequest};
use crate::license::{self, Classification};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Not GPL (or a conflicting manifest): left alone.
    SkippedNotGpl(Classification),
    SkippedArchived,
    /// The default branch already matches.
    UpToDate,
    /// Dry run: these files would change.
    WouldUpdate(Vec<String>),
    /// `committed` is empty when the sync branch already had every change.
    Updated {
        committed: Vec<String>,
        pr: u64,
        pr_created: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub repo: String,
    pub action: Action,
}

pub const TITLE: &str = "chore(license): sync license files from pyrlyn/ci";

fn body(changed: &[String], source: &str, draft: bool) -> String {
    let files: String = changed.iter().map(|f| format!("- `{f}`\n")).collect();
    let state = if draft {
        "Opened as a draft: mark it ready for review and merge it, or set `auto-merge: true` for \
         this repository in pyrlyn/ci `licenses/targets.yml`."
    } else {
        "Auto-merge is enabled: it merges once the required checks pass."
    };
    format!(
        "Automated update from pyrlyn/ci{source}: the canonical license files (`licenses/`) \
         and the README license-sync block.\n\nFiles brought in line:\n{files}\n{state}\n\nThe \
         branch is maintained by pyrlyn/ci's license-sync; edits here are overwritten.\n"
    )
}

/// Sync one target. `source` is appended to the provenance (e.g. "@abc1234").
pub fn sync_repo(
    gh: &dyn GitHub,
    cfg: &Config,
    target: &Target,
    source: &str,
    dry_run: bool,
) -> Result<Outcome> {
    let repo = target.repo.clone();
    let info = gh.repo(&repo)?;
    let out = |action| {
        Ok(Outcome {
            repo: repo.clone(),
            action,
        })
    };
    if info.archived {
        return out(Action::SkippedArchived);
    }
    let base = info.default_branch.clone();
    let read_base = |p: &str| gh.read_file(&repo, &base, p).ok().flatten();
    let class = license::classify(&read_base, info.spdx.as_deref());
    if !class.gpl {
        return out(Action::SkippedNotGpl(class));
    }
    let want = drift::desired(cfg, target, &read_base)?;
    let changed: Vec<_> = want
        .iter()
        .filter(|d| d.status() != Status::Match)
        .collect();
    if changed.is_empty() {
        return out(Action::UpToDate);
    }
    let names: Vec<String> = changed.iter().map(|d| d.path.clone()).collect();
    if dry_run {
        return out(Action::WouldUpdate(names));
    }

    let branch = &cfg.branch;
    let head = gh.branch_head(&repo, branch)?;
    let (parent, on_branch) = match &head {
        Some(h) => (h.clone(), branch.clone()),
        None => (
            gh.branch_head(&repo, &base)?
                .unwrap_or_else(|| base.clone()),
            base.clone(),
        ),
    };
    let mut files = Vec::new();
    for d in &changed {
        if gh.read_file(&repo, &on_branch, &d.path)?.as_deref() != Some(d.want.as_str()) {
            files.push((d.path.clone(), d.want.clone()));
        }
    }
    let committed: Vec<String> = files.iter().map(|(p, _)| p.clone()).collect();
    if !files.is_empty() {
        let msg = format!("{TITLE}{source}");
        gh.commit_files(&repo, branch, &parent, &files, &msg)?;
    }
    let draft = !target.auto_merge;
    let (pr, pr_created) = match gh.find_open_pr(&repo, branch)? {
        Some(n) => (n, false),
        None => {
            let req = PrRequest {
                head: branch.clone(),
                base: base.clone(),
                title: TITLE.into(),
                body: body(&names, source, draft),
                label: cfg.label.clone(),
                draft,
            };
            (gh.open_pr(&repo, &req)?, true)
        }
    };
    if target.auto_merge && pr_created {
        gh.enable_auto_merge(&repo, pr)?;
    }
    out(Action::Updated {
        committed,
        pr,
        pr_created,
    })
}

/// Sync every configured target (or only `only`, when non-empty).
pub fn sync_all(
    gh: &dyn GitHub,
    cfg: &Config,
    only: &[String],
    source: &str,
    dry_run: bool,
) -> Result<Vec<Outcome>> {
    cfg.targets
        .iter()
        .filter(|t| only.is_empty() || only.iter().any(|o| o.eq_ignore_ascii_case(&t.repo)))
        .map(|t| sync_repo(gh, cfg, t, source, dry_run))
        .collect()
}

/// Organization repositories that are GPL but not configured, and the non-GPL ones with
/// their detected license, for the maintainer to decide.
pub fn inventory(
    gh: &dyn GitHub,
    cfg: &Config,
    org: &str,
) -> Result<Vec<(String, Classification, bool)>> {
    let mut out = Vec::new();
    for r in gh.list_repos(org)? {
        if r.archived {
            continue;
        }
        let read = |p: &str| gh.read_file(&r.repo, &r.default_branch, p).ok().flatten();
        let class = license::classify(&read, r.spdx.as_deref());
        let targeted = cfg
            .targets
            .iter()
            .any(|t| t.repo.eq_ignore_ascii_case(&r.repo));
        out.push((r.repo, class, targeted));
    }
    Ok(out)
}
