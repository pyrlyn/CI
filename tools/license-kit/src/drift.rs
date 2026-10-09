//! What a target repository should contain, and how its copies differ from that.

use std::fmt::Write as _;

use anyhow::Result;
use similar::TextDiff;

use crate::config::{Config, Target};
use crate::readme;

/// One managed file: its current text (`None` when missing) and the text it should have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Desired {
    pub path: String,
    pub current: Option<String>,
    pub want: String,
}

impl Desired {
    pub fn status(&self) -> Status {
        match &self.current {
            None => Status::Missing,
            Some(c) if *c == self.want => Status::Match,
            Some(c) if c.replace("\r\n", "\n") == self.want => Status::Crlf,
            Some(c)
                if c.trim_end_matches(['\r', '\n']) == self.want.trim_end_matches(['\r', '\n']) =>
            {
                Status::FinalNewline
            }
            Some(_) => Status::Content,
        }
    }
}

/// Comparison is byte-exact: CRLF line endings or a different final newline are drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Match,
    Missing,
    Crlf,
    FinalNewline,
    Content,
}

impl Status {
    pub fn describe(self) -> &'static str {
        match self {
            Status::Match => "matches pyrlyn/ci",
            Status::Missing => "missing",
            Status::Crlf => "CRLF line endings",
            Status::FinalNewline => "final newline differs",
            Status::Content => "content differs",
        }
    }
}

/// Every managed file of `target` (the canonical copies, then the README block). `read`
/// returns a file of the target repository by path.
pub fn desired(
    cfg: &Config,
    target: &Target,
    read: &dyn Fn(&str) -> Option<String>,
) -> Result<Vec<Desired>> {
    let files = cfg.target_files(target);
    let mut out = Vec::new();
    for (kind, path) in &files {
        out.push(Desired {
            path: path.clone(),
            current: read(path),
            want: cfg.canonical_text(kind)?,
        });
    }
    if let (Some(readme), Some(snippet)) = (&target.readme, cfg.snippet()?) {
        let pairs: Vec<(String, String)> = files.into_iter().collect();
        let block = readme::block(&snippet, readme, &pairs);
        let current = read(readme);
        let want = readme::with_block(current.as_deref().unwrap_or(""), &block);
        out.push(Desired {
            path: readme.clone(),
            current,
            want,
        });
    }
    Ok(out)
}

/// Markdown report (job summary) of a drift check; `true` when anything drifted.
pub fn report(items: &[Desired]) -> (String, bool) {
    let mut s = String::from("## License check\n\n");
    let mut drift = false;
    for d in items {
        let st = d.status();
        if st == Status::Match {
            let _ = writeln!(s, "- `{}`: {}", d.path, st.describe());
            continue;
        }
        drift = true;
        let _ = writeln!(s, "- `{}`: **{}**", d.path, st.describe());
        let cur = d.current.as_deref().unwrap_or("");
        let diff = TextDiff::from_lines(cur, &d.want)
            .unified_diff()
            .header(
                &format!("a/{} (this repository)", d.path),
                &format!("b/{} (pyrlyn/ci)", d.path),
            )
            .to_string();
        let diff: String = diff.chars().take(60_000).collect();
        let _ = writeln!(s, "\n```diff\n{diff}\n```\n");
    }
    if drift {
        s.push_str(
            "\nThe canonical files live in pyrlyn/ci `licenses/`; update this repository's \
             copies from there (the `chore/license-sync` pull request does it).\n",
        );
    }
    (s, drift)
}
