use oro_pack::*;
use std::env;
use std::path::Path;

#[test]
fn pkg_json_has_files() -> std::io::Result<()> {
    let mut cwd = env::current_dir()?;
    cwd.push("fixtures/explicit_files");
    env::set_current_dir(cwd)?;

    let mut pack = OroPack::new();
    let mut expected_paths = vec![
        Path::new("src/module.js"),
        Path::new("package.json"),
        Path::new("README.md"),
    ];

    pack.load();

    expected_paths.sort();

    assert_eq!(expected_paths, pack.project_paths());

    Ok(())
}
