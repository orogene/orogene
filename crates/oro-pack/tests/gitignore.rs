use directories::UserDirs;
use oro_pack::OroPack;
use std::env;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn git_ignore() {
    if cfg!(windows) {
        let user_dirs = UserDirs::new().unwrap();
        env::set_var("TMP", user_dirs.home_dir());
    }

    let dir = tempdir().unwrap();
    let dir_path = dir.path();
    let pkg_path = dir_path.join("package.json");

    fs::write(
        pkg_path,
        r#"
    { 
        "name": "testpackage",
        "files": []
    }
    "#,
    )
    .unwrap();

    fs::write(dir_path.join("index.js"), "").unwrap();
    fs::write(dir_path.join(".gitignore"), "index.js").unwrap();

    env::set_current_dir(&dir).unwrap();

    let mut pack = OroPack::new();

    let mut expected_paths = vec![Path::new("package.json")];

    pack.load();

    let mut files = pack.project_paths();

    assert_eq!(expected_paths.sort(), files.sort());

    dir.close().unwrap();
}
