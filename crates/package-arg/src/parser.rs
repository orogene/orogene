use std::path::PathBuf;

use oro_error_code::OroErrCode as ErrCode;
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};

use nom::branch::alt;
use nom::bytes::complete::{tag_no_case as tag, take_till1};
use nom::character::complete::{anychar, char, one_of};
use nom::combinator::{all_consuming, cut, map, map_res, opt, recognize, rest};
use nom::error::{context, convert_error, ParseError, VerboseError};
use nom::multi::many1;
use nom::sequence::{delimited, preceded, tuple};
use nom::{Err, IResult};

use crate::types::{PackageArg, PackageArgError, VersionReq};

const JS_ENCODED: &'static AsciiSet = {
    &NON_ALPHANUMERIC
        .remove(b'-')
        .remove(b'_')
        .remove(b'.')
        .remove(b'!')
        .remove(b'~')
        .remove(b'*')
        .remove(b'\'')
        .remove(b'(')
        .remove(b')')
};

pub fn parse_package_arg<I: AsRef<str>>(input: I) -> Result<PackageArg, PackageArgError> {
    let input = &input.as_ref()[..];
    match all_consuming(package_arg::<VerboseError<&str>>)(input) {
        Ok((_, arg)) => Ok(arg),
        Err(err) => Err(PackageArgError::ParseError(ErrCode::OR1000 {
            input: input.into(),
            msg: format!(
                "{}",
                match err {
                    Err::Error(e) => convert_error(input, e),
                    Err::Failure(e) => convert_error(input, e),
                    Err::Incomplete(_) => "More data was needed".into(),
                }
            ),
        }))?,
    }
}

/// package-arg := alias | ( [ "npm:" ] npm-pkg ) | ( [ "ent:" ] ent-pkg ) | ( [ "file:" ] path )
fn package_arg<'a, E>(input: &'a str) -> IResult<&'a str, PackageArg, E>
where
    E: ParseError<&'a str>,
{
    context(
        "package arg",
        alt((
            alias,
            preceded(opt(tag("file:")), path),
            preceded(opt(tag("npm:")), npm_pkg),
        )),
    )(input)
}

/// prefixed_package-arg := ( "npm:" npm-pkg ) | ( "ent:" ent-pkg ) | ( [ "file:" ] path )
fn prefixed_package_arg<'a, E>(input: &'a str) -> IResult<&'a str, PackageArg, E>
where
    E: ParseError<&'a str>,
{
    context(
        "package spec",
        alt((
            // Paths don't need to be prefixed, but they can be.
            preceded(opt(tag("file:")), path),
            preceded(tag("npm:"), npm_pkg),
        )),
    )(input)
}

// alias := [ [ '@' ], not('/')+ '/' ] not('@/')+ '@' prefixed-package-arg
fn alias<'a, E>(input: &'a str) -> IResult<&'a str, PackageArg, E>
where
    E: ParseError<&'a str>,
{
    context(
        "alias",
        map(
            tuple((
                opt(scope),
                map_res(take_till1(|c| c == '@' || c == '/'), no_url_encode),
                tag("@"),
                prefixed_package_arg,
            )),
            |(scope, name, _, arg)| {
                let mut fullname = String::new();
                if let Some(scope) = scope {
                    fullname.push_str(&scope);
                    fullname.push_str("/");
                }
                fullname.push_str(name);
                PackageArg::Alias {
                    name: fullname,
                    package: Box::new(arg),
                }
            },
        ),
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
                map_res(take_till1(|c| c == '/'), no_url_encode),
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

/// npm-pkg := [ '@' not('/')+ '/' ] not('@/')+ [ '@' version-req ]
fn npm_pkg<'a, E>(input: &'a str) -> IResult<&'a str, PackageArg, E>
where
    E: ParseError<&'a str>,
{
    context(
        "npm package",
        map(
            tuple((
                opt(delimited(
                    char('@'),
                    map_res(take_till1(|c| c == '/'), no_url_encode),
                    char('/'),
                )),
                map_res(take_till1(|x| x == '@' || x == '/'), no_url_encode),
                opt(preceded(tag("@"), cut(version_req))),
            )),
            |(scope_opt, name, req)| PackageArg::Npm {
                scope: scope_opt.map(|x| x.into()),
                name: name.into(),
                requested: req,
            },
        ),
    )(input)
}

fn version_req<'a, E>(input: &'a str) -> IResult<&'a str, VersionReq, E>
where
    E: ParseError<&'a str>,
{
    context(
        "version requirement",
        alt((semver_version, semver_range, version_tag)),
    )(input)
}

fn semver_version<'a, E>(input: &'a str) -> IResult<&'a str, VersionReq, E>
where
    E: ParseError<&'a str>,
{
    let (input, version) = map_res(take_till1(|_| false), semver::Version::parse)(input)?;
    Ok((input, VersionReq::Version(version)))
}

fn semver_range<'a, E>(input: &'a str) -> IResult<&'a str, VersionReq, E>
where
    E: ParseError<&'a str>,
{
    let (input, range) = map_res(take_till1(|_| false), semver::VersionReq::parse)(input)?;
    Ok((input, VersionReq::Range(range)))
}

fn version_tag<'a, E>(input: &'a str) -> IResult<&'a str, VersionReq, E>
where
    E: ParseError<&'a str>,
{
    context(
        "dist tag",
        map(map_res(take_till1(|_| false), no_url_encode), |t| {
            VersionReq::Tag(t.into())
        }),
    )(input)
}

fn no_url_encode(tag: &str) -> Result<&str, PackageArgError> {
    if format!("{}", utf8_percent_encode(tag, JS_ENCODED)) == tag {
        Ok(tag)
    } else {
        Err(PackageArgError::InvalidCharacters(tag.into()))
    }
}

/// path := ( relative-dir | absolute-dir )
fn path<'a, E>(input: &'a str) -> IResult<&'a str, PackageArg, E>
where
    E: ParseError<&'a str>,
{
    map(alt((relative_path, absolute_path)), |p| PackageArg::Dir {
        path: p,
    })(input)
}

/// relative-path := [ '.' ] '.' path-sep .*
fn relative_path<'a, E>(input: &'a str) -> IResult<&'a str, PathBuf, E>
where
    E: ParseError<&'a str>,
{
    map(
        recognize(tuple((tag("."), opt(tag(".")), many1(path_sep), rest))),
        |p| PathBuf::from(p),
    )(input)
}

/// absolute-path := [ alpha ':' ] path-sep+ [ '?' path-sep+ ] .*
fn absolute_path<'a, E>(input: &'a str) -> IResult<&'a str, PathBuf, E>
where
    E: ParseError<&'a str>,
{
    map(
        recognize(preceded(
            delimited(
                opt(preceded(
                    map_res(anychar, |c| {
                        if c.is_alphabetic() {
                            Ok(c)
                        } else {
                            Err(PackageArgError::InvalidDriveLetter(c))
                        }
                    }),
                    tag(":"),
                )),
                many1(path_sep),
                opt(preceded(tag("?"), many1(path_sep))),
            ),
            rest,
        )),
        |p| PathBuf::from(p),
    )(input)
}

/// path-sep := ( '/' | '\' )
fn path_sep<'a, E>(input: &'a str) -> IResult<&'a str, char, E>
where
    E: ParseError<&'a str>,
{
    one_of("/\\")(input)
}
