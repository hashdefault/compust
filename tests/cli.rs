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

/// Runs `--check-config` on `text` and returns whether it succeeded and what it printed to
/// standard error.
fn check_config_text(text: &str) -> std::io::Result<(bool, String)> {
    let mut config = tempfile::NamedTempFile::new()?;
    std::io::Write::write_all(&mut config, text.as_bytes())?;
    let output = Command::new(env!("CARGO_BIN_EXE_compust"))
        .env_remove("DISPLAY")
        .args(["--check-config", "--config"])
        .arg(config.path())
        .output()?;
    Ok((
        output.status.success(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    ))
}

/// The names that a deserialization error lists after "expected one of", such as every field
/// of a table that rejected an unknown one.
fn expected_names(error: &str) -> Option<Vec<String>> {
    let (_, list) = error.split_once("expected one of ")?;
    let list = list.lines().next()?;
    Some(
        list.split(", ")
            .map(|name| name.trim_matches('`').to_owned())
            .collect(),
    )
}

/// The words of the headings that follow `.TP` in a manual page, with roff's escaped hyphens
/// undone.
fn manual_headings(page: &str) -> Vec<String> {
    let mut headings = Vec::new();
    let mut lines = page.lines();
    while let Some(line) = lines.next() {
        if line == ".TP"
            && let Some(heading) = lines.next()
        {
            headings.extend(
                heading
                    .replace("\\-", "-")
                    .split(|c: char| c.is_whitespace() || c == '"' || c == ',')
                    .map(str::to_owned),
            );
        }
    }
    headings
}

/// The words that `.B` or `.BR` sets in bold at the start of a line of a manual page.
fn manual_bold_words(page: &str) -> Vec<&str> {
    page.lines()
        .filter_map(|line| {
            line.strip_prefix(".BR ")
                .or_else(|| line.strip_prefix(".B "))
        })
        .filter_map(|rest| rest.split_whitespace().next())
        .collect()
}

/// The text of a manual page's section, from its `.SH` heading to the next one.
fn manual_section<'a>(page: &'a str, heading: &str) -> Option<&'a str> {
    let (_, section) = page.split_once(&format!("\n.SH {heading}\n"))?;
    Some(
        section
            .split_once("\n.SH ")
            .map_or(section, |(section, _)| section),
    )
}

fn read_manual(name: &str) -> std::io::Result<String> {
    std::fs::read_to_string(format!("{}/docs/man/{name}", env!("CARGO_MANIFEST_DIR")))
}

#[test]
fn command_manual_page_describes_every_option() {
    let headings = manual_headings(&read_manual("compust.1").unwrap());
    let help = Command::new(env!("CARGO_BIN_EXE_compust"))
        .env_remove("DISPLAY")
        .arg("--help")
        .output()
        .unwrap();
    let help = String::from_utf8(help.stdout).unwrap();
    let options: Vec<_> = help
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|word| word.starts_with('-') && word.len() > 1)
        .collect();
    assert!(options.contains(&"--check-config"), "{options:?}");
    for option in options {
        assert!(
            headings.iter().any(|heading| heading == option),
            "compust.1 does not describe {option}"
        );
    }
}

#[test]
fn configuration_manual_page_describes_every_setting() {
    let page = read_manual("compust.toml.5").unwrap();
    let global_headings = manual_headings(manual_section(&page, "GLOBAL SETTINGS").unwrap());
    let rule_headings = manual_headings(manual_section(&page, "RULES").unwrap());
    let (_, error) = check_config_text("unknown_setting = 1\n").unwrap();
    let globals = expected_names(&error).unwrap();
    let (_, error) = check_config_text("[[rules]]\nunknown_setting = 1\n").unwrap();
    let rule_fields = expected_names(&error).unwrap();
    assert!(globals.contains(&"rules".to_owned()), "{globals:?}");
    assert!(page.contains(".B [[rules]]"));
    for field in globals.iter().filter(|field| *field != "rules") {
        assert!(
            global_headings.contains(field),
            "compust.toml.5 does not describe the setting {field}"
        );
    }
    for field in &rule_fields {
        assert!(
            rule_headings.contains(field),
            "compust.toml.5 does not describe the rule field {field}"
        );
    }

    let (_, error) =
        check_config_text("[[rules]]\nwindow_type = \"unknown_type\"\nblur = false\n").unwrap();
    let bold = manual_bold_words(&page);
    for window_type in expected_names(&error).unwrap() {
        assert!(
            bold.contains(&window_type.as_str()),
            "compust.toml.5 does not name the window type {window_type}"
        );
    }
}

#[test]
fn configuration_manual_page_example_validates() {
    let page = read_manual("compust.toml.5").unwrap();
    let (_, example) = page.split_once("\n.EX\n").unwrap();
    let (example, _) = example.split_once("\n.EE\n").unwrap();
    assert!(
        !example.contains('\\'),
        "the example needs roff escapes undone"
    );
    let (valid, error) = check_config_text(example).unwrap();
    assert!(valid, "{error}");
}
