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
