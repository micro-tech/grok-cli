//! Vision model handling for grok-cli
//!
//! Provides logic to detect when an image is present and automatically
//! switch to a vision-capable model for that turn only.

use std::sync::OnceLock;

/// List of known vision-capable Grok models.
/// These are used when an image is detected in the conversation.
static VISION_MODELS: OnceLock<Vec<&'static str>> = OnceLock::new();

fn get_vision_models() -> &'static [&'static str] {
    VISION_MODELS.get_or_init(|| {
        vec![
            "grok-2-vision-1212",
            "grok-4-vision",
            "grok-vision",
        ]
    })
}

/// Check if the given model name is a vision-capable model.
pub fn is_vision_model(model: &str) -> bool {
    get_vision_models().iter().any(|m| model.contains(m))
}

/// Return a recommended vision model.
/// Currently returns the first known vision model.
pub fn recommended_vision_model() -> &'static str {
    get_vision_models().first().copied().unwrap_or("grok-2-vision-1212")
}

/// Detect whether we should use a vision model for this turn.
/// This is a simple heuristic: if the message contains an image reference
/// (detected via the image module), we recommend switching.
pub fn should_use_vision_model(message: &str) -> bool {
    crate::tools::image::extract_image_from_message(message).is_some()
        || message.to_lowercase().contains("image")
        || message.to_lowercase().contains("picture")
        || message.to_lowercase().contains("photo")
}

/// Returns the best vision model to fall back to when the current model
/// does not support vision. xAI's current dedicated vision model is used.
pub fn get_vision_fallback_model() -> &'static str {
    "grok-2-vision-1212"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_vision_models() {
        assert!(is_vision_model("grok-2-vision-1212"));
        assert!(!is_vision_model("grok-4.3"));
    }

    #[test]
    fn recommends_a_vision_model() {
        let model = recommended_vision_model();
        assert!(is_vision_model(model));
    }
}
