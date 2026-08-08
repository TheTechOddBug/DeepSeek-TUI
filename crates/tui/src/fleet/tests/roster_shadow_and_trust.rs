use super::*;
use tempfile::TempDir;

fn write_profile(dir: &Path, filename: &str, contents: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join(filename), contents).unwrap();
}

#[test]
fn workspace_shadow_of_personal_file_is_recorded_and_reported() {
    // #5098: editing the personal builder.toml changed nothing because a
    // project copy silently shadowed it. The roster must report that the
    // shadowed personal file exists and is ignored.
    let tmp = TempDir::new().unwrap();
    let personal_dir = tmp.path().join("personal");
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    write_profile(
        &personal_dir,
        "builder.toml",
        "id = \"builder\"\nrole_hint = \"builder\"\nmodel = \"deepseek-v4-flash\"\n",
    );
    write_profile(
        &workspace.join(".codewhale").join("agents"),
        "builder.toml",
        "id = \"builder\"\nrole_hint = \"builder\"\nmodel = \"deepseek-v4-pro\"\n",
    );

    let roster = FleetRoster::load_with_personal_dir(
        &FleetConfigToml::default(),
        &workspace,
        Some(&personal_dir),
        true,
    );

    let builder = roster.get("builder").expect("builder member");
    assert_eq!(builder.origin, ProfileOrigin::Workspace);
    let shadows: Vec<_> = roster.shadowed_for("builder").collect();
    // The chain is built-in → personal → workspace; both displacements
    // are recorded, and the file-on-file one names the ignored personal
    // copy explicitly.
    assert_eq!(shadows.len(), 2, "full shadow chain: {shadows:?}");
    let shadow = shadows
        .iter()
        .find(|shadow| shadow.shadowed_origin == ProfileOrigin::Personal)
        .expect("personal file shadow is recorded");
    assert!(shadow.shadowed_source.ends_with("builder.toml"));
    assert_eq!(shadow.winner_origin, ProfileOrigin::Workspace);
    assert!(
        shadows
            .iter()
            .any(|shadow| shadow.shadowed_origin == ProfileOrigin::BuiltIn),
        "the built-in displacement is recorded too: {shadows:?}"
    );
    assert!(
        roster.shadowed().iter().any(|s| s.id == "builder"),
        "roster-level shadow log carries the record"
    );
}

#[test]
fn project_scope_profiles_are_skipped_when_the_layer_is_not_trusted() {
    // #5098: `load_workspace_agent_profiles_tolerant` applied no trust
    // check — a cloned repo's .codewhale/agents/*.toml silently joined
    // the dispatch roster. With project config disabled
    // (`--no-project-config`), the whole layer stays out.
    let tmp = TempDir::new().unwrap();
    let workspace = tmp.path().join("workspace");
    write_profile(
        &workspace.join(".codewhale").join("agents"),
        "builder.toml",
        "id = \"builder\"\nrole_hint = \"builder\"\nmodel = \"gpt-5.6-luna\"\n",
    );

    let gated =
        FleetRoster::load_with_personal_dir(&FleetConfigToml::default(), &workspace, None, false);
    let builder = gated.get("builder").expect("built-in builder remains");
    assert_eq!(
        builder.origin,
        ProfileOrigin::BuiltIn,
        "untrusted project profile must not join the roster"
    );
    assert_ne!(
        builder.profile.model.as_deref(),
        Some("gpt-5.6-luna"),
        "foreign project pin must not reach dispatch"
    );

    let trusted =
        FleetRoster::load_with_personal_dir(&FleetConfigToml::default(), &workspace, None, true);
    assert_eq!(
        trusted.get("builder").expect("builder").origin,
        ProfileOrigin::Workspace,
        "trusted project profile wins as before"
    );
}
