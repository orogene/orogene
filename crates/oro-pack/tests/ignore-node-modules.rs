use fs::File;
use oro_pack::OroPack;
use std::env;
use std::fs;
use std::io::Write as _;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn ignore_node_modules() -> std::io::Result<()> {
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
    fs::create_dir_all(dir_path.join("node_modules/lib")).unwrap();

    let _a = File::create(dir_path.join("src/module.js"))?;
    let _b = File::create(dir_path.join("node_modules/lib/module.js"))?;

    env::set_current_dir(dir.path())?;

    let mut pack = OroPack::new();
    let mut expected_paths = vec![Path::new("package.json"), Path::new("src/module.js")];

    pack.load();

    expected_paths.sort();

    assert_eq!(expected_paths, pack.project_paths());

    env::set_current_dir(cwd)?;

    drop(pkg_json);

    drop(_a);
    drop(_b);

    dir.close()?;

    Ok(())
}
