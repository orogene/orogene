use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{alphanumeric1, digit1};
use nom::combinator::{all_consuming, map, map_res, opt, recognize};
use nom::error::{context, convert_error, ParseError, VerboseError};
use nom::multi::separated_list;
use nom::sequence::{preceded, tuple};
use nom::{Err, IResult};

use thiserror::Error;

use std::fmt;

mod version_req;

#[derive(Debug, Error)]
pub enum SemverError {
    #[error("{input}: {msg}")]
    ParseError { input: String, msg: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Identifier {
    /// An identifier that's solely numbers.
    Numeric(u64),
    /// An identifier with letters and numbers.
    AlphaNumeric(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Version {
    major: usize,
    minor: usize,
    patch: usize,
    build: Vec<Identifier>,
    pre_release: Vec<Identifier>,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl std::convert::From<(usize, usize, usize)> for Version {
    fn from((major, minor, patch): (usize, usize, usize)) -> Self {
        Version {
            major,
            minor,
            patch,
            build: Vec::new(),
            pre_release: Vec::new(),
        }
    }
}

pub fn parse<S: AsRef<str>>(input: S) -> Result<Version, SemverError> {
    let input = &input.as_ref()[..];

    match all_consuming(version::<VerboseError<&str>>)(input) {
        Ok((_, arg)) => Ok(arg),
        Err(err) => Err(SemverError::ParseError {
            input: input.into(),
            msg: match err {
                Err::Error(e) => convert_error(input, e),
                Err::Failure(e) => convert_error(input, e),
                Err::Incomplete(_) => "More data was needed".into(),
            },
        }),
    }
}

enum Extras {
    Build(Vec<Identifier>),
    Release(Vec<Identifier>),
    ReleaseAndBuild(Vec<Identifier>, Vec<Identifier>),
}

impl Extras {
    fn values(self) -> (Vec<Identifier>, Vec<Identifier>) {
        use Extras::*;
        match self {
            Release(ident) => (ident, Vec::new()),
            Build(ident) => (Vec::new(), ident),
            ReleaseAndBuild(a, b) => (a, b),
        }
    }
}

/// <valid semver> ::= <version core>
///                 | <version core> "-" <pre-release>
///                 | <version core> "+" <build>
///                 | <version core> "-" <pre-release> "+" <build>
fn version<'a, E>(input: &'a str) -> IResult<&'a str, Version, E>
where
    E: ParseError<&'a str>,
{
    context(
        "version",
        map(
            tuple((
                version_core,
                opt(alt((
                    map(tuple((pre_release, build)), |(b, pr)| {
                        Extras::ReleaseAndBuild(b, pr)
                    }),
                    map(pre_release, Extras::Release),
                    map(build, Extras::Build),
                ))),
            )),
            |((major, minor, patch), extras)| {
                let (pre_release, build) = if let Some(e) = extras {
                    e.values()
                } else {
                    (Vec::new(), Vec::new())
                };
                Version {
                    major,
                    minor,
                    patch,
                    pre_release,
                    build,
                }
            },
        ),
    )(input)
}

/// <version core> ::= <major> "." <minor> "." <patch>
fn version_core<'a, E>(input: &'a str) -> IResult<&'a str, (usize, usize, usize), E>
where
    E: ParseError<&'a str>,
{
    context(
        "version core",
        map(
            tuple((number, tag("."), number, tag("."), number)),
            |(major, _, minor, _, patch)| (major, minor, patch),
        ),
    )(input)
}

// I believe build, pre_release, and identifier are not 100% spec compliant.
fn build<'a, E>(input: &'a str) -> IResult<&'a str, Vec<Identifier>, E>
where
    E: ParseError<&'a str>,
{
    context(
        "build version",
        preceded(tag("+"), separated_list(tag("."), identifier)),
    )(input)
}

fn pre_release<'a, E>(input: &'a str) -> IResult<&'a str, Vec<Identifier>, E>
where
    E: ParseError<&'a str>,
{
    context(
        "pre_release version",
        preceded(tag("-"), separated_list(tag("."), identifier)),
    )(input)
}

fn identifier<'a, E>(input: &'a str) -> IResult<&'a str, Identifier, E>
where
    E: ParseError<&'a str>,
{
    context(
        "identifier",
        alt((
            map(digit1, |res: &str| {
                let val: u64 = str::parse(res).unwrap();
                Identifier::Numeric(val)
            }),
            map(alphanumeric1, |res: &str| {
                Identifier::AlphaNumeric(res.to_string())
            }),
        )),
    )(input)
}

fn number<'a, E>(input: &'a str) -> IResult<&'a str, usize, E>
where
    E: ParseError<&'a str>,
{
    map_res(recognize(digit1), str::parse)(input)
}

#[cfg(test)]
mod tests {
    use super::Identifier::*;
    use super::*;

    #[test]
    fn trivial_version_number() {
        let v = parse("1.2.34").unwrap();

        assert_eq!(
            v,
            Version {
                major: 1,
                minor: 2,
                patch: 34,
                build: Vec::new(),
                pre_release: Vec::new(),
            }
        );
    }

    #[test]
    fn version_with_build() {
        let v = parse("1.2.34+123.456").unwrap();

        assert_eq!(
            v,
            Version {
                major: 1,
                minor: 2,
                patch: 34,
                build: vec![Numeric(123), Numeric(456)],
                pre_release: Vec::new(),
            }
        );
    }

    #[test]
    fn version_with_pre_release() {
        let v = parse("1.2.34-abc.123").unwrap();

        assert_eq!(
            v,
            Version {
                major: 1,
                minor: 2,
                patch: 34,
                pre_release: vec![AlphaNumeric("abc".into()), Numeric(123)],
                build: Vec::new(),
            }
        );
    }

    #[test]
    fn version_with_pre_release_and_build() {
        let v = parse("1.2.34-abc.123+1").unwrap();

        assert_eq!(
            v,
            Version {
                major: 1,
                minor: 2,
                patch: 34,
                pre_release: vec![AlphaNumeric("abc".into()), Numeric(123)],
                build: vec![Numeric(1),]
            }
        );
    }
}
