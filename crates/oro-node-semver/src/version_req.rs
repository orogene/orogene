use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{digit1};
use nom::combinator::{all_consuming, map, map_res, opt, recognize};
use nom::error::{context, convert_error, ParseError, VerboseError};
use nom::sequence::{tuple};
use nom::{Err, IResult};

use crate::{Identifier, SemverError};

#[derive(Debug, Copy, Clone)]
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

#[derive(Debug, Copy, Clone)]
enum WildCardVersion {
    Minor,
    Patch,
}

#[derive(Debug)]
struct Predicate {
    operation: Operation,
    major: usize,
    minor: Option<usize>,
    patch: Option<usize>,
    pre_release: Vec<Identifier>,
}

impl Predicate {
    fn normalized_str(&self) -> String {
        format!(
            "{}{}.{}.{}",
            self.operation.to_string(),
            self.major,
            self.minor.unwrap(),
            self.patch.unwrap(),
        )
    }
}

#[derive(Debug)]
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

fn predicate_with_default<'a, E>(
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

fn predicates<'a, E>(input: &'a str) -> IResult<&'a str, Vec<Predicate>, E>
where
    E: ParseError<&'a str>,
{
    context(
        "sequence of predicate",
        // TODO: the ` - ` is significant to get from...to
        map(
            tuple((
                predicate_with_default(Operation::GreaterThanEquals),
                tag(" - "),
                predicate_with_default(Operation::LessThanEquals),
            )),
            |(a, _, b)| vec![a, b],
        ),
    )(input)
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

impl VersionReq {
    fn normalized_str(&self) -> String {
        format!(
            "{} {}",
            self.predicates[0].normalized_str(),
            self.predicates[1].normalized_str()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_a_range() {
        let [input, expected_normalized] = ["1.0.0 - 2.0.0", ">=1.0.0 <=2.0.0"];

        let parsed = parse(input).expect("unable to parse");

        assert_eq!(expected_normalized, parsed.normalized_str());
    }
}
