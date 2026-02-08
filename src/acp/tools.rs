use crate::acp::security::SecurityPolicy;
use anyhow::{Result, anyhow};
use glob::glob;
use regex::Regex;
use serde_json::{Value, json};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::Command;

/// Read file content
pub fn read_file(path: &str, security: &SecurityPolicy) -> Result<String> {
    // Resolve path to absolute canonical form
    let resolved_path = security
        .resolve_path(path)
        .map_err(|e| anyhow!("Failed to resolve path '{}': {}", path, e))?;

    // Check trust on resolved path
    if !security.is_path_trusted(&resolved_path) {
        return Err(anyhow!("Access denied: Path is not in a trusted directory"));
    }

    if !resolved_path.exists() {
        return Err(anyhow!("File not found: {}", resolved_path.display()));
    }

    fs::read_to_string(&resolved_path).map_err(|e| anyhow!("Failed to read file: {}", e))
}

/// Write content to file
pub fn write_file(path: &str, content: &str, security: &SecurityPolicy) -> Result<String> {
    // Convert to absolute path first (without canonicalizing yet)
    let path_ref = Path::new(path);
    let absolute_path = if path_ref.is_absolute() {
        path_ref.to_path_buf()
    } else {
        security.working_directory().join(path_ref)
    };

    // Create parent directories first if they don't exist
    if let Some(parent) = absolute_path.parent() {
        fs::create_dir_all(parent).map_err(|e| anyhow!("Failed to create directory: {}", e))?;
    }

    // Now resolve to canonical form (after directories exist)
    let resolved_path = security
        .resolve_path(path)
        .map_err(|e| anyhow!("Failed to resolve path '{}': {}", path, e))?;

    // Check trust on resolved path
    if !security.is_path_trusted(&resolved_path) {
        return Err(anyhow!("Access denied: Path is not in a trusted directory"));
    }

    fs::write(&resolved_path, content).map_err(|e| anyhow!("Failed to write file: {}", e))?;
    Ok(format!("Successfully wrote to {}", resolved_path.display()))
}

/// Replace text in a file
pub fn replace(
    path: &str,
    old_string: &str,
    new_string: &str,
    expected_replacements: Option<u32>,
    security: &SecurityPolicy,
) -> Result<String> {
    // Resolve path to absolute canonical form
    let resolved_path = security
        .resolve_path(path)
        .map_err(|e| anyhow!("Failed to resolve path '{}': {}", path, e))?;

    // Check trust on resolved path
    if !security.is_path_trusted(&resolved_path) {
        return Err(anyhow!("Access denied: Path is not in a trusted directory"));
    }

    if !resolved_path.exists() {
        return Err(anyhow!("File not found: {}", resolved_path.display()));
    }

    let content =
        fs::read_to_string(&resolved_path).map_err(|e| anyhow!("Failed to read file: {}", e))?;

    let occurrences = content.matches(old_string).count();
    if occurrences == 0 {
        return Err(anyhow!(
            "Failed to replace: '{}' not found in file. Use read_file to verify content.",
            old_string
        ));
    }

    if let Some(expected) = expected_replacements
        && occurrences != expected as usize
    {
        return Err(anyhow!(
            "Failed to replace: Expected {} occurrences, but found {}.",
            expected,
            occurrences
        ));
    }

    let new_content = content.replace(old_string, new_string);
    fs::write(&resolved_path, new_content).map_err(|e| anyhow!("Failed to write file: {}", e))?;

    Ok(format!(
        "Successfully replaced {} occurrence(s) in {}",
        occurrences,
        resolved_path.display()
    ))
}

/// Save a fact to long-term memory
pub fn save_memory(fact: &str) -> Result<String> {
    let home_dir = dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
    let grok_dir = home_dir.join(".grok");

    if !grok_dir.exists() {
        fs::create_dir_all(&grok_dir)
            .map_err(|e| anyhow!("Failed to create .grok directory: {}", e))?;
    }

    let memory_file = grok_dir.join("memory.md");
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&memory_file)
        .map_err(|e| anyhow!("Failed to open memory file: {}", e))?;

    writeln!(file, "- {}", fact).map_err(|e| anyhow!("Failed to write to memory file: {}", e))?;

    Ok("Fact saved to memory.".to_string())
}

/// List directory contents
/// List files in a directory
pub fn list_directory(path: &str, security: &SecurityPolicy) -> Result<String> {
    // Resolve path to absolute canonical form
    let resolved_path = security
        .resolve_path(path)
        .map_err(|e| anyhow!("Failed to resolve path '{}': {}", path, e))?;

    // Check trust on resolved path
    if !security.is_path_trusted(&resolved_path) {
        return Err(anyhow!("Access denied: Path is not in a trusted directory"));
    }

    if !resolved_path.exists() {
        return Err(anyhow!("Directory not found: {}", resolved_path.display()));
    }

    if !resolved_path.is_dir() {
        return Err(anyhow!(
            "Path is not a directory: {}",
            resolved_path.display()
        ));
    }

    let mut entries = Vec::new();
    for entry in
        fs::read_dir(&resolved_path).map_err(|e| anyhow!("Failed to read directory: {}", e))?
    {
        let entry = entry?;
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let is_dir = path.is_dir();
        entries.push(format!("{}{}", name, if is_dir { "/" } else { "" }));
    }

    Ok(entries.join("\n"))
}

/// Find files using glob pattern
pub fn glob_search(pattern: &str, security: &SecurityPolicy) -> Result<String> {
    // Glob patterns might traverse anywhere, so we need to filter results
    // based on security policy.
    let mut matches = Vec::new();
    for entry in glob(pattern).map_err(|e| anyhow!("Failed to read glob pattern: {}", e))? {
        match entry {
            Ok(path) => {
                if security.is_path_trusted(&path) {
                    matches.push(path.display().to_string());
                }
            }
            Err(e) => return Err(anyhow!("Error matching glob: {}", e)),
        }
    }

    if matches.is_empty() {
        Ok("No files found matching pattern".to_string())
    } else {
        Ok(matches.join("\n"))
    }
}

/// Search file content using regex (grep-like)
pub fn search_file_content(path: &str, pattern: &str, security: &SecurityPolicy) -> Result<String> {
    // Resolve path to absolute canonical form
    let resolved_path = security
        .resolve_path(path)
        .map_err(|e| anyhow!("Failed to resolve path '{}': {}", path, e))?;

    // Check trust on resolved path
    if !security.is_path_trusted(&resolved_path) {
        return Err(anyhow!("Access denied: Path is not in a trusted directory"));
    }

    let re = Regex::new(pattern).map_err(|e| anyhow!("Invalid regex pattern: {}", e))?;

    if resolved_path.is_dir() {
        // Simple recursive search if directory
        let mut results = Vec::new();
        for entry in walkdir::WalkDir::new(&resolved_path) {
            let entry = entry.map_err(|e| anyhow!("Error walking directory: {}", e))?;
            if entry.file_type().is_file() {
                let entry_path = entry.path();
                if !security.is_path_trusted(entry_path) {
                    continue;
                }

                let file =
                    File::open(entry_path).map_err(|e| anyhow!("Failed to open file: {}", e))?;
                let reader = BufReader::new(file);

                for (i, line) in reader.lines().enumerate() {
                    match line {
                        Ok(line) => {
                            if re.is_match(&line) {
                                results.push(format!(
                                    "{}:{}: {}",
                                    entry_path.display(),
                                    i + 1,
                                    line
                                ));
                            }
                        }
                        Err(_) => continue, // Skip binary or invalid UTF-8 files
                    }
                }
            }
        }
        if results.is_empty() {
            Ok("No matches found".to_string())
        } else {
            Ok(results.join("\n"))
        }
    } else {
        // Single file search
        let file = File::open(&resolved_path).map_err(|e| anyhow!("Failed to open file: {}", e))?;
        let reader = BufReader::new(file);
        let mut results = Vec::new();

        for (i, line) in reader.lines().enumerate() {
            match line {
                Ok(line) => {
                    if re.is_match(&line) {
                        results.push(format!("{}:{}: {}", resolved_path.display(), i + 1, line));
                    }
                }
                Err(_) => continue,
            }
        }
        if results.is_empty() {
            Ok("No matches found".to_string())
        } else {
            Ok(results.join("\n"))
        }
    }
}

/// Run shell command
pub fn run_shell_command(command: &str, security: &SecurityPolicy) -> Result<String> {
    security.validate_shell_command(command)?;

    // Check if we are in a trusted directory (implicit in policy check usually,
    // but here we might want to check CWD if not done yet)
    // For now, simple execution

    if cfg!(target_os = "windows") {
        // Convert bash-style && to PowerShell-style ; for command chaining
        let powershell_command = command.replace(" && ", "; ");

        let output = Command::new("powershell")
            .args(["-Command", &powershell_command])
            .output()
            .map_err(|e| anyhow!("Failed to execute command: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            Ok(format!(
                "Command failed with code {}:\nStdout: {}\nStderr: {}",
                output.status, stdout, stderr
            ))
        } else {
            Ok(format!("Stdout: {}\nStderr: {}", stdout, stderr))
        }
    } else {
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .map_err(|e| anyhow!("Failed to execute command: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            Ok(format!(
                "Command failed with code {}:\nStdout: {}\nStderr: {}",
                output.status, stdout, stderr
            ))
        } else {
            Ok(format!("Stdout: {}\nStderr: {}", stdout, stderr))
        }
    }
}

/// Check if web search is properly configured
pub fn is_web_search_configured() -> bool {
    let api_key = std::env::var("GOOGLE_API_KEY");
    let cx = std::env::var("GOOGLE_CX");

    if api_key.is_err() || cx.is_err() {
        return false;
    }

    // Additional validation: GOOGLE_CX should NOT look like an API key
    // API keys typically start with "AIza", while CX IDs are different format
    if let Ok(cx_val) = &cx {
        // If CX looks like an API key, it's probably misconfigured
        if cx_val.starts_with("AIza") {
            eprintln!("⚠️  WARNING: GOOGLE_CX appears to be an API key, not a Search Engine ID!");
            eprintln!(
                "   GOOGLE_CX should be your Custom Search Engine ID (e.g., '017576662512468239146:omuauf_lfve')"
            );
            eprintln!("   Get it from: https://cse.google.com/cse/");
            return false;
        }
    }

    true
}

/// Perform a web search using Google Custom Search API
pub async fn web_search(query: &str) -> Result<String> {
    let api_key = std::env::var("GOOGLE_API_KEY").map_err(|_| {
        anyhow!(
            "GOOGLE_API_KEY environment variable not set.\n\
            To use web search:\n\
            1. Get a Google API key: https://console.cloud.google.com/apis/credentials\n\
            2. Create a Custom Search Engine: https://cse.google.com/cse/\n\
            3. Set environment variables:\n\
               export GOOGLE_API_KEY=your_api_key\n\
               export GOOGLE_CX=your_search_engine_id"
        )
    })?;
    let cx = std::env::var("GOOGLE_CX").map_err(|_| {
        anyhow!(
            "GOOGLE_CX environment variable not set.\n\
            To use web search:\n\
            1. Create a Custom Search Engine: https://cse.google.com/cse/\n\
            2. Get the Search Engine ID from the control panel\n\
            3. Set environment variable:\n\
               export GOOGLE_CX=your_search_engine_id"
        )
    })?;

    let url = format!(
        "https://www.googleapis.com/customsearch/v1?key={}&cx={}&q={}",
        api_key,
        cx,
        urlencoding::encode(query)
    );

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Could not read error body".to_string());
        return Err(anyhow!(
            "Search request failed: {}\n\
            Error details: {}\n\
            \n\
            Common causes:\n\
            - Custom Search API not enabled: https://console.cloud.google.com/apis/library/customsearch.googleapis.com\n\
            - Billing not enabled (required even for free tier)\n\
            - Invalid GOOGLE_CX (should be Search Engine ID, not API key)\n\
            - API key restrictions blocking the request\n\
            \n\
            Verify your setup at: https://console.cloud.google.com/",
            status,
            error_body
        ));
    }

    let json: Value = response.json().await?;

    let mut results = Vec::new();
    if let Some(items) = json.get("items").and_then(|i| i.as_array()) {
        for item in items {
            let title = item
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or("No title");
            let link = item
                .get("link")
                .and_then(|l| l.as_str())
                .unwrap_or("No link");
            let snippet = item.get("snippet").and_then(|s| s.as_str()).unwrap_or("");
            results.push(format!(
                "Title: {}\nLink: {}\nSnippet: {}\n",
                title, link, snippet
            ));
        }
    }

    if results.is_empty() {
        Ok("No results found".to_string())
    } else {
        Ok(results.join("\n---\n"))
    }
}

/// Fetch content from a URL
pub async fn web_fetch(url: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let response = client
        .get(url)
        .header("User-Agent", "grok-cli/0.1.0")
        .send()
        .await
        .map_err(|e| {
            anyhow!(
                "Failed to fetch URL '{}': {}\n\
            This could be due to:\n\
            - Network connectivity issues\n\
            - Invalid URL\n\
            - Server not responding\n\
            - Firewall/proxy blocking the request",
                url,
                e
            )
        })?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Failed to fetch URL '{}': HTTP {}\n\
            The server returned an error status code.",
            url,
            response.status()
        ));
    }

    let text = response.text().await?;
    // Basic cleanup: take first 10000 chars to avoid overloading context
    let truncated = if text.len() > 10000 {
        format!("{}... (truncated)", &text[..10000])
    } else {
        text
    };

    Ok(truncated)
}

/// Get tool definitions for the LLM
pub fn get_tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read the content of a file",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The path to the file to read"
                        }
                    },
                    "required": ["path"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "Write content to a file",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The path to the file to write"
                        },
                        "content": {
                            "type": "string",
                            "description": "The content to write"
                        }
                    },
                    "required": ["path", "content"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "replace",
                "description": "Replace text in a file",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The path to the file to modify"
                        },
                        "old_string": {
                            "type": "string",
                            "description": "The string to be replaced"
                        },
                        "new_string": {
                            "type": "string",
                            "description": "The new string to replace with"
                        },
                        "expected_replacements": {
                            "type": "integer",
                            "description": "Expected number of replacements (optional)"
                        }
                    },
                    "required": ["path", "old_string", "new_string"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "save_memory",
                "description": "Save a fact to long-term memory",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "fact": {
                            "type": "string",
                            "description": "The fact to remember"
                        }
                    },
                    "required": ["fact"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "list_directory",
                "description": "List files and directories in a path",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The directory path to list"
                        }
                    },
                    "required": ["path"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "glob_search",
                "description": "Find files matching a glob pattern",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "The glob pattern to match (e.g. **/*.rs)"
                        }
                    },
                    "required": ["pattern"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "search_file_content",
                "description": "Search for text patterns in files using regex",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The file or directory to search in"
                        },
                        "pattern": {
                            "type": "string",
                            "description": "The regex pattern to search for"
                        }
                    },
                    "required": ["path", "pattern"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "run_shell_command",
                "description": "Execute a shell command",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The command to execute"
                        }
                    },
                    "required": ["command"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "web_search",
                "description": "Search the web using Google Custom Search",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "The search query"
                        }
                    },
                    "required": ["query"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "web_fetch",
                "description": "Fetch content from a URL",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "The URL to fetch"
                        }
                    },
                    "required": ["url"]
                }
            }
        }),
    ]
}

/// Get only the tool definitions that are properly configured and available
pub fn get_available_tool_definitions() -> Vec<Value> {
    let all_tools = get_tool_definitions();

    // Filter out web_search if not configured
    all_tools
        .into_iter()
        .filter(|tool| {
            if let Some(name) = tool
                .get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
            {
                // Filter out web_search if credentials not configured
                if name == "web_search" && !is_web_search_configured() {
                    return false;
                }
            }
            true
        })
        .collect()
}

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
    async fn test_web_search_missing_api_key() {
        unsafe {
            // Ensure API keys are not set for this test
            std::env::remove_var("GOOGLE_API_KEY");
            std::env::remove_var("GOOGLE_CX");
        }

        let result = web_search("test query").await;
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("GOOGLE_API_KEY") || error_msg.contains("not set"));
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
        // Test with a URL that will timeout (non-routable IP)
        let result = web_fetch("http://10.255.255.1").await;
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_is_web_search_configured() {
        // Save current state
        let api_key = std::env::var("GOOGLE_API_KEY").ok();
        let cx = std::env::var("GOOGLE_CX").ok();

        unsafe {
            // Test not configured
            std::env::remove_var("GOOGLE_API_KEY");
            std::env::remove_var("GOOGLE_CX");
            assert!(!is_web_search_configured());

            // Test partially configured
            std::env::set_var("GOOGLE_API_KEY", "test_key");
            assert!(!is_web_search_configured());

            // Test fully configured
            std::env::set_var("GOOGLE_CX", "test_cx");
            assert!(is_web_search_configured());

            // Restore state
            std::env::remove_var("GOOGLE_API_KEY");
            std::env::remove_var("GOOGLE_CX");
            if let Some(key) = api_key {
                std::env::set_var("GOOGLE_API_KEY", key);
            }
            if let Some(cx_val) = cx {
                std::env::set_var("GOOGLE_CX", cx_val);
            }
        }
    }

    #[test]
    #[serial]
    fn test_get_available_tool_definitions() {
        // Save current state
        let api_key = std::env::var("GOOGLE_API_KEY").ok();
        let cx = std::env::var("GOOGLE_CX").ok();

        unsafe {
            // Test without web search configured
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
                !has_web_search,
                "web_search should be filtered out when not configured"
            );

            // Restore state
            if let Some(key) = api_key {
                std::env::set_var("GOOGLE_API_KEY", key);
            }
            if let Some(cx_val) = cx {
                std::env::set_var("GOOGLE_CX", cx_val);
            }
        }
    }
}
