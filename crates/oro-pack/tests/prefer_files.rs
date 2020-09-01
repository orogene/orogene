use fs::File;
use oro_pack::OroPack;
use std::env;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn ignore_cruft() {
    let dir = tempdir().unwrap();
    let dir_path = dir.path();
    let pkg_path = dir_path.join("package.json");

    fs::write(
        pkg_path,
        r#"
    { 
        "name": "testpackage",
        "files": [
            "yarn.lock"
        ]
    }
    "#,
    )
    .unwrap();

    let _a = File::create(dir_path.join("yarn.lock")).unwrap();

    env::set_current_dir(&dir).unwrap();

    let mut pack = OroPack::new();

    let mut expected_paths = vec![Path::new("package.json"), Path::new("yarn.lock")];

    pack.load();

    let mut files = pack.project_paths();

    assert_eq!(expected_paths.sort(), files.sort());

    drop(_a);

    dir.close().unwrap();
}
