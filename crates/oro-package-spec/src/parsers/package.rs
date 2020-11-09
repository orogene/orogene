use nom::branch::alt;
use nom::bytes::complete::tag_no_case as tag;
use nom::combinator::opt;
use nom::error::{context, ParseError};
use nom::sequence::preceded;
use nom::IResult;

use crate::parsers::{alias, git, npm, path};
use crate::PackageSpec;

/// package-spec := alias | ( [ "npm:" ] npm-pkg ) | ( [ "file:" ] path ) | git-pkg
pub fn package_spec<'a, E>(input: &'a str) -> IResult<&'a str, PackageSpec, E>
where
    E: ParseError<&'a str>,
{
    context(
        "package arg",
        alt((
            alias::alias_spec,
            preceded(opt(tag("file:")), path::path_spec),
            git::git_spec,
            preceded(opt(tag("npm:")), npm::npm_spec),
        )),
    )(input)
}
