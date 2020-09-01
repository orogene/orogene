use fs::File;
use oro_pack::OroPack;
use std::env;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn git_ignore() {
    let dir = tempdir().unwrap();
    let dir_path = dir.path();
    let pkg_path = dir_path.join("package.json");

    fs::write(
        pkg_path,
        r#"
    { 
        "name": "testpackage"
    }
    "#,
    )
    .unwrap();

    let _a = File::create(dir_path.join("index.js")).unwrap();
    let _b = File::create(dir_path.join(".gitignore")).unwrap();

    env::set_current_dir(&dir).unwrap();

    let mut pack = OroPack::new();

    let mut expected_paths = vec![Path::new("package.json")];

    pack.load();

    let mut files = pack.project_paths();

    assert_eq!(expected_paths.sort(), files.sort());

    drop(_a);
    drop(_b);

    dir.close().unwrap();
}
