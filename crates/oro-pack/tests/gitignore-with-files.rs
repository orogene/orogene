use fs::File;
use oro_pack::OroPack;
use std::env;
use std::fs;
use std::io::Write as _;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn git_ignore() -> std::io::Result<()> {
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
           "*.ts"
        ]
    }
    "#
        .as_bytes(),
    )?;

    let mut _gitignore = File::create(dir_path.join(".gitignore"))?;

    _gitignore.write_all("sub/module.js\n*.ts\n!important/*.ts\n!yarn.lock".as_bytes())?;

    fs::create_dir_all(dir_path.join("sub/sub")).unwrap();
    fs::create_dir_all(dir_path.join("important")).unwrap();

    let _a = File::create(dir_path.join("sub/module.js"))?;
    let _b = File::create(dir_path.join("sub/sub/module.js"))?;
    let _c = File::create(dir_path.join("module.js"))?;
    let _d = File::create(dir_path.join("types.ts"))?;
    let _e = File::create(dir_path.join("important/include.ts"))?;
    let _f = File::create(dir_path.join("yarn.lock"))?;

    env::set_current_dir(dir.path())?;

    let mut pack = OroPack::new();
    let mut expected_paths = vec![
        Path::new("package.json"),
        Path::new("important/include.ts"),
        Path::new("types.ts"),
    ];

    pack.load();

    expected_paths.sort();

    assert_eq!(expected_paths, pack.project_paths());

    env::set_current_dir(cwd)?;

    drop(pkg_json);
    drop(_gitignore);

    drop(_a);
    drop(_b);
    drop(_c);

    dir.close()?;

    Ok(())
}
