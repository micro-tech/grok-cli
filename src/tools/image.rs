//! Image / Vision support for grok-cli
//!
//! Provides utilities for detecting, validating, and preparing images
//! (local files + URLs) to be sent to vision-capable models.

use anyhow::{bail, Result};
use std::path::Path;

/// Supported image extensions
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif"];

/// Check if a path points to a supported image file
pub fn is_image_path(path: &str) -> bool {
    let path = Path::new(path);
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str())
    } else {
        false
    }
}

/// Check if a string looks like an image URL
pub fn is_image_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    (lower.starts_with("http://") || lower.starts_with("https://"))
        && IMAGE_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

/// Encode a local image file to base64
pub fn encode_local_image(path: &str) -> Result<String> {
    if !is_image_path(path) {
        bail!("Not a supported image file: {}", path);
    }

    let bytes = std::fs::read(path)?;
    Ok(base64::encode(bytes))
}

/// Prepare image content for the API.
/// Returns either a base64 data URL or the original URL.
pub fn prepare_image_content(path_or_url: &str) -> Result<String> {
    if is_image_url(path_or_url) {
        Ok(path_or_url.to_string())
    } else if Path::new(path_or_url).exists() {
        let b64 = encode_local_image(path_or_url)?;
        let ext = Path::new(path_or_url)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png");
        Ok(format!("data:image/{};base64,{}", ext, b64))
    } else {
        bail!("Image not found: {}", path_or_url)
    }
}

/// Extract the first image path or URL found in a free-text message.
/// This enables direct prompt usage like:
/// "analyze this: ./diagram.png" or "what's in https://example.com/chart.jpg"
pub fn extract_image_from_message(message: &str) -> Option<String> {
    // Simple heuristic: look for tokens that look like image paths or URLs
    for token in message.split_whitespace() {
        let cleaned = token.trim_matches(|c: char| c == ',' || c == '.' || c == '!' || c == '?');
        if is_image_path(cleaned) || is_image_url(cleaned) {
            return Some(cleaned.to_string());
        }
    }
    None
}

/// Print a nice TUI feedback line when an image is attached.
/// Example output:
///   [🖼️  image attached: diagram.png]
pub fn print_image_attached_feedback(path: &str) {
    use colored::Colorize;
    println!(
        "{} {}",
        "[🖼️  image attached]".bright_cyan(),
        path.bright_yellow()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_image_extensions() {
        assert!(is_image_path("photo.png"));
        assert!(is_image_path("chart.JPG"));
        assert!(!is_image_path("notes.txt"));
    }

    #[test]
    fn detects_image_urls() {
        assert!(is_image_url("https://example.com/pic.jpg"));
        assert!(!is_image_url("not-an-image"));
    }

    #[test]
    fn extracts_image_from_text() {
        assert_eq!(
            extract_image_from_message("look at this ./diagram.png"),
            Some("./diagram.png".to_string())
        );
        assert_eq!(
            extract_image_from_message("check https://site.com/chart.jpg please"),
            Some("https://site.com/chart.jpg".to_string())
        );
        assert_eq!(extract_image_from_message("just some text"), None);
    }
}
