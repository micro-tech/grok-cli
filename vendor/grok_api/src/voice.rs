//! Real-time Voice API for Grok (xAI) Speech-to-Speech models.
//!
//! This module provides [`VoiceSession`], a WebSocket client for the xAI
//! `wss://api.x.ai/v1/realtime` endpoint, constructed with
//! [`VoiceSessionBuilder`].
//!
//! # Quick Start
//!
//! ```no_run
//! use grok_api::voice::{VoiceSession, VoiceEvent};
//!
//! #[tokio::main]
//! async fn main() -> grok_api::Result<()> {
//!     let mut session = VoiceSession::builder("your-api-key")
//!         .instructions("You are a helpful voice assistant.")
//!         .server_vad()
//!         .build()
//!         .await?;
//!
//!     // Stream audio chunks in from a microphone / file…
//!     session.append_audio("base64encodedaudio==").await?;
//!
//!     while let Some(event) = session.next_event().await? {
//!         match event {
//!             VoiceEvent::AudioDelta { delta, .. } => {
//!                 // play back the base64-encoded PCM chunk
//!                 let _ = delta;
//!             }
//!             VoiceEvent::Done { usage } => {
//!                 println!("Done. Usage: {:?}", usage);
//!                 break;
//!             }
//!             _ => {}
//!         }
//!     }
//!     Ok(())
//! }
//! ```

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use rand::RngExt;
use serde_json::json;
use tokio::net::TcpStream;
use tokio::time::sleep;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{Message, client::IntoClientRequest, http::HeaderValue},
};
use tracing::{debug, info, warn};
use url::Url;

use crate::{
    error::{Error, Result},
    models::Model,
    retry::RetryConfig,
};

/// Underlying WebSocket stream type (TLS or plain TCP, selected at runtime).
type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// xAI real-time WebSocket base URL.
const REALTIME_URL: &str = "wss://api.x.ai/v1/realtime";

/// Default voice model used by [`VoiceSessionBuilder`] when none is specified.
const DEFAULT_VOICE_MODEL: &str = "grok-voice-think-fast-2.0";

// ─── Public types ──────────────────────────────────────────────────────────────

/// Audio codec for input or output audio streams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioCodec {
    /// Linear PCM (16-bit signed, little-endian).
    Pcm,
    /// G.711 µ-law (8-bit, 8 kHz).
    PcmU,
    /// G.711 A-law (8-bit, 8 kHz).
    PcmA,
    /// Opus variable-bitrate codec.
    Opus,
}

/// Audio format: a codec plus an optional sample rate.
///
/// Use the constructor methods ([`AudioFormat::pcm`], [`AudioFormat::pcmu`],
/// [`AudioFormat::pcma`], [`AudioFormat::opus`]) rather than constructing
/// directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioFormat {
    codec: AudioCodec,
    rate: Option<u32>,
}

impl AudioFormat {
    /// Linear PCM at `rate` Hz (e.g. `24_000`).
    ///
    /// ```
    /// use grok_api::voice::AudioFormat;
    /// assert_eq!(AudioFormat::pcm(24_000).codec_str(), "audio/pcm");
    /// ```
    pub fn pcm(rate: u32) -> Self {
        Self {
            codec: AudioCodec::Pcm,
            rate: Some(rate),
        }
    }

    /// G.711 µ-law at 8 kHz.
    pub fn pcmu() -> Self {
        Self {
            codec: AudioCodec::PcmU,
            rate: None,
        }
    }

    /// G.711 A-law at 8 kHz.
    pub fn pcma() -> Self {
        Self {
            codec: AudioCodec::PcmA,
            rate: None,
        }
    }

    /// Opus variable-bitrate codec.
    pub fn opus() -> Self {
        Self {
            codec: AudioCodec::Opus,
            rate: None,
        }
    }

    /// Returns the MIME-type string sent in `session.update`.
    pub fn codec_str(&self) -> &'static str {
        match self.codec {
            AudioCodec::Pcm => "audio/pcm",
            AudioCodec::PcmU => "audio/pcmu",
            AudioCodec::PcmA => "audio/pcma",
            AudioCodec::Opus => "audio/opus",
        }
    }

    /// Returns the sample rate, if one was specified.
    pub fn rate(&self) -> Option<u32> {
        self.rate
    }
}

/// How the model decides when a user's speaking turn ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnDetectionKind {
    /// Server-side voice-activity detection — the model decides automatically.
    ServerVad,
    /// Manual — the client signals end-of-turn by calling
    /// [`VoiceSession::commit_audio`].
    Manual,
}

/// Turn-detection configuration for a [`VoiceSession`].
#[derive(Debug, Clone)]
pub struct TurnDetection {
    /// Detection strategy.
    pub kind: TurnDetectionKind,
    /// VAD activation threshold (0.1 – 0.9).
    /// Only meaningful with [`TurnDetectionKind::ServerVad`].
    pub threshold: Option<f32>,
    /// Milliseconds of silence before the turn is considered ended.
    pub silence_duration_ms: Option<u32>,
    /// Milliseconds of audio prepended before the detected speech onset.
    pub prefix_padding_ms: Option<u32>,
    /// Milliseconds of inactivity before the session auto-closes.
    pub idle_timeout_ms: Option<u32>,
}

impl TurnDetection {
    /// Server-side VAD with default parameters.
    pub fn server_vad() -> Self {
        Self {
            kind: TurnDetectionKind::ServerVad,
            threshold: None,
            silence_duration_ms: None,
            prefix_padding_ms: None,
            idle_timeout_ms: None,
        }
    }

    /// Manual turn detection — the client commits audio explicitly.
    pub fn manual() -> Self {
        Self {
            kind: TurnDetectionKind::Manual,
            threshold: None,
            silence_duration_ms: None,
            prefix_padding_ms: None,
            idle_timeout_ms: None,
        }
    }

    /// Set the VAD activation threshold (0.1 – 0.9).
    pub fn threshold(mut self, value: f32) -> Self {
        self.threshold = Some(value);
        self
    }

    /// Silence duration before end-of-turn is declared (ms).
    pub fn silence_duration_ms(mut self, ms: u32) -> Self {
        self.silence_duration_ms = Some(ms);
        self
    }

    /// Audio prepended before the detected speech segment (ms).
    pub fn prefix_padding_ms(mut self, ms: u32) -> Self {
        self.prefix_padding_ms = Some(ms);
        self
    }

    /// Idle timeout before the session auto-closes (ms).
    pub fn idle_timeout_ms(mut self, ms: u32) -> Self {
        self.idle_timeout_ms = Some(ms);
        self
    }
}

/// Reasoning intensity for supported voice models.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReasoningEffort {
    /// Full reasoning — highest quality, higher latency.
    High,
    /// No extended reasoning — lowest latency.
    None,
}

/// Token-usage statistics returned in [`VoiceEvent::Done`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceUsage {
    /// Tokens consumed by the input audio/text.
    pub input_tokens: Option<u32>,
    /// Tokens in the generated audio/text output.
    pub output_tokens: Option<u32>,
    /// Total tokens used this turn.
    pub total_tokens: Option<u32>,
}

/// Events received from the xAI real-time WebSocket.
#[derive(Debug)]
pub enum VoiceEvent {
    /// The server created a new session.
    SessionCreated {
        /// Server-assigned session ID.
        session_id: String,
    },
    /// The server acknowledged a `session.update` message.
    SessionUpdated,
    /// The server created a new conversation item.
    ConversationCreated {
        /// Server-assigned conversation ID.
        conversation_id: String,
    },
    /// A chunk of base64-encoded output audio is available to play.
    AudioDelta {
        /// Item ID within the conversation.
        item_id: String,
        /// Base64-encoded PCM or Opus audio bytes.
        delta: String,
    },
    /// All output audio for an item has been sent.
    AudioDone {
        /// Item ID.
        item_id: String,
    },
    /// A partial transcript of the model's spoken output.
    TranscriptDelta {
        /// Item ID.
        item_id: String,
        /// Partial transcript text.
        delta: String,
    },
    /// The complete speech-to-text transcript for an item.
    TranscriptDone {
        /// Item ID.
        item_id: String,
        /// Full transcript string.
        transcript: String,
    },
    /// Live transcription of the user's microphone input was updated.
    InputTranscriptUpdated {
        /// Item ID.
        item_id: String,
        /// Transcript text so far.
        transcript: String,
    },
    /// The model is requesting a function / tool call.
    FunctionCall {
        /// Function name to invoke.
        name: String,
        /// Opaque call ID; echo it back with
        /// [`VoiceSession::send_function_result`].
        call_id: String,
        /// JSON-encoded arguments string.
        arguments: String,
    },
    /// The model finished generating a response.
    Done {
        /// Token usage breakdown, if the server provided it.
        usage: Option<VoiceUsage>,
    },
    /// The server reported an error.
    Error {
        /// Error code string.
        code: String,
        /// Human-readable message.
        message: String,
    },
    /// An event type not yet handled by this version of the library.
    Other {
        /// Raw `"type"` field from the server message.
        event_type: String,
    },
}

// ─── Private session configuration ────────────────────────────────────────────

/// All session parameters serialised in a `session.update` message.
struct SessionConfig {
    voice: Option<String>,
    instructions: Option<String>,
    turn_detection: Option<TurnDetection>,
    reasoning_effort: Option<ReasoningEffort>,
    input_format: Option<AudioFormat>,
    output_format: Option<AudioFormat>,
    input_transport: Option<String>,
    output_transport: Option<String>,
    language_hint: Option<String>,
    keyterms: Vec<String>,
    output_speed: Option<f32>,
    resumption_enabled: bool,
}

// ─── VoiceSessionBuilder ───────────────────────────────────────────────────────

/// Builder for a [`VoiceSession`].
///
/// Construct with [`VoiceSession::builder`], configure with the fluent
/// methods, then call [`VoiceSessionBuilder::build`] to open the WebSocket.
///
/// # Example
///
/// ```no_run
/// use grok_api::voice::{VoiceSession, TurnDetection, AudioFormat};
///
/// # #[tokio::main]
/// # async fn main() -> grok_api::Result<()> {
/// let mut session = VoiceSession::builder("your-api-key")
///     .model("grok-voice-think-fast-2.0")
///     .instructions("You are a concise assistant.")
///     .server_vad()
///     .input_format(AudioFormat::pcm(16_000))
///     .output_format(AudioFormat::pcm(24_000))
///     .build()
///     .await?;
/// # Ok(()) }
/// ```
pub struct VoiceSessionBuilder {
    api_key: String,
    model: String,
    voice: Option<String>,
    instructions: Option<String>,
    turn_detection: Option<TurnDetection>,
    reasoning_effort: Option<ReasoningEffort>,
    input_format: Option<AudioFormat>,
    output_format: Option<AudioFormat>,
    input_transport: Option<String>,
    output_transport: Option<String>,
    language_hint: Option<String>,
    keyterms: Vec<String>,
    output_speed: Option<f32>,
    resumption_enabled: bool,
}

impl VoiceSessionBuilder {
    fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: DEFAULT_VOICE_MODEL.to_string(),
            voice: None,
            instructions: None,
            turn_detection: None,
            reasoning_effort: None,
            input_format: None,
            output_format: None,
            input_transport: None,
            output_transport: None,
            language_hint: None,
            keyterms: Vec::new(),
            output_speed: None,
            resumption_enabled: false,
        }
    }

    /// Override the voice model (default: `"grok-voice-think-fast-2.0"`).
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Voice name / ID for TTS synthesis.
    pub fn voice(mut self, voice: impl Into<String>) -> Self {
        self.voice = Some(voice.into());
        self
    }

    /// System-level instructions for the session.
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    /// Enable server-side VAD.
    ///
    /// Shorthand for `.turn_detection(TurnDetection::server_vad())`.
    pub fn server_vad(self) -> Self {
        self.turn_detection(TurnDetection::server_vad())
    }

    /// Configure turn detection explicitly.
    pub fn turn_detection(mut self, td: TurnDetection) -> Self {
        self.turn_detection = Some(td);
        self
    }

    /// Set the reasoning effort for this session.
    pub fn reasoning_effort(mut self, effort: ReasoningEffort) -> Self {
        self.reasoning_effort = Some(effort);
        self
    }

    /// Set the input audio format.
    pub fn input_format(mut self, fmt: AudioFormat) -> Self {
        self.input_format = Some(fmt);
        self
    }

    /// Set the output audio format.
    pub fn output_format(mut self, fmt: AudioFormat) -> Self {
        self.output_format = Some(fmt);
        self
    }

    /// Set the input transport mode (`"json"` or `"binary"`).
    pub fn input_transport(mut self, transport: impl Into<String>) -> Self {
        self.input_transport = Some(transport.into());
        self
    }

    /// Set the output transport mode (`"json"` or `"binary"`).
    pub fn output_transport(mut self, transport: impl Into<String>) -> Self {
        self.output_transport = Some(transport.into());
        self
    }

    /// BCP-47 language hint for ASR transcription (e.g. `"en"`, `"fr"`).
    pub fn language_hint(mut self, lang: impl Into<String>) -> Self {
        self.language_hint = Some(lang.into());
        self
    }

    /// Add a single keyword to boost ASR recognition accuracy.
    pub fn keyterm(mut self, term: impl Into<String>) -> Self {
        self.keyterms.push(term.into());
        self
    }

    /// TTS output speed multiplier (0.7 – 1.5; default `1.0`).
    ///
    /// Values outside the valid range are clamped silently to avoid an API
    /// error that would be harder to diagnose.
    pub fn output_speed(mut self, speed: f32) -> Self {
        self.output_speed = Some(speed.clamp(0.7, 1.5));
        self
    }

    /// Enable session resumption.
    ///
    /// On reconnect, the `conversation_id` is passed as a query parameter so
    /// the server can replay prior conversation history.
    pub fn session_resumption(mut self) -> Self {
        self.resumption_enabled = true;
        self
    }

    /// Validate configuration, open the WebSocket, and return a ready
    /// [`VoiceSession`].
    ///
    /// # Errors
    ///
    /// - [`Error::EmptyApiKey`] — `api_key` is blank or whitespace-only.
    /// - [`Error::InvalidConfig`] — `model` is a *known* non-voice model.
    ///   Unknown model strings are allowed (future models).
    /// - [`Error::Network`] — WebSocket handshake failed (e.g. bad credentials,
    ///   server unreachable).
    pub async fn build(self) -> Result<VoiceSession> {
        // 1. Validate API key.
        if self.api_key.trim().is_empty() {
            return Err(Error::EmptyApiKey);
        }

        // 2. Guard against known non-voice models early; unknown strings are
        //    allowed (they may be future models not yet in the local Model enum).
        if let Some(m) = Model::parse(&self.model) {
            if !m.is_voice_model() {
                return Err(Error::InvalidConfig(format!(
                    "'{}' is not a voice model. Use a voice model such as '{}'.",
                    self.model, DEFAULT_VOICE_MODEL,
                )));
            }
        }

        let config = SessionConfig {
            voice: self.voice,
            instructions: self.instructions,
            turn_detection: self.turn_detection,
            reasoning_effort: self.reasoning_effort,
            input_format: self.input_format,
            output_format: self.output_format,
            input_transport: self.input_transport,
            output_transport: self.output_transport,
            language_hint: self.language_hint,
            keyterms: self.keyterms,
            output_speed: self.output_speed,
            resumption_enabled: self.resumption_enabled,
        };

        // 3. Connect — no conversation_id on a fresh session.
        let ws = VoiceSession::connect(&self.api_key, &self.model, None).await?;

        let mut session = VoiceSession {
            ws,
            api_key: self.api_key,
            model: self.model,
            config,
            conversation_id: None,
            retry_config: RetryConfig::default(),
        };

        // 4. Send initial session.update so the server applies our config.
        session.send_session_update().await?;

        Ok(session)
    }
}

// ─── VoiceSession ──────────────────────────────────────────────────────────────

/// An active real-time voice session with the xAI Speech-to-Speech API.
///
/// Obtained from [`VoiceSession::builder`]. All network I/O is async and
/// runs over a single persistent WebSocket connection. The session is
/// Starlink-aware: [`VoiceSession::reconnect`] implements exponential
/// backoff with jitter tuned for satellite drop patterns.
pub struct VoiceSession {
    ws: WsStream,
    api_key: String,
    model: String,
    config: SessionConfig,
    conversation_id: Option<String>,
    retry_config: RetryConfig,
}

impl VoiceSession {
    /// Create a [`VoiceSessionBuilder`] for the given API key.
    pub fn builder(api_key: impl Into<String>) -> VoiceSessionBuilder {
        VoiceSessionBuilder::new(api_key)
    }

    /// Receive the next server event, blocking until one arrives.
    ///
    /// Returns `Ok(None)` when the WebSocket closes cleanly. Ping, pong,
    /// and binary frames are silently discarded (tungstenite handles
    /// ping/pong replies automatically).
    pub async fn next_event(&mut self) -> Result<Option<VoiceEvent>> {
        loop {
            match self.ws.next().await {
                None => return Ok(None),

                Some(Ok(Message::Text(s))) => {
                    // Utf8Bytes implements Deref<Target=str>; coerce via &*s.
                    debug!("Voice WS received ({} bytes)", s.len());
                    let value: serde_json::Value = serde_json::from_str(&s)?;
                    let event = parse_voice_event(value, &mut self.conversation_id);
                    if let VoiceEvent::Error {
                        ref code,
                        ref message,
                    } = event
                    {
                        warn!("Voice API error [{}]: {}", code, message);
                    }
                    return Ok(Some(event));
                }

                Some(Ok(Message::Close(_))) => return Ok(None),

                // Ping / Pong / Binary / Frame — handled internally or ignored.
                Some(Ok(_)) => continue,

                Some(Err(e)) => {
                    let msg = e.to_string();
                    let lower = msg.to_lowercase();
                    if lower.contains("connection reset")
                        || lower.contains("broken pipe")
                        || lower.contains("connection closed")
                    {
                        return Err(Error::ConnectionDropped);
                    }
                    return Err(Error::Network(msg));
                }
            }
        }
    }

    /// Send a plain-text user turn (no audio required).
    ///
    /// Useful for hybrid or test workflows that mix text and voice.
    pub async fn send_text(&mut self, text: impl Into<String>) -> Result<()> {
        let text = text.into();
        debug!("Sending text turn ({} chars)", text.len());
        self.send_json(json!({
            "type": "conversation.item.create",
            "item": {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": text}]
            }
        }))
        .await
    }

    /// Append a chunk of base64-encoded audio to the server-side input buffer.
    pub async fn append_audio(&mut self, audio_base64: impl Into<String>) -> Result<()> {
        self.send_json(json!({
            "type": "input_audio_buffer.append",
            "audio": audio_base64.into()
        }))
        .await
    }

    /// Commit the accumulated audio buffer, signalling end-of-turn.
    ///
    /// Only meaningful when using [`TurnDetectionKind::Manual`].
    pub async fn commit_audio(&mut self) -> Result<()> {
        debug!("Committing audio buffer");
        self.send_json(json!({"type": "input_audio_buffer.commit"}))
            .await
    }

    /// Discard buffered audio that has not yet been committed.
    pub async fn clear_audio(&mut self) -> Result<()> {
        debug!("Clearing audio buffer");
        self.send_json(json!({"type": "input_audio_buffer.clear"}))
            .await
    }

    /// Ask the model to generate a response from the current conversation.
    pub async fn request_response(&mut self) -> Result<()> {
        debug!("Requesting model response");
        self.send_json(json!({"type": "response.create"})).await
    }

    /// Like [`VoiceSession::request_response`], but override system
    /// instructions for this response turn only.
    pub async fn request_response_with_instructions(
        &mut self,
        instructions: impl Into<String>,
    ) -> Result<()> {
        let instructions = instructions.into();
        debug!(
            "Requesting response with instructions override ({} chars)",
            instructions.len()
        );
        self.send_json(json!({
            "type": "response.create",
            "response": {"instructions": instructions}
        }))
        .await
    }

    /// Return the result of a function call to the model.
    pub async fn send_function_result(
        &mut self,
        call_id: impl Into<String>,
        output: impl Into<String>,
    ) -> Result<()> {
        let call_id = call_id.into();
        let output = output.into();
        debug!("Sending function result for call_id={}", call_id);
        self.send_json(json!({
            "type": "conversation.item.create",
            "item": {
                "type": "function_call_output",
                "call_id": call_id,
                "output": output
            }
        }))
        .await
    }

    /// Interrupt the model's current audio output.
    pub async fn interrupt(&mut self) -> Result<()> {
        debug!("Interrupting model response");
        self.send_json(json!({"type": "response.cancel"})).await
    }

    /// Speak a hard-coded line of text via TTS, bypassing model generation.
    ///
    /// This uses the xAI `force_message` extension.
    /// Set `interruptible = true` to allow the user to cut off playback.
    pub async fn force_message(
        &mut self,
        text: impl Into<String>,
        interruptible: bool,
    ) -> Result<()> {
        let text = text.into();
        debug!(
            "force_message: {} chars, interruptible={}",
            text.len(),
            interruptible
        );
        self.send_json(json!({
            "type": "force_message",
            "text": text,
            "interruptible": interruptible
        }))
        .await
    }

    /// Re-send the current session configuration mid-session.
    ///
    /// Call this after you have changed builder-originated parameters
    /// that should take effect immediately.
    pub async fn update_session(&mut self) -> Result<()> {
        debug!("Re-sending session.update");
        self.send_session_update().await
    }

    /// The server-assigned conversation ID.
    ///
    /// Returns `None` until a [`VoiceEvent::ConversationCreated`] event has
    /// been received.
    pub fn conversation_id(&self) -> Option<&str> {
        self.conversation_id.as_deref()
    }

    /// Reconnect after a WebSocket drop using exponential backoff with jitter.
    ///
    /// The first reconnect attempt is made **immediately** (no initial sleep).
    /// Subsequent attempts use exponential backoff capped at `max_delay_ms`.
    /// When `session_resumption` was enabled on the builder, the existing
    /// `conversation_id` is passed to the server so it can replay history.
    ///
    /// Returns [`Error::MaxRetriesExceeded`] once all retry attempts are
    /// exhausted. The backoff is deliberately generous to handle Starlink
    /// satellite connection gaps.
    pub async fn reconnect(&mut self) -> Result<()> {
        let mut attempt = 0u32;
        loop {
            let conv_id = if self.config.resumption_enabled {
                self.conversation_id.as_deref()
            } else {
                None
            };

            match Self::connect(&self.api_key, &self.model, conv_id).await {
                Ok(ws) => {
                    self.ws = ws;
                    self.send_session_update().await?;
                    info!("Voice WebSocket reconnected on attempt {}", attempt + 1);
                    return Ok(());
                }
                Err(e) => {
                    attempt += 1;
                    if attempt >= self.retry_config.max_retries {
                        warn!(
                            "Voice WebSocket: all {} reconnect attempts exhausted",
                            self.retry_config.max_retries
                        );
                        return Err(Error::MaxRetriesExceeded(self.retry_config.max_retries));
                    }

                    // Exponential backoff capped at max_delay_ms.
                    let base = self.retry_config.base_delay_ms;
                    let delay_ms = base
                        .saturating_mul(2u64.saturating_pow(attempt.saturating_sub(1)))
                        .min(self.retry_config.max_delay_ms);
                    let delay_ms = if self.retry_config.use_jitter {
                        let jitter = rand::rng().random_range(0..=(delay_ms / 4));
                        delay_ms + jitter
                    } else {
                        delay_ms
                    };

                    warn!(
                        "Voice WebSocket reconnect attempt {} failed (backoff {}ms): {}",
                        attempt, delay_ms, e
                    );
                    sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }
    }

    /// Close the WebSocket session cleanly by sending a WebSocket close frame.
    ///
    /// Call this before dropping the session when a clean close is needed.
    /// If not called, the TCP connection is closed at the OS level on drop
    /// (the remote sees a TCP RST rather than a WebSocket close handshake).
    pub async fn close(mut self) -> Result<()> {
        debug!("Closing voice WebSocket cleanly");
        self.ws
            .close(None)
            .await
            .map_err(|e| Error::Network(e.to_string()))
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Serialize `value` to JSON and send it as a WebSocket text frame.
    async fn send_json(&mut self, value: serde_json::Value) -> Result<()> {
        let text = serde_json::to_string(&value)?;
        debug!("Voice WS sending ({} bytes)", text.len());
        self.ws
            .send(Message::Text(text.into()))
            .await
            .map_err(|e| Error::Network(e.to_string()))
    }

    /// Open a WebSocket connection to the xAI real-time endpoint.
    ///
    /// Attaches `Authorization: Bearer <api_key>`. Query parameters are
    /// percent-encoded via the `url` crate to prevent injection. If
    /// `conversation_id` is provided it is appended for session resumption.
    async fn connect(
        api_key: &str,
        model: &str,
        conversation_id: Option<&str>,
    ) -> Result<WsStream> {
        // Build the URL with properly encoded query parameters (S3).
        let mut url = Url::parse(REALTIME_URL)
            .map_err(|e| Error::InvalidConfig(format!("Invalid realtime URL: {e}")))?
            .clone();
        {
            let mut q = url.query_pairs_mut();
            q.append_pair("model", model);
            if let Some(conv_id) = conversation_id {
                q.append_pair("conversation_id", conv_id);
            }
        }

        debug!("Opening voice WebSocket: {}", url.as_str());

        let auth_value = HeaderValue::from_str(&format!("Bearer {api_key}"))
            .map_err(|e| Error::InvalidConfig(e.to_string()))?;

        let mut request = url
            .as_str()
            .into_client_request()
            .map_err(|e| Error::InvalidConfig(e.to_string()))?;

        request.headers_mut().insert("Authorization", auth_value);

        let (ws, _response) = connect_async(request)
            .await
            .map_err(|e| Error::Network(e.to_string()))?;

        debug!("Voice WebSocket handshake complete");
        Ok(ws)
    }

    /// Serialise the current [`SessionConfig`] and dispatch a `session.update`.
    async fn send_session_update(&mut self) -> Result<()> {
        let payload = build_session_payload(&self.config);
        self.send_json(json!({
            "type": "session.update",
            "session": payload
        }))
        .await
    }
}

// ─── JSON helpers ──────────────────────────────────────────────────────────────

/// Build the `session` JSON object for a `session.update` message.
fn build_session_payload(config: &SessionConfig) -> serde_json::Value {
    let mut obj = serde_json::Map::new();

    if let Some(ref v) = config.voice {
        obj.insert("voice".into(), json!(v));
    }
    if let Some(ref i) = config.instructions {
        obj.insert("instructions".into(), json!(i));
    }

    if let Some(ref td) = config.turn_detection {
        let kind = match td.kind {
            TurnDetectionKind::ServerVad => "server_vad",
            TurnDetectionKind::Manual => "none",
        };
        let mut td_obj = serde_json::Map::new();
        td_obj.insert("type".into(), json!(kind));
        if let Some(t) = td.threshold {
            td_obj.insert("threshold".into(), json!(t));
        }
        if let Some(s) = td.silence_duration_ms {
            td_obj.insert("silence_duration_ms".into(), json!(s));
        }
        if let Some(p) = td.prefix_padding_ms {
            td_obj.insert("prefix_padding_ms".into(), json!(p));
        }
        if let Some(i) = td.idle_timeout_ms {
            td_obj.insert("idle_timeout_ms".into(), json!(i));
        }
        obj.insert("turn_detection".into(), serde_json::Value::Object(td_obj));
    }

    if let Some(ref effort) = config.reasoning_effort {
        let s = match effort {
            ReasoningEffort::High => "high",
            ReasoningEffort::None => "none",
        };
        obj.insert("reasoning".into(), json!({"effort": s}));
    }

    if let Some(ref fmt) = config.input_format {
        let mut f = serde_json::Map::new();
        f.insert("codec".into(), json!(fmt.codec_str()));
        if let Some(r) = fmt.rate {
            f.insert("sample_rate".into(), json!(r));
        }
        obj.insert("input_audio_format".into(), serde_json::Value::Object(f));
    }

    if let Some(ref fmt) = config.output_format {
        let mut f = serde_json::Map::new();
        f.insert("codec".into(), json!(fmt.codec_str()));
        if let Some(r) = fmt.rate {
            f.insert("sample_rate".into(), json!(r));
        }
        obj.insert("output_audio_format".into(), serde_json::Value::Object(f));
    }

    if let Some(ref t) = config.input_transport {
        obj.insert("input_audio_transcription".into(), json!({"transport": t}));
    }
    if let Some(ref t) = config.output_transport {
        obj.insert("output_audio_transport".into(), json!(t));
    }
    if let Some(ref l) = config.language_hint {
        obj.insert("language".into(), json!(l));
    }
    if !config.keyterms.is_empty() {
        obj.insert("keyterms".into(), json!(config.keyterms));
    }
    if let Some(s) = config.output_speed {
        obj.insert("output_audio_speed".into(), json!(s));
    }

    serde_json::Value::Object(obj)
}

/// Parse a server-sent event JSON value into a [`VoiceEvent`].
///
/// Updates `conversation_id` when a `conversation.created` event is seen.
fn parse_voice_event(event: serde_json::Value, conversation_id: &mut Option<String>) -> VoiceEvent {
    let event_type = event["type"].as_str().unwrap_or("").to_string();

    match event_type.as_str() {
        "session.created" => VoiceEvent::SessionCreated {
            session_id: event["session"]["id"].as_str().unwrap_or("").to_string(),
        },

        "session.updated" => VoiceEvent::SessionUpdated,

        "conversation.created" => {
            let id = event["conversation"]["id"]
                .as_str()
                .unwrap_or("")
                .to_string();
            *conversation_id = Some(id.clone());
            VoiceEvent::ConversationCreated {
                conversation_id: id,
            }
        }

        "response.output_audio.delta" => VoiceEvent::AudioDelta {
            item_id: event["item_id"].as_str().unwrap_or("").to_string(),
            delta: event["delta"].as_str().unwrap_or("").to_string(),
        },

        "response.output_audio.done" => VoiceEvent::AudioDone {
            item_id: event["item_id"].as_str().unwrap_or("").to_string(),
        },

        "response.audio_transcript.delta" => VoiceEvent::TranscriptDelta {
            item_id: event["item_id"].as_str().unwrap_or("").to_string(),
            delta: event["delta"].as_str().unwrap_or("").to_string(),
        },

        "response.audio_transcript.done" => VoiceEvent::TranscriptDone {
            item_id: event["item_id"].as_str().unwrap_or("").to_string(),
            transcript: event["transcript"].as_str().unwrap_or("").to_string(),
        },

        "conversation.item.input_audio_transcription.updated" => {
            VoiceEvent::InputTranscriptUpdated {
                item_id: event["item_id"].as_str().unwrap_or("").to_string(),
                transcript: event["transcript"].as_str().unwrap_or("").to_string(),
            }
        }

        "response.function_call_arguments.done" => VoiceEvent::FunctionCall {
            name: event["name"].as_str().unwrap_or("").to_string(),
            call_id: event["call_id"].as_str().unwrap_or("").to_string(),
            arguments: event["arguments"].as_str().unwrap_or("").to_string(),
        },

        "response.done" => {
            let usage = parse_usage(&event["response"]["usage"]);
            VoiceEvent::Done { usage }
        }

        "error" => VoiceEvent::Error {
            code: event["error"]["code"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            message: event["error"]["message"].as_str().unwrap_or("").to_string(),
        },

        _ => VoiceEvent::Other { event_type },
    }
}

/// Parse a `usage` JSON object into a [`VoiceUsage`].
///
/// Returns `None` if the value is `null` or not an object.
fn parse_usage(usage: &serde_json::Value) -> Option<VoiceUsage> {
    if !usage.is_object() {
        return None;
    }
    Some(VoiceUsage {
        input_tokens: usage["input_tokens"].as_u64().map(|n| n as u32),
        output_tokens: usage["output_tokens"].as_u64().map(|n| n as u32),
        total_tokens: usage["total_tokens"].as_u64().map(|n| n as u32),
    })
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── AudioFormat ──────────────────────────────────────────────────────────

    #[test]
    fn test_audio_codec_strings() {
        assert_eq!(AudioFormat::pcm(24_000).codec_str(), "audio/pcm");
        assert_eq!(AudioFormat::pcmu().codec_str(), "audio/pcmu");
        assert_eq!(AudioFormat::pcma().codec_str(), "audio/pcma");
        assert_eq!(AudioFormat::opus().codec_str(), "audio/opus");

        // Sample rate stored and exposed correctly.
        assert_eq!(AudioFormat::pcm(16_000).rate(), Some(16_000));
        assert_eq!(AudioFormat::opus().rate(), None);
    }

    // ── TurnDetection ────────────────────────────────────────────────────────

    #[test]
    fn test_turn_detection_server_vad() {
        let td = TurnDetection::server_vad();
        assert_eq!(td.kind, TurnDetectionKind::ServerVad);
        assert!(td.threshold.is_none());
        assert!(td.silence_duration_ms.is_none());
        assert!(td.prefix_padding_ms.is_none());
        assert!(td.idle_timeout_ms.is_none());
    }

    #[test]
    fn test_turn_detection_manual() {
        let td = TurnDetection::manual();
        assert_eq!(td.kind, TurnDetectionKind::Manual);
        assert!(td.threshold.is_none());
    }

    #[test]
    fn test_turn_detection_with_threshold() {
        let td = TurnDetection::server_vad()
            .threshold(0.7)
            .silence_duration_ms(300)
            .prefix_padding_ms(100)
            .idle_timeout_ms(10_000);

        assert_eq!(td.kind, TurnDetectionKind::ServerVad);
        assert_eq!(td.threshold, Some(0.7));
        assert_eq!(td.silence_duration_ms, Some(300));
        assert_eq!(td.prefix_padding_ms, Some(100));
        assert_eq!(td.idle_timeout_ms, Some(10_000));
    }

    // ── VoiceSessionBuilder validation ───────────────────────────────────────

    #[tokio::test]
    async fn test_builder_rejects_empty_api_key() {
        let result = VoiceSession::builder("").build().await;
        assert!(matches!(result, Err(crate::error::Error::EmptyApiKey)));
    }

    #[tokio::test]
    async fn test_builder_rejects_non_voice_model() {
        // grok-4.5 is a well-known language model — not a voice model.
        let result = VoiceSession::builder("test-api-key")
            .model("grok-4.5")
            .build()
            .await;

        assert!(result.is_err());
        assert!(
            matches!(result, Err(crate::error::Error::InvalidConfig(_))),
            "expected InvalidConfig for non-voice model"
        );
    }

    /// Integration test: requires live network access to `api.x.ai`.
    /// Run with `cargo test -- --ignored test_builder_accepts_valid_voice_model_string`.
    #[tokio::test]
    #[ignore = "requires live network access to wss://api.x.ai"]
    async fn test_builder_accepts_valid_voice_model_string() {
        // The validation passes for a valid voice model string.
        // The subsequent connect() will fail (no real server), but we only
        // care that InvalidConfig is NOT returned.
        let result = VoiceSession::builder("test-api-key")
            .model("grok-voice-think-fast-2.0")
            .build()
            .await;

        // Should fail with a Network error (no real server), not InvalidConfig.
        assert!(result.is_err());
        assert!(
            !matches!(result, Err(crate::error::Error::InvalidConfig(_))),
            "should not be InvalidConfig for a valid voice model"
        );
    }

    // ── Event parsing ────────────────────────────────────────────────────────

    #[test]
    fn test_voice_event_parse_session_created() {
        let json = serde_json::json!({
            "type": "session.created",
            "session": {"id": "sess_abc123"}
        });
        let mut conv_id = None;
        match parse_voice_event(json, &mut conv_id) {
            VoiceEvent::SessionCreated { session_id } => {
                assert_eq!(session_id, "sess_abc123");
            }
            other => panic!("Expected SessionCreated, got {:?}", other),
        }
    }

    #[test]
    fn test_voice_event_parse_conversation_created() {
        let json = serde_json::json!({
            "type": "conversation.created",
            "conversation": {"id": "conv_xyz"}
        });
        let mut conv_id: Option<String> = None;
        match parse_voice_event(json, &mut conv_id) {
            VoiceEvent::ConversationCreated { conversation_id } => {
                assert_eq!(conversation_id, "conv_xyz");
            }
            other => panic!("Expected ConversationCreated, got {:?}", other),
        }
        // Side-effect: the shared state should be updated.
        assert_eq!(conv_id.as_deref(), Some("conv_xyz"));
    }

    #[test]
    fn test_voice_event_parse_audio_delta() {
        let json = serde_json::json!({
            "type": "response.output_audio.delta",
            "item_id": "item_abc",
            "delta": "SGVsbG8gV29ybGQ="
        });
        let mut conv_id = None;
        match parse_voice_event(json, &mut conv_id) {
            VoiceEvent::AudioDelta { item_id, delta } => {
                assert_eq!(item_id, "item_abc");
                assert_eq!(delta, "SGVsbG8gV29ybGQ=");
            }
            other => panic!("Expected AudioDelta, got {:?}", other),
        }
        assert!(conv_id.is_none());
    }

    #[test]
    fn test_voice_event_parse_done_with_usage() {
        let json = serde_json::json!({
            "type": "response.done",
            "response": {
                "usage": {
                    "input_tokens": 120,
                    "output_tokens": 80,
                    "total_tokens": 200
                }
            }
        });
        let mut conv_id = None;
        match parse_voice_event(json, &mut conv_id) {
            VoiceEvent::Done { usage: Some(u) } => {
                assert_eq!(u.input_tokens, Some(120));
                assert_eq!(u.output_tokens, Some(80));
                assert_eq!(u.total_tokens, Some(200));
            }
            other => panic!("Expected Done with usage, got {:?}", other),
        }
    }

    #[test]
    fn test_voice_event_parse_done_no_usage() {
        let json = serde_json::json!({
            "type": "response.done",
            "response": {}
        });
        let mut conv_id = None;
        match parse_voice_event(json, &mut conv_id) {
            VoiceEvent::Done { usage: None } => {}
            other => panic!("Expected Done with no usage, got {:?}", other),
        }
    }

    #[test]
    fn test_voice_event_parse_error() {
        let json = serde_json::json!({
            "type": "error",
            "error": {
                "code": "auth_error",
                "message": "Invalid API key"
            }
        });
        let mut conv_id = None;
        let event = parse_voice_event(json, &mut conv_id);
        assert!(
            matches!(&event, VoiceEvent::Error { code, .. } if code == "auth_error"),
            "expected Error event with code=auth_error"
        );
    }

    #[test]
    fn test_voice_event_parse_unknown_type() {
        let json = serde_json::json!({"type": "mystery.event.v9"});
        let mut conv_id = None;
        let event = parse_voice_event(json, &mut conv_id);
        assert!(
            matches!(&event, VoiceEvent::Other { event_type } if event_type == "mystery.event.v9")
        );
    }

    // ── Session payload builder ───────────────────────────────────────────────

    #[test]
    fn test_session_payload_minimal() {
        let config = SessionConfig {
            voice: None,
            instructions: None,
            turn_detection: None,
            reasoning_effort: None,
            input_format: None,
            output_format: None,
            input_transport: None,
            output_transport: None,
            language_hint: None,
            keyterms: Vec::new(),
            output_speed: None,
            resumption_enabled: false,
        };
        let payload = build_session_payload(&config);
        // Minimal config produces an empty object.
        assert!(payload.as_object().unwrap().is_empty());
    }

    #[test]
    fn test_session_payload_with_all_params() {
        let config = SessionConfig {
            voice: Some("sage".to_string()),
            instructions: Some("Be brief.".to_string()),
            turn_detection: Some(
                TurnDetection::server_vad()
                    .threshold(0.5)
                    .silence_duration_ms(200)
                    .prefix_padding_ms(50),
            ),
            reasoning_effort: Some(ReasoningEffort::High),
            input_format: Some(AudioFormat::pcm(16_000)),
            output_format: Some(AudioFormat::pcm(24_000)),
            input_transport: Some("json".to_string()),
            output_transport: Some("binary".to_string()),
            language_hint: Some("en".to_string()),
            keyterms: vec!["Rust".to_string(), "Cargo".to_string()],
            output_speed: Some(1.2),
            resumption_enabled: true,
        };

        let p = build_session_payload(&config);

        assert_eq!(p["voice"], "sage");
        assert_eq!(p["instructions"], "Be brief.");

        assert_eq!(p["turn_detection"]["type"], "server_vad");
        assert_eq!(p["turn_detection"]["threshold"], 0.5);
        assert_eq!(p["turn_detection"]["silence_duration_ms"], 200);
        assert_eq!(p["turn_detection"]["prefix_padding_ms"], 50);

        assert_eq!(p["reasoning"]["effort"], "high");

        assert_eq!(p["input_audio_format"]["codec"], "audio/pcm");
        assert_eq!(p["input_audio_format"]["sample_rate"], 16_000);
        assert_eq!(p["output_audio_format"]["codec"], "audio/pcm");
        assert_eq!(p["output_audio_format"]["sample_rate"], 24_000);

        assert_eq!(p["language"], "en");
        assert_eq!(p["keyterms"][0], "Rust");
        assert_eq!(p["keyterms"][1], "Cargo");

        let speed = p["output_audio_speed"].as_f64().unwrap();
        assert!((speed - 1.2_f64).abs() < 1e-6);
    }

    #[test]
    fn test_session_payload_manual_turn_detection() {
        let config = SessionConfig {
            turn_detection: Some(TurnDetection::manual()),
            voice: None,
            instructions: None,
            reasoning_effort: None,
            input_format: None,
            output_format: None,
            input_transport: None,
            output_transport: None,
            language_hint: None,
            keyterms: Vec::new(),
            output_speed: None,
            resumption_enabled: false,
        };
        let p = build_session_payload(&config);
        assert_eq!(p["turn_detection"]["type"], "none");
    }

    #[test]
    fn test_session_payload_reasoning_effort_none() {
        let config = SessionConfig {
            reasoning_effort: Some(ReasoningEffort::None),
            voice: None,
            instructions: None,
            turn_detection: None,
            input_format: None,
            output_format: None,
            input_transport: None,
            output_transport: None,
            language_hint: None,
            keyterms: Vec::new(),
            output_speed: None,
            resumption_enabled: false,
        };
        let p = build_session_payload(&config);
        assert_eq!(p["reasoning"]["effort"], "none");
    }
}
