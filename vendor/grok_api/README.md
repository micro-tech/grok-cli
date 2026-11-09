# Vendored grok_api 0.2.4 — TEMPORARY (review H5, Nov 6 M5)

This directory vendors the published `grok_api` 0.2.4 crate (from crates.io)
with ONE backported fix. It exists only because the fix cannot be obtained
any other way right now:

- **The fix (H5):** the published 0.2.4 maps an HTTP 429 to a bare
  `Error::RateLimit` unit variant, discarding the response body AND the
  `Retry-After` header. This vendor preserves both: `RateLimit` now carries
  `body: String` and `retry_after_secs: Option<u64>`, and `client.rs` reads
  the `Retry-After` header before consuming the body. That is what lets
  grok-cli's backend (`src/router/backends/grok.rs`, H4) honor `Retry-After`
  instead of guessing a backoff.
- **Why not 0.3.1:** `grok_api` 0.3.1 is NOT published on crates.io (latest
  there is 0.2.4, 2026-09-23). It exists only as a commented-out local path
  dep. Publishing 0.3.1 is John's call (his private crate).
- **Why not the path dep:** the commented-out `path = "../grok_crate/grok_api"`
  does not exist in this checkout and would break the build for anyone
  without that exact sibling layout.
- **Why not just wait:** the 429 body is needed for H4's Retry-After
  handling, which is part of the Nov 6 must-fix batch.

## What is vendored

- `src/*.rs` — the 0.2.4 library source, line endings normalized to LF.
- `Cargo.toml` — based on the published `Cargo.toml.orig`, with examples
  and dev-dependencies omitted (only the library is vendored).
- `LICENSE.txt` — the crate's license file.

Only `src/error.rs` and `src/client.rs` differ from the published 0.2.4
(the `RateLimit` variant shape and the 429 branch). Everything else is
byte-identical to the published source apart from line endings.

## Removal

When `grok_api` 0.3.1 (or later) is published to crates.io with the 429
body preserved, delete this directory, remove the `[patch.crates-io]`
section from the workspace `Cargo.toml`, bump the `grok_api` pin, and
delete this README. Do NOT publish anything from this vendor.
