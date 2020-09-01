use fs::File;
use oro_pack::OroPack;
use std::env;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn ignore_cruft() {
    let dir = tempdir().unwrap();
    let dir_path = dir.path();
    let pkg_path = dir_path.join("package.json");

    fs::write(
        pkg_path,
        r#"
    { 
        "name": "testpackage"
    }
    "#,
    )
    .unwrap();

    /*     fs::create_dir_all(dir_path.join("build")).unwrap();
    fs::create_dir_all(dir_path.join("npmrc")).unwrap();
    fs::create_dir_all(dir_path.join("archived-packages")).unwrap();
    fs::create_dir_all(dir_path.join(".svn")).unwrap();
    fs::create_dir_all(dir_path.join(".git")).unwrap();
    fs::create_dir_all(dir_path.join(".hg")).unwrap();
    fs::create_dir_all(dir_path.join("CVS")).unwrap();
    fs::create_dir_all(dir_path.join("ds-store/.DS_Store")).unwrap();
    fs::create_dir_all(dir_path.join("folder/._sub-folder")).unwrap(); */

    let _a = File::create(dir_path.join("yarn.lock")).unwrap();
    let _b = File::create(dir_path.join(".gitignore")).unwrap();
    let _c = File::create(dir_path.join(".npmignore")).unwrap();
    let _d = File::create(dir_path.join(".wafpickle-7")).unwrap();
    // let _e = File::create(dir_path.join("build/config.gypi")).unwrap();
    let _f = File::create(dir_path.join("npm-debug.log")).unwrap();
    let _g = File::create(dir_path.join(".npmrc")).unwrap();
    // let _h = File::create(dir_path.join("npmrc/.npmrc")).unwrap();
    let _i = File::create(dir_path.join(".test.swp")).unwrap();
    let _j = File::create(dir_path.join(".DS_Store")).unwrap();
    // let _k = File::create(dir_path.join("ds-store/.DS_Store/file")).unwrap();
    let _l = File::create(dir_path.join("._redirects")).unwrap();
    // let _m = File::create(dir_path.join("folder/._sub-folder/secret_file")).unwrap();
    let _n = File::create(dir_path.join("package-lock.json")).unwrap();
    // let _o = File::create(dir_path.join("archived-packages/archived-package")).unwrap();
    let _p = File::create(dir_path.join(".lock_wscript")).unwrap();
    // let _r = File::create(dir_path.join(".svn/svn-file")).unwrap();
    // let _s = File::create(dir_path.join(".git/git-file")).unwrap();
    // let _t = File::create(dir_path.join(".hg/hg-file")).unwrap();
    // let _u = File::create(dir_path.join("CVS/cvs-file")).unwrap();
    let _v = File::create(dir_path.join("file.orig")).unwrap();

    env::set_current_dir(&dir).unwrap();

    let mut pack = OroPack::new();

    let mut expected_paths = vec![Path::new("package.json")];

    pack.load();

    let mut files = pack.project_paths();

    assert_eq!(expected_paths.sort(), files.sort());

    drop(_a);
    drop(_b);
    drop(_c);
    drop(_d);
    // drop(_e);
    drop(_f);
    drop(_g);
    // drop(_h);
    drop(_i);
    drop(_j);
    // drop(_k);
    drop(_l);
    // drop(_m);
    drop(_n);
    // drop(_o);
    drop(_p);
    // drop(_r);
    // drop(_s);
    // drop(_t);
    // drop(_u);
    drop(_v);

    dir.close().unwrap();
}
