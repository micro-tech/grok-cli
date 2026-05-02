//! Backend implementations for the CPU router.
//!
//! Currently only the Grok (xAI) backend is implemented.
//! Additional backends (Ollama, Gemini, etc.) can be added here in the future
//! by following the same pattern as [`GrokBackend`].

pub mod grok;

pub use grok::GrokBackend;
