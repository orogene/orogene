use nom::branch::alt;
use nom::bytes::complete::{tag_no_case as tag, take_till1, take_while};
use nom::combinator::{cut, map, map_res, opt, peek, rest};
use nom::error::context;
use nom::sequence::{preceded, terminated};
use nom::IResult;
use oro_node_semver::VersionReq;
use url::Url;

use crate::error::SpecParseError;
use crate::parsers::util;
use crate::{GitHost, GitInfo, PackageSpec};

/// `git-spec := git-shorthand | git-scp | git-url`
pub(crate) fn git_spec<'a>(
    input: &'a str,
) -> IResult<&'a str, PackageSpec, SpecParseError<&'a str>> {
    context(
        "git package",
        map(alt((git_shorthand, git_url, git_scp)), PackageSpec::Git),
    )(input)
}

/// `git-shorthand := [ hosted-git-prefix ] not('/')+ '/' repo`
fn git_shorthand<'a>(input: &'a str) -> IResult<&'a str, GitInfo, SpecParseError<&'a str>> {
    let (input, maybe_host) = opt(hosted_git_prefix)(input)?;
    let (input, owner) = map_res(take_till1(|c| c == '/'), util::no_url_encode)(input)?;
    let (input, repo) = preceded(tag("/"), take_while(|c| c != '#'))(input)?;
    let (input, (committish, semver)) = committish(input)?;
    Ok((
        input,
        GitInfo::Hosted {
            host: maybe_host.unwrap_or(GitHost::GitHub),
            owner: owner.into(),
            repo: repo.into(),
            committish: committish.map(String::from),
            semver,
            requested: None,
        },
    ))
}

/// `hosted-git-prefix := 'github:' | 'bitbucket:' | 'gist:' | 'gitlab:'`
fn hosted_git_prefix<'a>(input: &'a str) -> IResult<&'a str, GitHost, SpecParseError<&'a str>> {
    map_res(
        terminated(
            alt((tag("github"), tag("gist"), tag("gitlab"), tag("bitbucket"))),
            tag(":"),
        ),
        |host: &str| host.parse(),
    )(input)
}

fn committish<'a>(
    input: &'a str,
) -> IResult<&'a str, (Option<String>, Option<VersionReq>), SpecParseError<&'a str>> {
    let (input, hash) = opt(preceded(
        tag("#"),
        alt((
            map(preceded(tag("semver:"), cut(semver_range)), |req| {
                (None, Some(req))
            }),
            map(map_res(rest, util::no_url_encode), |com| (Some(com), None)),
        )),
    ))(input)?;
    Ok((
        input,
        if let Some((maybe_comm, maybe_semver)) = hash {
            (maybe_comm.map(String::from), maybe_semver)
        } else {
            (None, None)
        },
    ))
}

fn semver_range<'a>(input: &'a str) -> IResult<&'a str, VersionReq, SpecParseError<&'a str>> {
    let (input, range) = map_res(take_till1(|_| false), VersionReq::parse)(input)?;
    Ok((input, range))
}

fn git_url<'a>(input: &'a str) -> IResult<&'a str, GitInfo, SpecParseError<&'a str>> {
    let (input, url) = preceded(
        alt((tag("git+"), peek(tag("git://")))),
        map_res(take_till1(|c| c == '#'), Url::parse),
    )(input)?;
    let (input, (committish, semver)) = committish(input)?;
    match url.host_str() {
        Some(host @ "github.com")
        | Some(host @ "gitlab.com")
        | Some(host @ "gist.github.com")
        | Some(host @ "bitbucket.org") => {
            let path = (&url.path()[1..])
                .split('/')
                .map(String::from)
                .collect::<Vec<String>>();
            if let [owner, repo] = &path[..] {
                Ok((
                    input,
                    GitInfo::Hosted {
                        host: match host {
                            "github.com" => GitHost::GitHub,
                            "gitlab.com" => GitHost::GitLab,
                            "gist.github.com" => GitHost::Gist,
                            "bitbucket.org" => GitHost::Bitbucket,
                            _ => unreachable!(),
                        },
                        owner: owner.clone(),
                        repo: if repo.ends_with(".git") {
                            String::from(&repo[..].replace(".git", ""))
                        } else {
                            repo.clone()
                        },
                        committish,
                        semver,
                        requested: Some(url.to_string()),
                    },
                ))
            } else {
                Ok((
                    input,
                    GitInfo::Url {
                        url,
                        committish,
                        semver,
                    },
                ))
            }
        }
        _ => Ok((
            input,
            GitInfo::Url {
                url,
                committish,
                semver,
            },
        )),
    }
}

fn git_scp<'a>(input: &'a str) -> IResult<&'a str, GitInfo, SpecParseError<&'a str>> {
    let (input, _) = preceded(opt(tag("git+")), tag("ssh://"))(input)?;
    let (input, username) = opt(terminated(take_till1(|c| c == '@'), tag("@")))(input)?;
    let (input, host) = take_till1(|c| c == ':' || c == '#')(input)?;
    let (input, path) = opt(preceded(tag(":"), take_till1(|c| c == '#')))(input)?;
    let (input, (committish, semver)) = committish(input)?;
    let mut raw = String::new();
    if let Some(username) = username {
        raw.push_str(username);
        raw.push('@');
    }
    raw.push_str(host);
    if let Some(path) = path {
        raw.push(':');
        raw.push_str(path);
    }
    match host {
        "github.com" | "gitlab.com" | "gist.github.com" | "bitbucket.org"
            if path.is_some() && username.is_some() =>
        {
            let path = path
                .unwrap()
                .split('/')
                .map(String::from)
                .collect::<Vec<String>>();
            if let [owner, repo] = &path[..] {
                let repo = if repo.ends_with(".git") {
                    String::from(&repo[..].replace(".git", ""))
                } else {
                    repo.clone()
                };
                Ok((
                    input,
                    GitInfo::Hosted {
                        host: match host {
                            "github.com" => GitHost::GitHub,
                            "gitlab.com" => GitHost::GitLab,
                            "gist.github.com" => GitHost::Gist,
                            "bitbucket.org" => GitHost::Bitbucket,
                            _ => unreachable!(),
                        },
                        owner: owner.clone(),
                        repo: if repo.ends_with(".git") {
                            String::from(&repo[..].replace(".git", ""))
                        } else {
                            repo
                        },
                        committish,
                        semver,
                        requested: Some(raw),
                    },
                ))
            } else {
                Ok((
                    input,
                    GitInfo::Ssh {
                        ssh: raw,
                        committish,
                        semver,
                    },
                ))
            }
        }
        _ => Ok((
            input,
            GitInfo::Ssh {
                ssh: raw,
                committish,
                semver,
            },
        )),
    }
}
