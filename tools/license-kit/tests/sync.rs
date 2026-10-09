//! (3) sync against fixture repositories in the in-memory GitHub: the right files land on the
//! sync branch, non-GPL repositories are skipped, matching repositories get no commit or pull
//! request, and a second run creates neither a duplicate commit nor a duplicate pull request.

mod common;

use std::collections::BTreeMap;

use common::*;
use license_kit::config::Config;
use license_kit::github::GitHub;
use license_kit::github::mock::MockGitHub;
use license_kit::sync::{self, Action};

const TARGETS: &str = "targets:
  - repo: pyrlyn/gpl-outdated
    files: {gpl: LICENSE, commercial: PRICING.md}
  - repo: pyrlyn/gpl-current
    files: {gpl: LICENSE, commercial: PRICING.md}
  - repo: pyrlyn/mit
    files: {gpl: LICENSE, commercial: PRICING.md}
  - repo: pyrlyn/conflict
    files: {gpl: LICENSE, commercial: PRICING.md}
  - repo: pyrlyn/automerge
    files: {gpl: COPYING, commercial: docs/COMMERCIAL.md}
    auto-merge: true
  - repo: pyrlyn/gpl-only
    files: {gpl: LICENSE, commercial: null}
    readme: null
";

fn repo_files(name: &str) -> BTreeMap<String, String> {
    let mut f = load_dir(&fixtures().join("repos").join(name));
    match name {
        "gpl-outdated" => {
            f.insert("LICENSE".into(), fixture_file("broken/LICENSE"));
        }
        "gpl-current" => {
            f.insert("LICENSE".into(), fixture_file("valid/LICENSE"));
            f.insert("PRICING.md".into(), fixture_file("valid/PRICING.md"));
        }
        "conflict" => {
            f.insert("LICENSE".into(), fixture_file("valid/LICENSE"));
        }
        _ => {}
    }
    f
}

fn setup() -> (MockGitHub, Config) {
    let gh = MockGitHub::default();
    for name in ["gpl-outdated", "gpl-current", "mit", "conflict"] {
        gh.add_repo(&format!("pyrlyn/{name}"), None, repo_files(name));
    }
    let mut auto = BTreeMap::new();
    auto.insert("COPYING".to_owned(), fixture_file("broken/LICENSE"));
    auto.insert("README.md".to_owned(), "# automerge\n".to_owned());
    gh.add_repo("pyrlyn/automerge", Some("GPL-3.0"), auto);
    let mut only = BTreeMap::new();
    only.insert("LICENSE".to_owned(), fixture_file("broken/LICENSE"));
    only.insert("README.md".to_owned(), "# gpl-only\n".to_owned());
    gh.add_repo("pyrlyn/gpl-only", Some("GPL-3.0"), only);
    (gh, config_with_targets(TARGETS))
}

fn action<'a>(out: &'a [sync::Outcome], repo: &str) -> &'a Action {
    &out.iter().find(|o| o.repo == repo).unwrap().action
}

#[test]
fn correct_files_land_on_the_sync_branch() {
    let (gh, cfg) = setup();
    let out = sync::sync_all(&gh, &cfg, &[], "@test", false).unwrap();
    let Action::Updated {
        committed,
        pr,
        pr_created,
    } = action(&out, "pyrlyn/gpl-outdated")
    else {
        panic!("{out:?}")
    };
    assert!(*pr_created);
    let mut c = committed.clone();
    c.sort();
    assert_eq!(c, ["LICENSE", "PRICING.md", "README.md"]);
    let b = "chore/license-sync";
    assert_eq!(
        gh.file("pyrlyn/gpl-outdated", b, "LICENSE").unwrap(),
        fixture_file("valid/LICENSE")
    );
    assert_eq!(
        gh.file("pyrlyn/gpl-outdated", b, "PRICING.md").unwrap(),
        fixture_file("valid/PRICING.md")
    );
    let readme = gh.file("pyrlyn/gpl-outdated", b, "README.md").unwrap();
    assert_eq!(readme.matches("<!-- license-sync:start -->").count(), 1);
    assert!(readme.contains("[PRICING.md](PRICING.md)"));
    assert!(readme.find("license-sync:end").unwrap() < readme.find("## Contributing").unwrap());
    assert_eq!(
        gh.file("pyrlyn/gpl-outdated", b, "Cargo.toml"),
        gh.file("pyrlyn/gpl-outdated", "main", "Cargo.toml")
    );
    // The default branch is never written.
    assert_eq!(
        gh.file("pyrlyn/gpl-outdated", "main", "LICENSE").unwrap(),
        fixture_file("broken/LICENSE")
    );
    assert!(gh.commits.borrow().iter().all(|c| c.branch == b));

    let prs = gh.open_prs("pyrlyn/gpl-outdated");
    assert_eq!(prs.len(), 1);
    assert_eq!(prs[0].number, *pr);
    assert!(prs[0].request.draft, "sync PRs are drafts by default");
    assert_eq!(prs[0].request.label, "license");
    assert_eq!(prs[0].request.base, "main");
    assert!(!prs[0].auto_merge);
}

#[test]
fn per_repo_paths_are_respected_and_auto_merge_opt_in() {
    let (gh, cfg) = setup();
    let out = sync::sync_all(&gh, &cfg, &["pyrlyn/automerge".into()], "", false).unwrap();
    assert!(matches!(
        action(&out, "pyrlyn/automerge"),
        Action::Updated { .. }
    ));
    let b = "chore/license-sync";
    assert_eq!(
        gh.file("pyrlyn/automerge", b, "COPYING").unwrap(),
        fixture_file("valid/LICENSE")
    );
    assert_eq!(
        gh.file("pyrlyn/automerge", b, "docs/COMMERCIAL.md")
            .unwrap(),
        fixture_file("valid/PRICING.md")
    );
    assert!(gh.file("pyrlyn/automerge", b, "LICENSE").is_none());
    assert!(
        gh.file("pyrlyn/automerge", b, "README.md")
            .unwrap()
            .contains("[docs/COMMERCIAL.md](docs/COMMERCIAL.md)")
    );
    let prs = gh.open_prs("pyrlyn/automerge");
    assert!(
        !prs[0].request.draft && prs[0].auto_merge,
        "auto-merge: true opens it ready with auto-merge"
    );
}

#[test]
fn non_gpl_repositories_are_skipped() {
    let (gh, cfg) = setup();
    let out = sync::sync_all(&gh, &cfg, &[], "", false).unwrap();
    for repo in ["pyrlyn/mit", "pyrlyn/conflict"] {
        let Action::SkippedNotGpl(c) = action(&out, repo) else {
            panic!("{repo}: {out:?}")
        };
        assert!(!c.gpl);
        assert!(gh.open_prs(repo).is_empty(), "{repo}: no PR");
        assert!(
            gh.commits.borrow().iter().all(|k| k.repo != repo),
            "{repo}: no commit"
        );
        assert!(
            gh.branch_head(repo, "chore/license-sync")
                .unwrap()
                .is_none(),
            "{repo}: no branch"
        );
    }
    let Action::SkippedNotGpl(c) = action(&out, "pyrlyn/mit") else {
        unreachable!()
    };
    assert_eq!(c.detected, "MIT");
    let Action::SkippedNotGpl(c) = action(&out, "pyrlyn/conflict") else {
        unreachable!()
    };
    assert_eq!(c.detected, "MIT OR Apache-2.0");
    assert!(c.reason.starts_with("conflict:"), "{}", c.reason);
}

#[test]
fn matching_repositories_get_no_commit_and_no_pr() {
    let (gh, cfg) = setup();
    let out = sync::sync_all(&gh, &cfg, &["pyrlyn/gpl-current".into()], "", false).unwrap();
    assert_eq!(action(&out, "pyrlyn/gpl-current"), &Action::UpToDate);
    assert!(gh.commits.borrow().is_empty());
    assert!(gh.prs.borrow().is_empty());
}

#[test]
fn second_run_creates_no_duplicate_commit_or_pr() {
    let (gh, cfg) = setup();
    sync::sync_all(&gh, &cfg, &[], "", false).unwrap();
    let commits = gh.commits.borrow().len();
    let prs = gh.prs.borrow().len();
    let out = sync::sync_all(&gh, &cfg, &[], "", false).unwrap();
    assert_eq!(gh.commits.borrow().len(), commits, "no new commit");
    assert_eq!(gh.prs.borrow().len(), prs, "no new PR");
    let Action::Updated {
        committed,
        pr_created,
        ..
    } = action(&out, "pyrlyn/gpl-outdated")
    else {
        panic!()
    };
    assert!(committed.is_empty() && !pr_created);
}

#[test]
fn a_canonical_change_fast_forwards_the_existing_branch() {
    let (gh, mut cfg) = setup();
    sync::sync_all(&gh, &cfg, &["pyrlyn/gpl-outdated".into()], "", false).unwrap();
    let head = gh
        .branch_head("pyrlyn/gpl-outdated", "chore/license-sync")
        .unwrap()
        .unwrap();
    // A new commercial text in infra: only that file is committed, on top of the branch.
    let dir = scratch("canonical");
    for f in ["LICENSE", "readme-snippet.md"] {
        std::fs::copy(infra_root().join("licenses").join(f), dir.join(f)).unwrap();
    }
    std::fs::write(
        dir.join("PRICING.md"),
        "# Commercial License\n\nNew terms.\n",
    )
    .unwrap();
    cfg.base = dir;
    let out = sync::sync_all(&gh, &cfg, &["pyrlyn/gpl-outdated".into()], "", false).unwrap();
    let Action::Updated {
        committed,
        pr_created,
        ..
    } = action(&out, "pyrlyn/gpl-outdated")
    else {
        panic!()
    };
    assert_eq!(committed, &["PRICING.md"]);
    assert!(!pr_created);
    let last = gh.commits.borrow().last().cloned().unwrap();
    assert_eq!(last.parent, head, "fast-forward, never forced");
    assert_eq!(gh.open_prs("pyrlyn/gpl-outdated").len(), 1);
}

#[test]
fn dry_run_writes_nothing() {
    let (gh, cfg) = setup();
    let out = sync::sync_all(&gh, &cfg, &[], "", true).unwrap();
    assert!(matches!(action(&out, "pyrlyn/gpl-outdated"), Action::WouldUpdate(f) if f.len() == 3));
    assert!(gh.commits.borrow().is_empty() && gh.prs.borrow().is_empty());
}

#[test]
fn inventory_lists_untargeted_and_non_gpl_repositories() {
    let (gh, cfg) = setup();
    gh.add_repo("pyrlyn/unlisted", None, {
        let mut f = BTreeMap::new();
        f.insert("LICENSE".to_owned(), fixture_file("valid/LICENSE"));
        f
    });
    let inv = sync::inventory(&gh, &cfg, "pyrlyn").unwrap();
    let get = |r: &str| inv.iter().find(|(n, _, _)| n == r).unwrap();
    assert!(get("pyrlyn/unlisted").1.gpl && !get("pyrlyn/unlisted").2);
    assert!(!get("pyrlyn/mit").1.gpl && get("pyrlyn/mit").2);
}

#[test]
fn a_null_kind_and_readme_are_left_out() {
    let (gh, cfg) = setup();
    let out = sync::sync_all(&gh, &cfg, &["pyrlyn/gpl-only".into()], "", false).unwrap();
    let Action::Updated { committed, .. } = action(&out, "pyrlyn/gpl-only") else {
        panic!("{out:?}")
    };
    assert_eq!(
        committed,
        &["LICENSE"],
        "no commercial file, no README block"
    );
    let b = "chore/license-sync";
    assert!(gh.file("pyrlyn/gpl-only", b, "PRICING.md").is_none());
    assert_eq!(
        gh.file("pyrlyn/gpl-only", b, "README.md").unwrap(),
        "# gpl-only\n"
    );
}
