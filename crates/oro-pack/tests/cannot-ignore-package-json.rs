use fs::File;
use oro_pack::OroPack;
use std::env;
use std::fs;
use std::io::Write as _;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn always_included() -> std::io::Result<()> {
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

    let mut _ig = File::create(dir_path.join(".gitignore"))?;
    _ig.write_all("package.json".as_bytes())?;

    env::set_current_dir(dir.path())?;

    let mut pack = OroPack::new();
    let mut expected_paths = vec![Path::new("package.json")];

    pack.load();

    expected_paths.sort();

    assert_eq!(expected_paths, pack.project_paths());

    env::set_current_dir(cwd)?;

    drop(pkg_json);
    drop(_ig);

    dir.close()?;

    Ok(())
}
