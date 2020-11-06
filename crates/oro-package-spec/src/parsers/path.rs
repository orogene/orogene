use std::path::PathBuf;

use nom::branch::alt;
use nom::bytes::complete::tag_no_case as tag;
use nom::character::complete::{anychar, one_of};
use nom::combinator::{map, map_res, opt, recognize, rest};
use nom::error::ParseError;
use nom::multi::{many0, many1};
use nom::sequence::{delimited, preceded, tuple};
use nom::IResult;
use oro_diagnostics::DiagnosticCode;

use crate::error::PackageSpecError;
use crate::PackageSpec;

/// path := ( relative-dir | absolute-dir )
pub fn path_spec<'a, E>(input: &'a str) -> IResult<&'a str, PackageSpec, E>
where
    E: ParseError<&'a str>,
{
    map(alt((relative_path, absolute_path)), |p| PackageSpec::Dir {
        path: p,
    })(input)
}

/// relative-path := [ '.' ] '.' [path-sep] .*
fn relative_path<'a, E>(input: &'a str) -> IResult<&'a str, PathBuf, E>
where
    E: ParseError<&'a str>,
{
    map(
        recognize(tuple((tag("."), opt(tag(".")), many0(path_sep), rest))),
        PathBuf::from,
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
                            Err(PackageSpecError::InvalidDriveLetter(
                                DiagnosticCode::OR1002,
                                c,
                            ))
                        }
                    }),
                    tag(":"),
                )),
                many1(path_sep),
                opt(preceded(tag("?"), many1(path_sep))),
            ),
            rest,
        )),
        PathBuf::from,
    )(input)
}

/// path-sep := ( '/' | '\' )
fn path_sep<'a, E>(input: &'a str) -> IResult<&'a str, char, E>
where
    E: ParseError<&'a str>,
{
    one_of("/\\")(input)
}
