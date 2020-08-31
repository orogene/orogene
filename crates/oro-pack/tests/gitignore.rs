use oro_pack::OroPack;
use std::env;
use std::fs;
use std::path::Path;
use tempdir::TempDir;

#[test]
fn git_ignore() {
    let dir = TempDir::new("test").unwrap();
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

    let expected_paths = vec![Path::new("package.json")];

    pack.load();

    let files = pack.project_paths();

    assert_eq!(expected_paths, files);

    dir.close().unwrap();
}
