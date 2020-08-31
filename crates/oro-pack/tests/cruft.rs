use oro_pack::OroPack;
use std::env;
use std::fs;
use std::path::Path;
use tempdir::TempDir;

#[test]
fn ignore_cruft() {
    let dir = TempDir::new("test").unwrap();
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

    fs::create_dir(dir_path.join("build")).unwrap();
    fs::create_dir(dir_path.join("npmrc")).unwrap();
    fs::create_dir(dir_path.join("archived-packages")).unwrap();
    fs::create_dir(dir_path.join(".svn")).unwrap();
    fs::create_dir(dir_path.join(".git")).unwrap();
    fs::create_dir(dir_path.join(".hg")).unwrap();
    fs::create_dir(dir_path.join("CVS")).unwrap();
    fs::create_dir_all(dir_path.join("ds-store/.DS_Store")).unwrap();
    fs::create_dir_all(dir_path.join("folder/._sub-folder")).unwrap();

    fs::write(dir_path.join("yarn.lock"), "").unwrap();
    fs::write(dir_path.join(".gitignore"), "").unwrap();
    fs::write(dir_path.join(".npmignore"), "").unwrap();
    fs::write(dir_path.join(".wafpickle-7"), "").unwrap();
    fs::write(dir_path.join("build/config.gypi"), "").unwrap();
    fs::write(dir_path.join("npm-debug.log"), "").unwrap();
    fs::write(dir_path.join("npmrc/.npmrc"), "").unwrap();
    fs::write(dir_path.join(".test.swp"), "").unwrap();
    fs::write(dir_path.join(".DS_Store"), "").unwrap();
    fs::write(dir_path.join("ds-store/.DS_Store/file"), "").unwrap();
    fs::write(dir_path.join("._redirects"), "").unwrap();
    fs::write(dir_path.join("folder/._sub-folder/secret_file"), "").unwrap();
    fs::write(dir_path.join("package-lock.json"), "").unwrap();
    fs::write(dir_path.join("archived-packages/archived-package"), "").unwrap();
    fs::write(dir_path.join(".lock_wscript"), "").unwrap();
    fs::write(dir_path.join(".svn/svn-file"), "").unwrap();
    fs::write(dir_path.join(".git/git-file"), "").unwrap();
    fs::write(dir_path.join(".hg/hg-file"), "").unwrap();
    fs::write(dir_path.join("CVS/cvs-file"), "").unwrap();
    fs::write(dir_path.join("file.orig"), "").unwrap();

    env::set_current_dir(&dir).unwrap();

    let mut pack = OroPack::new();

    let expected_paths = vec![Path::new("yarn.lock"), Path::new("package.json")];

    pack.load();

    let files = pack.project_paths();

    assert_eq!(expected_paths, files);

    dir.close().unwrap();
}
