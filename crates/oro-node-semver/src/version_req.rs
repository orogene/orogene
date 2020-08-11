use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{digit1, space0};
use nom::combinator::{all_consuming, map, map_res, opt, recognize};
use nom::error::{context, convert_error, ParseError, VerboseError};
use nom::sequence::tuple;
use nom::{Err, IResult};

use crate::{Identifier, SemverError};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Operation {
    Exact,
    GreaterThan,
    GreaterThanEquals,
    LessThan,
    LessThanEquals,
    Compatible,
    WildCard(WildCardVersion),
}

impl std::string::ToString for Operation {
    fn to_string(&self) -> String {
        use Operation::*;
        match self {
            GreaterThan => ">".into(),
            GreaterThanEquals => ">=".into(),
            LessThan => "<".into(),
            LessThanEquals => "<=".into(),
            _ => "***".into(),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum WildCardVersion {
    Minor,
    Patch,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct Predicate {
    operation: Operation,
    major: usize,
    minor: Option<usize>,
    patch: Option<usize>,
    pre_release: Vec<Identifier>,
}

impl ToString for Predicate {
    fn to_string(&self) -> String {
        format!(
            "{}{}.{}.{}",
            self.operation.to_string(),
            self.major,
            self.minor.unwrap_or_else(|| 0),
            self.patch.unwrap_or_else(|| 0),
        )
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct VersionReq {
    predicates: Vec<Predicate>,
}

pub fn parse<S: AsRef<str>>(input: S) -> Result<VersionReq, SemverError> {
    let input = &input.as_ref()[..];

    match all_consuming(predicates::<VerboseError<&str>>)(input) {
        Ok((_, predicates)) => Ok(VersionReq { predicates }),
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

fn predicates<'a, E>(input: &'a str) -> IResult<&'a str, Vec<Predicate>, E>
where
    E: ParseError<&'a str>,
{
    context(
        "predicate alternatives",
        alt((hypenated_with_only_major, full_version_range)),
    )(input)
}

fn hypenated_with_only_major<'a, E>(input: &'a str) -> IResult<&'a str, Vec<Predicate>, E>
where
    E: ParseError<&'a str>,
{
    context(
        "full_version_range",
        map(tuple((number, spaced_hypen, number)), |(left, _, right)| {
            vec![
                Predicate {
                    operation: Operation::GreaterThanEquals,
                    major: left,
                    minor: None,
                    patch: None,
                    pre_release: Vec::new(),
                },
                Predicate {
                    operation: Operation::LessThan,
                    major: right + 1,
                    minor: None,
                    patch: None,
                    pre_release: Vec::new(),
                },
            ]
        }),
    )(input)
}

fn full_version_range<'a, E>(input: &'a str) -> IResult<&'a str, Vec<Predicate>, E>
where
    E: ParseError<&'a str>,
{
    context(
        "full_version_range",
        map(
            tuple((
                version_with_major_minor_patch(Operation::GreaterThanEquals),
                spaced_hypen,
                version_with_major_minor_patch(Operation::LessThanEquals),
            )),
            |(a, _, b)| vec![a, b],
        ),
    )(input)
}

fn spaced_hypen<'a, E>(input: &'a str) -> IResult<&'a str, (), E>
where
    E: ParseError<&'a str>,
{
    map(tuple((space0, tag("-"), space0)), |_| ())(input)
}

fn version_with_major_minor_patch<'a, E>(
    default_op: Operation,
) -> impl Fn(&'a str) -> IResult<&'a str, Predicate, E>
where
    E: ParseError<&'a str>,
{
    return move |input| {
        context(
            "single predicate",
            map(
                tuple((opt(operation), number, tag("."), number, tag("."), number)),
                |(maybe_op, major, _, minor, _, patch)| Predicate {
                    operation: maybe_op.unwrap_or_else(|| default_op),
                    major,
                    minor: Some(minor),
                    patch: Some(patch),
                    pre_release: Vec::new(),
                },
            ),
        )(input)
    };
}

fn operation<'a, E>(input: &'a str) -> IResult<&'a str, Operation, E>
where
    E: ParseError<&'a str>,
{
    use Operation::*;
    context(
        "operation",
        alt((
            // TODO: Add more as needed
            map(tag(">="), |_| GreaterThanEquals),
            map(tag(">"), |_| GreaterThan),
        )),
    )(input)
}

// outright duplicated
fn number<'a, E>(input: &'a str) -> IResult<&'a str, usize, E>
where
    E: ParseError<&'a str>,
{
    map_res(recognize(digit1), str::parse)(input)
}

impl ToString for VersionReq {
    fn to_string(&self) -> String {
        format!(
            "{} {}",
            self.predicates[0].to_string(),
            self.predicates[1].to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! range_parse_tests {
        ($($name:ident => $vals:expr),+) => {
            $(
                #[test]
                fn $name() {
                    let [input, expected] = $vals;

                    let parsed = parse(input).expect("unable to parse");

                    assert_eq!(expected, parsed.to_string());
                }
            )+
        }

    }

    range_parse_tests![
        parse_a_range =>        ["1.0.0 - 2.0.0", ">=1.0.0 <=2.0.0"],
        only_major_versions =>  ["1 - 2",         ">=1.0.0 <3.0.0"]
    ];
}
