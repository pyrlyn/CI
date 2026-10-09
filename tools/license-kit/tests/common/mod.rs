//! Shared test helpers: paths, scratch directories, fixture loading.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use license_kit::config::Config;

pub fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The pyrlyn/ci checkout this crate lives in.
pub fn infra_root() -> PathBuf {
    crate_dir()
        .join("../..")
        .canonicalize()
        .expect("infra root")
}

pub fn fixtures() -> PathBuf {
    crate_dir().join("tests/fixtures")
}

pub fn real_config_path() -> PathBuf {
    infra_root().join("licenses/targets.yml")
}

pub fn real_config() -> Config {
    Config::load(&real_config_path()).expect("licenses/targets.yml")
}

/// A config over the real canonical files with the given targets YAML.
pub fn config_with_targets(targets_yaml: &str) -> Config {
    let base = infra_root().join("licenses");
    let text = format!(
        "canonical:\n  gpl: LICENSE\n  commercial: PRICING.md\nreadme-snippet: readme-snippet.md\n{targets_yaml}"
    );
    Config::parse(&text, base).expect("test config")
}

/// A fresh, empty scratch directory.
pub fn scratch(name: &str) -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "license-kit-{}-{}-{name}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Copy a fixture directory into a scratch directory.
pub fn copy_fixture(name: &str) -> PathBuf {
    let dst = scratch(name);
    for e in fs::read_dir(fixtures().join(name)).unwrap() {
        let e = e.unwrap();
        fs::copy(e.path(), dst.join(e.file_name())).unwrap();
    }
    dst
}

/// Files of a fixture directory as a path -> text map.
pub fn load_dir(dir: &Path) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    for e in fs::read_dir(dir).unwrap() {
        let e = e.unwrap();
        if e.path().is_file() {
            m.insert(
                e.file_name().to_string_lossy().into_owned(),
                fs::read_to_string(e.path()).unwrap(),
            );
        }
    }
    m
}

pub fn fixture_file(rel: &str) -> String {
    fs::read_to_string(fixtures().join(rel)).unwrap()
}
