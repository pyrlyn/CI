//! The license-sync block of a README, between the start and end markers.

use std::path::Path;

pub const START: &str = "<!-- license-sync:start -->";
pub const END: &str = "<!-- license-sync:end -->";

/// The block for a README at `readme`, with `{kind}` replaced by each file's path relative to
/// the README's directory.
pub fn block(snippet: &str, readme: &str, files: &[(String, String)]) -> String {
    let dir = Path::new(readme)
        .parent()
        .map(|p| p.to_string_lossy().into_owned());
    let mut text = snippet.trim().to_owned();
    for (kind, path) in files {
        text = text.replace(
            &format!("{{{kind}}}"),
            &relative(dir.as_deref().unwrap_or(""), path),
        );
    }
    format!("{START}\n{text}\n{END}")
}

fn relative(dir: &str, path: &str) -> String {
    if dir.is_empty() {
        return path.to_owned();
    }
    let ups = dir.split('/').filter(|s| !s.is_empty()).count();
    format!("{}{path}", "../".repeat(ups))
}

/// `text` with `block` present exactly once: replaced in place between existing markers,
/// otherwise appended to the end of the License section (a new `## License` section when
/// there is none).
pub fn with_block(text: &str, block: &str) -> String {
    if let (Some(s), Some(e)) = (text.find(START), text.find(END)) {
        if s < e {
            return format!("{}{block}{}", &text[..s], &text[e + END.len()..]);
        }
    }
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let heading = |l: &str| -> Option<usize> {
        let hashes = l.chars().take_while(|c| *c == '#').count();
        if !(1..=6).contains(&hashes) {
            return None;
        }
        let rest = l[hashes..].strip_prefix(|c: char| c == ' ' || c == '\t')?;
        let r = rest.trim_start().to_lowercase();
        (r.starts_with("license") || r.starts_with("licence")).then_some(hashes)
    };
    let Some(start) = lines.iter().position(|l| heading(l).is_some()) else {
        let body = text.trim_end_matches('\n');
        let sep = if body.is_empty() { "" } else { "\n\n" };
        return format!("{body}{sep}## License\n\n{block}\n");
    };
    let level = heading(lines[start]).unwrap_or(2);
    let end = (start + 1..lines.len())
        .find(|&i| {
            let h = lines[i].chars().take_while(|c| *c == '#').count();
            (1..=level).contains(&h) && lines[i][h..].starts_with([' ', '\t'])
        })
        .unwrap_or(lines.len());
    let before: String = lines[..start].concat();
    let section: String = lines[start..end].concat();
    let after: String = lines[end..].concat();
    let section = section.trim_end_matches('\n');
    let tail = if after.is_empty() {
        String::new()
    } else {
        format!("\n{after}")
    };
    format!("{before}{section}\n\n{block}\n{tail}")
}
