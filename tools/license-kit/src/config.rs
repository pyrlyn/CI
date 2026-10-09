//! `licenses/targets.yml`: canonical files and per-repository targets.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

/// Default sync branch; also the prefix the auto-merge decision accepts.
pub const DEFAULT_BRANCH: &str = "chore/license-sync";
/// Default label of sync pull requests.
pub const DEFAULT_LABEL: &str = "license";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
    /// Kind (`gpl`, `commercial`, ...) -> file name relative to the config's directory.
    pub canonical: BTreeMap<String, String>,
    /// README block text, relative to the config's directory; `{kind}` is replaced by that
    /// kind's path relative to the README.
    #[serde(default)]
    pub readme_snippet: Option<String>,
    #[serde(default = "default_branch")]
    pub branch: String,
    #[serde(default = "default_label")]
    pub label: String,
    #[serde(default)]
    pub targets: Vec<Target>,
    /// Directory the canonical paths are relative to (the config file's directory).
    #[serde(skip)]
    pub base: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Target {
    /// `owner/name`.
    pub repo: String,
    /// Kind -> path in the target repository; `null` leaves that kind out (e.g. no
    /// commercial license). Missing kinds use the canonical file name.
    #[serde(default)]
    pub files: BTreeMap<String, Option<String>>,
    /// README that gets the license-sync block; `None` (`readme: null`) skips it.
    #[serde(default = "default_readme")]
    pub readme: Option<String>,
    /// Open the sync PR ready for review with GitHub auto-merge instead of as a draft.
    #[serde(default)]
    pub auto_merge: bool,
}

fn default_branch() -> String {
    DEFAULT_BRANCH.into()
}
fn default_label() -> String {
    DEFAULT_LABEL.into()
}
fn default_readme() -> Option<String> {
    Some("README.md".into())
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let text =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let base = path.parent().map(Path::to_path_buf).unwrap_or_default();
        Self::parse(&text, base).with_context(|| format!("parsing {}", path.display()))
    }

    pub fn parse(text: &str, base: PathBuf) -> Result<Self> {
        let mut cfg: Config = serde_yaml::from_str(text)?;
        cfg.base = base;
        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<()> {
        if self.canonical.is_empty() {
            bail!("`canonical` lists no files");
        }
        let mut seen = std::collections::BTreeSet::new();
        for t in &self.targets {
            if t.repo.split('/').count() != 2 {
                bail!("target `{}` is not owner/name", t.repo);
            }
            if !seen.insert(t.repo.to_lowercase()) {
                bail!("target `{}` is listed twice", t.repo);
            }
            for kind in t.files.keys() {
                if !self.canonical.contains_key(kind) {
                    bail!("target `{}` maps unknown kind `{kind}`", t.repo);
                }
            }
        }
        Ok(())
    }

    /// Path of a canonical file on disk.
    pub fn canonical_path(&self, file: &str) -> PathBuf {
        self.base.join(file)
    }

    /// Canonical text of `kind`, read byte-exact.
    pub fn canonical_text(&self, kind: &str) -> Result<String> {
        let file = self
            .canonical
            .get(kind)
            .with_context(|| format!("no canonical `{kind}`"))?;
        let path = self.canonical_path(file);
        fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))
    }

    pub fn snippet(&self) -> Result<Option<String>> {
        match &self.readme_snippet {
            None => Ok(None),
            Some(f) => {
                let path = self.canonical_path(f);
                let text = fs::read_to_string(&path)
                    .with_context(|| format!("reading {}", path.display()))?;
                Ok(Some(text))
            }
        }
    }

    /// The configured target, or one with default paths for an unlisted repository.
    pub fn target(&self, repo: &str) -> Target {
        self.targets
            .iter()
            .find(|t| t.repo.eq_ignore_ascii_case(repo))
            .cloned()
            .unwrap_or_else(|| Target {
                repo: repo.into(),
                files: BTreeMap::new(),
                readme: default_readme(),
                auto_merge: false,
            })
    }

    /// Kind -> path in the target, defaulting to the canonical file name; kinds the target
    /// maps to `null` are left out.
    pub fn target_files(&self, t: &Target) -> BTreeMap<String, String> {
        self.canonical
            .iter()
            .filter_map(|(kind, file)| match t.files.get(kind) {
                Some(None) => None,
                Some(Some(path)) => Some((kind.clone(), path.clone())),
                None => Some((kind.clone(), file.clone())),
            })
            .collect()
    }
}
