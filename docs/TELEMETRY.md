# Codewhale product telemetry

**Status for 0.9.4: opt-in, and off by default.** Nothing is collected until you
answer the first-run notice with "Enable" — before that answer no event is
recorded, no batch is assembled, and `$CODEWHALE_HOME/telemetry/` is not even
created. Answering "Enable" is the entire consent decision; there is no other
way to turn this on, and a `telemetry = true` written before 0.9.4 does not
count as one.

**There is now a real endpoint.** An enabled session sends its batches to the
first-party ingest service at `https://telemetry.codewhale.net/v1/telemetry`,
which is the shipped default for `telemetry_endpoint`. What that service is,
what it stores, and what it structurally cannot store is spelled out in "What
the endpoint does" below. The default decides only *where* an enabled session's
batches go — it changes nothing about whether a session collects.

**To send nothing anywhere, keep telemetry off** (see "Turning it off"). To stay
enabled but contact nobody, set `telemetry_endpoint = ""`: batches are then
appended to `$CODEWHALE_HOME/telemetry/dryrun.jsonl` on your own machine, byte
for byte what the server would have received, and no HTTP client is ever
constructed. That file is how you audit this document against reality.

This document is the schema. It is not a summary of the schema: a test in
`crates/telemetry` parses the field names out of this file and asserts set
equality against the structs the serializer actually uses, so a field that is
here and not in the code — or in the code and not here — fails the build.

## Turning it off

There are two off switches and they do different things. Both stop collection
completely; only one of them erases anything.

```sh
codewhale config set telemetry false     # opt out: stops collection and erases state
CODEWHALE_TELEMETRY=0 codewhale          # kill switch: stops collection, erases nothing
codewhale --telemetry false              # the same kill switch, for one command
```

**`telemetry = false` in the config file is the opt-out.** It is a floor:
`--telemetry true` and `CODEWHALE_TELEMETRY=1` both lose to it, because a
setting you can undo by accident from a wrapper script is not a setting. It
deletes the random install id, truncates every buffered event and every
dry-run record, and writes a tombstone that a session already running re-checks
before it appends anything and before it sends anything. If any part of that
wipe fails, the tombstone is still there and the buffer is undrainable — a
failed wipe fails closed. Every later run re-asserts the tombstone for as long
as the setting stands, so it survives; turning telemetry back on means writing
`telemetry = true` in the same place, and that is also what clears it. Nothing
buffered before that point is ever sent.

**The environment variable and the flag are kill switches, not opt-outs.**
Telemetry is off for the run, nothing is written, nothing is sent — and
nothing on disk is touched or deleted. That is deliberate: a harness or agent
that sets `CODEWHALE_TELEMETRY=0` for one command must not silently discard the
install id and the dry-run records of the person who owns the machine. If you
want the erasing kind, use the config file.

`CODEWHALE_TELEMETRY` (and its `DEEPSEEK_TELEMETRY` alias) accepts
`0`, `1`, `true`, `false`, `yes`, `no`, `on`, `off`, `enabled`, `disabled`.
A value this list cannot read also resolves to off — a typo in a kill switch
must never resolve to "on".

The first-run notice is not shown at all when either switch is already set:
it never asks a question this environment would override, and answering it
never rewrites a `telemetry = false` you put there yourself.

A repo-local `.codewhale/config.toml` can set neither `telemetry` nor
`telemetry_endpoint`, and a workspace `.env` can set neither. Someone else's
repository cannot turn your telemetry on or aim it at a host of their choosing.

## Where it lives, and how much of your disk it uses

Everything is under `$CODEWHALE_HOME/telemetry/` (`0700`), every file `0600`:

| file | role |
|---|---|
| `buffer.jsonl` | pending events, one JSON object per line |
| `buffer.jsonl.lock` | a sibling lock file; only compaction takes it |
| `dryrun.jsonl` | where batches go when the endpoint is configured empty |
| `state.json` | the last app version seen and the last flush attempt |
| `install_id.json` | the random install id and when it was minted |
| `disabled` | the tombstone; present means nothing is appended or sent |

Both `buffer.jsonl` and `dryrun.jsonl` are rings capped at 512 records or
256 KiB, whichever comes first, with the oldest dropped. The documented
footprint ceiling for the whole directory is therefore **512 KiB plus a few
hundred bytes of metadata**.

The install id is a random v4 UUID. It is never derived from your hostname,
MAC address, `machine-id`, home directory, username, or executable path — a
derived id is a device fingerprint that survives reinstall and re-identifies
you across your own opt-out. It is regenerated whenever
`$CODEWHALE_HOME/telemetry/` is cleared, which opting out does automatically,
and in any case every 90 days.

There is no factory-reset command in Codewhale, so this document does not
claim one.

## When anything is sent, and where

Nothing is sent at all unless telemetry resolved on **and** the first-run notice
was answered with "Enable" on this machine. Given both, there is exactly one
flush point: an attempt during shutdown, bounded at three seconds. There is no
startup flush, mid-session flush, per-turn flush, or per-tool-call flush. The
shutdown flush re-resolves your setting from disk immediately beforehand, so
`codewhale config set telemetry false` written from another terminal stops the
flush of a session that is already running.

A flush is **one `POST`** to the resolved endpoint — by default
`https://telemetry.codewhale.net/v1/telemetry`. The request carries a
`content-type: application/json` header, a
`user-agent: codewhale-telemetry/<app_version>`, and the batch body. That is
all: no cookies (the HTTP client is built without a cookie jar to disable), no
redirects (refused outright), no `Authorization` header, no custom headers, and
no query string. The response body is discarded unread; only the status class is
looked at.

`https://` is required. Plain `http://` is accepted only for a loopback host, so
you can point the client at a recorder of your own and read the wire form
directly; no environment variable overrides that refusal. An endpoint the client
refuses turns telemetry off for the run rather than falling back to a different
destination.

Any failure — DNS, connect, TLS, timeout, non-2xx — drops the batch. There is
no retry, no backoff, and no re-queue. A permanently offline machine attempts at
most once per flush point and never grows a queue.

---

## Event schema

`SCHEMA_VERSION = 1`. Every field is an integer, a boolean, or a **closed enum string**, except exactly three bounded strings: `app_version`, `git_sha`, `panic_site`. Each of the three has a written rule and a test pinning the rule. **There is no free-form string type in this schema, and no open-keyed map.** That is the property that makes red line 3 enforceable rather than aspirational.

### Batch envelope — sent on every POST

```jsonc
{
  "schema_version": 1,
  "sent_at":     "2026-08-03T18:04:11Z",   // RFC3339 UTC, second precision
  "install_id":  "3f2a…",                  // uuid v4, rotates every 90 days
  "app_version": "0.9.4",
  "git_sha":     null,                     // non-null only for release-CI builds
  "surface":     "tui",
  "os":          "macos",
  "arch":        "aarch64",
  "libc":        "none",
  "tty":         true,
  "events":      [ … ]
}
```

| Field | Type | Source anchor | Rule |
|---|---|---|---|
| `schema_version` | `u32` | const in `crates/telemetry/src/event.rs` | Bumped on any field add/remove/retype. Never reused. Pinned by a golden snapshot test. |
| `sent_at` | RFC3339 | `chrono::Utc::now()` | Second precision. Per-**batch** only — events carry no timestamps at all. |
| `install_id` | uuid v4 | `crates/telemetry/src/envelope.rs` | Random, never derived, rotated every 90 days. See "Where it lives" above. |
| `app_version` | string | `env!("CARGO_PKG_VERSION")`, as at `crates/tui/src/tui/ui.rs:17894` | Must match `^\d+\.\d+\.\d+(-[0-9A-Za-z.]+)?$`. |
| `git_sha` | string \| null | `option_env!("CODEWHALE_RELEASE_BUILD_SHA")` — a **new** rustc-env | First 12 hex chars. Emitted **only** when `codewhale_build_support::release_build_sha` saw `DEEPSEEK_BUILD_SHA` or `GITHUB_SHA` in the build environment, i.e. only for release-CI builds. `null` for every locally built binary, unconditionally, with no runtime lookup of any kind. **Never** `CODEWHALE_BUILD_COMMIT` — that falls back to `git_commit` and is the builder's private HEAD. **Never** `Thread.git_sha` (`crates/state/src/lib.rs:88`) — that is the user's workspace commit and a red line, one identifier away by name. |
| `surface` | enum | set explicitly at each subcommand dispatch | `tui \| exec \| cli \| app-server \| mcp-server \| serve`. **Not derivable from the executable**: `codewhale-tui` serves at least five surfaces, and app-server runs *in-process* inside `codewhale` (`crates/cli/src/lib.rs:3945-3968`), so `current_exe()` would report every app-server session as CLI. `desktop` is omitted — no desktop surface exists. Which of these can emit is governed by the consent record, not by the surface: see "Which surfaces emit" below. |
| `os` | enum | `std::env::consts::OS`, as at `crates/cli/src/update.rs:41` | Whitelist: `linux \| macos \| windows \| freebsd \| android \| other`. |
| `arch` | enum | `std::env::consts::ARCH` | `x86_64 \| aarch64 \| other`. |
| `libc` | enum | `cfg!(target_env)` — **compile time** | `gnu \| musl \| none`. Runtime detection reads distro vendor strings; compile-time is free and leaks nothing. |
| `tty` | bool | `std::io::IsTerminal`, as at `crates/tui/src/tui/ui.rs:1169` | `stdin().is_terminal() && stdout().is_terminal()`. Varies, because consent is machine-scoped. |
| `events` | array | the drained buffer | Every element is one of the four events below and nothing else. Capped at 200 events or 64 KiB per batch; a batch that would exceed either cap leaves the remainder buffered for the next flush. |

**`os_major` is not collected.** Reading it costs unsafe FFI on two platforms plus a file parser on a third, in the one crate whose entire value is being small enough to audit — and `os`, `arch`, and `libc` are free and answer the platform question. It may be reconsidered if the stored data ever shows that the OS-version cut is what triage is missing; a hunch is not that evidence.

### Which surfaces emit

A surface emits when — and only when — a notice decision was recorded on this machine and resolves to opt-in. The notice is only ever rendered on a TTY. So:

- **`tui`** — emits after the user answers the notice.
- **`exec`, `cli`, `app-server`, `mcp-server`, `serve`** — emit only on a machine where the notice was already answered interactively. On a fresh home (CI, a container, a new user) they emit nothing, and nothing is written to disk.
- **Fleet workers never emit**, on any surface, by construction (`crates/tui/src/fleet/host.rs:1362`).

`docs/TELEMETRY.md` states this so nobody reads a structural zero as an adoption zero.

### Event: `install_or_upgrade`

Emitted once when `state.json`'s `last_version` differs from `app_version`.

```jsonc
{ "event": "install_or_upgrade", "kind": "upgrade", "previous_version": "0.9.3" }
```

| Field | Type | Source | Rule |
|---|---|---|---|
| `kind` | enum | derived | `install` (no prior record) \| `upgrade` \| `downgrade`. |
| `previous_version` | string \| null | `$CODEWHALE_HOME/telemetry/state.json` **only** | Same regex as `app_version`. Never derived from session history or config mtimes — those files have a different privacy contract. |

### Event: `session_start`

```jsonc
{ "event": "session_start", "source": "interactive" }
```

`source` is `SessionSource` (`crates/state/src/lib.rs:34-41`) stringified by `session_source_to_str` (`:1909-1917`): `interactive | resume | fork | api | unknown`.

### Event: `session_end`

The workhorse. Everything a session accumulated ships here, once.

```jsonc
{
  "event": "session_end",
  "duration_bucket": "1m_10m",
  "exit_class": "clean",
  "cold_start_bucket": "250_1000",
  "providers": ["deepseek", "custom"],
  "counters": { "turns": 14, "tool_calls": 61, "fleet_dispatch": 0, "workflow_run": 0,
                "subagent_spawn": 2, "mcp_server_connected": 0, "memory_search": 0,
                "approval_modal_shown": 0, "approval_auto_allowed": 0,
                "command_palette_open": 3 },
  "errors":   { "auth_preflight_failed": 0, "provider_http_4xx": 0, "provider_http_5xx": 1,
                "tool_denied_by_policy": 0, "tool_timeout": 0, "network_error": 0 },
  "turn_wall": { "lt_5s": 9, "5_30s": 4, "30_120s": 1, "gte_120s": 0 }
}
```

**`counters` and `errors` are `#[derive(Serialize)]` structs of named `u32` fields, not maps.** Every field is serialized including zeros. The key set is closed by the compiler: adding a counter requires editing `crates/telemetry/src/event.rs`, which is where the doc-match test lives.

**`duration_bucket`** — `chrono` delta from `app.session_started_at` (`crates/tui/src/tui/app.rs:1767`). Half-open, seconds: `lt_1m` (`d < 60`), `1m_10m` (`60 ≤ d < 600`), `10m_60m` (`600 ≤ d < 3600`), `gt_60m` (`d ≥ 3600`).

**`exit_class`** — `clean | signal | panic | error`. **Derived from an explicit `AtomicU8`, never from an exit code.** `RunTerminationReason::Canceled` maps to exit 130 (`crates/tui/src/core/runtime_contract/termination.rs:52`), the same value the signal task uses (`crates/tui/src/main.rs:686`, 128+SIGINT), so a code-based derivation would report every Esc-cancelled turn as a signal. The atomic is set by the panic hook (`crates/tui/src/main.rs:1353`), by the signal task (`:669-689`) before `std::process::exit`, and on the clean path from `RunTerminationReason::is_success()` (`crates/tui/src/core/runtime_contract/termination.rs:44-46`) — `error` otherwise. Do **not** use `exec_failure_exit_code` (`crates/tui/src/main.rs:9680-9685`): it knows only `{75, 1}` and would report an approval-required exit (3) as a generic failure.

**`cold_start_bucket`** — from `startup_trace::elapsed_ms()`, which reads `PROCESS_START` directly and is independent of the startup summary's buffer clear (`crates/tui/src/startup_trace.rs:33-48`). Boundaries: `lt_250`, `250_1000`, `1000_3000`, `gte_3000`. Absent on non-TUI surfaces.

**`providers`** — sorted, deduplicated array of `ProviderKind::as_str()` (`crates/config/src/provider_kind.rs:252-254`, a `&'static str` from a closed enum; `Custom` yields the literal `"custom"`). **The API takes `ProviderKind` by value, never `&str`.** Do not call `ProviderKind::parse` or `parse_config_identity` (`:257`, `:287`) — those are for config-table resolution. **Do not read** `provider_identity_for_persistence()` (`crates/tui/src/tui/app.rs:4635-4641`), `provider_id_for_persistence()` (`:4644-4646`), `ExecStreamMeta.provider_id` (`crates/tui/src/main.rs:9506`), or `PlannedTurnRoute.effective_provider_label` (`crates/tui/src/turn_route_plan.rs:189-193`) — all four return the customer's own `[providers.<name>]` table key when the route is Custom. This is the single most likely leak in the feature: it is one field away from the natural seam and `/status` already prints it (`crates/tui/src/commands/groups/config/status.rs:24-28`). **No model id is ever sent, for any provider** — `crates/tui/src/safe_label.rs:11-15` documents that a model id can be a path, a URL, or a deployment id that is itself a credential.

**`counters`** — closed field set. Every bump happens at the **call site**, never inside a conditionally-entered handler:

| field | source anchor |
|---|---|
| `turns` | `crates/tui/src/tui/ui.rs:4196` — the *caller* of `execute_turn_end_observer_hook`. Never inside it: that function's first statement is `if !app.hooks.has_hooks_for_event(HookEvent::TurnEnd) { return Ok(()); }` (`:1806-1808`), and the natural future optimization hoists that check to the call site, silently zeroing the counter for every user without hooks. |
| `tool_calls` | `crates/tui/src/core/engine/tool_execution.rs:474` — surface-agnostic, fires for exec and CLI too |
| `fleet_dispatch` | `crates/tui/src/fleet/manager.rs:334/351/363` |
| `workflow_run` | counted from the **`WorkflowAction` variant discriminant** returned by `parse_workflow_action` (`crates/tui/src/tools/workflow.rs:738-751`), never from `input["action"]`. The JSON Schema at `:775-779` is what is published *to the model* — a declaration, not a guard; the real parse also accepts `spawn\|wait\|list\|inspect\|stop\|abort`, and its reject arm at `:746-748` embeds the model string verbatim. |
| `subagent_spawn` | `crates/tui/src/tui/ui.rs:1763-1776` |
| `mcp_server_connected` | count of `.connected` in the snapshot at `crates/tui/src/mcp.rs:3795-3809`; never `name`, `command_or_url`, or `error` — server names are user-chosen and routinely internal infra |
| `memory_search` | tool name at `crates/tui/src/tools/native_memory.rs:60-61`, counted at the tool_execution choke point |
| `approval_modal_shown` | `crates/tui/src/tui/ui.rs:4670` (consumer of `Event::ApprovalRequired`, `crates/tui/src/core/events.rs:414`) |
| `approval_auto_allowed` | `crates/tui/src/core/engine.rs:5571`. Count only. Never `matched_rule`, `reason()`, the command, or argv — `auto_allow` patterns are user-authored command strings (`crates/tui/src/command_safety.rs:35/309`) |
| `command_palette_open` | `crates/tui/src/tui/ui.rs:6192-6213` and `crates/tui/src/tui/mouse_ui.rs:1326` |

**`errors`** — closed field set. Every value is a **variant discriminant**, never `err.to_string()`:

| field | source anchor |
|---|---|
| `auth_preflight_failed` | discriminant of `CredentialReadiness` (`crates/workflow/src/fleet_preflight.rs:37-58`) / `ProviderAuthClass` (`crates/tui/src/provider_readiness.rs:32`). Discriminant only — `Missing { detail }` carries free text |
| `provider_http_4xx` | `status.as_u16() / 100 == 4`, captured at `crates/tui/src/client/chat.rs:595` and `:673` **before** the `bail!`. One row per field, because the doc-match test reads this table field for field |
| `provider_http_5xx` | `status.as_u16() / 100 == 5`, same capture points |
| `tool_denied_by_policy` | the `permission_denied` arm of the 8-variant match at `crates/tui/src/core/engine/tool_execution.rs:487-495` |
| `tool_timeout` | the `timeout` arm, same match |
| `network_error` | `retry_reason_label_and_human()`'s `&'static str` half, `crates/tui/src/client.rs:2570-2584` |

Why discriminants and nothing else: `ToolError::PathEscape`'s `Display` *is* an absolute path (`crates/tools/src/lib.rs:61`); `fim.rs:48-50`'s `Display` *is* a literal source fragment the model emitted; `secrets/src/lib.rs:50`'s `Display` carries the secret store's absolute path; every `LlmError` variant carries the raw provider HTTP body verbatim (`crates/tui/src/llm_client/mod.rs:455-511`), and a 400 from a content filter routinely echoes the prompt.

**`turn_wall`** — a per-session histogram of counts, never per-turn events. `lt_5s`, `5_30s`, `30_120s`, `gte_120s`. Source `crates/tui/src/tui/ui.rs:4196`, which already has `duration` in hand.

### Event: `panic`

Appended **synchronously** by the panic hook, because a `session_end` may never be written.

```jsonc
{ "event": "panic", "site": "crates/tui/src/tui/ui.rs:8801:17" }
```

`site` comes from `panic_info.location()` (`crates/tui/src/main.rs:1368-1371`) or `Location::caller()` (`crates/tui/src/utils.rs:523`). **Allowlist reduction, not optional:** emit verbatim only if `file()` starts with `crates/`; otherwise emit the literal `"<dep>"`. Must match `^crates/[A-Za-z0-9_/.-]+\.rs:\d+:\d+$` or `^<dep>$`. There is no `--remap-path-prefix` in this repo (no `.cargo/config.toml`; `Cargo.toml:69-74` sets only `lto`/`strip`/`codegen-units`), so a panic inside a registry dependency yields `/Users/<builder>/.cargo/registry/src/…/ratatui-0.29.0/src/…` — the **build machine's username**, shipped from every user's binary.

**The panic message is never sent.** The hook at `crates/tui/src/main.rs:1361-1367` builds `msg` from the payload; telemetry must not read it. A slicing panic embeds the entire string being sliced, and this tree slices user and model text in dozens of places.

### What the endpoint does — a shipping gate, not a footnote

This section was a gate on configuring any non-loopback endpoint. The endpoint is now configured by default, so this is a description of a service that exists rather than a promise about one that might.

**What it is.** `https://telemetry.codewhale.net/v1/telemetry` — a Cloudflare Worker named `codewhale-telemetry-ingest`, whose complete source is in this repository at [`telemetry-ingest/`](../telemetry-ingest/). It is the only component; there is no queue, no proxy, and no other service in the path. It is write-only: nothing in the Worker can read back what was stored, and querying happens out of band through Cloudflare's SQL API with the owner's token. The hostname is deliberately self-describing, so anyone inspecting their own network traffic can tell what it is from the name alone.

**What it stores.** Everything in this document and nothing else, in Workers Analytics Engine — one row per event. The validator in `telemetry-ingest/src/schema.ts` is a **closed** field set: an unknown key anywhere in a batch rejects the whole batch with `400`. A future client bug that starts attaching a path, a prompt, or a provider table name gets refused by the server rather than quietly stored. `telemetry-ingest/test/schema-doc.test.ts` parses the field names and enum spellings back out of *this file* and asserts set equality against the validator, and `telemetry-ingest/test/ingest.test.ts` posts the Rust client's own pinned golden batch and asserts it is accepted byte for byte — so this document, the client, and the endpoint cannot drift apart without a red test.

Batches are **IP-stripped at ingest**. No IP is stored, logged, or joined to `install_id` — and that is structural rather than a setting anyone could flip. An Analytics Engine row is exactly `_sample_interval`, `blob1`–`blob20`, `dataset`, `double1`–`double20`, `index1`, and `timestamp`. Every one of those columns is written by the Worker's own `writeDataPoint` call; there is no implicit column, so **there is no IP, country, or geo column** — no slot one could occupy even if the code wanted it there. And the code cannot want it:

- The handler reads exactly **two** request headers — `content-type` and `content-length`. Nothing else, ever.
- It never touches the request's `cf` property, so country, colo, city, region, ASN, timezone, and coordinates are never in scope.
- The function that builds every stored row cannot see the request at all; its input type is the validated batch body.
- Nothing logs. `invocation_logs` is off in `wrangler.jsonc` — Cloudflare describes those as "enriched with information available to Cloudflare in the context of the invocation", which is exactly the class of automatic per-request record this service will not keep — and there is no `console.*` call in the source.
- Rate limiting is keyed on `install_id` from the validated body, never on a network address. An IP-keyed limiter would mean this Worker handles IPs. It is the weaker limiter and it is the right trade.
- `telemetry-ingest/test/no-ip.test.ts` reads the shipped source as text and fails the build if any address or geo name appears, if the set of headers read grows past two, if a `console.*` call is added, or if a `Response` is ever constructed with a body.

**Retention: three months.** That is Cloudflare's fixed window for Analytics Engine and it is not configurable, so it is a ceiling rather than a policy — there is no setting that could make it longer.

**No third-party analytics processor** sits between the client and storage. There is no ad SDK, no analytics SDK, and no session replay, in the endpoint or in the runtime binary.

**Every response is a bare status with an empty body** — `204` accepted, `400` schema violation, `404`/`405` wrong path or method, `413` oversized, `415` wrong content type, `429` rate-limited, `500` internal. The endpoint cannot echo back what it received or what it holds, and because the client drops the batch on anything that is not 2xx, a rejection is invisible to you by construction and a server error can never surface as a client-visible failure.

`install_id` rotates client-side every 90 days (`rotated_at` in `install_id.json`), so no single identifier spans a long history. This costs longitudinal accuracy and the docs say so: **no count derived from `install_id` is a user count.** It is a lower bound on distinct machine-installs in a window, and it undercounts a returning user across a rotation.

**Turning it off deletes what was kept locally, not what was already sent.** `codewhale config set telemetry false` erases the install id, the buffer, and the dry-run records on your machine, and stops anything further. Rows already accepted by the endpoint are keyed only by a rotating random id that is now gone; they age out with the three-month window. There is no deletion API, and this document does not claim one.

### What is never collected — the public red-line list

Prompts; completions; tool arguments; diffs; patches; file contents; filenames; absolute or relative paths; git remotes; repo names; branch names; workspace commit SHAs; memory entries; chat history; API keys, tokens, cookies, or `Authorization` headers (including any boolean asserting a key exists); model ids of any kind; custom provider table names; MCP server names, commands, or URLs; approval rule text; error message bodies; panic message text; per-event timestamps; keystrokes; clipboard; screenshots; microphone; camera; location; and any third-party ad or analytics SDK — there are none in the runtime binary and none may be added.

Two named traps for the implementer. `crates/state/src/lib.rs` persists `git_sha`, `git_branch`, `git_origin_url`, `cwd`, and `path` on the threads table (`:88, :394, :648`): a payload builder that accepts a `Thread` or `ThreadMeta` and derives `Serialize` breaches the contract in one line. **Never derive `Serialize` over an existing state type** — build every telemetry struct from scratch with explicit fields. And `crates/core/src/lib.rs:1381-1390` is the one place in the tree where the word `telemetry` sits inside a JSON object next to `prompt`, `base_url`, and `has_api_key`. It is the object someone will copy. Do not.

---
