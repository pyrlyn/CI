//! Detects whether a repository is GPL-licensed from its LICENSE file and manifests.
//!
//! A repository counts as GPL only when a license file reads as the GNU GPL (or GitHub
//! reports a GPL SPDX id) and no root manifest declares a license expression without GPL:
//! `LICENSE` saying GPL-3.0 while `Cargo.toml` says `MIT OR Apache-2.0` is a conflict and is
//! skipped for the maintainer to decide.

/// Files read to classify a repository.
pub const LICENSE_FILES: &[&str] = &["LICENSE", "LICENSE.md", "LICENSE.txt", "COPYING"];
pub const MANIFESTS: &[&str] = &["Cargo.toml", "package.json"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Classification {
    pub gpl: bool,
    /// Detected license, e.g. `GPL-3.0`, `MIT`, `MIT OR Apache-2.0`, `none`.
    pub detected: String,
    pub reason: String,
}

/// License named by a license file's text.
pub fn license_of_text(text: &str) -> Option<&'static str> {
    let head: String = text.chars().take(600).collect::<String>().to_lowercase();
    if head.contains("gnu affero general public license") {
        Some("AGPL-3.0")
    } else if head.contains("gnu lesser general public license") {
        Some("LGPL")
    } else if head.contains("gnu general public license") {
        if head.contains("version 3") {
            Some("GPL-3.0")
        } else {
            Some("GPL")
        }
    } else if head.contains("apache license") {
        Some("Apache-2.0")
    } else if head.contains("mit license") || head.contains("permission is hereby granted") {
        Some("MIT")
    } else {
        None
    }
}

/// License expression of a root manifest, if it declares one.
pub fn manifest_license(name: &str, text: &str) -> Option<String> {
    match name {
        "Cargo.toml" => {
            let v: toml::Value = toml::from_str(text).ok()?;
            let pick = |t: Option<&toml::Value>| {
                t.and_then(|t| t.get("license"))
                    .and_then(|l| l.as_str())
                    .map(str::to_owned)
            };
            pick(v.get("package"))
                .or_else(|| pick(v.get("workspace").and_then(|w| w.get("package"))))
        }
        "package.json" => {
            let v: serde_json::Value = serde_json::from_str(text).ok()?;
            v.get("license").and_then(|l| l.as_str()).map(str::to_owned)
        }
        _ => None,
    }
}

fn is_gpl(expr: &str) -> bool {
    let e = expr.to_uppercase();
    e.contains("GPL") && !e.contains("LGPL") && !e.contains("AGPL")
}

/// Classify a repository. `read` returns a root file's text; `github_spdx` is GitHub's
/// detected SPDX id (`None` or `NOASSERTION` when GitHub could not tell, e.g. dual licenses).
pub fn classify(
    read: &dyn Fn(&str) -> Option<String>,
    github_spdx: Option<&str>,
) -> Classification {
    let file = LICENSE_FILES
        .iter()
        .find_map(|f| read(f).map(|t| (*f, license_of_text(&t))));
    let spdx = github_spdx.filter(|s| !s.is_empty() && *s != "NOASSERTION" && *s != "NONE");
    let file_license = file
        .and_then(|(_, l)| l)
        .map(str::to_owned)
        .or(spdx.map(str::to_owned));
    let manifests: Vec<(String, String)> = MANIFESTS
        .iter()
        .filter_map(|m| {
            read(m)
                .and_then(|t| manifest_license(m, &t))
                .map(|l| (m.to_string(), l))
        })
        .collect();

    if let Some((m, l)) = manifests.iter().find(|(_, l)| !is_gpl(l)) {
        let detected = l.clone();
        let reason = match &file_license {
            Some(f) if is_gpl(f) => format!("conflict: license file is {f} but {m} declares `{l}`"),
            _ => format!("{m} declares `{l}`"),
        };
        return Classification {
            gpl: false,
            detected,
            reason,
        };
    }
    match file_license {
        Some(f) if is_gpl(&f) => {
            let detected = manifests
                .first()
                .map(|(_, l)| l.clone())
                .unwrap_or(f.clone());
            Classification {
                gpl: true,
                detected,
                reason: format!("license file is {f}"),
            }
        }
        Some(f) => Classification {
            gpl: false,
            reason: format!("license file is {f}"),
            detected: f,
        },
        None => match manifests.first() {
            Some((m, l)) => Classification {
                gpl: true,
                detected: l.clone(),
                reason: format!("{m} declares `{l}` (no license file)"),
            },
            None => Classification {
                gpl: false,
                detected: "none".into(),
                reason: "no license file or manifest license".into(),
            },
        },
    }
}
