use package_arg::{PackageArg, PackageArgError, VersionReq};

type Result<T> = std::result::Result<T, PackageArgError>;

fn ppa(input: &str) -> Result<PackageArg> {
    input.parse()
}

fn version_req(input: &str) -> Option<VersionReq> {
    Some(VersionReq::Range(
        semver::VersionReq::parse_compat(input, semver::Compat::Node).unwrap(),
    ))
}

#[test]
fn npm_pkg_basic() -> Result<()> {
    let res = ppa("hello-world")?;
    assert_eq!(
        res,
        PackageArg::Npm {
            scope: None,
            name: "hello-world".into(),
            requested: None
        }
    );
    Ok(())
}

#[test]
fn npm_pkg_tag() -> Result<()> {
    let res = ppa("hello-world@latest")?;
    assert_eq!(
        res,
        PackageArg::Npm {
            scope: None,
            name: "hello-world".into(),
            requested: Some(VersionReq::Tag("latest".into()))
        }
    );
    Ok(())
}

#[test]
fn alias_npm_pkg_basic() -> Result<()> {
    let res = ppa("foo@npm:hello-world")?;
    assert_eq!(
        res,
        PackageArg::Alias {
            name: "foo".into(),
            package: Box::new(PackageArg::Npm {
                scope: None,
                name: "hello-world".into(),
                requested: None
            })
        }
    );
    Ok(())
}

#[test]
fn alias_not_recursive() -> Result<()> {
    let res = ppa("foo@bar@npm:hello-world");
    assert!(res.is_err());
    Ok(())
}

#[test]
fn npm_pkg_prefixed() -> Result<()> {
    let res = ppa("npm:hello-world")?;
    assert_eq!(
        res,
        PackageArg::Npm {
            scope: None,
            name: "hello-world".into(),
            requested: None
        }
    );
    Ok(())
}

#[test]
fn npm_pkg_scoped() -> Result<()> {
    let res = ppa("@hello/world")?;
    assert_eq!(
        res,
        PackageArg::Npm {
            scope: Some("hello".into()),
            name: "world".into(),
            requested: None
        }
    );
    Ok(())
}

#[test]
fn npm_pkg_with_req() -> Result<()> {
    let res = ppa("hello-world@1.2.3")?;
    assert_eq!(
        res,
        PackageArg::Npm {
            scope: None,
            name: "hello-world".into(),
            requested: Some(VersionReq::Version(
                semver::Version::parse("1.2.3").unwrap()
            ))
        }
    );
    Ok(())
}

#[test]
fn npm_pkg_with_tag() -> Result<()> {
    let res = ppa("hello-world@howdy")?;
    assert_eq!(
        res,
        PackageArg::Npm {
            scope: None,
            name: "hello-world".into(),
            requested: Some(VersionReq::Tag("howdy".into())),
        }
    );
    Ok(())
}

#[test]
fn npm_pkg_scoped_with_req() -> Result<()> {
    let res = ppa("@hello/world@1.2.3")?;
    assert_eq!(
        res,
        PackageArg::Npm {
            scope: Some("hello".into()),
            name: "world".into(),
            requested: Some(VersionReq::Version(
                semver::Version::parse("1.2.3").unwrap()
            ))
        }
    );
    Ok(())
}

#[test]
fn npm_pkg_prefixed_with_req() -> Result<()> {
    let res = ppa("npm:@hello/world@1.2.3")?;
    assert_eq!(
        res,
        PackageArg::Npm {
            scope: Some("hello".into()),
            name: "world".into(),
            requested: Some(VersionReq::Version(
                semver::Version::parse("1.2.3").unwrap()
            ))
        }
    );
    Ok(())
}

#[test]
fn odd_npm_example_with_prerelease() -> Result<()> {
    let res = ppa("world@>1.1.0-beta-10")?;
    assert_eq!(
        res,
        PackageArg::Npm {
            scope: None,
            name: "world".into(),
            requested: version_req(">1.1.0-beta-10"),
        }
    );
    Ok(())
}

#[test]
fn approximately_equivalent_version() -> Result<()> {
    let res = ppa("world@~1.1.0")?;
    assert_eq!(
        res,
        PackageArg::Npm {
            scope: None,
            name: "world".into(),
            requested: version_req("~1.1.0"),
        }
    );
    Ok(())
}

#[test]
fn compatible_equivalent_version() -> Result<()> {
    let res = ppa("world@^1.1.0")?;
    assert_eq!(
        res,
        PackageArg::Npm {
            scope: None,
            name: "world".into(),
            requested: version_req("^1.1.0"),
        }
    );
    Ok(())
}

#[test]
fn x_version() -> Result<()> {
    let res = ppa("world@1.1.x")?;
    assert_eq!(
        res,
        PackageArg::Npm {
            scope: None,
            name: "world".into(),
            requested: version_req("1.1.x"),
        }
    );
    Ok(())
}

#[test]
fn hyphen_version_range() -> Result<()> {
    let res = ppa("world@1.5.0 - 2.1.0")?;
    assert_eq!(
        res,
        PackageArg::Npm {
            scope: None,
            name: "world".into(),
            requested: version_req("1.5.0 - 2.1.0"),
        }
    );
    Ok(())
}

#[test]
fn alternate_version_ranges() -> Result<()> {
    let res = ppa("world@1.5.0 - 2.1.0 || 2.3.x")?;
    assert_eq!(
        res,
        PackageArg::Npm {
            scope: None,
            name: "world".into(),
            requested: version_req("1.5.0 - 2.1.0 || 2.3.x"),
        }
    );
    Ok(())
}

#[test]
fn npm_pkg_bad_tag() -> Result<()> {
    let res = ppa("hello-world@%&W$@#$");
    assert!(res.is_err());
    Ok(())
}
