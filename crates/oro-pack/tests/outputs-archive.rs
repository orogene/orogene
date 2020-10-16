use async_std::fs::File as AsyncFile;
use async_std::io as AsyncIO;
use async_std::prelude::*;
use async_std::task::block_on;
use async_tar::Archive;
use fs::File;
use oro_pack::OroPack;
use std::env;
use std::fs;
use std::io::Write as _;
use tempfile::tempdir;

async fn load_archive() -> AsyncIO::Result<Vec<String>> {
    let mut archived_files: Vec<String> = Vec::new();

    let archive = Archive::new(AsyncFile::open("testpackage.tar").await.unwrap());
    let mut entries = archive.entries().unwrap();

    while let Some(f) = entries.next().await {
        let file = f.unwrap();
        let path = file.path();
        archived_files.push(path.unwrap().display().to_string());
    }

    Ok(archived_files)
}

#[test]
fn outputs_archive() -> AsyncIO::Result<()> {
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
          "lib/index.js",
          "lib/index.es.js",
          "!lib/*.png"
        ]
    }
    "#
        .as_bytes(),
    )?;

    fs::create_dir_all(dir_path.join("lib")).unwrap();

    let _a = File::create(dir_path.join("README.md"))?;
    let _b = File::create(dir_path.join("lib/index.js"))?;
    let _c = File::create(dir_path.join("lib/index.es.js"))?;
    let _d = File::create(dir_path.join("lib/pic.png"))?;

    env::set_current_dir(dir.path())?;

    let mut pack = OroPack::new();

    pack.load();

    pack.pack()?;

    let expected_paths = vec![
        "README.md",
        "lib/index.es.js",
        "lib/index.js",
        "package.json",
    ];

    let archived_files = block_on(load_archive())?;

    assert_eq!(expected_paths, archived_files);

    env::set_current_dir(cwd)?;

    drop(pkg_json);

    drop(_a);
    drop(_b);
    drop(_c);
    drop(_d);

    dir.close()?;

    Ok(())
}
