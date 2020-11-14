use std::path::PathBuf;

use oro_package_spec::{PackageSpec, PackageSpecError};

type Result<T> = std::result::Result<T, PackageSpecError>;

fn parse(input: &str) -> Result<PackageSpec> {
    input.parse()
}

#[test]
fn relative_path_current_dir() -> Result<()> {
    let res = parse("./")?;
    assert_eq!(
        res,
        PackageSpec::Dir {
            path: PathBuf::from("./"),
        }
    );
    Ok(())
}

#[test]
fn relative_path_current_dir_no_slash() -> Result<()> {
    let res = parse(".")?;
    assert_eq!(
        res,
        PackageSpec::Dir {
            path: PathBuf::from("."),
        }
    );
    Ok(())
}

#[test]
fn relative_path_unix() -> Result<()> {
    let res = parse("./foo/bar/baz")?;
    assert_eq!(
        res,
        PackageSpec::Dir {
            path: PathBuf::from("./foo/bar/baz"),
        }
    );
    Ok(())
}

#[test]
fn absolute_path_unix() -> Result<()> {
    let res = parse("/foo/bar/baz")?;
    assert_eq!(
        res,
        PackageSpec::Dir {
            path: PathBuf::from("/foo/bar/baz"),
        }
    );
    Ok(())
}

#[test]
fn relative_path_windows() -> Result<()> {
    let res = parse(".\\foo\\bar\\baz")?;
    assert_eq!(
        res,
        PackageSpec::Dir {
            path: PathBuf::from(".\\foo\\bar\\baz"),
        }
    );
    Ok(())
}

#[test]
fn absolute_path_windows() -> Result<()> {
    let res = parse("C:\\foo\\bar\\baz")?;
    assert_eq!(
        res,
        PackageSpec::Dir {
            path: PathBuf::from("C:\\foo\\bar\\baz"),
        }
    );
    Ok(())
}

#[test]
fn absolute_path_windows_qmark() -> Result<()> {
    let res = parse("\\\\?\\foo\\bar\\baz")?;
    assert_eq!(
        res,
        PackageSpec::Dir {
            path: PathBuf::from("\\\\?\\foo\\bar\\baz"),
        }
    );
    Ok(())
}

#[test]
fn absolute_path_windows_double_slash() -> Result<()> {
    let res = parse("\\\\foo\\bar\\baz")?;
    assert_eq!(
        res,
        PackageSpec::Dir {
            path: PathBuf::from("\\\\foo\\bar\\baz"),
        }
    );
    Ok(())
}

#[test]
fn absolute_path_windows_multiple_drive_letters() -> Result<()> {
    let res = parse("ACAB:\\foo\\bar\\baz");
    assert!(res.is_err());
    Ok(())
}

#[test]
fn named() -> Result<()> {
    let res = parse("foo@./hey")?;
    assert_eq!(
        res,
        PackageSpec::Alias {
            name: "foo".into(),
            spec: Box::new(PackageSpec::Dir {
                path: PathBuf::from("./hey"),
            })
        }
    );
    Ok(())
}

#[test]
fn spaces() -> Result<()> {
    // NOTE: This succeeds in NPM, but we treat it as an error because we
    // require ./ for relative paths.
    let res = parse("@f fo o al/ a d s ;f");
    assert!(res.is_err());
    Ok(())
}
