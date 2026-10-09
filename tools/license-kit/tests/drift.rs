//! (1) drift-check: the valid copy passes; broken, missing, CRLF and final-newline copies fail.

mod common;

use std::fs;
use std::process::Command;

use common::*;
use license_kit::drift::{self, Status};

fn statuses(dir: &std::path::Path) -> Vec<(String, Status)> {
    let cfg = real_config();
    let read = |p: &str| fs::read_to_string(dir.join(p)).ok();
    drift::desired(&cfg, &cfg.target("pyrlyn/cox"), &read)
        .unwrap()
        .into_iter()
        .map(|d| (d.path.clone(), d.status()))
        .collect()
}

fn cli(dir: &std::path::Path) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_license-kit"))
        .args(["drift-check", "--config"])
        .arg(real_config_path())
        .args(["--repo", "pyrlyn/cox"])
        .arg(dir)
        .env_remove("GITHUB_STEP_SUMMARY")
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn valid_fixture_is_identical_to_canonical() {
    let root = infra_root();
    for f in ["LICENSE", "PRICING.md"] {
        assert_eq!(
            fs::read(fixtures().join("valid").join(f)).unwrap(),
            fs::read(root.join("licenses").join(f)).unwrap(),
            "fixtures/valid/{f} must equal licenses/{f}"
        );
    }
}

#[test]
fn valid_passes() {
    assert!(
        statuses(&fixtures().join("valid"))
            .iter()
            .all(|(_, s)| *s == Status::Match)
    );
    let (code, _) = cli(&fixtures().join("valid"));
    assert_eq!(code, 0);
}

#[test]
fn broken_fails_with_a_diff() {
    let s = statuses(&fixtures().join("broken"));
    assert!(s.contains(&("LICENSE".into(), Status::Content)));
    assert!(s.contains(&("PRICING.md".into(), Status::Content)));
    assert!(s.contains(&("README.md".into(), Status::Match)));
    let (code, out) = cli(&fixtures().join("broken"));
    assert_eq!(code, 1, "drift exits 1, not an error (2)");
    assert!(
        out.contains("-                       Version 4, 29 June 2007"),
        "{out}"
    );
    assert!(
        out.contains("+                       Version 3, 29 June 2007"),
        "{out}"
    );
    assert!(out.contains("::error file=LICENSE::"));
}

#[test]
fn missing_file_fails() {
    let dir = copy_fixture("valid");
    fs::remove_file(dir.join("PRICING.md")).unwrap();
    assert!(statuses(&dir).contains(&("PRICING.md".into(), Status::Missing)));
    assert_eq!(cli(&dir).0, 1);
}

#[test]
fn missing_readme_block_fails() {
    let dir = copy_fixture("valid");
    fs::write(dir.join("README.md"), "# x\n\n## License\n\nGPL\n").unwrap();
    assert!(statuses(&dir).contains(&("README.md".into(), Status::Content)));
    assert_eq!(cli(&dir).0, 1);
}

#[test]
fn crlf_is_drift() {
    let dir = copy_fixture("valid");
    let text = fs::read_to_string(dir.join("LICENSE"))
        .unwrap()
        .replace('\n', "\r\n");
    fs::write(dir.join("LICENSE"), text).unwrap();
    assert!(statuses(&dir).contains(&("LICENSE".into(), Status::Crlf)));
    assert_eq!(cli(&dir).0, 1);
}

#[test]
fn final_newline_is_drift_both_ways() {
    for (name, edit) in [("stripped", 0), ("doubled", 1)] {
        let dir = copy_fixture("valid");
        let text = fs::read_to_string(dir.join("PRICING.md")).unwrap();
        let text = if edit == 0 {
            text.trim_end().to_owned()
        } else {
            format!("{text}\n")
        };
        fs::write(dir.join("PRICING.md"), text).unwrap();
        assert!(
            statuses(&dir).contains(&("PRICING.md".into(), Status::FinalNewline)),
            "{name}"
        );
        assert_eq!(cli(&dir).0, 1, "{name}");
    }
}

#[test]
fn header_check_is_off_by_default_and_flags_missing_headers() {
    let dir = copy_fixture("valid");
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("src/a.rs"),
        "// SPDX-License-Identifier: GPL-3.0-or-later\nfn a() {}\n",
    )
    .unwrap();
    fs::write(dir.join("src/b.rs"), "fn b() {}\n").unwrap();
    fs::create_dir_all(dir.join("tests/fixtures")).unwrap();
    fs::write(dir.join("tests/fixtures/c.rs"), "fn c() {}\n").unwrap();
    let git = |args: &[&str]| {
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(args)
                .status()
                .unwrap()
                .success()
        )
    };
    git(&["init", "-q"]);
    git(&["add", "-A"]);
    assert_eq!(cli(&dir).0, 0, "headers are not checked without --headers");
    let out = Command::new(env!("CARGO_BIN_EXE_license-kit"))
        .args(["drift-check", "--headers", "--config"])
        .arg(real_config_path())
        .args(["--repo", "pyrlyn/cox"])
        .arg(&dir)
        .env_remove("GITHUB_STEP_SUMMARY")
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text.contains("`src/b.rs`") && !text.contains("a.rs") && !text.contains("c.rs"),
        "{text}"
    );
}
