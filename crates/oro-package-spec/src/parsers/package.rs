use nom::branch::alt;
use nom::bytes::complete::tag_no_case as tag;
use nom::combinator::opt;
use nom::error::context;
use nom::sequence::preceded;
use nom::{IResult, Parser};

use crate::PackageSpec;
use crate::error::SpecParseError;
use crate::parsers::{alias, git, npm, path};

/// package-spec := alias | ( [ "npm:" ] npm-pkg ) | ( [ "file:" ] path ) | git-pkg
pub(crate) fn package_spec(input: &str) -> IResult<&str, PackageSpec, SpecParseError<&str>> {
    context(
        "package arg",
        alt((
            alias::alias_spec,
            preceded(opt(tag("file:")), path::path_spec),
            git::git_spec,
            preceded(opt(tag("npm:")), npm::npm_spec),
        )),
    )
    .parse(input)
}
