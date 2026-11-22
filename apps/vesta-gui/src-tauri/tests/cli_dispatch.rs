//! Terminal help must work without initializing a display or starting Tauri.
use std::process::Command;

#[test]
fn desktop_executable_dispatches_terminal_help_without_a_display() {
    for args in [
        vec!["--help"],
        vec!["flashcards", "generate", "--help"],
        vec!["sync", "--help"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_vesta"))
            .args(&args)
            .env_remove("DISPLAY")
            .env_remove("WAYLAND_DISPLAY")
            .output()
            .expect("run Vesta terminal help");
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let help = String::from_utf8(output.stdout).unwrap();
        assert!(help.contains("Usage:"), "{args:?}: {help}");
    }
}

#[test]
fn unknown_terminal_command_is_an_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_vesta"))
        .arg("not-a-vesta-command")
        .env_remove("DISPLAY")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not-a-vesta-command"));
}
