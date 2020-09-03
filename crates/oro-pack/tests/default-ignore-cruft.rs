use fs::File;
use oro_pack::OroPack;
use std::env;
use std::fs;
use std::io::Write as _;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn default_ignore_cruft() -> std::io::Result<()> {
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

    fs::create_dir_all(dir_path.join("build")).unwrap();
    fs::create_dir_all(dir_path.join("npmrc")).unwrap();
    fs::create_dir_all(dir_path.join("archived-packages")).unwrap();
    fs::create_dir_all(dir_path.join(".svn")).unwrap();
    fs::create_dir_all(dir_path.join(".git")).unwrap();
    fs::create_dir_all(dir_path.join(".hg")).unwrap();
    fs::create_dir_all(dir_path.join("CVS")).unwrap();
    fs::create_dir_all(dir_path.join("ds-store/.DS_Store")).unwrap();
    fs::create_dir_all(dir_path.join("folder/._sub-folder")).unwrap();

    let _a = File::create(dir_path.join("yarn.lock"))?;
    let _b = File::create(dir_path.join(".gitignore"))?;
    let _c = File::create(dir_path.join(".npmignore"))?;
    let _d = File::create(dir_path.join(".wafpickle-7"))?;
    let _e = File::create(dir_path.join("build/config.gypi"))?;
    let _f = File::create(dir_path.join("npm-debug.log"))?;
    let _g = File::create(dir_path.join(".npmrc"))?;
    let _h = File::create(dir_path.join("npmrc/.npmrc"))?;
    let _i = File::create(dir_path.join(".test.swp"))?;
    let _j = File::create(dir_path.join(".DS_Store"))?;
    let _k = File::create(dir_path.join("ds-store/.DS_Store/file"))?;
    let _l = File::create(dir_path.join("._redirects"))?;
    let _m = File::create(dir_path.join("folder/._sub-folder/secret_file"))?;
    let _n = File::create(dir_path.join("package-lock.json"))?;
    let _o = File::create(dir_path.join("archived-packages/archived-package"))?;
    let _p = File::create(dir_path.join(".lock_wscript"))?;
    let _r = File::create(dir_path.join(".svn/svn-file"))?;
    let _s = File::create(dir_path.join(".git/git-file"))?;
    let _t = File::create(dir_path.join(".hg/hg-file"))?;
    let _u = File::create(dir_path.join("CVS/cvs-file"))?;
    let _v = File::create(dir_path.join("file.orig"))?;

    env::set_current_dir(dir.path())?;

    let mut pack = OroPack::new();
    let mut expected_paths = vec![Path::new("package.json")];

    pack.load();

    expected_paths.sort();

    assert_eq!(expected_paths, pack.project_paths());

    env::set_current_dir(cwd)?;

    drop(pkg_json);

    drop(_a);
    drop(_b);
    drop(_c);
    drop(_d);
    drop(_e);
    drop(_f);
    drop(_g);
    drop(_h);
    drop(_i);
    drop(_j);
    drop(_k);
    drop(_l);
    drop(_m);
    drop(_n);
    drop(_o);
    drop(_p);
    drop(_r);
    drop(_s);
    drop(_t);
    drop(_u);
    drop(_v);

    dir.close()?;

    Ok(())
}
