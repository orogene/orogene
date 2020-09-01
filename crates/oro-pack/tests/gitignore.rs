use fs::File;
use oro_pack::OroPack;
use std::env;
use std::fs;
use std::io::Write as _;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn git_ignore() -> std::io::Result<()> {
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

    let _a = File::create(dir_path.join("index.js"))?;
    let _b = File::create(dir_path.join(".gitignore"))?;

    env::set_current_dir(&dir)?;

    let mut pack = OroPack::new();

    let mut expected_paths = vec![Path::new("package.json")];

    pack.load();

    let mut files = pack.project_paths();

    assert_eq!(expected_paths.sort(), files.sort());

    drop(pkg_json);

    drop(_a);
    drop(_b);

    dir.close()?;

    Ok(())
}
