use std::path::PathBuf;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

macro_rules! assert_fixture {
    ($from:expr) => {{
        let shim_name = $from;
        let tempdir = tempfile::tempdir_in(fixtures()).unwrap();
        let from = fixtures().join(&shim_name);
        let to = tempdir.path().join("shim");
        oro_shim_bin::shim_bin(&from, &to).unwrap();
        insta::assert_snapshot!(
            shim_name,
            std::fs::read_to_string(&to).unwrap().replace('\r', "\\r")
        );
        insta::assert_snapshot!(
            format!("{shim_name}.ps1"),
            std::fs::read_to_string(to.with_extension("ps1"))
                .unwrap()
                .replace('\r', "\\r")
        );
        insta::assert_snapshot!(
            format!("{shim_name}.cmd"),
            std::fs::read_to_string(to.with_extension("cmd"))
                .unwrap()
                .replace('\r', "\\r")
        );
    }};
}

#[test]
fn no_shebang() {
    assert_fixture!("from.exe");
}

#[test]
fn env_shebang() {
    assert_fixture!("from.env");
}

#[test]
fn env_shebang_with_args() {
    assert_fixture!("from.env.args");
}

#[test]
fn env_shebang_vars() {
    assert_fixture!("from.env.variables");
}

#[test]
fn explicit_shebang() {
    assert_fixture!("from.sh");
}

#[test]
fn explicit_shebang_with_args() {
    assert_fixture!("from.sh.args");
}

#[test]
fn multiple_variables() {
    assert_fixture!("from.env.multiple.variables");
}

#[test]
fn shebang_with_env_s() {
    assert_fixture!("from.env.S");
}
