use fs::File;
use oro_pack::OroPack;
use std::env;
use std::fs;
use std::io::Write as _;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn always_included_files() -> std::io::Result<()> {
    let cwd = env::current_dir()?;

    let dir = tempdir()?;
    let dir_path = dir.path();
    let pkg_path = dir_path.join("package.json");

    let mut pkg_json = File::create(pkg_path)?;

    pkg_json.write_all(
        r#"
    { 
        "name": "testpackage",
        "files": []
    }
    "#
        .as_bytes(),
    )?;

    let _a = File::create(dir_path.join("README.md"))?;
    let _c = File::create(dir_path.join("README"))?;
    let _d = File::create(dir_path.join("license"))?;
    let _e = File::create(dir_path.join("LICENSE.md"))?;
    let _f = File::create(dir_path.join("CHANGELOG.md"))?;
    let _g = File::create(dir_path.join("ChANGeLOG"))?;

    env::set_current_dir(dir.path())?;

    let mut pack = OroPack::new();
    let mut expected_paths = vec![
        Path::new("package.json"),
        Path::new("README.md"),
        Path::new("README"),
        Path::new("license"),
        Path::new("LICENSE.md"),
        Path::new("CHANGELOG.md"),
        Path::new("ChANGeLOG"),
    ];

    pack.load();

    expected_paths.sort();

    assert_eq!(expected_paths, pack.project_paths());

    env::set_current_dir(cwd)?;

    drop(pkg_json);

    drop(_a);
    drop(_c);
    drop(_d);
    drop(_e);
    drop(_f);
    drop(_g);

    dir.close()?;

    Ok(())
}
