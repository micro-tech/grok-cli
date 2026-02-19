# Code Commands Quick Start Guide

## Overview

The Grok CLI provides powerful tools for working with code, allowing you to explain, review, generate, and fix code using Grok AI. This guide covers the available code commands and how to use them effectively.

## Available Code Commands

| Command | Description |
|---------|-------------|
| `grok code explain` | Get a detailed explanation of provided code or a file. |
| `grok code review` | Receive a comprehensive review of your code focusing on security, performance, style, bugs, and maintainability. |
| `grok code generate` | Generate code based on a description of requirements. |
| `grok code fix` | Fix specific issues in a code file with AI assistance. |

## Quick Start

### 1. Explain Code

Use the `explain` command to understand what a piece of code does and how it works. This is great for learning or debugging.

```bash
# Explain code from a file
grok code explain --file path/to/your/code.rs

# Explain code directly as input
grok code explain "fn main() { println!(\"Hello, World!\"); }"
```

**Expected Output:**
- A detailed breakdown of the code’s purpose, step-by-step operation, key concepts, and potential improvements.

### 2. Review Code

Use the `review` command to get a professional code review, identifying issues, vulnerabilities, and areas for improvement.

```bash
# Review code from a file
grok code review --file path/to/your/code.py

# Review with specific focus areas
grok code review --file path/to/your/code.js --focus "security, performance"

# Review code directly as input
grok code review "function add(a, b) { return a + b; }"
```

**Expected Output:**
- A comprehensive review covering bugs, security issues, performance optimizations, style, and maintainability suggestions.

### 3. Generate Code

Use the `generate` command to create code based on your requirements. Specify the language and optionally save the output to a file.

```bash
# Generate Python code
grok code generate "Create a function to calculate factorial" --language Python

# Generate Rust code and save to a file
grok code generate "Implement a basic web server" --language Rust --output server.rs
```

**Expected Output:**
- Clean, well-documented code matching your requirements, displayed in the terminal or saved to a file.

### 4. Fix Code

Use the `fix` command to address specific issues in your code files with AI-generated solutions.

```bash
# Fix an issue in a file
grok code fix path/to/broken/code.c --issue "Segmentation fault when accessing array index"
```

**Expected Output:**
- An analysis of the issue, a corrected version of the code, and an explanation of the changes made. Always review fixes carefully before applying them.

## Best Practices

### ✅ DO

- **Provide clear input**: When explaining or reviewing code, ensure the code snippet or file is relevant to avoid vague responses.
- **Specify focus for reviews**: Use the `--focus` flag to prioritize specific areas like security or performance if needed.
- **Define requirements precisely**: For code generation, detailed descriptions yield better results (e.g., "Create a REST API in Node.js with endpoints for user CRUD operations").
- **Review fixes manually**: Always inspect AI-suggested fixes before integrating them into your project.

### ❌ DON'T

- **Don't input overly large files**: Very large code files may exceed token limits or result in less focused responses. Break them into smaller parts if possible.
- **Don't omit issue details**: When fixing code, vague issue descriptions (e.g., "It doesn't work") can lead to incorrect or irrelevant fixes.
- **Don't rely solely on AI**: Use the AI’s suggestions as a starting point, but apply your own judgment for final decisions.

## Troubleshooting

### "No code provided" Error

```bash
grok code explain ""
✗ Error: No code provided to explain
```

**Solution:** Ensure you provide a valid code snippet or file path with the `--file` flag.

### "Failed to read file" Error

```bash
grok code review --file nonexistent.rs
✗ Error: Failed to read file 'nonexistent.rs': No such file or directory
```

**Solution:** Verify the file path is correct and the file exists.

### Generated Code Not Saving

```bash
grok code generate "Write a script" --language Bash --output /invalid/path/script.sh
⚠ Warning: Failed to save to file: Permission denied
```

**Solution:** Check that you have write permissions in the specified directory and that the path is valid.

## Next Steps

1. **Try explaining sample code**: Use `grok code explain` on a small script or snippet to understand its functionality.
2. **Review your project**: Run `grok code review` on critical files to identify potential issues before deployment.
3. **Generate boilerplate**: Save time by generating repetitive code structures with `grok code generate`.
4. **Fix bugs quickly**: Use `grok code fix` to troubleshoot and resolve specific errors in your codebase.

## Resources

- [Full CLI Documentation](../README.md)
- [Skills Quick Start Guide](SKILLS_QUICK_START.md)

## Summary

```bash
# Explain code from a file
grok code explain --file main.rs

# Review code with specific focus
grok code review --file app.js --focus "security"

# Generate code and save it
grok code generate "Create a login form" --language HTML --output login.html

# Fix an issue in a file
grok code fix broken.py --issue "TypeError on line 12"
```

**Remember:** Grok’s code tools are designed to assist, not replace, your expertise. Use them to accelerate development and learning, but always validate the results!
