use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_oj-agent")
}

#[test]
fn cli_help_version_and_errors_keep_their_output() {
    let help = Command::new(binary()).arg("--help").output().unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("Commands:"));

    let version = Command::new(binary()).arg("--version").output().unwrap();
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).contains(env!("CARGO_PKG_VERSION")));

    let invalid = Command::new(binary())
        .arg("--unknown-option")
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("--unknown-option"));
}

#[test]
fn cli_json_and_runtime_errors_keep_their_output() {
    let tools = Command::new(binary()).arg("tool").output().unwrap();
    assert!(tools.status.success());
    let definitions: serde_json::Value = serde_json::from_slice(&tools.stdout).unwrap();
    assert!(
        definitions
            .as_array()
            .is_some_and(|tools| !tools.is_empty())
    );

    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing.json");
    let error = Command::new(binary())
        .arg("validate")
        .arg(missing)
        .output()
        .unwrap();
    assert!(!error.status.success());
    assert!(String::from_utf8_lossy(&error.stderr).contains("Error:"));
}

#[test]
fn cli_supports_file_redirection() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("help.txt");
    let output = std::fs::File::create(&path).unwrap();
    let status = Command::new(binary())
        .arg("--help")
        .stdout(output)
        .status()
        .unwrap();
    assert!(status.success());
    assert!(std::fs::read_to_string(path).unwrap().contains("Commands:"));
}

#[cfg(windows)]
#[test]
fn windows_executable_uses_gui_subsystem() {
    let exe = std::fs::read(binary()).unwrap();
    assert_eq!(&exe[..2], b"MZ");
    let pe = u32::from_le_bytes(exe[0x3c..0x40].try_into().unwrap()) as usize;
    assert_eq!(&exe[pe..pe + 4], b"PE\0\0");
    // Both PE32 and PE32+ put Subsystem at offset 68 in the optional header.
    let offset = pe + 24 + 68;
    assert_eq!(
        u16::from_le_bytes(exe[offset..offset + 2].try_into().unwrap()),
        2
    );
}
