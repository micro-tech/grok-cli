# Tools Quick Reference Card

## 🛠️ Available Tools Overview

### 📁 File Operations
```
read_file(path)              - Read file content
write_file(path, content)    - Write to file
replace(path, old, new)      - Replace text in file
```

### 🔍 Search & Discovery
```
list_directory(path)                    - List directory contents
glob_search(pattern)                    - Find files by pattern
search_file_content(path, pattern)      - Search text in files (regex)
```

### ⚡ Execution & Web
```
run_shell_command(command)   - Execute shell command
web_search(query)            - Search the web
web_fetch(url)               - Fetch URL content
```

### 💾 Memory
```
save_memory(fact)            - Save to long-term memory
```

---

## 📝 Quick Examples

### Read a file
```
"Can you read src/main.rs?"
→ Grok uses: read_file("src/main.rs")
```

### Find all Rust files
```
"Show me all Rust files"
→ Grok uses: glob_search("**/*.rs")
```

### Search for TODOs
```
"Find all TODO comments"
→ Grok uses: search_file_content(".", "TODO|FIXME")
```

### Replace text
```
"Change port 8080 to 3000 in config.rs"
→ Grok uses: replace("src/config.rs", "port = 8080", "port = 3000")
```

### Run tests
```
"Run the test suite"
→ Grok uses: run_shell_command("cargo test")
```

---

## 🎯 Common Patterns

### Refactoring
```
1. read_file() - Get current code
2. replace() - Make changes
3. run_shell_command("cargo check") - Verify
```

### Project Analysis
```
1. list_directory() - See structure
2. glob_search() - Find specific files
3. search_file_content() - Find patterns
```

### Code Review
```
1. glob_search("**/*.rs") - Find files
2. read_file() - Read each file
3. search_file_content() - Find issues
```

---

## 🔒 Security Notes

✅ **Trusted directories only** - Operations restricted to safe paths  
✅ **No directory traversal** - Path validation enforced  
✅ **Command restrictions** - Shell commands monitored  
✅ **Rate limiting** - Network operations throttled  

---

## 💡 Pro Tips

### File Operations
- Always read before writing
- Use `replace` for precision edits
- Verify paths with `list_directory` first

### Search Operations
- Use glob for file names
- Use content search for text
- Test regex patterns carefully

### Shell Commands
- Keep commands simple
- Handle errors gracefully
- Use timeouts for long operations

### Best Practices
- Start with narrow scope
- Verify before executing
- Check results after operations
- Use appropriate tool for task

---

## 🚀 Getting Started

### Interactive Mode
```bash
grok interactive
```
Then type: `/tools` to see all tools

### ACP Server (for Zed)
```bash
grok acp stdio
```
Tools auto-available in editor

### Quick Query
```bash
grok query "Find all main functions"
```

---

## 📊 Glob Pattern Examples

```
**/*.rs          - All Rust files (recursive)
src/**/*.toml    - All TOML in src/ tree
*.json           - JSON in current dir
test_*.rs        - Test files with prefix
**/Cargo.toml    - All Cargo.toml files
```

---

## 🔎 Regex Pattern Examples

```
fn main           - Find main functions
TODO|FIXME       - Find TODO/FIXME comments
struct \w+       - Find struct definitions
pub async fn     - Find public async functions
#\[derive.*\]    - Find derive attributes
```

---

## ⚙️ Configuration

### Set API Key
```bash
export GROK_API_KEY=your_key_here
```

### Trust Directory
In `~/.config/grok-cli/config.json`:
```json
{
  "acp": {
    "trusted_directories": ["/path/to/project"]
  }
}
```

---

## 🆘 Troubleshooting

| Error | Solution |
|-------|----------|
| Permission denied | Add to trusted_directories |
| File not found | Check path with list_directory |
| Tool timeout | Increase timeout in config |
| Max iterations | Break into smaller tasks |

---

## 📚 More Info

- Full docs: `docs/TOOLS.md`
- Examples: `docs/TOOLS.md#examples`
- Security: `docs/TOOLS.md#security-model`
- Config: `docs/TOOLS.md#configuration-reference`

---

**Need Help?** Type `/help` in interactive mode or visit the GitHub repo!