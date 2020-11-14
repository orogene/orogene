use oro_package_spec::{GitHost, GitInfo, PackageSpec, PackageSpecError};
use url::Url;

type Result<T> = std::result::Result<T, PackageSpecError>;

fn parse(input: &str) -> Result<PackageSpec> {
    input.parse()
}

#[test]
fn git_spec_hosted_basic() -> Result<()> {
    let res = parse("github:foo/bar")?;
    assert_eq!(
        res,
        PackageSpec::Git(GitInfo::Hosted {
            host: GitHost::GitHub,
            owner: "foo".into(),
            repo: "bar".into(),
            committish: None,
            semver: None,
            requested: None,
        })
    );
    Ok(())
}

#[test]
fn git_spec_hosted_supported_hosts() -> Result<()> {
    let res = parse("github:foo/bar")?;
    assert_eq!(
        res,
        PackageSpec::Git(GitInfo::Hosted {
            host: GitHost::GitHub,
            owner: "foo".into(),
            repo: "bar".into(),
            committish: None,
            semver: None,
            requested: None,
        })
    );
    let res = parse("gitlab:foo/bar")?;
    assert_eq!(
        res,
        PackageSpec::Git(GitInfo::Hosted {
            host: GitHost::GitLab,
            owner: "foo".into(),
            repo: "bar".into(),
            committish: None,
            semver: None,
            requested: None,
        })
    );
    let res = parse("bitbucket:foo/bar")?;
    assert_eq!(
        res,
        PackageSpec::Git(GitInfo::Hosted {
            host: GitHost::Bitbucket,
            owner: "foo".into(),
            repo: "bar".into(),
            committish: None,
            semver: None,
            requested: None,
        })
    );
    let res = parse("gist:foo/bar")?;
    assert_eq!(
        res,
        PackageSpec::Git(GitInfo::Hosted {
            host: GitHost::Gist,
            owner: "foo".into(),
            repo: "bar".into(),
            committish: None,
            semver: None,
            requested: None,
        })
    );
    let res = parse("garbag:foo/bar");
    assert!(res.is_err());
    Ok(())
}

#[test]
fn git_spec_hosted_committish() -> Result<()> {
    let res = parse("github:foo/bar#dsfargeg")?;
    assert_eq!(
        res,
        PackageSpec::Git(GitInfo::Hosted {
            host: GitHost::GitHub,
            owner: "foo".into(),
            repo: "bar".into(),
            committish: Some("dsfargeg".into()),
            semver: None,
            requested: None,
        })
    );
    Ok(())
}

#[test]
fn git_spec_hosted_semver() -> Result<()> {
    let res = parse("github:foo/bar#semver:^1.2.3")?;
    assert_eq!(
        res,
        PackageSpec::Git(GitInfo::Hosted {
            host: GitHost::GitHub,
            owner: "foo".into(),
            repo: "bar".into(),
            committish: None,
            semver: Some("^1.2.3".parse().unwrap()),
            requested: None,
        })
    );
    let res = parse("github:foo/bar#semver:1.2.3")?;
    assert_eq!(
        res,
        PackageSpec::Git(GitInfo::Hosted {
            host: GitHost::GitHub,
            owner: "foo".into(),
            repo: "bar".into(),
            committish: None,
            semver: Some("1.2.3".parse().unwrap()),
            requested: None,
        })
    );
    Ok(())
}

#[test]
fn git_spec_hosted_implicit_github() -> Result<()> {
    let res = parse("foo/bar")?;
    assert_eq!(
        res,
        PackageSpec::Git(GitInfo::Hosted {
            host: GitHost::GitHub,
            owner: "foo".into(),
            repo: "bar".into(),
            committish: None,
            semver: None,
            requested: None,
        })
    );
    Ok(())
}

#[test]
fn git_spec_url_basic() -> Result<()> {
    let res = parse("git://foo.com/foo/bar")?;
    assert_eq!(
        res,
        PackageSpec::Git(GitInfo::Url {
            url: Url::parse("git://foo.com/foo/bar").unwrap(),
            committish: None,
            semver: None,
        })
    );
    Ok(())
}

#[test]
fn git_spec_url_gitplus() -> Result<()> {
    let res = parse("git+https://foo.com/foo/bar")?;
    assert_eq!(
        res,
        PackageSpec::Git(GitInfo::Url {
            url: Url::parse("https://foo.com/foo/bar").unwrap(),
            committish: None,
            semver: None
        })
    );
    Ok(())
}

#[test]
fn git_spec_url_committish() -> Result<()> {
    let res = parse("git://foo.com/foo/bar#mybranch")?;
    assert_eq!(
        res,
        PackageSpec::Git(GitInfo::Url {
            url: Url::parse("git://foo.com/foo/bar").unwrap(),
            committish: Some("mybranch".into()),
            semver: None,
        })
    );
    Ok(())
}

#[test]
fn git_spec_url_semver() -> Result<()> {
    let res = parse("git://foo.com/foo/bar#semver:^1.2.3")?;
    assert_eq!(
        res,
        PackageSpec::Git(GitInfo::Url {
            url: Url::parse("git://foo.com/foo/bar").unwrap(),
            committish: None,
            semver: Some("^1.2.3".parse().unwrap()),
        })
    );
    Ok(())
}

#[test]
fn git_spec_url_hosted() -> Result<()> {
    let res = parse("git+https://github.com/foo/bar")?;
    assert_eq!(
        res,
        PackageSpec::Git(GitInfo::Hosted {
            host: GitHost::GitHub,
            owner: "foo".into(),
            repo: "bar".into(),
            committish: None,
            semver: None,
            requested: Some("https://github.com/foo/bar".into()),
        })
    );
    Ok(())
}

#[test]
fn git_spec_url_hosted_dotgit() -> Result<()> {
    let res = parse("git+https://github.com/foo/bar.git")?;
    assert_eq!(
        res,
        PackageSpec::Git(GitInfo::Hosted {
            host: GitHost::GitHub,
            owner: "foo".into(),
            repo: "bar".into(),
            committish: None,
            semver: None,
            requested: Some("https://github.com/foo/bar.git".into()),
        })
    );
    Ok(())
}

#[test]
fn git_spec_url_hosted_committish() -> Result<()> {
    let res = parse("git+https://github.com/foo/bar.git#mybranch")?;
    assert_eq!(
        res,
        PackageSpec::Git(GitInfo::Hosted {
            host: GitHost::GitHub,
            owner: "foo".into(),
            repo: "bar".into(),
            committish: Some("mybranch".into()),
            semver: None,
            requested: Some("https://github.com/foo/bar.git".into()),
        })
    );
    Ok(())
}

#[test]
fn git_spec_scp_basic() -> Result<()> {
    let res = parse("ssh://blah@foo.com:foo/bar")?;
    assert_eq!(
        res,
        PackageSpec::Git(GitInfo::Ssh {
            ssh: "blah@foo.com:foo/bar".into(),
            committish: None,
            semver: None,
        })
    );
    let res = parse("git+ssh://blah@foo.com:foo/bar")?;
    assert_eq!(
        res,
        PackageSpec::Git(GitInfo::Ssh {
            ssh: "blah@foo.com:foo/bar".into(),
            committish: None,
            semver: None,
        })
    );
    Ok(())
}

#[test]
fn git_spec_scp_committish() -> Result<()> {
    let res = parse("git+ssh://blah@foo.com:foo/bar#heythere")?;
    assert_eq!(
        res,
        PackageSpec::Git(GitInfo::Ssh {
            ssh: "blah@foo.com:foo/bar".into(),
            committish: Some("heythere".into()),
            semver: None,
        })
    );
    Ok(())
}

#[test]
fn git_spec_scp_semver() -> Result<()> {
    let res = parse("git+ssh://blah@foo.com:foo/bar#semver:^1.2.3")?;
    assert_eq!(
        res,
        PackageSpec::Git(GitInfo::Ssh {
            ssh: "blah@foo.com:foo/bar".into(),
            committish: None,
            semver: Some("^1.2.3".parse().unwrap()),
        })
    );
    Ok(())
}

#[test]
fn git_spec_scp_hosted() -> Result<()> {
    let res = parse("git+ssh://git@github.com:foo/bar")?;
    assert_eq!(
        res,
        PackageSpec::Git(GitInfo::Hosted {
            host: GitHost::GitHub,
            owner: "foo".into(),
            repo: "bar".into(),
            committish: None,
            semver: None,
            requested: Some("git@github.com:foo/bar".into()),
        })
    );
    Ok(())
}

#[test]
fn git_spec_scp_hosted_dotgit() -> Result<()> {
    let res = parse("git+ssh://git@github.com:foo/bar.git")?;
    assert_eq!(
        res,
        PackageSpec::Git(GitInfo::Hosted {
            host: GitHost::GitHub,
            owner: "foo".into(),
            repo: "bar".into(),
            committish: None,
            semver: None,
            requested: Some("git@github.com:foo/bar.git".into()),
        })
    );
    Ok(())
}
