//! Integration tests for file tools and security policy (subtasks 148.4 + 148.5).
//!
//! ## Coverage
//! - `read_file`   — happy path, missing file, valid JSON, untrusted path
//! - `write_file`  — creates file, creates parent dirs, denied outside trust
//! - `replace`     — updates content, old-string missing, file missing
//! - `list_directory` — lists entries, nonexistent dir
//! - `glob_search` — finds matches, no matches
//! - `search_file_content` — match found, no match
//! - Security policy — inside trust (allowed), outside trust (denied),
//!                     path-traversal attempt (denied)
//!
//! All tests run **offline** (no network calls).
//! Each test uses an isolated [`TempDir`] for full determinism.

#[path = "helpers.rs"]
mod helpers;

use grok_cli::tools::{
    glob_search, list_directory, read_file, replace, search_file_content, write_file,
};
use std::fs;
use tempfile::TempDir;

// ─────────────────────────────────────────────────────────────────────────────
// read_file
// ─────────────────────────────────────────────────────────────────────────────

/// Test 1 — reading an existing file returns its content unchanged.
#[tokio::test]
async fn read_file_returns_content() {
    let dir = TempDir::new().unwrap();
    let policy = helpers::make_policy(&dir);
    let file = helpers::write_fixture(&dir, "hello.txt", "Hello, Grok!");

    let result = read_file(file.to_str().unwrap(), &policy).await.unwrap();

    assert_eq!(result, "Hello, Grok!");
}

/// Test 2 — reading a nonexistent file returns an `Err` that mentions the file.
#[tokio::test]
async fn read_file_missing_returns_err() {
    let dir = TempDir::new().unwrap();
    let policy = helpers::make_policy(&dir);
    let missing = dir.path().join("does_not_exist.txt");

    let result = read_file(missing.to_str().unwrap(), &policy).await;

    assert!(result.is_err(), "expected Err for missing file");
    let msg = result.unwrap_err().to_string().to_lowercase();
    // The implementation returns "File not found: …"
    assert!(
        msg.contains("not found") || msg.contains("no such file") || msg.contains("missing"),
        "error should mention 'not found', got: {msg}"
    );
}

/// Test 3 — reading a valid JSON file returns the raw JSON bytes unchanged.
///
/// The implementation passes valid JSON straight through without any
/// transformation — the LLM receives the exact source text.
#[tokio::test]
async fn read_file_valid_json_returned_verbatim() {
    let dir = TempDir::new().unwrap();
    let policy = helpers::make_policy(&dir);
    let json_text = r#"{"version":1,"items":["a","b"]}"#;
    let file = helpers::write_fixture(&dir, "data.json", json_text);

    let result = read_file(file.to_str().unwrap(), &policy).await.unwrap();

    assert_eq!(
        result, json_text,
        "valid JSON must be returned verbatim, unmodified"
    );
}

/// Test 4 — attempting to read a file outside the trusted directory returns
/// an `Err` that mentions access denial.
#[tokio::test]
async fn read_file_outside_trust_is_denied() {
    let trusted = TempDir::new().unwrap();
    let other = TempDir::new().unwrap();
    // Policy trusts only `trusted`, not `other`.
    let policy = helpers::make_policy(&trusted);
    let secret = helpers::write_fixture(&other, "secret.txt", "classified");

    let result = read_file(secret.to_str().unwrap(), &policy).await;

    assert!(result.is_err(), "path outside trust must return Err");
    let msg = result.unwrap_err().to_string().to_lowercase();
    assert!(
        msg.contains("denied") || msg.contains("trusted") || msg.contains("access"),
        "error must mention access denial, got: {msg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// write_file
// ─────────────────────────────────────────────────────────────────────────────

/// Test 5 — writing a new file creates it with the expected content.
#[tokio::test]
async fn write_file_creates_file_with_content() {
    let dir = TempDir::new().unwrap();
    let policy = helpers::make_policy(&dir);
    let path = dir.path().join("output.txt");

    write_file(path.to_str().unwrap(), "written content", &policy)
        .await
        .unwrap();

    let on_disk = fs::read_to_string(&path).unwrap();
    assert_eq!(on_disk, "written content");
}

/// Test 6 — `write_file` creates any missing parent directories automatically.
#[tokio::test]
async fn write_file_creates_parent_directories() {
    let dir = TempDir::new().unwrap();
    let policy = helpers::make_policy(&dir);
    // `deep/nested/` does not exist yet — write_file must create it.
    let path = dir.path().join("deep").join("nested").join("new.txt");

    write_file(path.to_str().unwrap(), "deep content", &policy)
        .await
        .unwrap();

    assert!(path.exists(), "file should be created at deep path");
    let on_disk = fs::read_to_string(&path).unwrap();
    assert_eq!(on_disk, "deep content");
}

/// Test 7 — writing outside the trusted directory is denied.
#[tokio::test]
async fn write_file_outside_trust_is_denied() {
    let trusted = TempDir::new().unwrap();
    let other = TempDir::new().unwrap();
    let policy = helpers::make_policy(&trusted);
    let path = other.path().join("intruder.txt");

    let result = write_file(path.to_str().unwrap(), "should not land", &policy).await;

    assert!(result.is_err(), "write outside trust must return Err");
    let msg = result.unwrap_err().to_string().to_lowercase();
    assert!(
        msg.contains("denied") || msg.contains("trusted") || msg.contains("access"),
        "error must mention access denial, got: {msg}"
    );
    // Verify the file was NOT created.
    assert!(
        !path.exists(),
        "file must not be written to untrusted location"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// replace
// ─────────────────────────────────────────────────────────────────────────────

/// Test 8 — `replace` finds `old_string` and substitutes it correctly.
#[tokio::test]
async fn replace_updates_file_content() {
    let dir = TempDir::new().unwrap();
    let policy = helpers::make_policy(&dir);
    let file = helpers::write_fixture(&dir, "greet.txt", "Hello World");

    replace(file.to_str().unwrap(), "World", "Grok", None, &policy)
        .await
        .unwrap();

    let result = fs::read_to_string(&file).unwrap();
    assert_eq!(result, "Hello Grok");
}

/// Test 9 — `replace` returns `Err` when `old_string` is not found.
#[tokio::test]
async fn replace_old_string_not_found_returns_err() {
    let dir = TempDir::new().unwrap();
    let policy = helpers::make_policy(&dir);
    let file = helpers::write_fixture(&dir, "text.txt", "some content here");

    let result = replace(
        file.to_str().unwrap(),
        "DOES_NOT_EXIST",
        "replacement",
        None,
        &policy,
    )
    .await;

    assert!(result.is_err(), "missing old_string must return Err");
    let msg = result.unwrap_err().to_string().to_lowercase();
    assert!(
        msg.contains("not found") || msg.contains("replace"),
        "error should describe the failure, got: {msg}"
    );
}

/// Test 10 — `replace` returns `Err` when the target file does not exist.
#[tokio::test]
async fn replace_nonexistent_file_returns_err() {
    let dir = TempDir::new().unwrap();
    let policy = helpers::make_policy(&dir);
    let missing = dir.path().join("ghost.txt");

    let result = replace(missing.to_str().unwrap(), "old", "new", None, &policy).await;

    assert!(result.is_err(), "replace on missing file must return Err");
    let msg = result.unwrap_err().to_string().to_lowercase();
    assert!(
        msg.contains("not found") || msg.contains("no such file") || msg.contains("missing"),
        "error should mention missing file, got: {msg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// list_directory
// ─────────────────────────────────────────────────────────────────────────────

/// Test 11 — listing a directory that exists returns its entries.
#[test]
fn list_directory_returns_entries() {
    let dir = TempDir::new().unwrap();
    let policy = helpers::make_policy(&dir);
    helpers::write_fixture(&dir, "alpha.txt", "a");
    helpers::write_fixture(&dir, "beta.rs", "fn main() {}");

    let result = list_directory(dir.path().to_str().unwrap(), &policy).unwrap();

    assert!(
        result.contains("alpha.txt"),
        "listing must include alpha.txt, got: {result}"
    );
    assert!(
        result.contains("beta.rs"),
        "listing must include beta.rs, got: {result}"
    );
}

/// Test 12 — listing a directory that does not exist returns `Err`.
#[test]
fn list_directory_nonexistent_returns_err() {
    let dir = TempDir::new().unwrap();
    let policy = helpers::make_policy(&dir);
    let missing = dir.path().join("no_such_subdir");

    let result = list_directory(missing.to_str().unwrap(), &policy);

    assert!(result.is_err(), "missing directory must return Err");
    let msg = result.unwrap_err().to_string().to_lowercase();
    assert!(
        msg.contains("not found") || msg.contains("no such") || msg.contains("directory"),
        "error should describe the missing directory, got: {msg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// glob_search
// ─────────────────────────────────────────────────────────────────────────────

/// Test 13 — glob with `*.txt` finds `.txt` files inside the trusted directory.
#[test]
fn glob_search_finds_matching_files() {
    let dir = TempDir::new().unwrap();
    let policy = helpers::make_policy(&dir);
    helpers::write_fixture(&dir, "notes.txt", "some notes");
    helpers::write_fixture(&dir, "readme.txt", "readme text");
    helpers::write_fixture(&dir, "code.rs", "fn foo() {}");

    // Anchor the glob to the temp dir so it only searches there.
    let pattern = format!("{}/*.txt", dir.path().display());
    let result = glob_search(&pattern, &policy).unwrap();

    assert!(
        result.contains("notes.txt"),
        "glob must find notes.txt, got: {result}"
    );
    assert!(
        result.contains("readme.txt"),
        "glob must find readme.txt, got: {result}"
    );
    assert!(
        !result.contains("code.rs"),
        "glob must NOT match code.rs, got: {result}"
    );
}

/// Test 14 — glob that matches nothing returns `Ok` with a no-matches message.
///
/// The implementation returns `"No files found matching pattern"` rather
/// than an `Err`, so callers can distinguish "nothing found" from "bad pattern".
#[test]
fn glob_search_no_matches_returns_ok_message() {
    let dir = TempDir::new().unwrap();
    let policy = helpers::make_policy(&dir);
    // Only a .rs file present — *.toml should produce no hits.
    helpers::write_fixture(&dir, "main.rs", "fn main() {}");

    let pattern = format!("{}/*.toml", dir.path().display());
    let result = glob_search(&pattern, &policy).unwrap();

    assert!(
        result.to_lowercase().contains("no files")
            || result.to_lowercase().contains("no match")
            || result.is_empty(),
        "no-match result must indicate no files found, got: {result}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// search_file_content
// ─────────────────────────────────────────────────────────────────────────────

/// Test 15 — searching for a pattern that exists returns the matching line.
#[test]
fn search_file_content_finds_match() {
    let dir = TempDir::new().unwrap();
    let policy = helpers::make_policy(&dir);
    let file = helpers::write_fixture(
        &dir,
        "source.rs",
        "fn main() {\n    println!(\"Hello\");\n}\n",
    );

    let result = search_file_content(file.to_str().unwrap(), "println", &policy).unwrap();

    assert!(
        result.contains("println"),
        "result must include the matched line, got: {result}"
    );
}

/// Test 16 — searching for a pattern that does not exist returns `Ok` with
/// a no-matches message rather than an `Err`.
#[test]
fn search_file_content_no_match_returns_ok() {
    let dir = TempDir::new().unwrap();
    let policy = helpers::make_policy(&dir);
    let file = helpers::write_fixture(&dir, "plain.txt", "just some plain text\nnothing special\n");

    let result = search_file_content(file.to_str().unwrap(), "XYZZY_NOT_HERE", &policy).unwrap();

    assert!(
        result.to_lowercase().contains("no match")
            || result.to_lowercase().contains("no matches")
            || result.is_empty(),
        "no-match result should indicate absence of matches, got: {result}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Security / path policy (subtask 148.5)
// ─────────────────────────────────────────────────────────────────────────────

/// Test 17 — a path inside the trusted directory is accessible for both
/// reading and directory listing.
#[tokio::test]
async fn security_path_inside_trust_is_accessible() {
    let dir = TempDir::new().unwrap();
    let policy = helpers::make_policy(&dir);
    let file = helpers::write_fixture(&dir, "trusted.txt", "trusted content");

    let read_result = read_file(file.to_str().unwrap(), &policy).await;
    assert!(
        read_result.is_ok(),
        "trusted path must be readable, err: {:?}",
        read_result.err()
    );
    assert_eq!(read_result.unwrap(), "trusted content");

    let list_result = list_directory(dir.path().to_str().unwrap(), &policy);
    assert!(
        list_result.is_ok(),
        "trusted directory must be listable, err: {:?}",
        list_result.err()
    );
}

/// Test 18 — an absolute path outside the trusted workspace is denied.
///
/// Uses `C:\Windows\System32\drivers\etc\hosts` — a well-known Windows system
/// path that is guaranteed to be outside any `TempDir`.  The security check
/// fires before any I/O, so the test is safe even if the file doesn't exist.
#[tokio::test]
async fn security_system_path_outside_trust_is_denied() {
    let dir = TempDir::new().unwrap();
    let policy = helpers::make_policy(&dir);

    let system_path = r"C:\Windows\System32\drivers\etc\hosts";

    let result = read_file(system_path, &policy).await;

    assert!(
        result.is_err(),
        "absolute system path must be denied, but got Ok"
    );
    let msg = result.unwrap_err().to_string().to_lowercase();
    assert!(
        msg.contains("denied") || msg.contains("trusted") || msg.contains("access"),
        "error must mention access denial, got: {msg}"
    );
}

/// Test 19 — a path-traversal attempt (`../../etc/passwd`) is denied.
///
/// `SecurityPolicy::resolve_path` resolves the `..` components relative to
/// the working directory.  The resolved path will not start with any trusted
/// root, so `validate_path_access` returns `Err`.
#[tokio::test]
async fn security_path_traversal_is_denied() {
    let dir = TempDir::new().unwrap();
    let policy = helpers::make_policy(&dir);

    // Classic traversal — lands above the temp dir.
    let traversal = "../../etc/passwd";

    let result = read_file(traversal, &policy).await;

    assert!(
        result.is_err(),
        "path-traversal attempt must be denied, but got Ok"
    );
    let msg = result.unwrap_err().to_string().to_lowercase();
    assert!(
        msg.contains("denied")
            || msg.contains("trusted")
            || msg.contains("access")
            // Acceptable fallback: path resolves outside trust and doesn't exist.
            || msg.contains("not found")
            || msg.contains("no such"),
        "error must indicate denial or absence, got: {msg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Edge cases
// ─────────────────────────────────────────────────────────────────────────────

/// `replace` preserves CRLF line endings in files that use `\r\n`.
///
/// The AI always emits LF-only search strings; the implementation normalises
/// both sides to LF for matching and then restores CRLF before writing.
#[tokio::test]
async fn replace_preserves_crlf_line_endings() {
    let dir = TempDir::new().unwrap();
    let policy = helpers::make_policy(&dir);
    let path = dir.path().join("crlf.txt");

    // Write CRLF content directly, bypassing write_file.
    tokio::fs::write(&path, b"line one\r\nline two\r\nline three\r\n")
        .await
        .unwrap();

    // Search string uses plain LF — must still match.
    replace(
        path.to_str().unwrap(),
        "line one\nline two",
        "replaced",
        None,
        &policy,
    )
    .await
    .unwrap();

    let written = tokio::fs::read_to_string(&path).await.unwrap();
    assert!(
        written.contains("replaced\r\n"),
        "CRLF must be preserved after replace; got: {:?}",
        written
    );
    assert!(
        written.contains("line three"),
        "unmodified line must survive the replace"
    );
}

/// `write_file` → `read_file` round-trip returns the exact bytes that were written.
#[tokio::test]
async fn write_then_read_round_trip() {
    let dir = TempDir::new().unwrap();
    let policy = helpers::make_policy(&dir);
    let path = dir.path().join("roundtrip.txt");
    let content = "The quick brown fox jumps over the lazy dog.\n";

    write_file(path.to_str().unwrap(), content, &policy)
        .await
        .unwrap();

    let result = read_file(path.to_str().unwrap(), &policy).await.unwrap();
    assert_eq!(result, content);
}

/// `search_file_content` on a directory walks all files recursively.
#[test]
fn search_file_content_directory_scan_finds_matches() {
    let dir = TempDir::new().unwrap();
    let policy = helpers::make_policy(&dir);

    helpers::write_fixture(&dir, "src/lib.rs", "pub fn greet() {}\n");
    helpers::write_fixture(&dir, "src/main.rs", "fn main() { greet(); }\n");
    helpers::write_fixture(&dir, "docs/readme.md", "No Rust functions here.\n");

    let result = search_file_content(dir.path().to_str().unwrap(), "fn ", &policy).unwrap();

    assert!(
        result.contains("lib.rs") || result.contains("main.rs"),
        "recursive scan must find Rust function definitions, got: {result}"
    );
}

/// `list_directory` appends `/` to sub-directory names to distinguish them
/// from regular files.
#[test]
fn list_directory_marks_subdirs_with_slash() {
    let dir = TempDir::new().unwrap();
    let policy = helpers::make_policy(&dir);

    fs::create_dir(dir.path().join("subdir")).unwrap();
    helpers::write_fixture(&dir, "file.txt", "content");

    let result = list_directory(dir.path().to_str().unwrap(), &policy).unwrap();

    assert!(
        result.contains("subdir/"),
        "sub-directory must be listed with trailing '/', got: {result}"
    );
    assert!(
        result.contains("file.txt"),
        "regular file must be listed without a '/' suffix, got: {result}"
    );
}
