use std::path::PathBuf;

use anyhow::Result;

use package_arg::PackageArg;

fn ppa(input: &str) -> Result<PackageArg> {
    input.parse()
}

#[test]
fn relative_path_current_dir() -> Result<()> {
    let res = ppa("./")?;
    assert_eq!(
        res,
        PackageArg::Dir {
            path: PathBuf::from("./")
        }
    );
    Ok(())
}

#[test]
fn relative_path_unix() -> Result<()> {
    let res = ppa("./foo/bar/baz")?;
    assert_eq!(
        res,
        PackageArg::Dir {
            path: PathBuf::from("./foo/bar/baz")
        }
    );
    Ok(())
}

#[test]
fn absolute_path_unix() -> Result<()> {
    let res = ppa("/foo/bar/baz")?;
    assert_eq!(
        res,
        PackageArg::Dir {
            path: PathBuf::from("/foo/bar/baz")
        }
    );
    Ok(())
}

#[test]
fn relative_path_windows() -> Result<()> {
    let res = ppa(".\\foo\\bar\\baz")?;
    assert_eq!(
        res,
        PackageArg::Dir {
            path: PathBuf::from(".\\foo\\bar\\baz")
        }
    );
    Ok(())
}

#[test]
fn absolute_path_windows() -> Result<()> {
    let res = ppa("C:\\foo\\bar\\baz")?;
    assert_eq!(
        res,
        PackageArg::Dir {
            path: PathBuf::from("C:\\foo\\bar\\baz")
        }
    );
    Ok(())
}

#[test]
fn absolute_path_windows_qmark() -> Result<()> {
    let res = ppa("\\\\?\\foo\\bar\\baz")?;
    assert_eq!(
        res,
        PackageArg::Dir {
            path: PathBuf::from("\\\\?\\foo\\bar\\baz")
        }
    );
    Ok(())
}

#[test]
fn absolute_path_windows_double_slash() -> Result<()> {
    let res = ppa("\\\\foo\\bar\\baz")?;
    assert_eq!(
        res,
        PackageArg::Dir {
            path: PathBuf::from("\\\\foo\\bar\\baz")
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
fn spaces() -> Result<()> {
    // NOTE: This succeeds in NPM, but we treat it as an error because we
    // require ./ for relative paths.
    let res = ppa("@f fo o al/ a d s ;f");
    assert!(res.is_err());
    Ok(())
}
