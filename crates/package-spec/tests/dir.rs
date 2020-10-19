use std::path::PathBuf;

use package_spec::{PackageArgError, PackageSpec};

type Result<T> = std::result::Result<T, PackageArgError>;

fn ppa(input: &str) -> Result<PackageSpec> {
    PackageSpec::from_string(input, "/root/")
}

#[test]
fn relative_path_current_dir() -> Result<()> {
    let res = ppa("./")?;
    assert_eq!(
        res,
        PackageSpec::Dir {
            path: PathBuf::from("./"),
            from: PathBuf::from("/root/")
        }
    );
    Ok(())
}

#[test]
fn relative_path_current_dir_no_slash() -> Result<()> {
    let res = ppa(".")?;
    assert_eq!(
        res,
        PackageSpec::Dir {
            path: PathBuf::from("."),
            from: PathBuf::from("/root/")
        }
    );
    Ok(())
}

#[test]
fn relative_path_unix() -> Result<()> {
    let res = ppa("./foo/bar/baz")?;
    assert_eq!(
        res,
        PackageSpec::Dir {
            path: PathBuf::from("./foo/bar/baz"),
            from: PathBuf::from("/root/"),
        }
    );
    Ok(())
}

#[test]
fn absolute_path_unix() -> Result<()> {
    let res = ppa("/foo/bar/baz")?;
    assert_eq!(
        res,
        PackageSpec::Dir {
            path: PathBuf::from("/foo/bar/baz"),
            from: PathBuf::from("/root/"),
        }
    );
    Ok(())
}

#[test]
fn relative_path_windows() -> Result<()> {
    let res = ppa(".\\foo\\bar\\baz")?;
    assert_eq!(
        res,
        PackageSpec::Dir {
            path: PathBuf::from(".\\foo\\bar\\baz"),
            from: PathBuf::from("/root/"),
        }
    );
    Ok(())
}

#[test]
fn absolute_path_windows() -> Result<()> {
    let res = ppa("C:\\foo\\bar\\baz")?;
    assert_eq!(
        res,
        PackageSpec::Dir {
            path: PathBuf::from("C:\\foo\\bar\\baz"),
            from: PathBuf::from("/root/"),
        }
    );
    Ok(())
}

#[test]
fn absolute_path_windows_qmark() -> Result<()> {
    let res = ppa("\\\\?\\foo\\bar\\baz")?;
    assert_eq!(
        res,
        PackageSpec::Dir {
            path: PathBuf::from("\\\\?\\foo\\bar\\baz"),
            from: PathBuf::from("/root/"),
        }
    );
    Ok(())
}

#[test]
fn absolute_path_windows_double_slash() -> Result<()> {
    let res = ppa("\\\\foo\\bar\\baz")?;
    assert_eq!(
        res,
        PackageSpec::Dir {
            path: PathBuf::from("\\\\foo\\bar\\baz"),
            from: PathBuf::from("/root/"),
        }
    );
    Ok(())
}

#[test]
fn absolute_path_windows_multiple_drive_letters() -> Result<()> {
    let res = ppa("ACAB:\\foo\\bar\\baz");
    assert!(res.is_err());
    Ok(())
}

#[test]
fn named() -> Result<()> {
    let res = ppa("foo@./hey")?;
    assert_eq!(
        res,
        PackageSpec::Alias {
            name: "foo".into(),
            package: Box::new(PackageSpec::Dir {
                path: PathBuf::from("./hey"),
                from: PathBuf::from("/root/"),
            })
        }
    );
    Ok(())
}

#[test]
fn spaces() -> Result<()> {
    // NOTE: This succeeds in NPM, but we treat it as an error because we
    // require ./ for relative paths.
    let res = ppa("@f fo o al/ a d s ;f");
    assert!(res.is_err());
    Ok(())
}
