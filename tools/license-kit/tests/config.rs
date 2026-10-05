//! (2) config: the sync configuration and the workflows point at canonical files that exist.

mod common;

use std::fs;

use common::*;
use serde_yaml::Value;

#[test]
fn canonical_files_exist() {
    let cfg = real_config();
    for (kind, file) in &cfg.canonical {
        let p = cfg.canonical_path(file);
        assert!(
            p.is_file(),
            "canonical `{kind}` -> {} is missing",
            p.display()
        );
        assert!(
            cfg.canonical_text(kind).unwrap().len() > 100,
            "canonical `{kind}` is empty"
        );
    }
    assert!(cfg.canonical.contains_key("gpl") && cfg.canonical.contains_key("commercial"));
    let snippet = cfg.snippet().unwrap().expect("readme-snippet configured");
    assert!(
        snippet.contains("{commercial}"),
        "the README line links the commercial file"
    );
}

#[test]
fn gpl_canonical_is_the_fsf_text_and_matches_infra_root() {
    let root = infra_root();
    let canon = fs::read_to_string(root.join("licenses/LICENSE")).unwrap();
    assert!(canon.starts_with("                    GNU GENERAL PUBLIC LICENSE\n                       Version 3, 29 June 2007\n"));
    assert!(
        canon
            .trim_end()
            .ends_with("<https://www.gnu.org/licenses/why-not-lgpl.html>.")
    );
    assert_eq!(
        canon,
        fs::read_to_string(root.join("LICENSE")).unwrap(),
        "infra's own LICENSE"
    );
}

#[test]
fn targets_are_pyrlyn_and_map_every_kind() {
    let cfg = real_config();
    assert!(!cfg.targets.is_empty());
    for t in &cfg.targets {
        assert!(
            t.repo.starts_with("pyrlyn/"),
            "{} is not a pyrlyn repository",
            t.repo
        );
        assert_ne!(t.repo, "pyrlyn/ci", "infra is the source, not a target");
        for kind in cfg.canonical.keys() {
            assert!(
                t.files.contains_key(kind),
                "{} does not map `{kind}`",
                t.repo
            );
        }
    }
}

fn workflow(name: &str) -> Option<Value> {
    let p = infra_root().join(".github/workflows").join(name);
    let text = fs::read_to_string(p).ok()?;
    Some(serde_yaml::from_str(&text).unwrap_or_else(|e| panic!("{name}: {e}")))
}

/// Every `licenses/<file>` and `tools/license-kit` path a workflow mentions exists.
fn referenced_paths_exist(name: &str, text: &str) {
    let root = infra_root();
    for (i, _) in text.match_indices("licenses/") {
        let start = text[..i]
            .rfind(|c: char| c.is_whitespace() || c == '"' || c == '\'')
            .map_or(0, |s| s + 1);
        let tok: String = text[start..]
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != '"' && *c != '\'')
            .collect();
        let rel = tok
            .trim_start_matches("infra/")
            .trim_end_matches(['`', ')', ',', ':', ';', '.']);
        if !rel.starts_with("licenses/")
            || rel.ends_with(".json")
            || rel.contains('*')
            || rel.contains('$')
        {
            continue;
        }
        assert!(
            root.join(rel).exists(),
            "{name} references {rel}, which does not exist"
        );
    }
}

#[test]
fn license_check_workflow_reads_infra_canonical_files() {
    let wf = workflow("license-check.yml").expect("license-check.yml");
    let text =
        fs::read_to_string(infra_root().join(".github/workflows/license-check.yml")).unwrap();
    let call = &wf["on"]["workflow_call"]["inputs"];
    assert_eq!(call["canonical-ref"]["default"], Value::from("main"));
    assert_eq!(
        call["check-headers"]["default"],
        Value::from(false),
        "header check is opt-in"
    );
    let steps = wf["jobs"]["check"]["steps"]
        .as_sequence()
        .expect("check steps");
    let infra = steps
        .iter()
        .find(|s| s["with"]["repository"].as_str() == Some("pyrlyn/ci"))
        .expect("a checkout of pyrlyn/ci");
    let sparse = infra["with"]["sparse-checkout"]
        .as_str()
        .unwrap_or_default();
    assert!(
        sparse.lines().any(|l| l.trim() == "licenses"),
        "sparse-checkout has licenses"
    );
    assert!(
        sparse.lines().any(|l| l.trim() == "tools/license-kit"),
        "sparse-checkout has the tool"
    );
    assert!(
        text.contains("licenses/targets.yml"),
        "uses licenses/targets.yml"
    );
    assert!(
        text.contains("tools/license-kit/Cargo.toml"),
        "builds tools/license-kit"
    );
    referenced_paths_exist("license-check.yml", &text);
}

#[test]
fn every_workflow_reference_to_licenses_exists() {
    let dir = infra_root().join(".github/workflows");
    for e in fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        let text = fs::read_to_string(&p).unwrap();
        if text.contains("licenses/") {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            serde_yaml::from_str::<Value>(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
            referenced_paths_exist(&name, &text);
        }
    }
}

#[test]
fn caller_example_points_at_the_reusable_workflow() {
    let doc =
        fs::read_to_string(infra_root().join("licenses/README.md")).expect("licenses/README.md");
    let uses = doc
        .lines()
        .find_map(|l| l.trim().strip_prefix("uses: "))
        .expect("a caller example with `uses:`");
    let path = uses
        .strip_prefix("pyrlyn/ci/")
        .expect("calls pyrlyn/ci")
        .split('@')
        .next()
        .unwrap();
    assert_eq!(path, ".github/workflows/license-check.yml");
    assert!(infra_root().join(path).is_file());
}
