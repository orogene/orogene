use nom::bytes::complete::tag;
use nom::character::complete::digit1;
use nom::combinator::{all_consuming, map, map_res, recognize};
use nom::error::{context, convert_error, ParseError, VerboseError};
use nom::sequence::tuple;
use nom::{Err, IResult};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SemverError {
    #[error("{input}: {msg}")]
    ParseError { input: String, msg: String },
}

#[derive(Debug, Eq, PartialEq)]
pub struct Version {
    major: usize,
    minor: usize,
    patch: usize,
}

pub fn parse<S: AsRef<str>>(input: S) -> Result<Version, SemverError> {
    let input = &input.as_ref()[..];

    match all_consuming(version_core::<VerboseError<&str>>)(input) {
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

/// <version core> ::= <major> "." <minor> "." <patch>
fn version_core<'a, E>(input: &'a str) -> IResult<&'a str, Version, E>
where
    E: ParseError<&'a str>,
{
    context(
        "version core",
        map(
            tuple((number, tag("."), number, tag("."), number)),
            |(major, _, minor, _, patch)| Version {
                major,
                minor,
                patch,
            },
        ),
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
            }
        );
    }
}
