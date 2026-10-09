use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use license_kit::automerge::{self, PrFacts};
use license_kit::config::Config;
use license_kit::github::GhCli;
use license_kit::{drift, headers, license, sync};

/// Sync and check pyrlyn/ci's canonical license files in GPL repositories.
#[derive(Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Exit 1 when DIR's license copies or README block differ from the canonical ones.
    DriftCheck {
        #[arg(long)]
        config: PathBuf,
        /// owner/name of the checked repository (selects its paths in the config).
        #[arg(long)]
        repo: String,
        /// Also require an SPDX-License-Identifier header in every tracked source file.
        #[arg(long)]
        headers: bool,
        /// Extra path prefix or substring excluded from the header check (repeatable).
        #[arg(long)]
        header_exclude: Vec<String>,
        dir: PathBuf,
    },
    /// Open or update the license-sync pull request of every GPL target (gh CLI, GH_TOKEN).
    Sync {
        #[arg(long)]
        config: PathBuf,
        /// Only these targets (owner/name, repeatable).
        #[arg(long)]
        repo: Vec<String>,
        /// Also list the organization's repositories: GPL ones missing from the config and
        /// non-GPL ones with their detected license.
        #[arg(long)]
        org: Option<String>,
        /// Provenance appended to the commit message, e.g. "@abc1234".
        #[arg(long, default_value = "")]
        source: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Write (or with --check verify) the README license-sync block in DIR; a non-GPL DIR is
    /// skipped.
    ReadmeLine {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        repo: String,
        #[arg(long)]
        check: bool,
        dir: PathBuf,
    },
    /// Print `merge` or `refuse` (and write `decision=` to $GITHUB_OUTPUT).
    AutomergeDecide {
        #[arg(long)]
        author: String,
        #[arg(long)]
        label: Vec<String>,
        #[arg(long)]
        head: String,
        #[arg(long)]
        draft: bool,
    },
}

fn summary(text: &str) -> Result<()> {
    print!("{text}");
    if let Ok(path) = std::env::var("GITHUB_STEP_SUMMARY") {
        if !path.is_empty() {
            fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?
                .write_all(text.as_bytes())?;
        }
    }
    Ok(())
}

fn read_in(dir: &Path) -> impl Fn(&str) -> Option<String> + '_ {
    move |p| fs::read_to_string(dir.join(p)).ok()
}

fn run() -> Result<ExitCode> {
    match Cli::parse().cmd {
        Cmd::DriftCheck {
            config,
            repo,
            headers: with_headers,
            header_exclude,
            dir,
        } => {
            let cfg = Config::load(&config)?;
            let items = drift::desired(&cfg, &cfg.target(&repo), &read_in(&dir))?;
            let (mut text, mut drifted) = drift::report(&items);
            for d in items.iter().filter(|d| d.status() != drift::Status::Match) {
                println!(
                    "::error file={}::{} ({})",
                    d.path,
                    d.path,
                    d.status().describe()
                );
            }
            if with_headers {
                let bad = headers::missing(&dir, &header_exclude)?;
                if bad.is_empty() {
                    text.push_str("- license headers: every source file has one\n");
                } else {
                    drifted = true;
                    println!(
                        "::error::{} source file(s) lack an {} header",
                        bad.len(),
                        headers::MARK
                    );
                    text.push_str(&format!(
                        "- license headers: **{} file(s) without** `{}`\n",
                        bad.len(),
                        headers::MARK
                    ));
                    for p in bad.iter().take(200) {
                        text.push_str(&format!("  - `{p}`\n"));
                    }
                }
            }
            summary(&text)?;
            Ok(if drifted {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            })
        }
        Cmd::Sync {
            config,
            repo,
            org,
            source,
            dry_run,
        } => {
            let cfg = Config::load(&config)?;
            let gh = GhCli;
            let mut text = String::from("## License sync\n\n");
            for o in sync::sync_all(&gh, &cfg, &repo, &source, dry_run)? {
                text.push_str(&format!("- {}: {:?}\n", o.repo, o.action));
            }
            if let Some(org) = org {
                text.push_str("\n### Organization inventory\n\n");
                for (r, c, targeted) in sync::inventory(&gh, &cfg, &org)? {
                    let note = match (c.gpl, targeted) {
                        (true, true) => "GPL, synced",
                        (true, false) => "**GPL but not in licenses/targets.yml**",
                        (false, true) => "**not GPL but listed: skipped**",
                        (false, false) => "not GPL, skipped",
                    };
                    text.push_str(&format!("- {r}: {} ({}): {note}\n", c.detected, c.reason));
                }
            }
            summary(&text)?;
            Ok(ExitCode::SUCCESS)
        }
        Cmd::ReadmeLine {
            config,
            repo,
            check,
            dir,
        } => {
            let cfg = Config::load(&config)?;
            let class = license::classify(&read_in(&dir), None);
            if !class.gpl {
                println!("skipped: {} is not GPL ({})", dir.display(), class.reason);
                return Ok(ExitCode::SUCCESS);
            }
            let target = cfg.target(&repo);
            let readme = target.readme.clone().context("target has no README")?;
            let items = drift::desired(&cfg, &target, &read_in(&dir))?;
            let d = items
                .iter()
                .find(|d| d.path == readme)
                .context("no README block configured")?;
            if d.status() == drift::Status::Match {
                println!("{readme}: up to date");
                return Ok(ExitCode::SUCCESS);
            }
            if check {
                println!("{readme}: license-sync block missing or stale");
                return Ok(ExitCode::FAILURE);
            }
            fs::write(dir.join(&readme), &d.want)?;
            println!("{readme}: updated");
            Ok(ExitCode::SUCCESS)
        }
        Cmd::AutomergeDecide {
            author,
            label,
            head,
            draft,
        } => {
            let d = automerge::decide(&PrFacts {
                author,
                labels: label,
                head_branch: head,
                draft,
            });
            let (word, why) = match &d {
                automerge::Decision::Merge(w) => ("merge", w),
                automerge::Decision::Refuse(w) => ("refuse", w),
            };
            println!("{word}: {why}");
            if let Ok(path) = std::env::var("GITHUB_OUTPUT") {
                if !path.is_empty() {
                    let mut f = fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)?;
                    writeln!(f, "decision={word}")?;
                }
            }
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("::error::{e:#}");
            ExitCode::from(2)
        }
    }
}
