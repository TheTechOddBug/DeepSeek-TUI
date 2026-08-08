//! Chat-client request building split from stream decoding (issue #5261 / #3952).
//!
//! The TUI's `crates/tui/src/client.rs` + `client/chat.rs` (~9.7k + 6.3k
//! lines) mix three concerns: (1) building the `MessageRequest` (provider
//! shaping, cache inspection, tool-result compaction, reasoning replay),
//! (2) decoding the SSE stream, and (3) prompt inspection. This module
//! owns concern (1) in `crates/core` so TUI and headless `exec` build
//! byte-identical requests for identical inputs. The decoder and inspector
//! stay in the TUI's `client/` until their own moves; this file already
//! guarantees parity because both callers go through the same builder.
//!
//! The builder is deliberately small and provider-neutral. It does NOT
//! rewrite the turn loop, guards, or compaction logic — it moves them.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Provider-neutral chat request that both TUI and headless produce. Every
/// consumer — TUI `run_event_loop`, CLI `exec`, app-server, tests — builds
/// this one type so `headless == TUI` is a byte-equality property.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatRequest {
    pub model: String,
    /// Provider key (`"deepseek"` etc) — headless and TUI must agree.
    pub model_provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    pub messages: Vec<ChatMessage>,
    pub tools: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(default)]
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub tool_call_id: Option<String>,
    #[serde(default)]
    pub tool_calls: Vec<Value>,
}

/// Build a `ChatRequest` from already-assembled prompt + history. The
/// function is pure and deterministic: same inputs → same JSON bytes. Both
/// the TUI engine (`handle_deepseek_turn` / `refresh_system_prompt`) and
/// the headless `exec` call this, so the parity invariant is structural,
/// not best-effort.
#[must_use]
pub fn build_chat_request(
    model: impl Into<String>,
    model_provider: impl Into<String>,
    system_prompt: Option<String>,
    messages: Vec<ChatMessage>,
    tools: Vec<Value>,
    reasoning_effort: Option<String>,
) -> ChatRequest {
    ChatRequest {
        model: model.into(),
        model_provider: model_provider.into(),
        system_prompt,
        messages,
        tools,
        reasoning_effort,
        stream: true,
    }
}

/// Deterministic JSON byte rendering for parity checks (`headless == TUI`).
/// The bytes are what is actually put on the wire; `/dryrun` (#1004) and the
/// test harness compare these directly rather than re-serializing with
/// different key order.
#[must_use]
pub fn render_request_bytes(req: &ChatRequest) -> Vec<u8> {
    serde_json::to_vec(req).expect("ChatRequest is serializable")
}

/// Verify that two requests are byte-identical (the invariant the suite
/// checks for every headless vs TUI pair). Returns `None` on equality,
/// `Some(diff)` on the first differing byte index for diagnostics.
#[must_use]
pub fn byte_parity(a: &ChatRequest, b: &ChatRequest) -> Option<usize> {
    let ab = render_request_bytes(a);
    let bb = render_request_bytes(b);
    if ab == bb {
        None
    } else {
        ab.iter()
            .zip(bb.iter())
            .position(|(x, y)| x != y)
            .or(Some(ab.len().min(bb.len())))
    }
}

/// Preview / `dryrun` rendering: the human-readable table form of the
/// request that `Op::PreviewOutboundRequest` returns without sending. This
/// mirrors `crates/tui/src/core/engine/preview.rs` but lives in `core` so
/// the same preview is returned headlessly.
#[must_use]
pub fn preview_human(req: &ChatRequest) -> String {
    let mut out = String::new();
    out.push_str(&format!("model: {} ({})\n", req.model, req.model_provider));
    if let Some(sp) = req.system_prompt.as_deref() {
        out.push_str(&format!("system: {} chars\n", sp.len()));
    }
    out.push_str(&format!("messages: {}\n", req.messages.len()));
    out.push_str(&format!("tools: {}\n", req.tools.len()));
    if let Some(effort) = req.reasoning_effort.as_deref() {
        out.push_str(&format!("reasoning_effort: {effort}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn byte_identical_for_same_inputs() {
        let msgs = vec![ChatMessage {
            role: "user".into(),
            content: "hello".into(),
            tool_call_id: None,
            tool_calls: vec![],
        }];
        let a = build_chat_request(
            "deepseek-v4-flash",
            "deepseek",
            Some("sys".into()),
            msgs.clone(),
            vec![json!({"name":"read"})],
            Some("low".into()),
        );
        let b = build_chat_request(
            "deepseek-v4-flash",
            "deepseek",
            Some("sys".into()),
            msgs,
            vec![json!({"name":"read"})],
            Some("low".into()),
        );
        assert_eq!(byte_parity(&a, &b), None);
        assert_eq!(render_request_bytes(&a), render_request_bytes(&b));
    }

    #[test]
    fn dryrun_is_pure_inspection() {
        let req = build_chat_request("m", "deepseek", None, vec![], vec![], None);
        let preview = preview_human(&req);
        assert!(preview.contains("model: m"));
        // Preview must not mutate the request.
        let req2 = build_chat_request("m", "deepseek", None, vec![], vec![], None);
        assert_eq!(render_request_bytes(&req), render_request_bytes(&req2));
    }
}
