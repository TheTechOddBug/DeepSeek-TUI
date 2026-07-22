# Layer 5.1: User Command Registry And Loading Boundary — Evidence Summary

**EPIC-002**: Staged command-boundary refactor (Hmbown/CodeWhale#2870)
**FEAT-010**: Layer 5.1 — User Command Registry And Loading Boundary
**Status**: ✅ Satisfied by existing code — no production changes needed
**Date**: 2026-07-03

## Acceptance Criteria Verification

| Criterion | Status | Evidence |
|-----------|--------|----------|
| Valid user command loads with metadata | ✅ Satisfied | `user_registry::tests::registry_loads_markdown_metadata` |
| Invalid frontmatter recoverable error | ✅ Satisfied | `user_registry::tests::invalid_frontmatter_dispatch_returns_user_command_error_without_builtin_fallback` |
| Hidden command loaded and dispatchable | ✅ Satisfied | `user_registry::tests::hidden_user_commands_still_dispatch_directly` |
| Allowed-tools metadata parsed and available | ✅ Satisfied | `user_registry::tests::registry_loads_markdown_metadata` + `empty_allowed_tools_frontmatter_blocks_all_tools` |
| Reload/lazy-load reflects file changes | ✅ Satisfied | `user_registry::tests::registry_reloads_when_existing_command_file_changes` |
| User commands separate from built-ins | ✅ Satisfied | Source: `commands::execute()` checks `user_registry::try_dispatch()` before built-in registry |

## Test Results

| Test suite | Count | Result |
|-----------|-------|--------|
| `user_registry` | 18/18 | ✅ PASS |
| `user_commands` | 24/24 | ✅ PASS |
| `commands::tests` (dispatch) | 60/60 | ✅ PASS |
| `command_palette` | 23/23 | ✅ PASS |
| `cargo check -p codewhale-tui` | — | ✅ PASS |
| `cargo fmt --check` | — | ✅ PASS |

## Source Architecture (Live Verification)

| Concern | Source file | Finding |
|---------|------------|---------|
| Built-in command registration | `crates/tui/src/commands/mod.rs`, `groups/mod.rs` | Built-ins via `groups::all_command_groups()` into trait-backed `CommandRegistry` |
| User command discovery | `crates/tui/src/commands/user_commands.rs` | Scans workspace `.codewhale`, `.deepseek`, `.claude`, `.cursor` dirs |
| User-command registry boundary | `crates/tui/src/commands/user_registry.rs` | `UserCommandRegistry` with commands, aliases, load_errors, invalid_commands |
| Metadata | `UserCommandMetadata` | name, body, description, argument_hint, allowed_tools, pausable, aliases, hidden |
| Dispatch precedence | `commands::execute()` + `user_registry::try_dispatch()` | User commands before built-ins; invalid commands return errors without built-in fallback |
| Hidden commands | `user_registry.rs`, `command_palette.rs`, `widgets/mod.rs` | Directly dispatchable, filtered from palette/slash completion |
| Allowed tools | `parse_allowed_tools()` + `try_dispatch()` | Parsed to normalized tool names, stored in `app.active_allowed_tools` |

## Boundary Contracts

- **Data**: `UserCommandMetadata` is the user-command boundary object
- **API**: `user_registry::with_registry_for_workspace()` is the borrowed registry access point
- **Dispatch**: User commands before built-ins; invalid commands don't fall through to built-ins
- **Documentation**: See `docs/architecture/command-dispatch.md`
