use fs::File;
use oro_pack::*;
use std::env;
use std::io::Write as _;
use std::{fs, path::Path};
use tempfile::tempdir;

#[test]
fn pkg_json_has_no_files() -> std::io::Result<()> {
    let cwd = env::current_dir()?;

    let dir = tempdir()?;
    let dir_path = dir.path();
    let pkg_path = dir_path.join("package.json");

    let mut pkg_json = File::create(pkg_path)?;

    pkg_json.write_all(
        r#"
    { 
        "name": "testpackage"
    }
    "#
        .as_bytes(),
    )?;

    fs::create_dir_all(dir_path.join("src")).unwrap();

    let _a = File::create(dir_path.join("src/module.js"))?;
    let _b = File::create(dir_path.join("src/index.js"))?;
    let _c = File::create(dir_path.join("README.md"))?;

    let mut pack = OroPack::new();
    let mut expected_paths = vec![
        Path::new("README.md"),
        Path::new("package.json"),
        Path::new("src/index.js"),
        Path::new("src/module.js"),
    ];

    env::set_current_dir(dir.path())?;

    pack.load();

    expected_paths.sort();

    assert_eq!(expected_paths, pack.project_paths());

    drop(pkg_json);
    drop(_a);
    drop(_b);
    drop(_c);

    env::set_current_dir(cwd)?;

    Ok(())
}
