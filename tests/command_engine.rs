use std::io::Write;
use std::process::{Command, Stdio};

fn run_kryon(input: &str) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kryon"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to start Kryon");

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();

    String::from_utf8_lossy(&child.wait_with_output().unwrap().stdout).to_string()
}

#[test]
fn set_get_delete_flow() {
    let output = run_kryon("SET name Kryon\nGET name\nDEL name\nGET name\nQUIT\n");

    assert!(output.contains("Kryon"));
    assert!(output.contains("(nil)"));
}

#[test]
fn clear_and_len_flow() {
    let output = run_kryon("SET a 100\nSET b 200\nLEN\nCLEAR\nLEN\nQUIT\n");

    assert!(output.contains("2"));
    assert!(output.contains("OK"));
    assert!(output.matches("0").count() >= 1);
}
