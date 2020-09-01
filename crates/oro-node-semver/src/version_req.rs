use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::space0;
use nom::combinator::{all_consuming, map, opt};
use nom::error::{context, convert_error, ParseError, VerboseError};
use nom::multi::separated_nonempty_list;
use nom::sequence::{preceded, tuple};
use nom::{Err, IResult};

use std::fmt;

use serde::de::{self, Deserialize, Deserializer, Visitor};
use serde::ser::{Serialize, Serializer};

use crate::{extras, number, Identifier, SemverError, Version};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Range {
    upper: Predicate,
    lower: Predicate,
}

impl Range {
    fn new(lower: Predicate, upper: Predicate) -> Self {
        Self { lower, upper }
    }
    fn at_least(p: Predicate) -> Self {
        Self {
            lower: p,
            upper: Predicate::Unbounded,
        }
    }

    fn at_most(p: Predicate) -> Self {
        Self {
            lower: Predicate::Unbounded,
            upper: p,
        }
    }

    fn exact(version: Version) -> Self {
        Range::new(
            Predicate::Including(version.clone()),
            Predicate::Including(version),
        )
    }

    fn satisfies(&self, version: &Version) -> bool {
        use Predicate::*;

        match (&self.lower, &self.upper) {
            (Including(lower), Unbounded) => lower <= version,
            (Including(lower), Excluding(upper)) => lower <= version && version < upper,
            (Including(lower), Including(upper)) => lower <= version && version <= upper,
            (Excluding(lower), Unbounded) => lower < version,
            (Excluding(lower), Excluding(upper)) => lower < version && version < upper,
            (Excluding(lower), Including(upper)) => lower < version && version <= upper,
            (Unbounded, Unbounded) => true,
            (Unbounded, Excluding(upper)) => version < upper,
            (Unbounded, Including(upper)) => version <= upper,
        }
    }

    fn allows_all(&self, other: &Range) -> bool {
        use Predicate::*;

        let allows_lower = match (&self.lower, &other.lower) {
            (Unbounded, _) => true,
            (Including(left), Including(right)) | (Including(left), Excluding(right)) => {
                left <= right
            }
            (Excluding(left), Including(right)) | (Excluding(left), Excluding(right)) => {
                left < right
            }
            (_, Unbounded) => false,
        };

        if !allows_lower {
            return false;
        }

        match (&self.upper, &other.upper) {
            (Unbounded, _) => true,
            (Including(left), Including(right)) | (Including(left), Excluding(right)) => {
                right <= left
            }
            (Excluding(left), Including(right)) | (Excluding(left), Excluding(right)) => {
                right < left
            }
            (_, Unbounded) => false,
        }
    }

    fn allows_any(&self, other: &Range) -> bool {
        use Predicate::*;

        match (&self.lower, &self.upper, &other.lower, &other.upper) {
            (_, Unbounded, _, Unbounded) | (Unbounded, _, Unbounded, _) => true,
            (Including(l), Including(r), Including(l2), Including(r2))
            | (Including(l), Including(r), Including(l2), Excluding(r2))
            | (Including(l), Excluding(r), Including(l2), Including(r2))
            | (Including(l), Excluding(r), Including(l2), Excluding(r2)) => {
                (l <= l2 && l2 <= r) || (l <= r2 && r2 <= r) || (l2 <= l && r <= r2)
            }
            (Including(l), _, Unbounded, Including(r2)) => l <= r2,
            (Including(l), _, Unbounded, Excluding(r2)) => l < r2,
            (Including(l), Including(r), Including(l2), Unbounded)
            | (Including(l), Including(r), Excluding(l2), Unbounded)
            | (Including(l), Excluding(r), Including(l2), Unbounded)
            | (Including(l), Excluding(r), Excluding(l2), Unbounded) => l <= l2 && l2 <= r,
            (Including(l), Unbounded, _, Including(r2)) => l <= r2,
            (Including(l), Unbounded, _, Excluding(r2)) => l <= r2,
            (Excluding(l), Unbounded, Unbounded, Including(r2)) => l < r2,
            (Excluding(l), Unbounded, Unbounded, Excluding(r2)) => l <= r2,
            (Excluding(l), Unbounded, _, Including(r2)) => l < r2,
            (Unbounded, Excluding(r), _, Including(r2)) => r2 < r,
            (Unbounded, Excluding(r), Excluding(l2), Unbounded) => l2 < r,
            (Unbounded, Excluding(r), Including(l2), Unbounded) => l2 < r,
            (Unbounded, Including(r), Including(l2), Including(_)) => l2 < r,
            (Unbounded, Including(r), Excluding(l2), Unbounded) => l2 < r,
            (Unbounded, Including(r), Including(l2), Unbounded) => l2 < r,
            e => todo!("{:#?}", e),
        }
    }

    fn intersect(&self, other: &Self) -> Option<Self> {
        use Predicate::*;

        let lower = match (&self.lower, &other.lower) {
            (Unbounded, any) => any.clone(),
            (any, Unbounded) => any.clone(),
            (Including(v1), Including(v2)) => Including(std::cmp::max(v1, v2).clone()),
            (Including(v1), Excluding(v2)) => {
                if v2 < v1 {
                    Including(v1.clone())
                } else {
                    Excluding(v2.clone())
                }
            }
            (Excluding(v1), Excluding(v2)) => Excluding(std::cmp::max(v1, v2).clone()),
            (Excluding(v1), Including(v2)) => {
                if v2 < v1 {
                    Excluding(v1.clone())
                } else {
                    Including(v2.clone())
                }
            }
        };

        let upper = match (&self.upper, &other.upper) {
            (Unbounded, any) => any.clone(),
            (any, Unbounded) => any.clone(),
            (Including(v1), Including(v2)) => Including(std::cmp::min(v1, v2).clone()),
            (Including(v1), Excluding(v2)) => {
                if v1 < v2 {
                    Including(v1.clone())
                } else {
                    Excluding(v2.clone())
                }
            }
            (Excluding(v1), Excluding(v2)) => Excluding(std::cmp::min(v1, v2).clone()),
            (Excluding(v1), Including(v2)) => {
                if v1 <= v2 {
                    Excluding(v1.clone())
                } else {
                    Including(v2.clone())
                }
            }
        };

        match (&lower, &upper) {
            (Including(lower), Excluding(upper))
            | (Excluding(lower), Excluding(upper))
            | (Excluding(lower), Including(upper))
                if upper <= lower =>
            {
                return None
            }
            (Including(lower), Including(upper)) if upper < lower => return None,
            _ => {}
        }

        Some(Range { lower, upper })
    }
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Predicate::*;
        match (&self.lower, &self.upper) {
            (Predicate::Unbounded, Unbounded) => write!(f, "WAT"), // TODO
            (Predicate::Unbounded, Including(v)) => write!(f, "<={}", v),
            (Predicate::Unbounded, Excluding(v)) => write!(f, "<{}", v),
            (Including(v), Predicate::Unbounded) => write!(f, ">={}", v),
            (Excluding(v), Predicate::Unbounded) => write!(f, ">{}", v),
            (Including(v), Including(v2)) if v == v2 => write!(f, "{}", v),
            (Including(v), Including(v2)) => write!(f, ">={} <={}", v, v2),
            (Including(v), Excluding(v2)) => write!(f, ">={} <{}", v, v2),
            (Excluding(v), Including(v2)) => write!(f, ">{} <={}", v, v2),
            (Excluding(v), Excluding(v2)) => write!(f, ">{} <{}", v, v2),
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Predicate {
    Excluding(Version), // < and >
    Including(Version), // <= and >=
    Unbounded,          // *
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VersionReq {
    predicates: Vec<Range>,
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Operation::*;
        match self {
            Exact => write!(f, ""),
            GreaterThan => write!(f, ">"),
            GreaterThanEquals => write!(f, ">="),
            LessThan => write!(f, "<"),
            LessThanEquals => write!(f, "<="),
        }
    }
}

impl VersionReq {
    pub fn satisfies(&self, version: &Version) -> bool {
        for range in &self.predicates {
            if range.satisfies(version) {
                return true;
            }
        }

        false
    }

    pub fn parse<S: AsRef<str>>(input: S) -> Result<Self, SemverError> {
        let input = &input.as_ref()[..];

        match all_consuming(many_predicates::<VerboseError<&str>>)(input) {
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

    pub fn allows_all(&self, other: &VersionReq) -> bool {
        for this in &self.predicates {
            for that in &other.predicates {
                if this.allows_all(&that) {
                    return true;
                }
            }
        }

        false
    }

    pub fn allows_any(&self, other: &VersionReq) -> bool {
        for this in &self.predicates {
            for that in &other.predicates {
                if this.allows_any(&that) {
                    return true;
                }
            }
        }

        false
    }

    pub fn intersect(&self, other: &Self) -> Option<Self> {
        let lefty = &self.predicates[0];
        let righty = &other.predicates[0];

        if let Some(range) = lefty.intersect(righty) {
            Some(Self {
                predicates: vec![range],
            })
        } else {
            None
        }
    }
}

impl std::str::FromStr for VersionReq {
    type Err = SemverError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        VersionReq::parse(s)
    }
}

impl Serialize for VersionReq {
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize VersionReq as a string.
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for VersionReq {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VersionReqVisitor;

        /// Deserialize `VersionReq` from a string.
        impl<'de> Visitor<'de> for VersionReqVisitor {
            type Value = VersionReq;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a SemVer version requirement as a string")
            }

            fn visit_str<E>(self, v: &str) -> ::std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                VersionReq::parse(v).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_str(VersionReqVisitor)
    }
}

fn many_predicates<'a, E>(input: &'a str) -> IResult<&'a str, Vec<Range>, E>
where
    E: ParseError<&'a str>,
{
    context(
        "many predicats",
        separated_nonempty_list(tag(" || "), predicates),
    )(input)
}

fn predicates<'a, E>(input: &'a str) -> IResult<&'a str, Range, E>
where
    E: ParseError<&'a str>,
{
    context(
        "predicate alternatives",
        alt((
            hyphenated_range,
            x_and_asterisk_version,
            no_operation_followed_by_version,
            any_operation_followed_by_version,
            caret,
            tilde,
            wildcard,
        )),
    )(input)
}

fn wildcard<'a, E>(input: &'a str) -> IResult<&'a str, Range, E>
where
    E: ParseError<&'a str>,
{
    context(
        "wildcard",
        map(x_or_asterisk, |_| Range {
            lower: Predicate::Including((0, 0, 0).into()),
            upper: Predicate::Unbounded,
        }),
    )(input)
}

fn x_or_asterisk<'a, E>(input: &'a str) -> IResult<&'a str, (), E>
where
    E: ParseError<&'a str>,
{
    map(alt((tag("x"), tag("*"))), |_| ())(input)
}

type PartialVersion = (
    u64,
    Option<u64>,
    Option<u64>,
    Vec<Identifier>,
    Vec<Identifier>,
);

fn partial_version<'a, E>(input: &'a str) -> IResult<&'a str, PartialVersion, E>
where
    E: ParseError<&'a str>,
{
    map(
        tuple((number, maybe_dot_number, maybe_dot_number, extras)),
        |(major, minor, patch, (pre_release, build))| (major, minor, patch, pre_release, build),
    )(input)
}

fn maybe_dot_number<'a, E>(input: &'a str) -> IResult<&'a str, Option<u64>, E>
where
    E: ParseError<&'a str>,
{
    opt(preceded(tag("."), number))(input)
}

fn any_operation_followed_by_version<'a, E>(input: &'a str) -> IResult<&'a str, Range, E>
where
    E: ParseError<&'a str>,
{
    use Operation::*;
    context(
        "operation followed by version",
        map(
            tuple((operation, preceded(space0, partial_version))),
            |parsed| match parsed {
                (GreaterThanEquals, (major, minor, patch, _, _)) => Range::at_least(
                    Predicate::Including((major, minor.unwrap_or(0), patch.unwrap_or(0)).into()),
                ),
                (GreaterThan, (major, Some(minor), Some(patch), pre_release, build)) => {
                    Range::at_least(Predicate::Excluding(Version {
                        major,
                        minor,
                        patch,
                        pre_release,
                        build,
                    })) // TODO: Pull through for the rest
                }
                (GreaterThan, (major, Some(minor), None, _, _)) => {
                    Range::at_least(Predicate::Including((major, minor + 1, 0).into()))
                }
                (GreaterThan, (major, None, None, _, _)) => {
                    Range::at_least(Predicate::Including((major + 1, 0, 0).into()))
                }
                (LessThan, (major, Some(minor), None, _, _)) => {
                    Range::at_most(Predicate::Excluding((major, minor, 0, 0).into()))
                }
                (LessThan, (major, minor, patch, _, _)) => Range::at_most(Predicate::Excluding(
                    (major, minor.unwrap_or(0), patch.unwrap_or(0)).into(),
                )),
                (LessThanEquals, (major, minor, None, _, _)) => Range::at_most(
                    Predicate::Including((major, minor.unwrap_or(0), 0, 0).into()),
                ),
                (LessThanEquals, (major, Some(minor), Some(patch), _, _)) => {
                    Range::at_most(Predicate::Including((major, minor, patch).into()))
                }
                (Exact, (major, Some(minor), Some(patch), _, _)) => {
                    Range::exact((major, minor, patch).into())
                }
                _ => unreachable!("Odd parsed version: {:?}", parsed),
            },
        ),
    )(input)
}

fn x_and_asterisk_version<'a, E>(input: &'a str) -> IResult<&'a str, Range, E>
where
    E: ParseError<&'a str>,
{
    context(
        "minor X patch X",
        map(
            tuple((
                number,
                preceded(
                    tag("."),
                    alt((
                        map(tuple((x_or_asterisk, tag("."), x_or_asterisk)), |_| None),
                        map(tuple((number, tag("."), x_or_asterisk)), |(minor, _, _)| {
                            Some(minor)
                        }),
                        map(x_or_asterisk, |_| None),
                    )),
                ),
            )),
            |(major, maybe_minor)| Range {
                upper: upper_bound(major, maybe_minor),
                lower: lower_bound(major, maybe_minor),
            },
        ),
    )(input)
}

fn lower_bound(major: u64, maybe_minor: Option<u64>) -> Predicate {
    Predicate::Including((major, maybe_minor.unwrap_or(0), 0).into())
}

fn upper_bound(major: u64, maybe_minor: Option<u64>) -> Predicate {
    if let Some(minor) = maybe_minor {
        Predicate::Excluding((major, minor + 1, 0, 0).into())
    } else {
        Predicate::Excluding((major + 1, 0, 0, 0).into())
    }
}

fn caret<'a, E>(input: &'a str) -> IResult<&'a str, Range, E>
where
    E: ParseError<&'a str>,
{
    context(
        "caret",
        map(
            preceded(tuple((tag("^"), space0)), partial_version),
            |parsed| match parsed {
                (0, None, None, _, _) => Range::at_most(Predicate::Excluding((1, 0, 0, 0).into())),
                (0, Some(minor), None, _, _) => Range::new(
                    Predicate::Including((0, minor, 0).into()),
                    Predicate::Excluding((0, minor + 1, 0, 0).into()),
                ),
                // TODO: can be compressed?
                (major, None, None, _, _) => Range::new(
                    Predicate::Including((major, 0, 0).into()),
                    Predicate::Excluding((major + 1, 0, 0, 0).into()),
                ),
                (major, Some(minor), None, _, _) => Range::new(
                    Predicate::Including((major, minor, 0).into()),
                    Predicate::Excluding((major + 1, 0, 0, 0).into()),
                ),
                (major, Some(minor), Some(patch), _, _) => Range::new(
                    Predicate::Including((major, minor, patch).into()),
                    Predicate::Excluding(match (major, minor, patch) {
                        (0, 0, n) => Version::from((0, 0, n + 1, 0)),
                        (0, n, _) => Version::from((0, n + 1, 0, 0)),
                        (n, _, _) => Version::from((n + 1, 0, 0, 0)),
                    }),
                ),
                _ => unreachable!(),
            },
        ),
    )(input)
}

fn tilde_gt<'a, E>(input: &'a str) -> IResult<&'a str, Option<&'a str>, E>
where
    E: ParseError<&'a str>,
{
    map(
        tuple((tag("~"), space0, opt(tag(">")), space0)),
        |(_, _, gt, _)| gt,
    )(input)
}

fn tilde<'a, E>(input: &'a str) -> IResult<&'a str, Range, E>
where
    E: ParseError<&'a str>,
{
    context(
        "tilde",
        map(tuple((tilde_gt, partial_version)), |parsed| match parsed {
            (Some(_gt), (major, None, None, _, _)) => Range::new(
                Predicate::Including((major, 0, 0).into()),
                Predicate::Excluding((major + 1, 0, 0, 0).into()),
            ),
            (Some(_gt), (major, Some(minor), Some(patch), _, _)) => Range::new(
                Predicate::Including((major, minor, patch).into()),
                Predicate::Excluding((major, minor + 1, 0, 0).into()),
            ),
            (None, (major, Some(minor), Some(patch), _, _)) => Range::new(
                Predicate::Including((major, minor, patch).into()),
                Predicate::Excluding((major, minor + 1, 0, 0).into()),
            ),
            (None, (major, Some(minor), None, _, _)) => Range::new(
                Predicate::Including((major, minor, 0).into()),
                Predicate::Excluding((major, minor + 1, 0, 0).into()),
            ),
            (None, (major, None, None, _, _)) => Range::new(
                Predicate::Including((major, 0, 0).into()),
                Predicate::Excluding((major + 1, 0, 0, 0).into()),
            ),
            _ => unreachable!("Should not have gotten here"),
        }),
    )(input)
}

fn hyphenated<'a, F, G, S, T, E>(
    left: F,
    right: G,
) -> impl Fn(&'a str) -> IResult<&'a str, (S, T), E>
where
    F: Fn(&'a str) -> IResult<&'a str, S, E>,
    G: Fn(&'a str) -> IResult<&'a str, T, E>,
    E: ParseError<&'a str>,
{
    move |input: &'a str| {
        context(
            "hyphenated",
            map(tuple((&left, spaced_hypen, &right)), |(l, _, r)| (l, r)),
        )(input)
    }
}

fn hyphenated_range<'a, E>(input: &'a str) -> IResult<&'a str, Range, E>
where
    E: ParseError<&'a str>,
{
    context(
        "hyphenated with major and minor",
        map(
            hyphenated(partial_version, partial_version),
            |((left, maybe_l_minor, maybe_l_patch, _, _), upper)| {
                Range::new(
                    Predicate::Including(
                        (left, maybe_l_minor.unwrap_or(0), maybe_l_patch.unwrap_or(0)).into(),
                    ),
                    match upper {
                        (major, None, None, _, _) => {
                            Predicate::Excluding((major + 1, 0, 0, 0).into())
                        }
                        (major, Some(minor), None, _, _) => {
                            Predicate::Excluding((major, minor + 1, 0, 0).into())
                        }
                        (major, Some(minor), Some(patch), _, _) => {
                            Predicate::Including((major, minor, patch).into())
                        }
                        _ => unreachable!("No way to a have a patch wtihout a minor"),
                    },
                )
            },
        ),
    )(input)
}

fn no_operation_followed_by_version<'a, E>(input: &'a str) -> IResult<&'a str, Range, E>
where
    E: ParseError<&'a str>,
{
    context(
        "major and minor",
        map(partial_version, |parsed| match parsed {
            (major, Some(minor), Some(patch), _, _) => Range::exact((major, minor, patch).into()),
            (major, maybe_minor, _, _, _) => Range::new(
                lower_bound(major, maybe_minor),
                upper_bound(major, maybe_minor),
            ),
        }),
    )(input)
}

fn spaced_hypen<'a, E>(input: &'a str) -> IResult<&'a str, (), E>
where
    E: ParseError<&'a str>,
{
    map(tuple((space0, tag("-"), space0)), |_| ())(input)
}

fn operation<'a, E>(input: &'a str) -> IResult<&'a str, Operation, E>
where
    E: ParseError<&'a str>,
{
    use Operation::*;
    context(
        "operation",
        alt((
            map(tag(">="), |_| GreaterThanEquals),
            map(tag(">"), |_| GreaterThan),
            map(tag("="), |_| Exact),
            map(tag("<="), |_| LessThanEquals),
            map(tag("<"), |_| LessThan),
        )),
    )(input)
}

impl fmt::Display for VersionReq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, range) in self.predicates.iter().enumerate() {
            if i > 0 {
                write!(f, "||")?;
            }
            write!(f, "{}", range)?;
        }
        Ok(())
    }
}

macro_rules! create_tests_for {
    ($func:ident $($name:ident => $version_range:expr , { $x:ident => $allows:expr, $y:ident => $denies:expr$(,)? }),+ ,$(,)?) => {

        #[cfg(test)]
        mod $func {
        use super::*;

            $(
                #[test]
                fn $name() {
                    let version_range = VersionReq::parse($version_range).unwrap();

                    let allows: Vec<VersionReq> = $allows.iter().map(|v| VersionReq::parse(v).unwrap()).collect();
                    for version in &allows {
                        assert!(version_range.$func(version), "should have allowed: {}", version);
                    }

                    let ranges: Vec<VersionReq> = $denies.iter().map(|v| VersionReq::parse(v).unwrap()).collect();
                    for version in &ranges {
                        assert!(!version_range.$func(version), "should have denied: {}", version);
                    }
                }
            )+
        }
    }
}

create_tests_for! {
    // The function we are testing:
    allows_all

    greater_than_eq_123   => ">=1.2.3", {
        allows => [">=2.0.0", ">2", "2.0.0", "0.1 || 1.4", "1.2.3", "2 - 7", ">2.0.0"],
        denies => ["1.0.0", "<1.2", ">=1.2.2", "1 - 3", "0.1 || <1.2.0", ">1.0.0"],
    },

    greater_than_123      => ">1.2.3", {
        allows => [">=2.0.0", ">2", "2.0.0", "0.1 || 1.4", ">2.0.0"],
        denies => ["1.0.0", "<1.2", ">=1.2.3", "1 - 3", "0.1 || <1.2.0", "<=3"],
    },

    eq_123  => "1.2.3", {
        allows => ["1.2.3"],
        denies => ["1.0.0", "<1.2", "1.x", ">=1.2.2", "1 - 3", "0.1 || <1.2.0"],
    },

    lt_123  => "<1.2.3", {
        allows => ["<=1.2.0", "<1", "1.0.0", "0.1 || 1.4"],
        denies => ["1 - 3", ">1", "2.0.0", "2.0 || >9", ">1.0.0"],
    },

    lt_eq_123 => "<=1.2.3", {
        allows => ["<=1.2.0", "<1", "1.0.0", "0.1 || 1.4", "1.2.3"],
        denies => ["1 - 3", ">1.0.0", ">=1.0.0"],
    },

    eq_123_or_gt_400  => "1.2.3 || >4", {
        allows => [ "1.2.3", ">4", "5.x", "5.2.x", ">=8.2.1", "2.0 || 5.6.7"],
        denies => ["<2", "1 - 7", "1.9.4 || 2-3"],
    },

    between_two_and_eight => "2 - 8", {
        allows => [ "2.2.3", "4 - 5"],
        denies => ["1 - 4", "5 - 9", ">3", "<=5"],
    },
}

create_tests_for! {
    // The function we are testing:
    allows_any

    greater_than_eq_123   => ">=1.2.3", {
        allows => ["<=1.2.4", "3.0.0", "<2", ">=3", ">3.0.0"],
        denies => ["<=1.2.0", "1.0.0", "<1", "<=1.2"],
    },

    greater_than_123   => ">1.2.3", {
        allows => ["<=1.2.4", "3.0.0", "<2", ">=3", ">3.0.0"],
        denies => ["<=1.2.3", "1.0.0", "<1", "<=1.2"],
    },

    eq_123   => "1.2.3", {
        allows => ["1.2.3", "1 - 2"],
        denies => ["<1.2.3", "1.0.0", "<=1.2", ">4.5.6", ">5"],
    },

    lt_eq_123  => "<=1.2.3", {
        allows => ["<=1.2.0", "<1.0.0", "1.0.0", ">1.0.0", ">=1.2.0"],
        denies => ["4.5.6", ">2.0.0", ">=2.0.0"],
    },

    lt_123  => "<1.2.3", {
        allows => ["<=2.2.0", "<2.0.0", "1.0.0", ">1.0.0", ">=1.2.0"],
        denies => ["2.0.0", ">1.8.0", ">=1.8.0"],
    },

    between_two_and_eight => "2 - 8", {
        allows => ["2.2.3", "4 - 10", ">4", ">4.0.0", "<=4.0.0", "<9.1.2"],
        denies => [">10", "10 - 11", "0 - 1"],
    },

    eq_123_or_gt_400  => "1.2.3 || >4", {
        allows => [ "1.2.3", ">3", "5.x", "5.2.x", ">=8.2.1", "2 - 7", "2.0 || 5.6.7"],
        denies => [ "1.9.4 || 2-3"],
    },
}

#[cfg(test)]
mod intersection {
    use super::*;

    fn v(range: &'static str) -> VersionReq {
        range.parse().unwrap()
    }

    #[test]
    fn gt_eq_123() {
        let base_range = v(">=1.2.3");

        let samples = vec![
            ("<=2.0.0", Some(">=1.2.3 <=2.0.0")),
            ("<2.0.0", Some(">=1.2.3 <2.0.0")),
            (">=2.0.0", Some(">=2.0.0")),
            (">2.0.0", Some(">2.0.0")),
            (">1.0.0", Some(">=1.2.3")),
            (">1.2.3", Some(">1.2.3")),
            ("<=1.2.3", Some("1.2.3")),
            ("2.0.0", Some("2.0.0")),
            ("1.1.1", None),
            ("<1.0.0", None),
        ];

        assert_ranges_match(base_range, samples);
    }

    #[test]
    fn gt_123() {
        let base_range = v(">1.2.3");

        let samples = vec![
            ("<=2.0.0", Some(">1.2.3 <=2.0.0")),
            ("<2.0.0", Some(">1.2.3 <2.0.0")),
            (">=2.0.0", Some(">=2.0.0")),
            (">2.0.0", Some(">2.0.0")),
            ("2.0.0", Some("2.0.0")),
            (">1.2.3", Some(">1.2.3")),
            ("<=1.2.3", None),
            ("1.1.1", None),
            ("<1.0.0", None),
        ];

        assert_ranges_match(base_range, samples);
    }

    #[test]
    fn eq_123() {
        let base_range = v("1.2.3");

        let samples = vec![
            ("<=2.0.0", Some("1.2.3")),
            ("<2.0.0", Some("1.2.3")),
            (">=2.0.0", None),
            (">2.0.0", None),
            ("2.0.0", None),
            ("1.2.3", Some("1.2.3")),
            (">1.2.3", None),
            ("<=1.2.3", Some("1.2.3")),
            ("1.1.1", None),
            ("<1.0.0", None),
        ];

        assert_ranges_match(base_range, samples);
    }

    #[test]
    fn lt_123() {
        let base_range = v("<1.2.3");

        let samples = vec![
            ("<=2.0.0", Some("<1.2.3")),
            ("<2.0.0", Some("<1.2.3")),
            (">=2.0.0", None),
            (">=1.0.0", Some(">=1.0.0 <1.2.3")),
            (">2.0.0", None),
            ("2.0.0", None),
            ("1.2.3", None),
            (">1.2.3", None),
            ("<=1.2.3", Some("<1.2.3")),
            ("1.1.1", Some("1.1.1")),
            ("<1.0.0", Some("<1.0.0")),
        ];

        assert_ranges_match(base_range, samples);
    }

    #[test]
    fn lt_eq_123() {
        let base_range = v("<=1.2.3");

        let samples = vec![
            ("<=2.0.0", Some("<=1.2.3")),
            ("<2.0.0", Some("<=1.2.3")),
            (">=2.0.0", None),
            (">=1.0.0", Some(">=1.0.0 <=1.2.3")),
            (">2.0.0", None),
            ("2.0.0", None),
            ("1.2.3", Some("1.2.3")),
            (">1.2.3", None),
            ("<=1.2.3", Some("<=1.2.3")),
            ("1.1.1", Some("1.1.1")),
            ("<1.0.0", Some("<1.0.0")),
        ];

        assert_ranges_match(base_range, samples);
    }

    fn assert_ranges_match(base: VersionReq, samples: Vec<(&'static str, Option<&'static str>)>) {
        for (other, expected) in samples {
            let other = v(other);
            let resulting_range = base.intersect(&other).map(|v| v.to_string());
            assert_eq!(
                resulting_range.clone(),
                expected.map(|e| e.to_string()),
                "{} ∩ {} := {}",
                base,
                other,
                resulting_range.unwrap_or("⊗".into())
            );
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
        let parsed = VersionReq::parse(">=1.2.3").expect("unable to parse");

        refute!(parsed.satisfies(&(0, 2, 3).into()), "major too low");
        refute!(parsed.satisfies(&(1, 1, 3).into()), "minor too low");
        refute!(parsed.satisfies(&(1, 2, 2).into()), "patch too low");
        assert!(parsed.satisfies(&(1, 2, 3).into()), "exact");
        assert!(parsed.satisfies(&(2, 2, 3).into()), "above");
    }

    #[test]
    fn greater_than() {
        let parsed = VersionReq::parse(">1.2.3").expect("unable to parse");

        refute!(parsed.satisfies(&(0, 2, 3).into()), "major too low");
        refute!(parsed.satisfies(&(1, 1, 3).into()), "minor too low");
        refute!(parsed.satisfies(&(1, 2, 2).into()), "patch too low");
        refute!(parsed.satisfies(&(1, 2, 3).into()), "exact");
        assert!(parsed.satisfies(&(1, 2, 4).into()), "above");
    }

    #[test]
    fn exact() {
        let parsed = VersionReq::parse("=1.2.3").expect("unable to parse");

        refute!(parsed.satisfies(&(1, 2, 2).into()), "patch too low");
        assert!(parsed.satisfies(&(1, 2, 3).into()), "exact");
        refute!(parsed.satisfies(&(1, 2, 4).into()), "above");
    }

    #[test]
    fn less_than() {
        let parsed = VersionReq::parse("<1.2.3").expect("unable to parse");

        assert!(parsed.satisfies(&(0, 2, 3).into()), "major below");
        assert!(parsed.satisfies(&(1, 1, 3).into()), "minor below");
        assert!(parsed.satisfies(&(1, 2, 2).into()), "patch below");
        refute!(parsed.satisfies(&(1, 2, 3).into()), "exact");
        refute!(parsed.satisfies(&(1, 2, 4).into()), "above");
    }

    #[test]
    fn less_than_equals() {
        let parsed = VersionReq::parse("<=1.2.3").expect("unable to parse");

        assert!(parsed.satisfies(&(0, 2, 3).into()), "major below");
        assert!(parsed.satisfies(&(1, 1, 3).into()), "minor below");
        assert!(parsed.satisfies(&(1, 2, 2).into()), "patch below");
        assert!(parsed.satisfies(&(1, 2, 3).into()), "exact");
        refute!(parsed.satisfies(&(1, 2, 4).into()), "above");
    }

    #[test]
    fn only_major() {
        let parsed = VersionReq::parse("1").expect("unable to parse");

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
    use serde_derive::{Deserialize, Serialize};

    use pretty_assertions::assert_eq;

    macro_rules! range_parse_tests {
        ($($name:ident => $vals:expr),+ ,$(,)?) => {
            $(
                #[test]
                fn $name() {
                    let [input, expected] = $vals;

                    let parsed = VersionReq::parse(input).expect("unable to parse");

                    assert_eq!(expected, parsed.to_string());
                }
            )+
        }

    }

    range_parse_tests![
        //       [input,   parsed and then `to_string`ed]
        exact => ["1.0.0", "1.0.0"],
        major_minor_patch_range => ["1.0.0 - 2.0.0", ">=1.0.0 <=2.0.0"],
        only_major_versions =>  ["1 - 2", ">=1.0.0 <3.0.0-0"],
        only_major_and_minor => ["1.0 - 2.0", ">=1.0.0 <2.1.0-0"],
        mixed_major_minor => ["1.2 - 3.4.5", ">=1.2.0 <=3.4.5"],
        mixed_major_minor_2 => ["1.2.3 - 3.4", ">=1.2.3 <3.5.0-0"],
        minor_minor_range => ["1.2 - 3.4", ">=1.2.0 <3.5.0-0"],
        single_sided_only_major => ["1", ">=1.0.0 <2.0.0-0"],
        single_sided_lower_equals_bound =>  [">=1.0.0", ">=1.0.0"],
        single_sided_lower_equals_bound_2 => [">=0.1.97", ">=0.1.97"],
        single_sided_lower_bound => [">1.0.0", ">1.0.0"],
        single_sided_upper_equals_bound => ["<=2.0.0", "<=2.0.0"],
        single_sided_upper_equals_bound_with_minor => ["<=2.0", "<=2.0.0-0"],
        single_sided_upper_bound => ["<2.0.0", "<2.0.0"],
        major_and_minor => ["2.3", ">=2.3.0 <2.4.0-0"],
        major_dot_x => ["2.x", ">=2.0.0 <3.0.0-0"],
        x_and_asterisk_version => ["2.x.x", ">=2.0.0 <3.0.0-0"],
        patch_x => ["1.2.x", ">=1.2.0 <1.3.0-0"],
        minor_asterisk_patch_asterisk => ["2.*.*", ">=2.0.0 <3.0.0-0"],
        patch_asterisk => ["1.2.*", ">=1.2.0 <1.3.0-0"],
        caret_zero => ["^0", "<1.0.0-0"],
        caret_zero_minor => ["^0.1", ">=0.1.0 <0.2.0-0"],
        caret_one => ["^1.0", ">=1.0.0 <2.0.0-0"],
        caret_minor => ["^1.2", ">=1.2.0 <2.0.0-0"],
        caret_patch => ["^0.0.1", ">=0.0.1 <0.0.2-0"],
        caret_with_patch =>   ["^0.1.2", ">=0.1.2 <0.2.0-0"],
        caret_with_patch_2 => ["^1.2.3", ">=1.2.3 <2.0.0-0"],
        tilde_one => ["~1", ">=1.0.0 <2.0.0-0"],
        tilde_minor => ["~1.0", ">=1.0.0 <1.1.0-0"],
        tilde_minor_2 => ["~2.4", ">=2.4.0 <2.5.0-0"],
        tilde_with_greater_than_patch => ["~>3.2.1", ">=3.2.1 <3.3.0-0"],
        tilde_major_minor_zero => ["~1.1.0", ">=1.1.0 <1.2.0-0"],
        grater_than_equals_one => [">=1", ">=1.0.0"],
        greater_than_one => [">1", ">=2.0.0"],
        less_than_one_dot_two => ["<1.2", "<1.2.0-0"],
        greater_than_one_dot_two => [">1.2", ">=1.3.0"],
        greater_than_with_prerelease => [">1.1.0-beta-10", ">1.1.0-beta-10"],
        either_one_version_or_the_other => ["0.1.20 || 1.2.4", "0.1.20||1.2.4"],
        either_one_version_range_or_another => [">=0.2.3 || <0.0.1", ">=0.2.3||<0.0.1"],
        either_x_version_works => ["1.2.x || 2.x", ">=1.2.0 <1.3.0-0||>=2.0.0 <3.0.0-0"],
        either_asterisk_version_works => ["1.2.* || 2.*", ">=1.2.0 <1.3.0-0||>=2.0.0 <3.0.0-0"],
        one_two_three_or_greater_than_four => ["1.2.3 || >4", "1.2.3||>=5.0.0"],
        any_version_asterisk => ["*", ">=0.0.0"],
        any_version_x => ["x", ">=0.0.0"],
        whitespace_1 => [">= 1.0.0", ">=1.0.0"],
        whitespace_2 => [">=  1.0.0", ">=1.0.0"],
        whitespace_3 => [">=   1.0.0", ">=1.0.0"],
        whitespace_4 => ["> 1.0.0", ">1.0.0"],
        whitespace_5 => [">  1.0.0", ">1.0.0"],
        whitespace_6 => ["<=   2.0.0", "<=2.0.0"],
        whitespace_7 => ["<= 2.0.0", "<=2.0.0"],
        whitespace_8 => ["<=  2.0.0", "<=2.0.0"],
        whitespace_9 => ["<    2.0.0", "<2.0.0"],
        whitespace_10 => ["<\t2.0.0", "<2.0.0"],
        whitespace_11 => ["^ 1", ">=1.0.0 <2.0.0-0"],
        whitespace_12 => ["~> 1", ">=1.0.0 <2.0.0-0"],
        whitespace_13 => ["~ 1.0", ">=1.0.0 <1.1.0-0"],
    ];

    /*
    // From here onwards we might have to deal with pre-release tags to?
    ["^0.0.1-beta", ">=0.0.1-beta <0.0.2-0"],
    ["^1.2.3-beta.4", ">=1.2.3-beta.4 <2.0.0-0"],
    [">01.02.03", ">1.2.3", true],
    [">01.02.03", null],
    ["~1.2.3beta", ">=1.2.3-beta <1.3.0-0", { loose: true }],
    ["~1.2.3beta", null],
    ["^ 1.2 ^ 1", ">=1.2.0 <2.0.0-0 >=1.0.0"],
    [">X", "<0.0.0-0"],
    ["<X", "<0.0.0-0"],
    ["<x <* || >* 2.x", "<0.0.0-0"],
    */

    #[derive(Serialize, Deserialize, Eq, PartialEq)]
    struct WithVersionReq {
        req: VersionReq,
    }

    #[test]
    fn read_version_req_from_string() {
        let v: WithVersionReq = serde_json::from_str(r#"{"req":"^1.2.3"}"#).unwrap();

        assert_eq!(v.req, "^1.2.3".parse().unwrap(),);
    }

    #[test]
    fn serialize_a_versionreq_to_string() {
        let output = serde_json::to_string(&WithVersionReq {
            req: VersionReq {
                predicates: vec![Range::at_most(Predicate::Excluding(
                    "1.2.3".parse().unwrap(),
                ))],
            },
        })
        .unwrap();
        let expected: String = r#"{"req":"<1.2.3"}"#.into();

        assert_eq!(output, expected);
    }
}

#[cfg(test)]
mod ranges {
    use super::*;

    #[test]
    fn one() {
        let r = Range::new(
            Predicate::Including((1, 2, 0).into()),
            Predicate::Excluding((3, 3, 4).into()),
        );

        assert_eq!(r.to_string(), ">=1.2.0 <3.3.4")
    }
}
