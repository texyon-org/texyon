//! Freezes the engine's direct wall-clock, timezone, and entropy surface.
//!
//! Every occurrence in `ALLOWED` is deterministic-mode-gated at its call
//! sites or consumes the configured build date. New sites must be classified
//! and contained before this allowlist is deliberately updated.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const PATTERNS: &[&str] = &[
    "gettimeofday",
    "localtime",
    "gmtime",
    "clock_gettime",
    "SystemTime::now",
    "Instant::now",
    "getrandom",
    "arc4random",
    "srand",
];

/// `(relative path, pattern, expected count)` in comment-stripped source.
const ALLOWED: &[(&str, &str, usize)] = &[
    // get_seconds_and_micros; every caller gates on deterministic_mode.
    ("src/texmfmp.rs", "gettimeofday", 1),
    // get_date_and_time's live branch plus makepdftime's unused live branch.
    ("src/texmfmp.rs", "localtime", 2),
    // get_date_and_time's deterministic branch plus makepdftime's UTC work.
    ("src/texmfmp.rs", "gmtime", 3),
];

fn collect_text_hits(
    relative_path: &str,
    text: &str,
    hits: &mut BTreeMap<(String, &'static str), usize>,
) {
    for line in text.lines() {
        // These APIs do not legitimately occur in source string literals, so
        // conservatively stripping line comments is sufficient for this guard.
        let code = match line.find("//") {
            Some(index) => &line[..index],
            None => line,
        };
        for &pattern in PATTERNS {
            let count = code.matches(pattern).count();
            if count > 0 {
                *hits.entry((relative_path.to_owned(), pattern)).or_insert(0) += count;
            }
        }
    }
}

fn collect_tree_hits(
    root: &Path,
    directory: &Path,
    hits: &mut BTreeMap<(String, &'static str), usize>,
) {
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_tree_hits(root, &path, hits);
            continue;
        }
        if !matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("rs") | Some("c") | Some("cpp") | Some("h")
        ) {
            continue;
        }

        let relative_path = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        collect_text_hits(
            &relative_path,
            &fs::read_to_string(path).unwrap_or_default(),
            hits,
        );
    }
}

fn audit_problems(
    hits: &BTreeMap<(String, &'static str), usize>,
    allowed: &BTreeMap<(String, &'static str), usize>,
) -> Vec<String> {
    let mut problems = Vec::new();
    for (key, &actual) in hits {
        match allowed.get(key) {
            Some(&expected) if expected == actual => {}
            Some(&expected) => problems.push(format!(
                "{}: `{}` found {actual} times, allowlist says {expected}",
                key.0, key.1
            )),
            None => problems.push(format!(
                "{}: `{}` found {actual} times, not in allowlist",
                key.0, key.1
            )),
        }
    }
    for (key, &expected) in allowed {
        if !hits.contains_key(key) {
            problems.push(format!(
                "{}: `{}` expected {expected} times but absent",
                key.0, key.1
            ));
        }
    }
    problems
}

fn allowlist() -> BTreeMap<(String, &'static str), usize> {
    ALLOWED
        .iter()
        .map(|&(file, pattern, count)| ((file.to_owned(), pattern), count))
        .collect()
}

#[test]
fn clock_and_entropy_surface_is_frozen() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut hits = BTreeMap::new();
    collect_tree_hits(&root, &root.join("src"), &mut hits);
    collect_tree_hits(&root, &root.join("xetex"), &mut hits);

    let problems = audit_problems(&hits, &allowlist());
    assert!(
        problems.is_empty(),
        "hidden-input surface changed:\n  {}\n\
         classify and contain new sites, then update ALLOWED deliberately",
        problems.join("\n  ")
    );
}

#[test]
fn unexpected_hidden_input_is_rejected() {
    let mut hits = BTreeMap::new();
    collect_text_hits(
        "src/injected.rs",
        "fn hidden_input() { let _ = std::time::SystemTime::now(); }",
        &mut hits,
    );

    let problems = audit_problems(&hits, &BTreeMap::new());
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("SystemTime::now"));
    assert!(problems[0].contains("not in allowlist"));
}
