use std::thread::sleep;
use std::time::Duration;

use std::io::Write;
use std::process::{Command, Stdio};

fn run_kryon(input: &str) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kryon"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();

    String::from_utf8_lossy(&child.wait_with_output().unwrap().stdout).to_string()
}

#[test]
fn expire_command() {
    let output = run_kryon("SET session abc\nEXPIRE session 1\nTTL session\nQUIT\n");

    assert!(output.contains("abc") || output.contains("1"));
}

#[test]
fn expiration_really_happens() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kryon"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"SET session abc\nEXPIRE session 1\n")
        .unwrap();

    sleep(Duration::from_secs(2));

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"GET session\nTTL session\nQUIT\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("(nil)"));
    assert!(stdout.contains("-2"));
}
