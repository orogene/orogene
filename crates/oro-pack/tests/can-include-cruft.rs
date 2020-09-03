use fs::File;
use oro_pack::OroPack;
use std::env;
use std::fs;
use std::io::Write as _;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn prefer_pkg_json_files() -> std::io::Result<()> {
    let cwd = env::current_dir()?;

    let dir = tempdir()?;
    let dir_path = dir.path();
    let pkg_path = dir_path.join("package.json");

    let mut pkg_json = File::create(pkg_path)?;

    pkg_json.write_all(
        r#"
    { 
        "name": "testpackage",
        "files": [
            "yarn.lock",
            ".npmrc",
            ".gitignore"
        ]
    }
    "#
        .as_bytes(),
    )?;

    let _a = File::create(dir_path.join("yarn.lock"))?;

    env::set_current_dir(&dir)?;

    let mut pack = OroPack::new();
    let mut expected_paths = vec![Path::new("package.json"), Path::new("yarn.lock")];

    pack.load();

    expected_paths.sort();

    assert_eq!(expected_paths, pack.project_paths());

    env::set_current_dir(cwd)?;

    drop(pkg_json);
    drop(_a);

    dir.close()?;

    Ok(())
}
