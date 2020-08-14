use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{digit1, space0};
use nom::combinator::{all_consuming, map, map_res, opt, recognize};
use nom::error::{context, convert_error, ParseError, VerboseError};
use nom::sequence::tuple;
use nom::{Err, IResult};

use crate::{SemverError, Version};

#[derive(Debug, Eq, PartialEq)]
enum Range {
    Open(Predicate),
    Closed { upper: Predicate, lower: Predicate },
}

impl ToString for Range {
    fn to_string(&self) -> String {
        match self {
            Range::Open(p) => p.to_string(),
            Range::Closed { lower, upper } => {
                format!("{} {}", lower.to_string(), upper.to_string())
            }
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Operation {
    Exact,
    GreaterThan,
    GreaterThanEquals,
    LessThan,
    LessThanEquals,
    Compatible, // Might turn this into a "normal" predicate
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
pub struct Predicate {
    operation: Operation,
    version: Version,
}

impl ToString for Predicate {
    fn to_string(&self) -> String {
        format!("{}{}", self.operation.to_string(), self.version.to_string(),)
    }
}

/*
 * Methods I'll likely want to have:
 *  * self.satisfies(some_version): true if some_version is within what self allows, false otherwise
 *  * self.intersect(other_version_req): returns a verion_req that would be accepted by both `self`
 *  and `other_version_req` or None if its impossible
 * ==> these methods could maybe live in `Range`?
 *
 * Unification:
 *   We currently only have a single Range, but with ` <2 || >42`  we will get multiple ranges.
 *   Unification means that we make sure that all ranges are disjoint, by checking for their
 *   intersection and then splitting them at the right place
 */

#[derive(Debug, Eq, PartialEq)]
pub struct VersionReq {
    predicates: Range,
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

fn predicates<'a, E>(input: &'a str) -> IResult<&'a str, Range, E>
where
    E: ParseError<&'a str>,
{
    context(
        "predicate alternatives",
        alt((
            hypenated_with_only_major,
            full_version_range,
            single_sided_lower_range,
        )),
    )(input)
}

fn single_sided_lower_range<'a, E>(input: &'a str) -> IResult<&'a str, Range, E>
where
    E: ParseError<&'a str>,
{
    context(
        "single greater than",
        map(
            version_with_major_minor_patch(Operation::GreaterThanEquals),
            Range::Open,
        ),
    )(input)
}

// TODO: Rename this something liked 'closed range'
fn hypenated_with_only_major<'a, E>(input: &'a str) -> IResult<&'a str, Range, E>
where
    E: ParseError<&'a str>,
{
    context(
        "hypenated with major and minor",
        map(
            tuple((
                number,
                maybe_dot_number,
                spaced_hypen,
                number,
                maybe_dot_number,
            )),
            |(lm, maybe_l_minor, _, right, maybe_r_minor)| Range::Closed {
                lower: Predicate {
                    operation: Operation::GreaterThanEquals,
                    version: (lm, maybe_l_minor.unwrap_or(0), 0).into(),
                },

                upper: {
                    if let Some(minor) = maybe_r_minor {
                        Predicate {
                            operation: Operation::LessThan,
                            version: (right, minor + 1, 0).into(),
                        }
                    } else {
                        Predicate {
                            operation: Operation::LessThan,
                            version: (right + 1, 0, 0).into(),
                        }
                    }
                },
            },
        ),
    )(input)
}

fn maybe_dot_number<'a, E>(input: &'a str) -> IResult<&'a str, Option<usize>, E>
where
    E: ParseError<&'a str>,
{
    opt(map(tuple((tag("."), number)), |(_, num)| num))(input)
}

fn full_version_range<'a, E>(input: &'a str) -> IResult<&'a str, Range, E>
where
    E: ParseError<&'a str>,
{
    context(
        "full version range",
        map(
            tuple((
                version_with_major_minor_patch(Operation::GreaterThanEquals),
                spaced_hypen,
                version_with_major_minor_patch(Operation::LessThanEquals),
            )),
            |(lower, _, upper)| Range::Closed { lower, upper },
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
                    version: (major, minor, patch).into(),
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
            map(tag("="), |_| Exact),
            map(tag("<="), |_| LessThanEquals),
            map(tag("<"), |_| LessThan),
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
        self.predicates.to_string()
    }
}

impl Predicate {
    fn satisfies(&self, version: &Version) -> bool {
        match self.operation {
            Operation::GreaterThanEquals => self.exact(version) || self.gt(version),
            Operation::GreaterThan => self.gt(version),
            Operation::Exact => self.exact(version),
            Operation::LessThan => self.lt(version),
            Operation::LessThanEquals => self.exact(version) || self.lt(version),
            _ => false,
        }
    }

    fn exact(&self, version: &Version) -> bool {
        let predicate = &self.version;
        predicate.major == version.major
            && predicate.minor == version.minor
            && predicate.patch == version.patch
    }

    fn gt(&self, version: &Version) -> bool {
        let predicate = &self.version;
        if predicate.major < version.major {
            return true;
        }
        if predicate.major > version.major {
            return false;
        }
        if predicate.minor > version.minor {
            return false;
        }
        if predicate.patch >= version.patch {
            return false;
        }
        true
    }

    fn lt(&self, version: &Version) -> bool {
        let predicate = &self.version;
        if predicate.major > version.major {
            return true;
        }
        if predicate.major < version.major {
            return false;
        }
        if predicate.minor < version.minor {
            return false;
        }
        if predicate.minor > version.minor {
            return true;
        }
        if predicate.patch <= version.patch {
            return false;
        }
        true
    }
}

impl VersionReq {
    fn satisfies(&self, version: &Version) -> bool {
        match &self.predicates {
            Range::Open(predicate) => predicate.satisfies(version),
            _ => false,
        }
    }
}

#[cfg(test)]
mod satisfies_ranges_tests {
    use super::*;

    macro_rules! refute {
        ($e:expr) => {
            assert!(!$e)
        };
        ($e:expr, $msg:expr) => {
            assert!(!$e, $msg)
        };
    }

    #[test]
    fn greater_than_equals() {
        let parsed = parse(">=1.2.3").expect("unable to parse");

        refute!(parsed.satisfies(&(0, 2, 3).into()), "major too low");
        refute!(parsed.satisfies(&(1, 1, 3).into()), "minor too low");
        refute!(parsed.satisfies(&(1, 2, 2).into()), "patch too low");
        assert!(parsed.satisfies(&(1, 2, 3).into()), "exact");
        assert!(parsed.satisfies(&(2, 2, 3).into()), "above");
    }

    #[test]
    fn greater_than() {
        let parsed = parse(">1.2.3").expect("unable to parse");

        refute!(parsed.satisfies(&(0, 2, 3).into()), "major too low");
        refute!(parsed.satisfies(&(1, 1, 3).into()), "minor too low");
        refute!(parsed.satisfies(&(1, 2, 2).into()), "patch too low");
        refute!(parsed.satisfies(&(1, 2, 3).into()), "exact");
        assert!(parsed.satisfies(&(1, 2, 4).into()), "above");
    }

    #[test]
    fn exact() {
        let parsed = parse("=1.2.3").expect("unable to parse");

        refute!(parsed.satisfies(&(1, 2, 2).into()), "patch too low");
        assert!(parsed.satisfies(&(1, 2, 3).into()), "exact");
        refute!(parsed.satisfies(&(1, 2, 4).into()), "above");
    }

    #[test]
    fn less_than() {
        let parsed = parse("<1.2.3").expect("unable to parse");

        assert!(parsed.satisfies(&(0, 2, 3).into()), "major below");
        assert!(parsed.satisfies(&(1, 1, 3).into()), "minor below");
        assert!(parsed.satisfies(&(1, 2, 2).into()), "patch below");
        refute!(parsed.satisfies(&(1, 2, 3).into()), "exact");
        refute!(parsed.satisfies(&(1, 2, 4).into()), "above");
    }

    #[test]
    fn less_than_equals() {
        let parsed = parse("<=1.2.3").expect("unable to parse");

        assert!(parsed.satisfies(&(0, 2, 3).into()), "major below");
        assert!(parsed.satisfies(&(1, 1, 3).into()), "minor below");
        assert!(parsed.satisfies(&(1, 2, 2).into()), "patch below");
        assert!(parsed.satisfies(&(1, 2, 3).into()), "exact");
        refute!(parsed.satisfies(&(1, 2, 4).into()), "above");
    }
}

/// https://github.com/npm/node-semver/blob/master/test/fixtures/range-parse.js
#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! range_parse_tests {
        ($($name:ident => $vals:expr),+ ,$(,)?) => {
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

    range_parse_tests![       //[ input         , parsed and then `to_string`ed]
        parse_a_range =>        ["1.0.0 - 2.0.0", ">=1.0.0 <=2.0.0"],
        only_major_versions =>  ["1 - 2", ">=1.0.0 <3.0.0"],
        only_major_and_minor => ["1.0 - 2.0", ">=1.0.0 <2.1.0"],
        single_sided_lower_equals_bound =>  [">=1.0.0", ">=1.0.0"],
        single_sided_lower_bound => [">1.0.0", ">1.0.0"],
        single_sided_uppwer_equals_bound => ["<=2.0.0", "<=2.0.0"],
        single_sided_uppwer_bound => ["<2.0.0", "<2.0.0"],
    ];
    /*
    ["1.0.0", "1.0.0", { loose: false }],
    [">=*", "*"],
    ["", "*"],
    ["*", "*"],
    ["*", "*"],
    ["1", ">=1.0.0 <2.0.0-0"],
    [">= 1.0.0", ">=1.0.0"],
    [">=  1.0.0", ">=1.0.0"],
    [">=   1.0.0", ">=1.0.0"],
    ["> 1.0.0", ">1.0.0"],
    [">  1.0.0", ">1.0.0"],
    ["<=   2.0.0", "<=2.0.0"],
    ["<= 2.0.0", "<=2.0.0"],
    ["<=  2.0.0", "<=2.0.0"],
    ["<    2.0.0", "<2.0.0"],
    ["<\t2.0.0", "<2.0.0"],
    [">=0.1.97", ">=0.1.97"],
    [">=0.1.97", ">=0.1.97"],
    ["0.1.20 || 1.2.4", "0.1.20||1.2.4"],
    [">=0.2.3 || <0.0.1", ">=0.2.3||<0.0.1"],
    [">=0.2.3 || <0.0.1", ">=0.2.3||<0.0.1"],
    [">=0.2.3 || <0.0.1", ">=0.2.3||<0.0.1"],
    ["||", "*"],
    ["2.x.x", ">=2.0.0 <3.0.0-0"],
    ["1.2.x", ">=1.2.0 <1.3.0-0"],
    ["1.2.x || 2.x", ">=1.2.0 <1.3.0-0||>=2.0.0 <3.0.0-0"],
    ["1.2.x || 2.x", ">=1.2.0 <1.3.0-0||>=2.0.0 <3.0.0-0"],
    ["x", "*"],
    ["2.*.*", ">=2.0.0 <3.0.0-0"],
    ["1.2.*", ">=1.2.0 <1.3.0-0"],
    ["1.2.* || 2.*", ">=1.2.0 <1.3.0-0||>=2.0.0 <3.0.0-0"],
    ["*", "*"],
    ["2", ">=2.0.0 <3.0.0-0"],
    ["2.3", ">=2.3.0 <2.4.0-0"],
    ["~2.4", ">=2.4.0 <2.5.0-0"],
    ["~2.4", ">=2.4.0 <2.5.0-0"],
    ["~>3.2.1", ">=3.2.1 <3.3.0-0"],
    ["~1", ">=1.0.0 <2.0.0-0"],
    ["~>1", ">=1.0.0 <2.0.0-0"],
    ["~> 1", ">=1.0.0 <2.0.0-0"],
    ["~1.0", ">=1.0.0 <1.1.0-0"],
    ["~ 1.0", ">=1.0.0 <1.1.0-0"],
    ["^0", "<1.0.0-0"],
    ["^ 1", ">=1.0.0 <2.0.0-0"],
    ["^0.1", ">=0.1.0 <0.2.0-0"],
    ["^1.0", ">=1.0.0 <2.0.0-0"],
    ["^1.2", ">=1.2.0 <2.0.0-0"],
    ["^0.0.1", ">=0.0.1 <0.0.2-0"],
    ["^0.0.1-beta", ">=0.0.1-beta <0.0.2-0"],
    ["^0.1.2", ">=0.1.2 <0.2.0-0"],
    ["^1.2.3", ">=1.2.3 <2.0.0-0"],
    ["^1.2.3-beta.4", ">=1.2.3-beta.4 <2.0.0-0"],
    ["<1", "<1.0.0-0"],
    ["< 1", "<1.0.0-0"],
    [">=1", ">=1.0.0"],
    [">= 1", ">=1.0.0"],
    ["<1.2", "<1.2.0-0"],
    ["< 1.2", "<1.2.0-0"],
    ["1", ">=1.0.0 <2.0.0-0"],
    [">01.02.03", ">1.2.3", true],
    [">01.02.03", null],
    ["~1.2.3beta", ">=1.2.3-beta <1.3.0-0", { loose: true }],
    ["~1.2.3beta", null],
    ["^ 1.2 ^ 1", ">=1.2.0 <2.0.0-0 >=1.0.0"],
    ["1.2 - 3.4.5", ">=1.2.0 <=3.4.5"],
    ["1.2.3 - 3.4", ">=1.2.3 <3.5.0-0"],
    ["1.2 - 3.4", ">=1.2.0 <3.5.0-0"],
    [">1", ">=2.0.0"],
    [">1.2", ">=1.3.0"],
    [">X", "<0.0.0-0"],
    ["<X", "<0.0.0-0"],
    ["<x <* || >* 2.x", "<0.0.0-0"],
    [">x 2.x || * || <x", "*"],
    */
}
