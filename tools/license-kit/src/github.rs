//! GitHub access behind a trait: the `gh` CLI in production, [`mock::MockGitHub`] in tests.

use std::io::Write as _;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoInfo {
    /// `owner/name`.
    pub repo: String,
    pub default_branch: String,
    /// GitHub's detected SPDX id (`None`/`NOASSERTION` when it could not tell).
    pub spdx: Option<String>,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrRequest {
    pub head: String,
    pub base: String,
    pub title: String,
    pub body: String,
    pub label: String,
    pub draft: bool,
}

pub trait GitHub {
    /// Every repository of an organization with GitHub's license detection.
    fn list_repos(&self, org: &str) -> Result<Vec<RepoInfo>>;
    fn repo(&self, repo: &str) -> Result<RepoInfo>;
    /// A file at a ref; `None` when it does not exist.
    fn read_file(&self, repo: &str, git_ref: &str, path: &str) -> Result<Option<String>>;
    /// Head commit of a branch; `None` when the branch does not exist.
    fn branch_head(&self, repo: &str, branch: &str) -> Result<Option<String>>;
    /// One commit on top of `parent` writing `files` (path, text), then create `branch` at it
    /// or fast-forward `branch` to it (never forced). Returns the new commit.
    fn commit_files(
        &self,
        repo: &str,
        branch: &str,
        parent: &str,
        files: &[(String, String)],
        message: &str,
    ) -> Result<String>;
    /// Number of the open pull request from `head`, if any.
    fn find_open_pr(&self, repo: &str, head: &str) -> Result<Option<u64>>;
    /// Open a pull request with a label (created when missing). Returns its number.
    fn open_pr(&self, repo: &str, pr: &PrRequest) -> Result<u64>;
    fn enable_auto_merge(&self, repo: &str, number: u64) -> Result<()>;
}

/// The authenticated `gh` CLI (GH_TOKEN in CI).
pub struct GhCli;

fn gh(args: &[&str], input: Option<&Value>) -> Result<(bool, String, String)> {
    let mut cmd = Command::new("gh");
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    if input.is_some() {
        cmd.stdin(Stdio::piped());
    }
    let mut child = cmd.spawn().context("running gh")?;
    if let Some(v) = input {
        child
            .stdin
            .take()
            .context("gh stdin")?
            .write_all(v.to_string().as_bytes())?;
    }
    let out = child.wait_with_output()?;
    Ok((
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    ))
}

fn gh_ok(args: &[&str], input: Option<&Value>) -> Result<String> {
    let (ok, out, err) = gh(args, input)?;
    if !ok {
        bail!("gh {} failed: {}", args.join(" "), err.trim());
    }
    Ok(out)
}

fn gh_json(args: &[&str], input: Option<&Value>) -> Result<Value> {
    Ok(serde_json::from_str(&gh_ok(args, input)?)?)
}

fn not_found(err: &str) -> bool {
    err.contains("HTTP 404") || err.contains("Not Found")
}

impl GitHub for GhCli {
    fn list_repos(&self, org: &str) -> Result<Vec<RepoInfo>> {
        let v = gh_json(
            &[
                "repo",
                "list",
                org,
                "--limit",
                "500",
                "--json",
                "nameWithOwner,defaultBranchRef,isArchived,licenseInfo",
            ],
            None,
        )?;
        Ok(v.as_array()
            .into_iter()
            .flatten()
            .map(|r| RepoInfo {
                repo: r["nameWithOwner"].as_str().unwrap_or_default().into(),
                default_branch: r["defaultBranchRef"]["name"]
                    .as_str()
                    .unwrap_or("main")
                    .into(),
                spdx: r["licenseInfo"]["spdxId"].as_str().map(str::to_owned),
                archived: r["isArchived"].as_bool().unwrap_or(false),
            })
            .collect())
    }

    fn repo(&self, repo: &str) -> Result<RepoInfo> {
        let r = gh_json(&["api", &format!("repos/{repo}")], None)?;
        Ok(RepoInfo {
            repo: repo.into(),
            default_branch: r["default_branch"].as_str().unwrap_or("main").into(),
            spdx: r["license"]["spdx_id"].as_str().map(str::to_owned),
            archived: r["archived"].as_bool().unwrap_or(false),
        })
    }

    fn read_file(&self, repo: &str, git_ref: &str, path: &str) -> Result<Option<String>> {
        let url = format!("repos/{repo}/contents/{path}?ref={git_ref}");
        let (ok, out, err) = gh(
            &["api", &url, "-H", "Accept: application/vnd.github.raw"],
            None,
        )?;
        match (ok, not_found(&err)) {
            (true, _) => Ok(Some(out)),
            (false, true) => Ok(None),
            (false, false) => bail!("reading {repo}:{path}@{git_ref}: {}", err.trim()),
        }
    }

    fn branch_head(&self, repo: &str, branch: &str) -> Result<Option<String>> {
        let (ok, out, err) = gh(
            &["api", &format!("repos/{repo}/git/ref/heads/{branch}")],
            None,
        )?;
        match (ok, not_found(&err)) {
            (true, _) => {
                let v: Value = serde_json::from_str(&out)?;
                Ok(v["object"]["sha"].as_str().map(str::to_owned))
            }
            (false, true) => Ok(None),
            (false, false) => bail!("reading {repo} branch {branch}: {}", err.trim()),
        }
    }

    fn commit_files(
        &self,
        repo: &str,
        branch: &str,
        parent: &str,
        files: &[(String, String)],
        message: &str,
    ) -> Result<String> {
        let base = gh_json(
            &["api", &format!("repos/{repo}/git/commits/{parent}")],
            None,
        )?;
        let entries: Vec<Value> = files
            .iter()
            .map(|(p, c)| json!({"path": p, "mode": "100644", "type": "blob", "content": c}))
            .collect();
        let tree_in = json!({"base_tree": base["tree"]["sha"], "tree": entries});
        let tree = gh_json(
            &[
                "api",
                "-X",
                "POST",
                &format!("repos/{repo}/git/trees"),
                "--input",
                "-",
            ],
            Some(&tree_in),
        )?;
        let commit_in = json!({"message": message, "tree": tree["sha"], "parents": [parent]});
        let commit = gh_json(
            &[
                "api",
                "-X",
                "POST",
                &format!("repos/{repo}/git/commits"),
                "--input",
                "-",
            ],
            Some(&commit_in),
        )?;
        let sha = commit["sha"].as_str().context("commit sha")?.to_owned();
        if self.branch_head(repo, branch)?.is_some() {
            let body = json!({"sha": sha, "force": false});
            gh_ok(
                &[
                    "api",
                    "-X",
                    "PATCH",
                    &format!("repos/{repo}/git/refs/heads/{branch}"),
                    "--input",
                    "-",
                ],
                Some(&body),
            )?;
        } else {
            let body = json!({"ref": format!("refs/heads/{branch}"), "sha": sha});
            gh_ok(
                &[
                    "api",
                    "-X",
                    "POST",
                    &format!("repos/{repo}/git/refs"),
                    "--input",
                    "-",
                ],
                Some(&body),
            )?;
        }
        Ok(sha)
    }

    fn find_open_pr(&self, repo: &str, head: &str) -> Result<Option<u64>> {
        let v = gh_json(
            &[
                "pr", "list", "-R", repo, "--head", head, "--state", "open", "--json", "number",
            ],
            None,
        )?;
        Ok(v.as_array()
            .and_then(|a| a.first())
            .and_then(|p| p["number"].as_u64()))
    }

    fn open_pr(&self, repo: &str, pr: &PrRequest) -> Result<u64> {
        let (ok, _, err) = gh(&["api", &format!("repos/{repo}/labels/{}", pr.label)], None)?;
        if !ok && not_found(&err) {
            let body = json!({"name": pr.label, "color": "0e8a16", "description": "License sync from pyrlyn/ci"});
            gh_ok(
                &[
                    "api",
                    "-X",
                    "POST",
                    &format!("repos/{repo}/labels"),
                    "--input",
                    "-",
                ],
                Some(&body),
            )?;
        }
        let mut args = vec![
            "pr", "create", "-R", repo, "--head", &pr.head, "--base", &pr.base, "--title",
            &pr.title, "--body", &pr.body, "--label", &pr.label,
        ];
        if pr.draft {
            args.push("--draft");
        }
        let url = gh_ok(&args, None)?;
        url.trim()
            .rsplit('/')
            .next()
            .and_then(|n| n.parse().ok())
            .context("pull request number")
    }

    fn enable_auto_merge(&self, repo: &str, number: u64) -> Result<()> {
        gh_ok(
            &[
                "pr",
                "merge",
                &number.to_string(),
                "-R",
                repo,
                "--auto",
                "--squash",
            ],
            None,
        )
        .map(|_| ())
    }
}

pub mod mock {
    //! In-memory GitHub for tests: repositories are maps of branch -> files.

    use std::cell::RefCell;
    use std::collections::BTreeMap;

    use anyhow::{Result, bail};

    use super::{GitHub, PrRequest, RepoInfo};

    #[derive(Debug, Clone, Default)]
    pub struct MockRepo {
        pub default_branch: String,
        pub spdx: Option<String>,
        /// branch -> (head commit, path -> text)
        pub branches: BTreeMap<String, (String, BTreeMap<String, String>)>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Commit {
        pub repo: String,
        pub branch: String,
        pub parent: String,
        pub files: Vec<(String, String)>,
        pub message: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Pr {
        pub repo: String,
        pub number: u64,
        pub request: PrRequest,
        pub open: bool,
        pub auto_merge: bool,
    }

    #[derive(Debug, Default)]
    pub struct MockGitHub {
        pub repos: RefCell<BTreeMap<String, MockRepo>>,
        pub commits: RefCell<Vec<Commit>>,
        pub prs: RefCell<Vec<Pr>>,
    }

    impl MockGitHub {
        /// Add a repository whose default branch `main` holds `files`.
        pub fn add_repo(&self, repo: &str, spdx: Option<&str>, files: BTreeMap<String, String>) {
            let mut branches = BTreeMap::new();
            branches.insert("main".to_owned(), (format!("{repo}@main-0"), files));
            self.repos.borrow_mut().insert(
                repo.to_owned(),
                MockRepo {
                    default_branch: "main".into(),
                    spdx: spdx.map(str::to_owned),
                    branches,
                },
            );
        }

        pub fn file(&self, repo: &str, branch: &str, path: &str) -> Option<String> {
            self.repos
                .borrow()
                .get(repo)?
                .branches
                .get(branch)?
                .1
                .get(path)
                .cloned()
        }

        pub fn open_prs(&self, repo: &str) -> Vec<Pr> {
            self.prs
                .borrow()
                .iter()
                .filter(|p| p.repo == repo && p.open)
                .cloned()
                .collect()
        }
    }

    impl GitHub for MockGitHub {
        fn list_repos(&self, org: &str) -> Result<Vec<RepoInfo>> {
            let prefix = format!("{org}/");
            Ok(self
                .repos
                .borrow()
                .iter()
                .filter(|(n, _)| n.starts_with(&prefix))
                .map(|(n, r)| RepoInfo {
                    repo: n.clone(),
                    default_branch: r.default_branch.clone(),
                    spdx: r.spdx.clone(),
                    archived: false,
                })
                .collect())
        }

        fn repo(&self, repo: &str) -> Result<RepoInfo> {
            let repos = self.repos.borrow();
            let Some(r) = repos.get(repo) else {
                bail!("no repository {repo}")
            };
            Ok(RepoInfo {
                repo: repo.into(),
                default_branch: r.default_branch.clone(),
                spdx: r.spdx.clone(),
                archived: false,
            })
        }

        fn read_file(&self, repo: &str, git_ref: &str, path: &str) -> Result<Option<String>> {
            Ok(self.file(repo, git_ref, path))
        }

        fn branch_head(&self, repo: &str, branch: &str) -> Result<Option<String>> {
            Ok(self
                .repos
                .borrow()
                .get(repo)
                .and_then(|r| r.branches.get(branch))
                .map(|b| b.0.clone()))
        }

        fn commit_files(
            &self,
            repo: &str,
            branch: &str,
            parent: &str,
            files: &[(String, String)],
            message: &str,
        ) -> Result<String> {
            let mut repos = self.repos.borrow_mut();
            let Some(r) = repos.get_mut(repo) else {
                bail!("no repository {repo}")
            };
            let base = r
                .branches
                .values()
                .find(|(head, _)| head == parent)
                .map(|(_, f)| f.clone())
                .ok_or_else(|| anyhow::anyhow!("unknown parent {parent}"))?;
            if let Some((head, _)) = r.branches.get(branch) {
                if head != parent {
                    bail!("non-fast-forward update of {branch}");
                }
            }
            let mut tree = base;
            for (p, c) in files {
                tree.insert(p.clone(), c.clone());
            }
            let n = self.commits.borrow().len() + 1;
            let sha = format!("{repo}@commit-{n}");
            r.branches.insert(branch.to_owned(), (sha.clone(), tree));
            self.commits.borrow_mut().push(Commit {
                repo: repo.into(),
                branch: branch.into(),
                parent: parent.into(),
                files: files.to_vec(),
                message: message.into(),
            });
            Ok(sha)
        }

        fn find_open_pr(&self, repo: &str, head: &str) -> Result<Option<u64>> {
            Ok(self
                .prs
                .borrow()
                .iter()
                .find(|p| p.repo == repo && p.open && p.request.head == head)
                .map(|p| p.number))
        }

        fn open_pr(&self, repo: &str, pr: &PrRequest) -> Result<u64> {
            if self.find_open_pr(repo, &pr.head)?.is_some() {
                bail!("a pull request for {} already exists", pr.head);
            }
            let number = self.prs.borrow().len() as u64 + 1;
            self.prs.borrow_mut().push(Pr {
                repo: repo.into(),
                number,
                request: pr.clone(),
                open: true,
                auto_merge: false,
            });
            Ok(number)
        }

        fn enable_auto_merge(&self, repo: &str, number: u64) -> Result<()> {
            for p in self.prs.borrow_mut().iter_mut() {
                if p.repo == repo && p.number == number {
                    if p.request.draft {
                        bail!("auto-merge cannot be enabled on a draft");
                    }
                    p.auto_merge = true;
                    return Ok(());
                }
            }
            bail!("no pull request {repo}#{number}")
        }
    }
}
