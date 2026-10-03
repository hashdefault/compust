use std::process::Command;

#[test]
fn help_succeeds_without_an_x_server() {
    let output = Command::new(env!("CARGO_BIN_EXE_compust"))
        .env_remove("DISPLAY")
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn example_configuration_validates_without_an_x_server() {
    let output = Command::new(env!("CARGO_BIN_EXE_compust"))
        .env_remove("DISPLAY")
        .args(["--check-config", "--config", "compust.example.toml"])
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn invalid_configuration_exits_with_failure_before_connecting() {
    use std::io::Write;
    let mut config = tempfile::NamedTempFile::new().unwrap();
    write!(config, "max_fps = 0").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_compust"))
        .env_remove("DISPLAY")
        .args(["--check-config", "--config"])
        .arg(config.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
}

/// Runs `--check-config` with the XDG variables pointed at temporary directories.
fn check_discovered(
    config_home: &std::path::Path,
    extra: &[&str],
) -> std::io::Result<std::process::Output> {
    let empty = tempfile::tempdir()?;
    Command::new(env!("CARGO_BIN_EXE_compust"))
        .env_remove("DISPLAY")
        .env("XDG_CONFIG_HOME", config_home)
        .env("HOME", empty.path())
        .env("XDG_CONFIG_DIRS", empty.path())
        .arg("--check-config")
        .args(extra)
        .output()
}

#[test]
fn discovers_configuration_in_xdg_config_home() {
    let home = tempfile::tempdir().unwrap();
    let file = home.path().join("compust/compust.toml");
    std::fs::create_dir(home.path().join("compust")).unwrap();
    std::fs::write(&file, "max_fps = 60").unwrap();
    let output = check_discovered(home.path(), &[]).unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains(&*file.to_string_lossy()));

    std::fs::write(&file, "max_fps = 0").unwrap();
    let output = check_discovered(home.path(), &[]).unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(&*file.to_string_lossy()));
}

#[test]
fn explicit_configuration_overrides_discovery() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir(home.path().join("compust")).unwrap();
    std::fs::write(home.path().join("compust/compust.toml"), "max_fps = 0").unwrap();
    let output = check_discovered(home.path(), &["--config", "compust.example.toml"]).unwrap();
    assert!(output.status.success());
}

#[test]
fn missing_configuration_uses_defaults() {
    let home = tempfile::tempdir().unwrap();
    let output = check_discovered(home.path(), &[]).unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("built-in defaults"));
}
