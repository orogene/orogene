use std::fmt;
use std::str::FromStr;

use oro_node_semver::VersionReq as Range;
use url::Url;

use crate::error::{PackageSpecError, SpecErrorKind};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GitHost {
    GitHub,
    Gist,
    GitLab,
    Bitbucket,
}

impl FromStr for GitHost {
    type Err = PackageSpecError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "github" => GitHost::GitHub,
            "gist" => GitHost::Gist,
            "gitlab" => GitHost::GitLab,
            "bitbucket" => GitHost::Bitbucket,
            _ => {
                return Err(PackageSpecError {
                    input: s.into(),
                    offset: 0,
                    kind: SpecErrorKind::InvalidGitHost(s.into()),
                })
            }
        })
    }
}

impl fmt::Display for GitHost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use GitHost::*;
        write!(
            f,
            "{}",
            match self {
                GitHub => "github",
                Gist => "gist",
                GitLab => "gitlab",
                Bitbucket => "bitbucket",
            }
        )?;
        Ok(())
    }
}

// TODO: impl FromStr? We already have a parser, just need to hook it up.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GitInfo {
    Hosted {
        owner: String,
        repo: String,
        host: GitHost,
        committish: Option<String>,
        semver: Option<Range>,
        requested: Option<String>,
    },
    Url {
        url: Url,
        committish: Option<String>,
        semver: Option<Range>,
    },
    Ssh {
        ssh: String,
        committish: Option<String>,
        semver: Option<Range>,
    },
}

impl GitInfo {
    pub fn ssh(&self) -> Option<String> {
        use GitHost::*;
        use GitInfo::*;
        match self {
            GitInfo::Url { .. } | Ssh { .. } => None,
            Hosted {
                ref host,
                ref owner,
                ref repo,
                ..
            } => Some(match host {
                GitHub => format!("git@github.com:{}/{}.git", owner, repo),
                Gist => format!("git@gist.github.com:/{}", repo),
                GitLab => format!("git@gitlab.com:{}/{}.git", owner, repo),
                Bitbucket => format!("git@bitbucket.com:{}/{}", owner, repo),
            })
            .map(|url| url.parse().expect("URL failed to parse")),
        }
    }

    pub fn https(&self) -> Option<Url> {
        use GitHost::*;
        use GitInfo::*;
        match self {
            GitInfo::Url { .. } | Ssh { .. } => None,
            Hosted {
                ref host,
                ref owner,
                ref repo,
                ..
            } => Some(match host {
                GitHub => format!("https://github.com/{}/{}.git", owner, repo),
                Gist => format!("https://gist.github.com/{}.git", repo),
                GitLab => format!("https://gitlab.com/{}/{}.git", owner, repo),
                Bitbucket => format!("https://bitbucket.com/{}/{}.git", owner, repo),
            })
            .map(|url| url.parse().expect("URL failed to parse")),
        }
    }

    pub fn tarball(&self) -> Option<Url> {
        use GitHost::*;
        use GitInfo::*;
        match self {
            GitInfo::Url { .. } | Ssh { .. } => None,
            Hosted {
                ref host,
                ref owner,
                ref repo,
                ref committish,
                ..
            } => committish
                .as_ref()
                .map(|commit| match host {
                    GitHub => format!(
                        "https://codeload.github.com/{}/{}/tag.gz/{}",
                        owner, repo, commit
                    ),
                    Gist => format!(
                        "https://codeload.github.com/gist/{}/tar.gz/{}",
                        repo, commit
                    ),
                    GitLab => format!(
                        "https://gitlan.com/{}/{}/repository/archive.tar.gz?ref={}",
                        owner, repo, commit
                    ),
                    Bitbucket => format!(
                        "https://bitbucket.org/{}/{}/get/{}.tar.gz",
                        owner, repo, commit
                    ),
                })
                .map(|url| url.parse().expect("Failed to parse URL.")),
        }
    }
}

impl fmt::Display for GitInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use GitInfo::*;
        match self {
            GitInfo::Url {
                url,
                committish,
                semver,
            } => {
                if url.scheme() != "git" {
                    write!(f, "git+")?;
                }
                write!(f, "{}", url)?;
                if let Some(comm) = committish {
                    write!(f, "#{}", comm)?;
                } else if let Some(semver) = semver {
                    write!(f, "#semver:{}", semver)?;
                }
            }
            Ssh {
                ssh,
                committish,
                semver,
            } => {
                write!(f, "git+ssh://{}", ssh)?;
                if let Some(comm) = committish {
                    write!(f, "#{}", comm)?;
                } else if let Some(semver) = semver {
                    write!(f, "#semver:{}", semver)?;
                }
            }
            Hosted {
                requested,
                owner,
                repo,
                host,
                committish,
                semver,
            } => {
                if let Some(requested) = requested {
                    if !requested.starts_with("git://") {
                        write!(f, "git+")?;
                    }
                    write!(f, "{}", requested)?;
                } else {
                    write!(f, "{}:{}/{}", host, owner, repo)?;
                }

                if let Some(comm) = committish {
                    write!(f, "#{}", comm)?;
                } else if let Some(semver) = semver {
                    write!(f, "#semver:{}", semver)?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_url() {
        let info = GitInfo::Url {
            url: "https://foo.com/hello.git".parse().unwrap(),
            committish: Some("deadbeef".into()),
            semver: None,
        };
        assert_eq!(
            String::from("git+https://foo.com/hello.git#deadbeef"),
            format!("{}", info)
        );
        let info = GitInfo::Url {
            url: "git://foo.org/goodbye.git".parse().unwrap(),
            committish: None,
            semver: Some("^1.2.3".parse().unwrap()),
        };
        assert_eq!(
            String::from("git://foo.org/goodbye.git#semver:>=1.2.3 <2.0.0-0"),
            format!("{}", info)
        );
    }

    #[test]
    fn display_ssh() {
        let info = GitInfo::Ssh {
            ssh: "git@foo.com:here.git".into(),
            committish: Some("deadbeef".into()),
            semver: None,
        };
        assert_eq!(
            String::from("git+ssh://git@foo.com:here.git#deadbeef"),
            format!("{}", info)
        );
        let info = GitInfo::Ssh {
            ssh: "git@foo.com:here.git".into(),
            committish: None,
            semver: Some("^1.2.3".parse().unwrap()),
        };
        assert_eq!(
            String::from("git+ssh://git@foo.com:here.git#semver:>=1.2.3 <2.0.0-0"),
            format!("{}", info)
        );
    }

    #[test]
    fn display_hosted() {
        let info = GitInfo::Hosted {
            owner: "foo".into(),
            repo: "bar".into(),
            host: GitHost::GitHub,
            committish: None,
            semver: None,
            requested: None,
        };
        assert_eq!(String::from("github:foo/bar"), format!("{}", info));
        let info = GitInfo::Hosted {
            owner: "foo".into(),
            repo: "bar".into(),
            host: GitHost::GitHub,
            committish: Some("deadbeef".into()),
            semver: None,
            requested: Some("https://github.com/foo/bar.git".into()),
        };
        assert_eq!(
            String::from("git+https://github.com/foo/bar.git#deadbeef"),
            format!("{}", info)
        );
        let info = GitInfo::Hosted {
            owner: "foo".into(),
            repo: "bar".into(),
            host: GitHost::GitHub,
            committish: Some("deadbeef".into()),
            semver: None,
            requested: Some("git://gitlab.com/foo/bar.git".into()),
        };
        assert_eq!(
            String::from("git://gitlab.com/foo/bar.git#deadbeef"),
            format!("{}", info)
        );
    }
}
