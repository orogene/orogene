use nom::branch::alt;
use nom::bytes::complete::{tag_no_case as tag, take_till1};
use nom::combinator::{map, map_res, opt};
use nom::error::{context, ParseError};
use nom::sequence::{preceded, tuple};
use nom::IResult;

use crate::parsers::{npm, path, util};
use crate::PackageSpec;

// alias_spec := [ [ '@' ], not('/')+ '/' ] not('@/')+ '@' prefixed-package-arg
pub fn alias_spec<'a, E>(input: &'a str) -> IResult<&'a str, PackageSpec, E>
where
    E: ParseError<&'a str>,
{
    context(
        "alias",
        map(
            tuple((
                opt(scope),
                map_res(take_till1(|c| c == '@' || c == '/'), util::no_url_encode),
                tag("@"),
                prefixed_package_spec,
            )),
            |(scope, name, _, arg)| {
                let mut fullname = String::new();
                if let Some(scope) = scope {
                    fullname.push_str(&scope);
                    fullname.push_str("/");
                }
                fullname.push_str(name);
                PackageSpec::Alias {
                    name: fullname,
                    package: Box::new(arg),
                }
            },
        ),
    )(input)
}

/// prefixed_package-arg := ( "npm:" npm-pkg ) | ( [ "file:" ] path )
fn prefixed_package_spec<'a, E>(input: &'a str) -> IResult<&'a str, PackageSpec, E>
where
    E: ParseError<&'a str>,
{
    context(
        "package spec",
        alt((
            // Paths don't need to be prefixed, but they can be.
            preceded(opt(tag("file:")), path::path_spec),
            preceded(tag("npm:"), npm::npm_spec),
        )),
    )(input)
}

fn scope<'a, E>(input: &'a str) -> IResult<&'a str, String, E>
where
    E: ParseError<&'a str>,
{
    context(
        "scope",
        map(
            tuple((
                opt(tag("@")),
                map_res(take_till1(|c| c == '/'), util::no_url_encode),
                tag("/"),
            )),
            |(at, scope, _)| {
                let mut out = String::new();
                if let Some(at) = at {
                    out.push_str(at);
                }
                out.push_str(scope);
                out
            },
        ),
    )(input)
}
