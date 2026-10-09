//! (6) automerge decision: Dependabot, or the `license` label on a license-sync branch; never
//! a draft, never anyone else.

use license_kit::automerge::{Decision, PrFacts, decide};

fn pr(author: &str, labels: &[&str], head: &str, draft: bool) -> PrFacts {
    PrFacts {
        author: author.into(),
        labels: labels.iter().map(|s| s.to_string()).collect(),
        head_branch: head.into(),
        draft,
    }
}

#[test]
fn merges_dependabot() {
    assert!(
        decide(&pr(
            "dependabot[bot]",
            &[],
            "dependabot/cargo/serde-1.0.200",
            false
        ))
        .is_merge()
    );
}

#[test]
fn merges_license_label_on_license_branches() {
    assert!(
        decide(&pr(
            "license-bot",
            &["license"],
            "chore/license-sync",
            false
        ))
        .is_merge()
    );
    assert!(
        decide(&pr(
            "listepo",
            &["license", "x"],
            "chore/license-update",
            false
        ))
        .is_merge()
    );
}

#[test]
fn prefixes_match_whole_segments() {
    assert!(decide(&pr("listepo", &["license"], "chore/license-sync/x", false)).is_merge());
    assert!(!decide(&pr("listepo", &["license"], "chore/licenses", false)).is_merge());
}

#[test]
fn refuses_drafts_even_when_otherwise_allowed() {
    for p in [
        pr("dependabot[bot]", &[], "dependabot/cargo/x", true),
        pr("license-bot", &["license"], "chore/license-sync", true),
    ] {
        assert_eq!(decide(&p), Decision::Refuse("draft pull request".into()));
    }
}

#[test]
fn refuses_everything_else() {
    for p in [
        pr("listepo", &[], "feat/x", false),
        pr("listepo", &["license"], "feat/x", false),
        pr("listepo", &[], "chore/license-sync", false),
        pr("dependabot", &[], "dependabot/cargo/x", false),
        pr("renovate[bot]", &["license"], "renovate/x", false),
        pr("listepo", &["License"], "chore/license-sync", false),
    ] {
        assert!(!decide(&p).is_merge(), "{p:?}");
    }
}

#[test]
fn cli_writes_the_decision() {
    let out_file = std::env::temp_dir().join(format!("license-kit-out-{}", std::process::id()));
    let _ = std::fs::remove_file(&out_file);
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_license-kit"))
        .args([
            "automerge-decide",
            "--author",
            "x",
            "--label",
            "license",
            "--head",
            "chore/license-sync",
        ])
        .env("GITHUB_OUTPUT", &out_file)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("merge:"));
    assert_eq!(
        std::fs::read_to_string(&out_file).unwrap(),
        "decision=merge\n"
    );
}
