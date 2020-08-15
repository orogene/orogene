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
}

impl std::string::ToString for Operation {
    fn to_string(&self) -> String {
        use Operation::*;
        match self {
            Exact => "".into(),
            GreaterThan => ">".into(),
            GreaterThanEquals => ">=".into(),
            LessThan => "<".into(),
            LessThanEquals => "<=".into(),
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
            minor_x_patch_x,
            hypenated_with_only_major,
            full_version_range,
            exact_version,
            only_major_and_minor,
            caret,
            tilde,
            open_range_with_full_version,
        )),
    )(input)
}

fn x_or_asterisk<'a, E>(input: &'a str) -> IResult<&'a str, (), E>
where
    E: ParseError<&'a str>,
{
    map(alt((tag("x"), tag("*"))), |_| ())(input)
}

fn minor_x_patch_x<'a, E>(input: &'a str) -> IResult<&'a str, Range, E>
where
    E: ParseError<&'a str>,
{
    context(
        "minor X patch X",
        map(
            tuple((
                number,
                tag("."),
                alt((map(x_or_asterisk, |_| None), map(number, |n| Some(n)))),
                tag("."),
                x_or_asterisk,
            )),
            |(major, _, maybe_minor, _, _)| Range::Closed {
                upper: upper_bound(major, maybe_minor),
                lower: lower_bound(major, maybe_minor),
            },
        ),
    )(input)
}

fn lower_bound(major: usize, maybe_minor: Option<usize>) -> Predicate {
    Predicate {
        operation: Operation::GreaterThanEquals,
        version: (major, maybe_minor.unwrap_or(0), 0).into(),
    }
}

fn upper_bound(major: usize, maybe_minor: Option<usize>) -> Predicate {
    if let Some(minor) = maybe_minor {
        Predicate {
            operation: Operation::LessThan,
            version: (major, minor + 1, 0).into(),
        }
    } else {
        Predicate {
            operation: Operation::LessThan,
            version: (major + 1, 0, 0).into(),
        }
    }
}

// open-sided range with a full version: n.n.n -> (v)
fn open_range_with_full_version<'a, E>(input: &'a str) -> IResult<&'a str, Range, E>
where
    E: ParseError<&'a str>,
{
    context(
        "single greater than",
        map(
            version_predicate_with(Operation::GreaterThanEquals),
            Range::Open,
        ),
    )(input)
}

fn caret<'a, E>(input: &'a str) -> IResult<&'a str, Range, E>
where
    E: ParseError<&'a str>,
{
    context(
        "caret",
        alt((
            map(
                tuple((tag("^"), number, tag("."), number, tag("."), number)),
                |(_, major, _, minor, _, patch)| Range::Closed {
                    lower: Predicate {
                        operation: Operation::GreaterThanEquals,
                        version: (major, minor, patch).into(),
                    },
                    upper: Predicate {
                        operation: Operation::LessThan,
                        version: (major, minor, patch + 1).into(),
                    },
                },
            ),
            map(tuple((tag("^0."), number)), |(_, minor)| Range::Closed {
                lower: Predicate {
                    operation: Operation::GreaterThanEquals,
                    version: (0, minor, 0).into(),
                },
                upper: Predicate {
                    operation: Operation::LessThan,
                    version: (0, minor + 1, 0).into(),
                },
            }),
            map(
                tuple((tag("^"), number, tag("."), number)),
                |(_, major, _, minor)| Range::Closed {
                    lower: Predicate {
                        operation: Operation::GreaterThanEquals,
                        version: (major, minor, 0).into(),
                    },
                    upper: Predicate {
                        operation: Operation::LessThan,
                        version: (major + 1, 0, 0).into(),
                    },
                },
            ),
            map(tag("^0"), |_| {
                Range::Open(Predicate {
                    operation: Operation::LessThan,
                    version: (1, 0, 0).into(),
                })
            }),
        )),
    )(input)
}

fn tilde<'a, E>(input: &'a str) -> IResult<&'a str, Range, E>
where
    E: ParseError<&'a str>,
{
    context(
        "tilde",
        alt((
            map(
                tuple((tag("~>"), number, tag("."), number, tag("."), number)),
                |(_, major, _, minor, _, patch)| Range::Closed {
                    lower: Predicate {
                        operation: Operation::GreaterThanEquals,
                        version: (major, minor, patch).into(),
                    },
                    upper: Predicate {
                        operation: Operation::LessThan,
                        version: (major, minor + 1, 0).into(),
                    },
                },
            ),
            map(
                tuple((tag("~"), number, tag("."), number)),
                |(_, major, _, minor)| Range::Closed {
                    lower: Predicate {
                        operation: Operation::GreaterThanEquals,
                        version: (major, minor, 0).into(),
                    },
                    upper: Predicate {
                        operation: Operation::LessThan,
                        version: (major, minor + 1, 0).into(),
                    },
                },
            ),
            map(tuple((tag("~"), number)), |(_, major)| Range::Closed {
                lower: Predicate {
                    operation: Operation::GreaterThanEquals,
                    version: (major, 0, 0).into(),
                },
                upper: Predicate {
                    operation: Operation::LessThan,
                    version: (major + 1, 0, 0).into(),
                },
            }),
        )),
    )(input)
}

/// takes two parses, and reads the input separated by a hypen
fn hypenated<'a, F, T, E>(left: F, right: F) -> impl Fn(&'a str) -> IResult<&'a str, (T, T), E>
where
    F: Fn(&'a str) -> IResult<&'a str, T, E>,
    E: ParseError<&'a str>,
{
    move |input: &'a str| {
        context(
            "hypenated",
            map(tuple((&left, spaced_hypen, &right)), |(l, _, r)| (l, r)),
        )(input)
    }
}

// hypenated range: n(.n) - n(.n) -> (v, v)
fn hypenated_with_only_major<'a, E>(input: &'a str) -> IResult<&'a str, Range, E>
where
    E: ParseError<&'a str>,
{
    context(
        "hypenated with major and minor",
        map(
            hypenated(
                tuple((number, maybe_dot_number)),
                tuple((number, maybe_dot_number)),
            ),
            |((left, maybe_l_minor), (right, maybe_r_minor))| Range::Closed {
                lower: lower_bound(left, maybe_l_minor),
                upper: upper_bound(right, maybe_r_minor),
            },
        ),
    )(input)
}

// only a major and maybe minor number to closed range: n(.n) => (v, v)
fn only_major_and_minor<'a, E>(input: &'a str) -> IResult<&'a str, Range, E>
where
    E: ParseError<&'a str>,
{
    context(
        "major and minor",
        map(tuple((number, maybe_dot_number)), |(major, maybe_minor)| {
            Range::Closed {
                lower: lower_bound(major, maybe_minor),
                upper: upper_bound(major, maybe_minor),
            }
        }),
    )(input)
}

fn maybe_dot_number<'a, E>(input: &'a str) -> IResult<&'a str, Option<usize>, E>
where
    E: ParseError<&'a str>,
{
    opt(map(tuple((tag("."), number)), |(_, num)| num))(input)
}

// hypenated range of two full versions: n.n.n - n.n.n -> (v, v)
fn full_version_range<'a, E>(input: &'a str) -> IResult<&'a str, Range, E>
where
    E: ParseError<&'a str>,
{
    context(
        "full version range",
        map(
            hypenated(
                version_predicate_with(Operation::GreaterThanEquals),
                version_predicate_with(Operation::LessThanEquals),
            ),
            |(lower, upper)| Range::Closed { lower, upper },
        ),
    )(input)
}

// hypenated range of two full versions: n.n.n - n.n.n -> (v, v)
fn exact_version<'a, E>(input: &'a str) -> IResult<&'a str, Range, E>
where
    E: ParseError<&'a str>,
{
    context(
        "exact version",
        map(full_version, |version| {
            Range::Open(Predicate {
                operation: Operation::Exact,
                version,
            })
        }),
    )(input)
}

fn spaced_hypen<'a, E>(input: &'a str) -> IResult<&'a str, (), E>
where
    E: ParseError<&'a str>,
{
    map(tuple((space0, tag("-"), space0)), |_| ())(input)
}

fn version_predicate_with<'a, E>(
    default_op: Operation,
) -> impl Fn(&'a str) -> IResult<&'a str, Predicate, E>
where
    E: ParseError<&'a str>,
{
    return move |input| {
        context(
            "full version",
            map(
                tuple((opt(operation), full_version)),
                |(maybe_op, version)| Predicate {
                    operation: maybe_op.unwrap_or_else(|| default_op),
                    version,
                },
            ),
        )(input)
    };
}

// n.n.n -> v
// outright duplicated
fn full_version<'a, E>(input: &'a str) -> IResult<&'a str, Version, E>
where
    E: ParseError<&'a str>,
{
    map(
        tuple((number, tag("."), number, tag("."), number)),
        |(major, _, minor, _, patch)| (major, minor, patch).into(),
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
            Operation::LessThan => !self.gt(version) && !self.exact(version),
            Operation::LessThanEquals => !self.gt(version),
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
}

impl VersionReq {
    fn satisfies(&self, version: &Version) -> bool {
        match &self.predicates {
            Range::Open(predicate) => predicate.satisfies(version),
            Range::Closed { upper, lower } => upper.satisfies(version) && lower.satisfies(version),
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

    #[test]
    fn only_major() {
        let parsed = parse("1").expect("unable to parse");

        refute!(parsed.satisfies(&(0, 2, 3).into()), "major below");
        assert!(parsed.satisfies(&(1, 0, 0).into()), "exact bottom of range");
        assert!(parsed.satisfies(&(1, 2, 2).into()), "middle");
        refute!(parsed.satisfies(&(2, 0, 0).into()), "exact top of range");
        refute!(parsed.satisfies(&(2, 7, 3).into()), "above");
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
        exact => ["1.0.0", "1.0.0"],
        major_minor_patch_range => ["1.0.0 - 2.0.0", ">=1.0.0 <=2.0.0"],
        only_major_versions =>  ["1 - 2", ">=1.0.0 <3.0.0"],
        only_major_and_minor => ["1.0 - 2.0", ">=1.0.0 <2.1.0"],
        single_sided_lower_equals_bound =>  [">=1.0.0", ">=1.0.0"],
        single_sided_lower_equals_bound_2 => [">=0.1.97", ">=0.1.97"],
        single_sided_lower_bound => [">1.0.0", ">1.0.0"],
        single_sided_upper_equals_bound => ["<=2.0.0", "<=2.0.0"],
        single_sided_upper_bound => ["<2.0.0", "<2.0.0"],
        single_major => ["1", ">=1.0.0 <2.0.0"],
        single_major_2 => ["2", ">=2.0.0 <3.0.0"],
        major_and_minor => ["2.3", ">=2.3.0 <2.4.0"],
        minor_x_patch_x => ["2.x.x", ">=2.0.0 <3.0.0"],
        patch_x => ["1.2.x", ">=1.2.0 <1.3.0"],
        minor_asterisk_patch_asterisk => ["2.*.*", ">=2.0.0 <3.0.0"],
        patch_asterisk => ["1.2.*", ">=1.2.0 <1.3.0"],
        caret_zero => ["^0", "<1.0.0"],
        caret_zero_minor => ["^0.1", ">=0.1.0 <0.2.0"],
        caret_one => ["^1.0", ">=1.0.0 <2.0.0"],
        caret_minor => ["^1.2", ">=1.2.0 <2.0.0"],
        caret_patch => ["^0.0.1", ">=0.0.1 <0.0.2"],
        tilde_one => ["~1", ">=1.0.0 <2.0.0"],
        tilde_minor => ["~1.0", ">=1.0.0 <1.1.0"],
        tilde_minor_2 => ["~2.4", ">=2.4.0 <2.5.0"],
        tidle_patch => ["~>3.2.1", ">=3.2.1 <3.3.0"],
    ];
    /*
    ["1.0.0", "1.0.0", { loose: false }],
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
    ["^ 1", ">=1.0.0 <2.0.0-0"],
    ["~> 1", ">=1.0.0 <2.0.0-0"],
    ["~ 1.0", ">=1.0.0 <1.1.0-0"],

    // Nice for pairing/
    ["0.1.20 || 1.2.4", "0.1.20||1.2.4"],
    [">=0.2.3 || <0.0.1", ">=0.2.3||<0.0.1"],
    ["1.2.x || 2.x", ">=1.2.0 <1.3.0-0||>=2.0.0 <3.0.0-0"],
    ["1.2.* || 2.*", ">=1.2.0 <1.3.0-0||>=2.0.0 <3.0.0-0"],

    // From here onwards we might have to deal with pre-release tags to?
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
    */
}
