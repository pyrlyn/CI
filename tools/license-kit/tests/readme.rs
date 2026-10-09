//! (4) README injector: one block between the markers, idempotent, non-GPL skipped.

mod common;

use std::fs;
use std::process::Command;

use common::*;
use license_kit::readme::{END, START, block, with_block};

const SNIPPET: &str =
    "Commercial use requires a paid license — see [{commercial}]({commercial}).\n";

fn files() -> Vec<(String, String)> {
    vec![
        ("commercial".into(), "PRICING.md".into()),
        ("gpl".into(), "LICENSE".into()),
    ]
}

#[test]
fn block_links_the_commercial_file_relative_to_the_readme() {
    assert!(block(SNIPPET, "README.md", &files()).contains("[PRICING.md](PRICING.md)"));
    assert!(
        block(SNIPPET, "docs/en/README.md", &files())
            .contains("[../../PRICING.md](../../PRICING.md)")
    );
}

#[test]
fn inserts_once_and_is_idempotent_in_every_shape() {
    let b = block(SNIPPET, "README.md", &files());
    let shapes = [
        ("empty", ""),
        ("no section", "# Demo\n\nText.\n"),
        ("section last", "# Demo\n\n## License\n\nGPL.\n"),
        (
            "section then more",
            "# Demo\n\n## License\n\nGPL.\n\n## More\n\nx\n",
        ),
        (
            "stale markers",
            &format!("# Demo\n\n## License\n\n{START}\nold\n{END}\n\n## More\n"),
        ),
        ("licence spelling", "# Demo\n\n### Licence\n\nGPL.\n"),
    ];
    for (name, text) in shapes {
        let once = with_block(text, &b);
        let twice = with_block(&once, &b);
        assert_eq!(once, twice, "{name}: second run changed the README");
        assert_eq!(once.matches(START).count(), 1, "{name}");
        assert_eq!(once.matches(END).count(), 1, "{name}");
        assert!(once.contains(&b), "{name}");
        assert!(!once.contains("old"), "{name}: stale block replaced");
    }
    let more = with_block("# Demo\n\n## License\n\nGPL.\n\n## More\n\nx\n", &b);
    assert!(
        more.find(END).unwrap() < more.find("## More").unwrap(),
        "inside the License section"
    );
    let none = with_block("# Demo\n\nText.\n", &b);
    assert!(
        none.contains("## License\n\n<!-- license-sync:start -->"),
        "{none}"
    );
}

fn readme_line(dir: &std::path::Path, extra: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_license-kit"))
        .arg("readme-line")
        .args(extra)
        .arg("--config")
        .arg(real_config_path())
        .args(["--repo", "pyrlyn/cox"])
        .arg(dir)
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn cli_adds_the_line_once_for_gpl() {
    let dir = copy_fixture("valid");
    fs::write(dir.join("README.md"), "# x\n\n## License\n\nGPL.\n").unwrap();
    assert_eq!(readme_line(&dir, &["--check"]).0, 1);
    assert_eq!(readme_line(&dir, &[]).0, 0);
    let first = fs::read_to_string(dir.join("README.md")).unwrap();
    assert_eq!(readme_line(&dir, &[]).1.trim(), "README.md: up to date");
    assert_eq!(fs::read_to_string(dir.join("README.md")).unwrap(), first);
    assert_eq!(first.matches(START).count(), 1);
    assert!(first.contains("[PRICING.md](PRICING.md)"));
    assert_eq!(readme_line(&dir, &["--check"]).0, 0);
}

#[test]
fn cli_skips_non_gpl() {
    for repo in ["mit", "conflict"] {
        let dir = copy_fixture(&format!("repos/{repo}"));
        if repo == "conflict" {
            fs::copy(fixtures().join("valid/LICENSE"), dir.join("LICENSE")).unwrap();
        }
        let before = fs::read_to_string(dir.join("README.md")).unwrap();
        let (code, out) = readme_line(&dir, &[]);
        assert_eq!(code, 0);
        assert!(out.starts_with("skipped:"), "{repo}: {out}");
        assert_eq!(
            fs::read_to_string(dir.join("README.md")).unwrap(),
            before,
            "{repo} untouched"
        );
    }
}
