use oro_node_semver::{Version as SemVerVersion, VersionReq as SemVerVersionReq};

use nom::branch::alt;
use nom::bytes::complete::{tag_no_case as tag, take_till1};
use nom::character::complete::char;
use nom::combinator::{cut, map, map_res, opt};
use nom::error::context;
use nom::sequence::{delimited, preceded, tuple};
use nom::IResult;

use crate::error::SpecParseError;
use crate::parsers::util;
use crate::{PackageSpec, VersionSpec};

/// npm-spec := [ '@' not('/')+ '/' ] not('@/')+ [ '@' version-req ]
pub(crate) fn npm_spec<'a>(
    input: &'a str,
) -> IResult<&'a str, PackageSpec, SpecParseError<&'a str>> {
    context(
        "npm package spec",
        map(
            tuple((
                opt(delimited(
                    char('@'),
                    map_res(take_till1(|c| c == '/'), util::no_url_encode),
                    char('/'),
                )),
                map_res(take_till1(|x| x == '@' || x == '/'), util::no_url_encode),
                opt(preceded(tag("@"), cut(version_req))),
            )),
            |(scope_opt, name, req)| {
                let name = if let Some(scope) = scope_opt {
                    format!("@{}/{}", scope, name)
                } else {
                    name.into()
                };
                PackageSpec::Npm {
                    scope: scope_opt.map(|x| x.into()),
                    name,
                    requested: req,
                }
            },
        ),
    )(input)
}

fn version_req<'a>(input: &'a str) -> IResult<&'a str, VersionSpec, SpecParseError<&'a str>> {
    context(
        "version requirement",
        alt((semver_version, semver_range, version_tag)),
    )(input)
}

fn semver_version<'a>(input: &'a str) -> IResult<&'a str, VersionSpec, SpecParseError<&'a str>> {
    let (input, version) = map_res(take_till1(|_| false), SemVerVersion::parse)(input)?;
    Ok((input, VersionSpec::Version(version)))
}

fn semver_range<'a>(input: &'a str) -> IResult<&'a str, VersionSpec, SpecParseError<&'a str>> {
    let (input, range) = map_res(take_till1(|_| false), SemVerVersionReq::parse)(input)?;
    Ok((input, VersionSpec::Range(range)))
}

fn version_tag<'a>(input: &'a str) -> IResult<&'a str, VersionSpec, SpecParseError<&'a str>> {
    context(
        "dist tag",
        map(map_res(take_till1(|_| false), util::no_url_encode), |t| {
            VersionSpec::Tag(t.into())
        }),
    )(input)
}
