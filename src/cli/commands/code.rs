//! Code command handler for grok-cli
//!
//! Handles code-related operations including explanation, review, generation,
//! and fixing code issues using Grok AI.

use anyhow::{anyhow, Result};
use colored::*;
use std::fs;
use std::path::Path;

use crate::api::grok::GrokClient;
use crate::cli::{
    create_spinner, format_code, print_error, print_info, print_success, print_warning,
};
use crate::config::RateLimitConfig;
use crate::CodeAction;

/// Handle code-related commands
pub async fn handle_code_action(
    action: CodeAction,
    api_key: &str,
    model: &str,
    timeout_secs: u64,
    max_retries: u32,
    rate_limit_config: RateLimitConfig,
) -> Result<()> {
    let client = GrokClient::with_settings(api_key, timeout_secs, max_retries)?
        .with_rate_limits(rate_limit_config);

    match action {
        CodeAction::Explain { input, file } => {
            handle_code_explain(client, &input, file, model).await
        }
        CodeAction::Review { input, file, focus } => {
            handle_code_review(client, &input, file, focus.as_deref(), model).await
        }
        CodeAction::Generate {
            description,
            language,
            output,
        } => {
            handle_code_generate(
                client,
                description,
                language.as_deref(),
                output.as_deref(),
                model,
            )
            .await
        }
        CodeAction::Fix { file, issue } => handle_code_fix(client, &file, issue, model).await,
    }
}

/// Handle code explanation requests
async fn handle_code_explain(
    client: GrokClient,
    input: &str,
    is_file: bool,
    model: &str,
) -> Result<()> {
    let (code, language) = if is_file || Path::new(input).exists() {
        print_info(&format!("Reading code from file: {}", input));
        let code = fs::read_to_string(input)
            .map_err(|e| anyhow!("Failed to read file '{}': {}", input, e))?;

        let language = detect_language_from_path(input);
        (code, language)
    } else {
        (input.to_string(), None)
    };

    if code.trim().is_empty() {
        return Err(anyhow!("No code provided to explain"));
    }

    print_info(&format!("Explaining code using model: {}", model));

    let spinner = create_spinner("Analyzing code...");

    let language_hint = language
        .as_ref()
        .map(|l| format!(" (detected language: {})", l))
        .unwrap_or_default();

    let system_prompt = "You are an expert software engineer and teacher. Your task is to explain code in a clear, educational manner. Focus on:
- What the code does (high-level purpose)
- How it works (step-by-step breakdown)
- Key programming concepts and patterns used
- Important details about the implementation
- Potential improvements or alternatives

Make your explanation accessible but thorough. Use examples when helpful.";

    let user_message = format!(
        "Please explain this code{}:\n\n```\n{}\n```",
        language_hint, code
    );

    let response = client
        .chat_completion(&user_message, Some(system_prompt), 0.3, 4096, model)
        .await;

    spinner.finish_and_clear();

    match response {
        Ok(explanation) => {
            print_success("Code explanation generated!");
            println!();
            println!("{}", "📖 Code Explanation:".cyan().bold());
            println!("{}", "═".repeat(50));
            println!("{}", explanation);

            if let Some(lang) = language {
                println!();
                println!("{}", format!("Language: {}", lang).dimmed());
            }
        }
        Err(e) => {
            print_error(&format!("Failed to generate explanation: {}", e));
            return Err(e);
        }
    }

    Ok(())
}

/// Handle code review requests
async fn handle_code_review(
    client: GrokClient,
    input: &str,
    is_file: bool,
    focus: Option<&str>,
    model: &str,
) -> Result<()> {
    let (code, language) = if is_file || Path::new(input).exists() {
        print_info(&format!("Reading code from file: {}", input));
        let code = fs::read_to_string(input)
            .map_err(|e| anyhow!("Failed to read file '{}': {}", input, e))?;

        let language = detect_language_from_path(input);
        (code, language)
    } else {
        (input.to_string(), None)
    };

    if code.trim().is_empty() {
        return Err(anyhow!("No code provided to review"));
    }

    print_info(&format!("Reviewing code using model: {}", model));

    let focus_areas = focus.unwrap_or("security, performance, style, bugs, maintainability");
    print_info(&format!("Focus areas: {}", focus_areas));

    let spinner = create_spinner("Reviewing code...");

    let language_hint = language
        .as_ref()
        .map(|l| format!(" (language: {})", l))
        .unwrap_or_default();

    let system_prompt = format!(
        "You are an expert code reviewer with years of experience in software development. \
        Review the provided code focusing on: {}. \n\n\
        Provide a comprehensive review covering:\n\
        - Issues and potential bugs\n\
        - Security vulnerabilities\n\
        - Performance improvements\n\
        - Code style and best practices\n\
        - Maintainability concerns\n\
        - Suggestions for improvement\n\n\
        Be specific, actionable, and constructive in your feedback. \
        Use examples when suggesting improvements.",
        focus_areas
    );

    let user_message = format!(
        "Please review this code{}:\n\n```\n{}\n```",
        language_hint, code
    );

    let response = client
        .chat_completion(&user_message, Some(&system_prompt), 0.2, 6144, model)
        .await;

    spinner.finish_and_clear();

    match response {
        Ok(review) => {
            print_success("Code review completed!");
            println!();
            println!("{}", "🔍 Code Review:".cyan().bold());
            println!("{}", "═".repeat(50));
            println!("{}", review);

            if let Some(lang) = language {
                println!();
                println!("{}", format!("Language: {}", lang).dimmed());
            }
            println!("{}", format!("Focus areas: {}", focus_areas).dimmed());
        }
        Err(e) => {
            print_error(&format!("Failed to generate review: {}", e));
            return Err(e);
        }
    }

    Ok(())
}

/// Handle code generation requests
async fn handle_code_generate(
    client: GrokClient,
    description: Vec<String>,
    language: Option<&str>,
    output_file: Option<&str>,
    model: &str,
) -> Result<()> {
    let combined_description = description.join(" ");

    if combined_description.trim().is_empty() {
        return Err(anyhow!("No description provided for code generation"));
    }

    let target_language = language.unwrap_or("Python");
    print_info(&format!(
        "Generating {} code using model: {}",
        target_language, model
    ));

    let spinner = create_spinner("Generating code...");

    let system_prompt = format!(
        "You are an expert {} developer. Generate clean, well-documented, and production-ready code \
        based on the user's requirements. Follow these guidelines:\n\
        - Write clear, readable code with appropriate comments\n\
        - Follow {} best practices and conventions\n\
        - Include error handling where appropriate\n\
        - Use meaningful variable and function names\n\
        - Add docstrings/documentation comments\n\
        - Consider edge cases and validation\n\n\
        Provide only the code with minimal explanation unless asked for more detail.",
        target_language, target_language
    );

    let user_message = format!(
        "Generate {} code for the following requirements:\n\n{}",
        target_language, combined_description
    );

    let response = client
        .chat_completion(&user_message, Some(&system_prompt), 0.1, 8192, model)
        .await;

    spinner.finish_and_clear();

    match response {
        Ok(generated_code) => {
            print_success("Code generated successfully!");
            println!();
            println!(
                "{}",
                format!("🚀 Generated {} Code:", target_language)
                    .cyan()
                    .bold()
            );
            println!("{}", "═".repeat(50));

            // Try to extract just the code from the response
            let clean_code = extract_code_from_response(&generated_code);
            println!("{}", format_code(&clean_code, Some(target_language)));

            // Save to file if requested
            if let Some(output_path) = output_file {
                match save_code_to_file(&clean_code, output_path) {
                    Ok(()) => {
                        print_success(&format!("Code saved to: {}", output_path));
                    }
                    Err(e) => {
                        print_warning(&format!("Failed to save to file: {}", e));
                        print_info("The generated code is displayed above.");
                    }
                }
            }

            println!();
            println!("{}", format!("Language: {}", target_language).dimmed());
            println!(
                "{}",
                format!("Requirements: {}", combined_description).dimmed()
            );
        }
        Err(e) => {
            print_error(&format!("Failed to generate code: {}", e));
            return Err(e);
        }
    }

    Ok(())
}

/// Handle code fixing requests
async fn handle_code_fix(
    client: GrokClient,
    file_path: &str,
    issue_description: Vec<String>,
    model: &str,
) -> Result<()> {
    let combined_issue = issue_description.join(" ");

    if combined_issue.trim().is_empty() {
        return Err(anyhow!("No issue description provided"));
    }

    print_info(&format!("Reading code from file: {}", file_path));

    let code = fs::read_to_string(file_path)
        .map_err(|e| anyhow!("Failed to read file '{}': {}", file_path, e))?;

    if code.trim().is_empty() {
        return Err(anyhow!("File is empty: {}", file_path));
    }

    let language = detect_language_from_path(file_path);
    print_info(&format!("Fixing code using model: {}", model));

    if let Some(ref lang) = language {
        print_info(&format!("Detected language: {}", lang));
    }

    let spinner = create_spinner("Analyzing and fixing code...");

    let language_hint = language
        .as_ref()
        .map(|l| format!(" (language: {})", l))
        .unwrap_or_default();

    let system_prompt =
        "You are an expert software engineer specializing in debugging and code fixes. \
        Your task is to analyze the provided code and fix the described issue. \n\n\
        Guidelines:\n\
        - Understand the issue thoroughly\n\
        - Identify the root cause\n\
        - Provide a complete, corrected version of the code\n\
        - Explain what was wrong and how you fixed it\n\
        - Ensure the fix doesn't introduce new issues\n\
        - Maintain the original code structure and style when possible\n\n\
        Format your response with:\n\
        1. Brief explanation of the issue\n\
        2. The corrected code\n\
        3. Explanation of the changes made"
            .to_string();

    let user_message = format!(
        "Here is the code{} that needs to be fixed:\n\n```\n{}\n```\n\n\
        Issue to fix: {}",
        language_hint, code, combined_issue
    );

    let response = client
        .chat_completion(&user_message, Some(&system_prompt), 0.1, 8192, model)
        .await;

    spinner.finish_and_clear();

    match response {
        Ok(fix_response) => {
            print_success("Code analysis and fix completed!");
            println!();
            println!("{}", "🔧 Code Fix:".cyan().bold());
            println!("{}", "═".repeat(50));
            println!("{}", fix_response);

            println!();
            println!("{}", format!("File: {}", file_path).dimmed());
            println!("{}", format!("Issue: {}", combined_issue).dimmed());

            if let Some(lang) = language {
                println!("{}", format!("Language: {}", lang).dimmed());
            }

            println!();
            print_warning(
                "⚠️  Please review the suggested fix carefully before applying it to your code.",
            );
        }
        Err(e) => {
            print_error(&format!("Failed to fix code: {}", e));
            return Err(e);
        }
    }

    Ok(())
}

/// Detect programming language from file extension
fn detect_language_from_path(path: &str) -> Option<String> {
    let extension = Path::new(path).extension()?.to_str()?.to_lowercase();

    let language = match extension.as_str() {
        "rs" => "Rust",
        "py" => "Python",
        "js" => "JavaScript",
        "ts" | "tsx" => "TypeScript",
        "java" => "Java",
        "cpp" | "cc" | "cxx" => "C++",
        "c" => "C",
        "cs" => "C#",
        "go" => "Go",
        "php" => "PHP",
        "rb" => "Ruby",
        "swift" => "Swift",
        "kt" => "Kotlin",
        "scala" => "Scala",
        "hs" => "Haskell",
        "ml" => "OCaml",
        "clj" => "Clojure",
        "elm" => "Elm",
        "dart" => "Dart",
        "lua" => "Lua",
        "r" => "R",
        "m" => "Objective-C",
        "sh" | "bash" => "Shell",
        "sql" => "SQL",
        "html" => "HTML",
        "css" => "CSS",
        "scss" | "sass" => "SCSS",
        "json" => "JSON",
        "yaml" | "yml" => "YAML",
        "toml" => "TOML",
        "xml" => "XML",
        "md" => "Markdown",
        _ => return None,
    };

    Some(language.to_string())
}

/// Extract clean code from a response that might contain explanations
fn extract_code_from_response(response: &str) -> String {
    // Try to find code blocks first
    if let Some(start) = response.find("```")
        && let Some(end) = response[start + 3..].find("```") {
            let code_block = &response[start + 3..start + 3 + end];
            // Remove language identifier from first line if present
            let lines: Vec<&str> = code_block.lines().collect();
            if !lines.is_empty() {
                let first_line = lines[0].trim();
                if first_line
                    .chars()
                    .all(|c| c.is_alphabetic() || c == '+' || c == '#')
                {
                    // First line is likely a language identifier
                    return lines[1..].join("\n").trim().to_string();
                }
            }
            return code_block.trim().to_string();
        }

    // If no code blocks found, return the entire response
    response.trim().to_string()
}

/// Save generated code to a file
fn save_code_to_file(code: &str, output_path: &str) -> Result<()> {
    // Create parent directories if they don't exist
    if let Some(parent) = Path::new(output_path).parent() {
        fs::create_dir_all(parent).map_err(|e| anyhow!("Failed to create directory: {}", e))?;
    }

    fs::write(output_path, code)
        .map_err(|e| anyhow!("Failed to write file '{}': {}", output_path, e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language_from_path() {
        assert_eq!(
            detect_language_from_path("main.rs"),
            Some("Rust".to_string())
        );
        assert_eq!(
            detect_language_from_path("script.py"),
            Some("Python".to_string())
        );
        assert_eq!(
            detect_language_from_path("app.js"),
            Some("JavaScript".to_string())
        );
        assert_eq!(
            detect_language_from_path("Component.tsx"),
            Some("TypeScript".to_string())
        );
        assert_eq!(detect_language_from_path("unknown.xyz"), None);
    }

    #[test]
    fn test_extract_code_from_response() {
        let response_with_blocks = "Here's the code:\n```rust\nfn main() {\n    println!(\"Hello\");\n}\n```\nThat should work!";
        let extracted = extract_code_from_response(response_with_blocks);
        assert_eq!(extracted, "fn main() {\n    println!(\"Hello\");\n}");

        let response_without_blocks = "fn main() {\n    println!(\"Hello\");\n}";
        let extracted2 = extract_code_from_response(response_without_blocks);
        assert_eq!(extracted2, "fn main() {\n    println!(\"Hello\");\n}");
    }
}
