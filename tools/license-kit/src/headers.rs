//! Optional check that tracked source files carry an SPDX-License-Identifier header.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

pub const MARK: &str = "SPDX-License-Identifier:";
pub const EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "mjs", "cjs", "swift", "kt", "kts", "cs", "py", "sh", "bash",
    "zsh", "go", "c", "h", "cpp", "hpp", "java", "dart",
];
/// Path substrings never checked: build output, dependencies, fixtures, generated code.
pub const EXCLUDE: &[&str] = &[
    "target/",
    "node_modules/",
    "vendor/",
    "third_party/",
    "fixtures/",
    "testdata/",
    "dist/",
    "build/",
    ".build/",
    "generated",
    ".min.js",
];

/// Whether `text` has the header in its first 10 lines.
pub fn has_header(text: &str) -> bool {
    text.lines().take(10).any(|l| l.contains(MARK))
}

/// Whether `path` is a source file the check covers.
pub fn covered(path: &str, extra_exclude: &[String]) -> bool {
    let ext_ok = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONS.contains(&e));
    let excluded = |x: &str| {
        if let Some(dir) = x.strip_suffix('/') {
            path.split('/').rev().skip(1).any(|seg| seg == dir)
        } else {
            path.contains(x)
        }
    };
    ext_ok
        && !EXCLUDE.iter().any(|x| excluded(x))
        && !extra_exclude
            .iter()
            .any(|x| path.starts_with(x.as_str()) || excluded(x))
}

/// Tracked files under `dir` (git ls-files) that lack the header.
pub fn missing(dir: &Path, extra_exclude: &[String]) -> Result<Vec<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["ls-files", "-z"])
        .output()
        .context("git ls-files")?;
    if !out.status.success() {
        bail!(
            "git ls-files failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let mut bad = Vec::new();
    for path in String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|p| !p.is_empty())
    {
        if !covered(path, extra_exclude) {
            continue;
        }
        let text = std::fs::read(dir.join(path)).unwrap_or_default();
        if !has_header(&String::from_utf8_lossy(&text)) {
            bad.push(path.to_owned());
        }
    }
    Ok(bad)
}
