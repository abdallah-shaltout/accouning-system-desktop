//! Phase C (C-3): a grep-based static check, in the same style as `architecture_rules.rs`, that
//! keeps C-2 true going forward — every test function body in `tests/*.rs` that calls
//! `TestDb::fresh()` must also call `.finish(` or `.finish_expecting(` before it ends, so a test
//! can never pass while leaving broken books behind. A separate file (not appended to
//! `architecture_rules.rs`) so it doesn't collide with other lanes editing that file, per the plan.
//!
//! This scans `tests/*.rs` itself (not `src/`), so it needs its own file-walking helpers rather
//! than reusing `architecture_rules.rs`'s (which are scoped to `CARGO_MANIFEST_DIR/src`).

use std::fs;
use std::path::{Path, PathBuf};

fn tests_root() -> PathBuf {
    // `tests/` runs with CARGO_MANIFEST_DIR = src-tauri/.
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}

/// Every `.rs` file directly under `tests/` (not `tests/support/`, `tests/all/`, or any other
/// subdirectory — those are harness/helper code, not test suites with `#[tokio::test]` functions).
fn top_level_test_files(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else { return vec![] };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    out.sort();
    out
}

/// One test function's extracted source (attributes above `fn` are not included — only the
/// `fn name(...) { ... }` body span, brace-matched).
struct TestFn {
    name: String,
    body: String,
}

/// Splits a file's source into its top-level `#[tokio::test]` function bodies. A plain brace
/// counter is enough here: test files don't contain raw strings with unbalanced braces in a way
/// that would confuse this (the same assumption `architecture_rules.rs` makes about `.rs` sources
/// in this repo).
fn extract_tokio_test_fns(source: &str) -> Vec<TestFn> {
    let lines: Vec<&str> = source.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim_start().starts_with("#[tokio::test") {
            // Skip forward over any further attributes (e.g. a second derive/attribute line)
            // until the `fn` line.
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim_start().starts_with('#') {
                j += 1;
            }
            if j >= lines.len() || !lines[j].trim_start().starts_with("async fn") && !lines[j].trim_start().starts_with("fn") {
                i += 1;
                continue;
            }
            let fn_line = lines[j].trim_start();
            let name = fn_line
                .trim_start_matches("pub ")
                .trim_start_matches("async ")
                .trim_start_matches("fn ")
                .split(['(', '<'])
                .next()
                .unwrap_or("<unknown>")
                .trim()
                .to_string();

            // Brace-match from the fn line to the end of the function body.
            let mut depth: i32 = 0;
            let mut started = false;
            let mut body_lines = Vec::new();
            let mut k = j;
            while k < lines.len() {
                let line = lines[k];
                depth += line.matches('{').count() as i32;
                depth -= line.matches('}').count() as i32;
                if line.contains('{') {
                    started = true;
                }
                body_lines.push(line);
                k += 1;
                if started && depth <= 0 {
                    break;
                }
            }
            out.push(TestFn { name, body: body_lines.join("\n") });
            i = k;
        } else {
            i += 1;
        }
    }
    out
}

#[derive(Debug)]
struct Violation {
    file: PathBuf,
    test_name: String,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{} calls TestDb::fresh() but never calls .finish( or .finish_expecting(", self.file.display(), self.test_name)
    }
}

fn check_every_fresh_test_calls_finish(root: &Path) -> Vec<Violation> {
    let mut violations = Vec::new();
    for file in top_level_test_files(root) {
        let source = fs::read_to_string(&file).unwrap_or_default();
        for test_fn in extract_tokio_test_fns(&source) {
            if !test_fn.body.contains("TestDb::fresh()") {
                continue;
            }
            let calls_finish = test_fn.body.contains(".finish(") || test_fn.body.contains(".finish_expecting(");
            if !calls_finish {
                violations.push(Violation { file: file.clone(), test_name: test_fn.name });
            }
        }
    }
    violations
}

#[test]
fn every_test_using_test_db_fresh_calls_finish() {
    let root = tests_root();
    let violations = check_every_fresh_test_calls_finish(&root);
    assert!(
        violations.is_empty(),
        "every #[tokio::test] that calls TestDb::fresh() must end with test_db.finish().await (or \
         finish_expecting(...)) so the Rust invariants run on every test database (phase-c C-1..C-3):\n{}",
        violations.iter().map(|v| v.to_string()).collect::<Vec<_>>().join("\n")
    );
}
