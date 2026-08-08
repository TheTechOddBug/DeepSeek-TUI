//! The kill switch has to reach the in-process runtime that would emit.
//!
//! The single `codewhale` binary resolves dispatcher overrides, states the
//! telemetry floor in its environment, and then calls `codewhale_tui::run`.
//! That runtime re-resolves telemetry before it can arm. These tests drive the
//! real binary through the keyless `features list` command and use the local
//! dry-run sink as the end-to-end observable: an enabled positive control must
//! write session events, while a kill switch must create no telemetry state.

#![cfg(unix)]

use std::fs;
use std::process::Command;

use codewhale_config::{SetupState, TELEMETRY_NOTICE_VERSION};
use tempfile::TempDir;

/// `CODEWHALE_TELEMETRY=0` beats `--telemetry true`, end to end.
///
/// The notice decision is recorded on this home on purpose: without it the run
/// would be off for want of consent, and the test would pass without the kill
/// switch ever being consulted.
#[test]
fn env_off_beats_cli_on_end_to_end() {
    // Positive control first: the flag reaches the in-process runtime and
    // arms its dry-run sink, so the assertion below is about the floor and not
    // about a command that never crossed the dispatch boundary.
    let on = dispatch_and_read_telemetry(None);
    let dry_run = on
        .dry_run
        .expect("`--telemetry true` with recorded consent must write the dry-run sink");
    assert!(
        dry_run.contains("\"event\":\"session_start\"")
            && dry_run.contains("\"event\":\"session_end\""),
        "the real in-process runtime must record a complete session: {dry_run}"
    );

    let off = dispatch_and_read_telemetry(Some("0"));
    assert!(
        !off.telemetry_dir_exists && off.dry_run.is_none(),
        "`CODEWHALE_TELEMETRY=0` must beat `--telemetry true` before the runtime arms"
    );
}

/// A value the resolver cannot parse resolves to off, rather than falling
/// through to the flag.
#[test]
fn an_unparseable_telemetry_env_value_keeps_the_in_process_runtime_off() {
    let evidence = dispatch_and_read_telemetry(Some("maybe"));
    assert!(
        !evidence.telemetry_dir_exists && evidence.dry_run.is_none(),
        "a typo in the kill switch must never arm the in-process runtime"
    );
}

struct DispatchEvidence {
    telemetry_dir_exists: bool,
    dry_run: Option<String>,
}

/// Run the real dispatcher into a keyless in-process command and report the
/// telemetry state it actually left behind.
fn dispatch_and_read_telemetry(telemetry_env: Option<&str>) -> DispatchEvidence {
    let fixture = TempDir::new().expect("fixture root");
    let home = fixture.path().join("home");
    let codewhale_home = fixture.path().join("codewhale-home");
    let workspace = fixture.path().join("workspace");
    for dir in [&home, &codewhale_home, &workspace] {
        fs::create_dir_all(dir).expect("create fixture dir");
    }

    // A recorded acceptance, so the run is not off for want of consent.
    let mut state = SetupState::default();
    state.record_telemetry_notice(TELEMETRY_NOTICE_VERSION, true);
    state
        .save_to(&codewhale_home.join("setup_state.json"))
        .expect("write setup state");

    let config_path = fixture.path().join("config.toml");
    fs::write(
        &config_path,
        // An explicitly empty endpoint is the network-free dry-run sink.
        "telemetry = true\ntelemetry_endpoint = \"\"\n",
    )
    .expect("write config");

    let mut command = Command::new(codewhale_binary());
    command
        .current_dir(&workspace)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").expect("PATH"))
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("CODEWHALE_HOME", &codewhale_home)
        .env("CODEWHALE_SECRET_BACKEND", "file")
        .env(
            "CODEWHALE_RELEASE_BASE_URL",
            "https://example.invalid/releases",
        )
        .arg("--config")
        .arg(&config_path)
        .args(["--telemetry", "true", "features", "list"]);
    if let Some(value) = telemetry_env {
        command.env("CODEWHALE_TELEMETRY", value);
    }
    let output = command.output().expect("run codewhale dispatcher");
    assert!(
        output.status.success(),
        "the in-process feature command must succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("feature\tstage\tenabled"),
        "the real in-process feature command must have run\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let telemetry_dir = codewhale_home.join("telemetry");
    let dry_run = match fs::read_to_string(telemetry_dir.join("dryrun.jsonl")) {
        Ok(contents) => Some(contents),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => panic!("read telemetry dry-run sink: {error}"),
    };
    DispatchEvidence {
        telemetry_dir_exists: telemetry_dir.exists(),
        dry_run,
    }
}

fn codewhale_binary() -> &'static str {
    env!("CARGO_BIN_EXE_codewhale")
}
