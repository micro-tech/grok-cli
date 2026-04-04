//! ACP tool shim — re-exports all tool implementations from [`crate::tools`].
//!
//! The actual implementations live in `crate::tools`, split across:
//!
//! * [`crate::tools::file_tools`] — `read_file`, `write_file`, `list_directory`, …
//! * [`crate::tools::shell_tools`] — `run_shell_command`
//! * [`crate::tools::web_tools`] — `web_search`, `web_fetch`
//! * [`crate::tools::memory_tools`] — `save_memory`
//! * [`crate::tools::registry`] — `execute_tool`, `get_tool_definitions`, `get_available_tool_definitions`
//!
//! This module exists so that call-sites in `crate::acp::mod` that use the
//! `tools::*` namespace (e.g. `tools::read_file(path, &policy)`) continue to
//! compile without modification.

// Re-export the full public surface of the top-level tools module.
pub use crate::tools::*;

// ── Tests ─────────────────────────────────────────────────────────────────────
// These tests exercise the tools through the ACP shim layer.  They call the
// same functions as the unit tests in each individual tool module, providing
// an extra integration-style check that the re-export chain is intact.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acp::security::SecurityPolicy;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let path_str = file_path.to_str().unwrap();

        let mut security = SecurityPolicy::new();
        security.add_trusted_directory(temp_dir.path());

        // Test write_file
        let write_result = write_file(path_str, "Hello, world!", &security);
        assert!(write_result.is_ok());

        // Test read_file
        let read_result = read_file(path_str, &security);
        assert!(read_result.is_ok());
        assert_eq!(read_result.unwrap(), "Hello, world!");

        // Test list_directory
        let list_result = list_directory(temp_dir.path().to_str().unwrap(), &security);
        assert!(list_result.is_ok());
        assert!(list_result.unwrap().contains("test.txt"));
    }

    #[test]
    fn test_read_multiple_files() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        fs::write(&file1, "Content 1").unwrap();
        fs::write(&file2, "Content 2").unwrap();

        let mut security = SecurityPolicy::new();
        security.add_trusted_directory(temp_dir.path());

        let paths = vec![
            file1.to_str().unwrap().to_string(),
            file2.to_str().unwrap().to_string(),
        ];

        let result = read_multiple_files(paths, &security);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("--- File:"));
        assert!(output.contains("Content 1"));
        assert!(output.contains("Content 2"));
    }

    #[test]
    fn test_list_code_definitions() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_code.rs");
        let code = r#"
            struct MyStruct {
                field: i32,
            }

            impl MyStruct {
                pub fn new() -> Self {
                    Self { field: 0 }
                }
            }

            fn main() {
                println!("Hello");
            }
        "#;
        fs::write(&file_path, code).unwrap();

        let mut security = SecurityPolicy::new();
        security.add_trusted_directory(temp_dir.path());

        let result = list_code_definitions(file_path.to_str().unwrap(), &security);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("struct MyStruct"));
        assert!(output.contains("impl MyStruct"));
        assert!(output.contains("pub fn new"));
        assert!(output.contains("fn main"));
        assert!(!output.contains("field: i32"));
        assert!(!output.contains("println!"));
    }

    #[test]
    fn test_glob_search() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.rs");
        fs::write(&file1, "content1").unwrap();
        fs::write(&file2, "content2").unwrap();

        let mut security = SecurityPolicy::new();
        security.add_trusted_directory(temp_dir.path());

        let pattern = temp_dir.path().join("*.txt");
        let result = glob_search(pattern.to_str().unwrap(), &security);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("file1.txt"));
    }

    #[test]
    fn test_search_content() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("test_grep.txt");
        fs::write(&file1, "hello world\nfoo bar\nhello rust").unwrap();

        let mut security = SecurityPolicy::new();
        security.add_trusted_directory(temp_dir.path());

        let result = search_file_content(file1.to_str().unwrap(), "hello", &security);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("1: hello world"));
        assert!(output.contains("3: hello rust"));
    }

    #[test]
    fn test_replace() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("replace.txt");
        let path_str = file_path.to_str().unwrap();

        let mut security = SecurityPolicy::new();
        security.add_trusted_directory(temp_dir.path());

        fs::write(&file_path, "Hello world, hello universe").unwrap();

        // Test successful replace
        let result = replace(path_str, "hello", "hi", None, &security);
        assert!(result.is_ok());
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Hello world, hi universe");

        // Test replace with expected count
        let result = replace(path_str, "universe", "cosmos", Some(1), &security);
        assert!(result.is_ok());
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Hello world, hi cosmos");

        // Test replace not found
        let result = replace(path_str, "missing", "nothing", None, &security);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));

        // Test replace count mismatch
        let result = replace(path_str, "hi", "hey", Some(5), &security);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Expected 5 occurrences")
        );
    }

    #[test]
    fn test_get_tool_definitions_updated() {
        let tools = get_tool_definitions();
        assert!(tools.iter().any(|t| t["function"]["name"] == "replace"));
        assert!(tools.iter().any(|t| t["function"]["name"] == "save_memory"));
        assert!(tools.iter().any(|t| t["function"]["name"] == "web_search"));
        assert!(tools.iter().any(|t| t["function"]["name"] == "web_fetch"));
    }

    #[tokio::test]
    #[serial]
    async fn test_web_search_works_without_keys() {
        unsafe {
            std::env::remove_var("GOOGLE_API_KEY");
            std::env::remove_var("GOOGLE_CX");
        }

        let result = web_search("test query").await;
        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(!error_msg.contains("GOOGLE_API_KEY"));
            assert!(!error_msg.contains("not set"));
        }
    }

    #[tokio::test]
    async fn test_web_fetch_invalid_url() {
        let result = web_fetch("not-a-valid-url").await;
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Failed to fetch") || error_msg.contains("invalid"));
    }

    #[tokio::test]
    async fn test_web_fetch_timeout() {
        let result = web_fetch("http://10.255.255.1").await;
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_is_web_search_configured() {
        assert!(is_web_search_configured());
    }

    #[test]
    #[serial]
    fn test_get_available_tool_definitions() {
        let api_key = std::env::var("GOOGLE_API_KEY").ok();
        let cx = std::env::var("GOOGLE_CX").ok();

        unsafe {
            std::env::remove_var("GOOGLE_API_KEY");
            std::env::remove_var("GOOGLE_CX");

            let tools = get_available_tool_definitions();
            let has_web_search = tools.iter().any(|t| {
                t.get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str())
                    == Some("web_search")
            });
            assert!(
                has_web_search,
                "web_search should be available even without configuration"
            );

            if let Some(key) = api_key {
                std::env::set_var("GOOGLE_API_KEY", key);
            }
            if let Some(cx_val) = cx {
                std::env::set_var("GOOGLE_CX", cx_val);
            }
        }
    }
}
