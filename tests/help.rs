use std::process::{Command, Output, Stdio};

static BIN: &str = env!("CARGO_BIN_EXE_oro");

#[test]
fn add_markdown() {
    insta::assert_snapshot!("add", sub_md("add"));
}

#[test]
fn apply_markdown() {
    insta::assert_snapshot!("apply", sub_md("apply"));
}

#[test]
fn login_markdown() {
    insta::assert_snapshot!("login", sub_md("login"));
}

#[test]
fn logout_markdown() {
    insta::assert_snapshot!("logout", sub_md("logout"));
}

#[test]
fn ping_markdown() {
    insta::assert_snapshot!("ping", sub_md("ping"));
}

#[test]
fn reapply_markdown() {
    insta::assert_snapshot!("reapply", sub_md("reapply"));
}

#[test]
fn remove_markdown() {
    insta::assert_snapshot!("remove", sub_md("remove"));
}

#[test]
fn view_markdown() {
    insta::assert_snapshot!("view", sub_md("view"));
}

fn sub_md(subcmd: &str) -> String {
    let output = Command::new(BIN)
        .arg("help-markdown")
        .arg(subcmd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute process");

    assert!(output.status.success(), "{}", format_output(&output));
    format_output(&output)
}

fn format_output(output: &Output) -> String {
    let stdout = std::str::from_utf8(&output.stdout).unwrap();
    let stderr = std::str::from_utf8(&output.stderr).unwrap();
    format!("stderr:\n{stderr}\nstdout:\n{stdout}")
}
