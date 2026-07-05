//! Vision API integration helpers
//!
//! This module provides functions to attach images to chat messages
//! so they can be sent to vision-capable Grok models.

use crate::tools::image::prepare_image_content;
use anyhow::Result;
use serde_json::{json, Value};

/// Create a user message that includes both text and an image.
/// This is the format expected by most vision APIs (including xAI/Grok).
///
/// Returns a JSON message object ready to be included in the messages array.
pub fn create_vision_message(text: &str, image_path_or_url: &str) -> Result<Value> {
    let image_content = prepare_image_content(image_path_or_url)?;

    // For now we use a simple structure that most vision APIs understand.
    // The actual grok_api crate may need a small update to support MessageContent::Image.
    Ok(json!({
        "role": "user",
        "content": [
            {
                "type": "text",
                "text": text
            },
            {
                "type": "image_url",
                "image_url": {
                    "url": image_content
                }
            }
        ]
    }))
}

/// Check if a message (as JSON) already contains image content.
pub fn message_has_image(msg: &Value) -> bool {
    if let Some(content) = msg.get("content") {
        if content.is_array() {
            return content.as_array().unwrap().iter().any(|part| {
                part.get("type").and_then(|t| t.as_str()) == Some("image_url")
            });
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_vision_message_structure() {
        let msg = create_vision_message("describe this", "./test.png").unwrap();
        assert_eq!(msg["role"], "user");
        assert!(msg["content"].is_array());
    }
}
