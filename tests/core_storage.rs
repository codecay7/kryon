use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn basic_command_flow() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kryon"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to start Kryon");

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"SET name Kryon\nGET name\nEXISTS name\nDEL name\nGET name\nQUIT\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Kryon"));
    assert!(stdout.contains("(nil)"));
}
