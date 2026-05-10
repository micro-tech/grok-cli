// fix_acp.mjs — applies all four fix sets to src/cli/commands/acp.rs
import { readFileSync, writeFileSync } from 'fs';

const path = 'src/cli/commands/acp.rs';
let src = readFileSync(path, 'utf8').replace(/\r\n/g, '\n');
const originalLen = src.length;
let changes = [];

// ─────────────────────────────────────────────────────────────────────────────
// FIX 1 — Replace all format!("") with e.to_string() inside Error::new(-32603)
// ─────────────────────────────────────────────────────────────────────────────
const bad_err = 'agent_client_protocol::Error::new(-32603, format!(""))';
const good_err = 'agent_client_protocol::Error::new(-32603, e.to_string())';
const f1count = (src.split(bad_err).length - 1);
src = src.split(bad_err).join(good_err);
changes.push(`Fix 1: replaced ${f1count} occurrences of format!("") in Error::new`);

// ─────────────────────────────────────────────────────────────────────────────
// FIX 2a — builtin block: if let Ok(crate_notif) → match  (in handle_session_prompt_v2)
// After Fix 1 the cx.send_notification call inside already uses e.to_string()
// ─────────────────────────────────────────────────────────────────────────────
const old2a = `            if let Ok(crate_notif) = local_notif_to_crate(&notif) {
                cx.send_notification(crate_notif)
                    .map_err(|e| agent_client_protocol::Error::new(-32603, e.to_string()))?;
            }
            agent.save_session_to_disk(&session_id).await.ok();
            return responder
                .respond(agent_client_protocol::schema::PromptResponse::new(
                    agent_client_protocol::schema::StopReason::EndTurn,
                ))
                .map_err(|e| agent_client_protocol::Error::new(-32603, e.to_string()));
        }

        // AI-assisted slash command`;
const new2a = `            match local_notif_to_crate(&notif) {
                Ok(crate_notif) => {
                    if let Err(e) = cx.send_notification(crate_notif) {
                        warn!("send_notification failed: {e}");
                    }
                }
                Err(e) => warn!("local_notif_to_crate failed (notification dropped): {e}"),
            }
            agent.save_session_to_disk(&session_id).await.ok();
            return responder
                .respond(agent_client_protocol::schema::PromptResponse::new(
                    agent_client_protocol::schema::StopReason::EndTurn,
                ))
                .map_err(|e| agent_client_protocol::Error::new(-32603, e.to_string()));
        }

        // AI-assisted slash command`;
if (!src.includes(old2a)) { console.error('Fix 2a: PATTERN NOT FOUND'); process.exit(1); }
src = src.replace(old2a, new2a);
changes.push('Fix 2a: builtin if-let-Ok → match in handle_session_prompt_v2');

// ─────────────────────────────────────────────────────────────────────────────
// FIX 2b — slash AI block: if let Ok(crate_notif) → match
// ─────────────────────────────────────────────────────────────────────────────
const old2b = `            if let Ok(crate_notif) = local_notif_to_crate(&notif) {
                cx.send_notification(crate_notif)
                    .map_err(|e| agent_client_protocol::Error::new(-32603, e.to_string()))?;
            }
            agent.save_session_to_disk(&session_id).await.ok();
            return responder
                .respond(agent_client_protocol::schema::PromptResponse::new(
                    agent_client_protocol::schema::StopReason::EndTurn,
                ))
                .map_err(|e| agent_client_protocol::Error::new(-32603, e.to_string()));
        }
    }

    // ── Normal AI chat ────────────────────────────────────────────────────────`;
const new2b = `            match local_notif_to_crate(&notif) {
                Ok(crate_notif) => {
                    if let Err(e) = cx.send_notification(crate_notif) {
                        warn!("send_notification failed: {e}");
                    }
                }
                Err(e) => warn!("local_notif_to_crate failed (notification dropped): {e}"),
            }
            agent.save_session_to_disk(&session_id).await.ok();
            return responder
                .respond(agent_client_protocol::schema::PromptResponse::new(
                    agent_client_protocol::schema::StopReason::EndTurn,
                ))
                .map_err(|e| agent_client_protocol::Error::new(-32603, e.to_string()));
        }
    }

    // ── Normal AI chat ────────────────────────────────────────────────────────`;
if (!src.includes(old2b)) { console.error('Fix 2b: PATTERN NOT FOUND'); process.exit(1); }
src = src.replace(old2b, new2b);
changes.push('Fix 2b: slash-AI if-let-Ok → match in handle_session_prompt_v2');

// ─────────────────────────────────────────────────────────────────────────────
// FIX 2c — normal AI block: if let Ok(crate_notif) → match
// ─────────────────────────────────────────────────────────────────────────────
const old2c = `    if let Ok(crate_notif) = local_notif_to_crate(&notif) {
        cx.send_notification(crate_notif)
            .map_err(|e| agent_client_protocol::Error::new(-32603, e.to_string()))?;
    }

    agent.save_session_to_disk(&session_id).await.ok();`;
const new2c = `    match local_notif_to_crate(&notif) {
        Ok(crate_notif) => {
            if let Err(e) = cx.send_notification(crate_notif) {
                warn!("send_notification failed: {e}");
            }
        }
        Err(e) => warn!("local_notif_to_crate failed (notification dropped): {e}"),
    }

    agent.save_session_to_disk(&session_id).await.ok();`;
if (!src.includes(old2c)) { console.error('Fix 2c: PATTERN NOT FOUND'); process.exit(1); }
src = src.replace(old2c, new2c);
changes.push('Fix 2c: normal-AI if-let-Ok → match in handle_session_prompt_v2');

// ─────────────────────────────────────────────────────────────────────────────
// FIX 2d — run_ai_and_collect: if let Ok(crate_notif) → match
// ─────────────────────────────────────────────────────────────────────────────
const old2d = `                    if let Ok(crate_notif) = local_notif_to_crate(&notif) {
                        if let Err(e) = cx.send_notification(crate_notif) {
                            warn!("send_notification error: {e}");
                        }
                    }`;
const new2d = `                    match local_notif_to_crate(&notif) {
                        Ok(crate_notif) => {
                            if let Err(e) = cx.send_notification(crate_notif) {
                                warn!("send_notification failed: {e}");
                            }
                        }
                        Err(e) => warn!("local_notif_to_crate failed (notification dropped): {e}"),
                    }`;
if (!src.includes(old2d)) { console.error('Fix 2d: PATTERN NOT FOUND'); process.exit(1); }
src = src.replace(old2d, new2d);
changes.push('Fix 2d: run_ai_and_collect if-let-Ok → match');

// ─────────────────────────────────────────────────────────────────────────────
// FIX 3a — Delete handle_untyped_dispatch + handle_json_rpc (both dead code)
//           Delete from the comment block before untyped_dispatch through to
//           just before the "/// Resolve a raw workspace path" doc-comment.
// ─────────────────────────────────────────────────────────────────────────────
const dead_start = '\n// ---------------------------------------------------------------------------\n// 111.3 untyped catch-all: session/fork, session/set_model, and any method';
const dead_end   = '\n/// Resolve a raw workspace path string';
const si_dead = src.indexOf(dead_start);
const ei_dead = src.indexOf(dead_end, si_dead);
if (si_dead < 0 || ei_dead < 0) {
  console.error(`Fix 3a: markers not found (si=${si_dead}, ei=${ei_dead})`); process.exit(1);
}
src = src.slice(0, si_dead) + src.slice(ei_dead);
changes.push('Fix 3a: deleted handle_untyped_dispatch + handle_json_rpc');

// ─────────────────────────────────────────────────────────────────────────────
// FIX 3b — Remove send_available_commands_update call from handle_session_load
// ─────────────────────────────────────────────────────────────────────────────
const old3b = `
    // Re-advertise slash commands so the client's command palette is populated.
    if let Err(e) = send_available_commands_update(writer, &session_id_str).await {
        warn!(
            "session/load: failed to send available_commands_update: {}",
            e
        );
    }

    // Per the ACP spec the agent MUST respond with null when done replaying.`;
const new3b = `
    // Per the ACP spec the agent MUST respond with null when done replaying.`;
if (!src.includes(old3b)) { console.error('Fix 3b: PATTERN NOT FOUND'); process.exit(1); }
src = src.replace(old3b, new3b);
changes.push('Fix 3b: removed send_available_commands_update call from handle_session_load');

// ─────────────────────────────────────────────────────────────────────────────
// FIX 3c — Rename writer → _writer in handle_session_load signature
//           Also update the doc-comment that referenced "Re-send available_commands_update"
// ─────────────────────────────────────────────────────────────────────────────
const old3c_sig = `async fn handle_session_load<W>(
    params: &Value,
    agent: &GrokAcpAgent,
    writer: &mut W,
) -> Result<Value>
where
    W: tokio::io::AsyncWrite + Unpin,`;
const new3c_sig = `async fn handle_session_load<W>(
    params: &Value,
    agent: &GrokAcpAgent,
    _writer: &mut W, // kept for API compatibility; caller sends notifications via cx
) -> Result<Value>
where
    W: tokio::io::AsyncWrite + Unpin,`;
if (!src.includes(old3c_sig)) { console.error('Fix 3c: signature PATTERN NOT FOUND'); process.exit(1); }
src = src.replace(old3c_sig, new3c_sig);
// Also patch the doc-comment bullet that says "Re-send available_commands_update"
src = src.replace(
  '///   3. Re-send `available_commands_update` so the client has the command list.',
  '///   3. Caller sends `available_commands_update` via cx after this function returns.'
);
changes.push('Fix 3c: writer → _writer in handle_session_load + doc-comment updated');

// ─────────────────────────────────────────────────────────────────────────────
// FIX 3d — Delete old handle_session_prompt<W> (writer-based, ~430 lines)
//           From "\nasync fn handle_session_prompt<W>(" to just before
//           "\n/// Send an `available_commands_update` notification"
// ─────────────────────────────────────────────────────────────────────────────
const sp_start = '\nasync fn handle_session_prompt<W>(';
const sp_end   = '\n/// Send an `available_commands_update` notification';
const si_sp = src.indexOf(sp_start);
const ei_sp = src.indexOf(sp_end, si_sp);
if (si_sp < 0 || ei_sp < 0) {
  console.error(`Fix 3d: markers not found (si=${si_sp}, ei=${ei_sp})`); process.exit(1);
}
src = src.slice(0, si_sp) + src.slice(ei_sp);
changes.push('Fix 3d: deleted old handle_session_prompt<W>');

// ─────────────────────────────────────────────────────────────────────────────
// FIX 3e — Delete send_available_commands_update<W> and send_text_update<W>
//           From "\n/// Send an `available_commands_update` notification" to just
//           before "\n/// Test ACP connection to a running server"
// ─────────────────────────────────────────────────────────────────────────────
const acu_start = '\n/// Send an `available_commands_update` notification';
const acu_end   = '\n/// Test ACP connection to a running server';
const si_acu = src.indexOf(acu_start);
const ei_acu = src.indexOf(acu_end, si_acu);
if (si_acu < 0 || ei_acu < 0) {
  console.error(`Fix 3e: markers not found (si=${si_acu}, ei=${ei_acu})`); process.exit(1);
}
src = src.slice(0, si_acu) + src.slice(ei_acu);
changes.push('Fix 3e: deleted send_available_commands_update<W> + send_text_update<W>');

// ─────────────────────────────────────────────────────────────────────────────
// FIX 3f — Clean up unused imports
// ─────────────────────────────────────────────────────────────────────────────
src = src.replace('use std::collections::HashMap;\n', '');
src = src.replace('use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};\n', '');
src = src.replace('use tokio::sync::{RwLock, oneshot};\n', 'use tokio::sync::RwLock;\n');
changes.push('Fix 3f: removed unused imports (HashMap, AsyncBufReadExt/AsyncWriteExt/BufReader, oneshot)');

// ─────────────────────────────────────────────────────────────────────────────
// FIX 4 — Add TODO + #[allow(dead_code)] to handle_session_set_model
// ─────────────────────────────────────────────────────────────────────────────
const old_set_model = 'async fn handle_session_set_model(params: &Value, agent: &GrokAcpAgent) -> Result<Value> {';
const new_set_model = `// TODO (task 111.3 follow-up): session/set_model is not yet wired as a typed
// on_receive_request handler because the ACP crate has no standard
// SetSessionConfigOptionRequest type.  To expose it, define a custom struct
// with #[derive(serde::Deserialize)] and register via on_receive_dispatch or a
// future crate extension point.  See Doc/acp-migration-map.md.
#[allow(dead_code)]
async fn handle_session_set_model(params: &Value, agent: &GrokAcpAgent) -> Result<Value> {`;
if (!src.includes(old_set_model)) { console.error('Fix 4: handle_session_set_model PATTERN NOT FOUND'); process.exit(1); }
src = src.replace(old_set_model, new_set_model);
changes.push('Fix 4: TODO + #[allow(dead_code)] added to handle_session_set_model');

// Also add #[allow(dead_code)] to handle_session_fork (no longer called)
const old_fork = '/// Handle a `session/fork` request — clone the source session into a new session ID.\nasync fn handle_session_fork(params: &Value, agent: &GrokAcpAgent) -> Result<Value> {';
const new_fork = `/// Handle a \`session/fork\` request — clone the source session into a new session ID.
// TODO (111.3 follow-up): wire into a typed on_receive_dispatch handler.
#[allow(dead_code)]
async fn handle_session_fork(params: &Value, agent: &GrokAcpAgent) -> Result<Value> {`;
if (src.includes(old_fork)) {
  src = src.replace(old_fork, new_fork);
  changes.push('Fix 3/4: #[allow(dead_code)] added to handle_session_fork');
}

// ─────────────────────────────────────────────────────────────────────────────
// Write the result back, preserving original line endings
// ─────────────────────────────────────────────────────────────────────────────
const finalSrc = src; // already LF-normalized for writing
writeFileSync(path, finalSrc, 'utf8');

console.log('\n=== Changes applied ===');
changes.forEach((c, i) => console.log(`  ${i + 1}. ${c}`));
console.log(`\nOriginal: ${originalLen} chars → Final: ${finalSrc.length} chars`);
console.log(`Removed: ${originalLen - finalSrc.length} chars`);
console.log('\nDone!');
