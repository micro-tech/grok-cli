# Tasks for grok_api Crate Update

Based on the analysis of `grok-cli` integration issues, the following changes are required in the `grok_api` crate to fully support tool use and prevent infinite loops.

## 1. ChatMessage Structure Update
The `ChatMessage` struct (used for sending requests) must support tool results.

- [ ] **Add `tool_call_id` field**:
  ```rust
  pub struct ChatMessage {
      pub role: String,
      pub content: Option<String>,
      pub tool_calls: Option<Vec<ToolCall>>,
      pub tool_call_id: Option<String>, // REQUIRED for role: "tool"
  }
  ```

## 2. Helper Constructors
Add a constructor for creating tool result messages easily.

- [ ] **Add `ChatMessage::tool`**:
  ```rust
  impl ChatMessage {
      pub fn tool(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
          Self {
              role: "tool".into(),
              content: Some(content.into()),
              tool_calls: None,
              tool_call_id: Some(tool_call_id.into()),
          }
      }
  }
  ```

## 3. MessageContent Usability
The `Message` struct (used for responses) uses `Option<MessageContent>`. This type needs to be easier to work with.

- [ ] **Implement `Display` for `MessageContent`**:
  Allow `println!("{}", content)` to work directly.
- [ ] **Add `text()` helper**:
  ```rust
  impl MessageContent {
      pub fn text(&self) -> &str {
          match self {
              MessageContent::Text(s) => s,
              // Handle other variants if any
          }
      }
  }
  ```

## 4. Serialization Verification
Ensure that `ChatMessage` serializes to the exact JSON format expected by the API.

- [ ] **Verify `role: "tool"` serialization**:
  ```json
  {
    "role": "tool",
    "content": "result string",
    "tool_call_id": "call_123"
  }
  ```
- [ ] **Verify `role: "assistant"` with tool calls**:
  ```json
  {
    "role": "assistant",
    "content": null,
    "tool_calls": [ ... ]
  }
  ```

## 5. ToolCall Structure
Ensure `ToolCall` has the correct fields.

- [ ] **Verify fields**:
  ```rust
  pub struct ToolCall {
      pub id: String,
      pub function: FunctionCall,
      pub call_type: String, // Serialize as "type"
  }
  ```
  Note: If the field is named `call_type`, ensure it has `#[serde(rename = "type")]`.
