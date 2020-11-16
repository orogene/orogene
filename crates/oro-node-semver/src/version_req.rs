use std::cmp::{Ord, Ordering, PartialOrd};
use std::fmt;

use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::space0;
use nom::combinator::{all_consuming, map, map_opt, opt};
use nom::error::context;
use nom::multi::separated_list1;
use nom::sequence::{preceded, tuple};
use nom::{Err, IResult};
use serde::de::{self, Deserialize, Deserializer, Visitor};
use serde::ser::{Serialize, Serializer};

use crate::{extras, number, Identifier, SemverError, SemverErrorKind, SemverParseError, Version};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct Range {
    upper: Bound,
    lower: Bound,
}

impl Range {
    fn new(lower: Bound, upper: Bound) -> Option<Self> {
        use Bound::*;
        use Predicate::*;

        match (lower, upper) {
            (Lower(Excluding(v1)), Upper(Including(v2)))
            | (Lower(Including(v1)), Upper(Excluding(v2)))
                if v1 == v2 =>
            {
                None
            }
            (Lower(Including(v1)), Upper(Including(v2))) if v1 == v2 => Some(Self {
                lower: Lower(Including(v1)),
                upper: Upper(Including(v2)),
            }),
            (lower, upper) if lower < upper => Some(Self { lower, upper }),
            _ => None,
        }
    }

    fn at_least(p: Predicate) -> Option<Self> {
        Range::new(Bound::Lower(p), Bound::upper())
    }

    fn at_most(p: Predicate) -> Option<Self> {
        Range::new(Bound::lower(), Bound::Upper(p))
    }

    fn exact(version: Version) -> Option<Self> {
        Range::new(
            Bound::Lower(Predicate::Including(version.clone())),
            Bound::Upper(Predicate::Including(version)),
        )
    }

    fn satisfies(&self, version: &Version) -> bool {
        use Bound::*;
        use Predicate::*;

        let lower_bound = match &self.lower {
            Lower(Including(lower)) => lower <= version,
            Lower(Excluding(lower)) => lower < version,
            Lower(Unbounded) => true,
            _ => unreachable!(
                "There should not have been an upper bound: {:#?}",
                self.lower
            ),
        };

        let upper_bound = match &self.upper {
            Upper(Including(upper)) => version <= upper,
            Upper(Excluding(upper)) => version < upper,
            Upper(Unbounded) => true,
            _ => unreachable!(
                "There should not have been an lower bound: {:#?}",
                self.lower
            ),
        };

        lower_bound && upper_bound
    }

    fn allows_all(&self, other: &Range) -> bool {
        self.lower <= other.lower && other.upper <= self.upper
    }

    fn allows_any(&self, other: &Range) -> bool {
        if other.upper < self.lower {
            return false;
        }

        if self.upper < other.lower {
            return false;
        }

        true
    }

    fn intersect(&self, other: &Self) -> Option<Self> {
        let lower = std::cmp::max(&self.lower, &other.lower);
        let upper = std::cmp::min(&self.upper, &other.upper);

        Range::new(lower.clone(), upper.clone())
    }

    fn difference(&self, other: &Self) -> Option<Vec<Self>> {
        use Bound::*;

        if let Some(overlap) = self.intersect(&other) {
            if &overlap == self {
                return None;
            }

            if self.lower < overlap.lower && overlap.upper < self.upper {
                return Some(vec![
                    Range::new(self.lower.clone(), Upper(overlap.lower.predicate().flip()))
                        .unwrap(),
                    Range::new(Lower(overlap.upper.predicate().flip()), self.upper.clone())
                        .unwrap(),
                ]);
            }

            if self.lower < overlap.lower {
                return Range::new(self.lower.clone(), Upper(overlap.lower.predicate().flip()))
                    .map(|f| vec![f]);
            }

            Range::new(Lower(overlap.upper.predicate().flip()), self.upper.clone()).map(|f| vec![f])
        } else {
            Some(vec![self.clone()])
        }
    }
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Bound::*;
        use Predicate::*;
        match (&self.lower, &self.upper) {
            (Lower(Unbounded), Upper(Unbounded)) => write!(f, "*"),
            (Lower(Unbounded), Upper(Including(v))) => write!(f, "<={}", v),
            (Lower(Unbounded), Upper(Excluding(v))) => write!(f, "<{}", v),
            (Lower(Including(v)), Upper(Unbounded)) => write!(f, ">={}", v),
            (Lower(Excluding(v)), Upper(Unbounded)) => write!(f, ">{}", v),
            (Lower(Including(v)), Upper(Including(v2))) if v == v2 => write!(f, "{}", v),
            (Lower(Including(v)), Upper(Including(v2))) => write!(f, ">={} <={}", v, v2),
            (Lower(Including(v)), Upper(Excluding(v2))) => write!(f, ">={} <{}", v, v2),
            (Lower(Excluding(v)), Upper(Including(v2))) => write!(f, ">{} <={}", v, v2),
            (Lower(Excluding(v)), Upper(Excluding(v2))) => write!(f, ">{} <{}", v, v2),
            _ => unreachable!("does not make sense"),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum Operation {
    Exact,
    GreaterThan,
    GreaterThanEquals,
    LessThan,
    LessThanEquals,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Predicate {
    Excluding(Version), // < and >
    Including(Version), // <= and >=
    Unbounded,          // *
}

impl Predicate {
    fn flip(&self) -> Self {
        use Predicate::*;
        match self {
            Excluding(v) => Including(v.clone()),
            Including(v) => Excluding(v.clone()),
            Unbounded => Unbounded,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum Bound {
    Lower(Predicate),
    Upper(Predicate),
}

impl Bound {
    fn upper() -> Self {
        Bound::Upper(Predicate::Unbounded)
    }

    fn lower() -> Self {
        Bound::Lower(Predicate::Unbounded)
    }

    fn predicate(&self) -> Predicate {
        use Bound::*;

        match self {
            Lower(p) => p.clone(),
            Upper(p) => p.clone(),
        }
    }
}

impl Ord for Bound {
    fn cmp(&self, other: &Self) -> Ordering {
        use Bound::*;
        use Predicate::*;

        match (self, other) {
            (Lower(Unbounded), Lower(Unbounded)) | (Upper(Unbounded), Upper(Unbounded)) => {
                Ordering::Equal
            }
            (Upper(Unbounded), _) | (_, Lower(Unbounded)) => Ordering::Greater,
            (Lower(Unbounded), _) | (_, Upper(Unbounded)) => Ordering::Less,

            (Upper(Including(v1)), Upper(Including(v2)))
            | (Upper(Including(v1)), Lower(Including(v2)))
            | (Upper(Excluding(v1)), Upper(Excluding(v2)))
            | (Upper(Excluding(v1)), Lower(Excluding(v2)))
            | (Lower(Including(v1)), Upper(Including(v2)))
            | (Lower(Including(v1)), Lower(Including(v2)))
            | (Lower(Excluding(v1)), Lower(Excluding(v2))) => v1.cmp(v2),

            (Lower(Excluding(v1)), Upper(Excluding(v2)))
            | (Lower(Including(v1)), Upper(Excluding(v2))) => {
                if v2 <= v1 {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            }
            (Upper(Including(v1)), Upper(Excluding(v2)))
            | (Upper(Including(v1)), Lower(Excluding(v2)))
            | (Lower(Excluding(v1)), Upper(Including(v2))) => {
                if v2 < v1 {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            }
            (Lower(Excluding(v1)), Lower(Including(v2))) => {
                if v1 < v2 {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
            (Lower(Including(v1)), Lower(Excluding(v2)))
            | (Upper(Excluding(v1)), Lower(Including(v2)))
            | (Upper(Excluding(v1)), Upper(Including(v2))) => {
                if v1 <= v2 {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
        }
    }
}

impl PartialOrd for Bound {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
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
    pub fn parse<S: AsRef<str>>(input: S) -> Result<Self, SemverError> {
        let input = &input.as_ref()[..];

        match all_consuming(many_predicates)(input) {
            Ok((_, predicates)) => Ok(VersionReq { predicates }),
            Err(err) => Err(match err {
                Err::Error(e) | Err::Failure(e) => SemverError {
                    input: input.into(),
                    offset: e.input.as_ptr() as usize - input.as_ptr() as usize,
                    kind: if let Some(kind) = e.kind {
                        kind
                    } else if let Some(ctx) = e.context {
                        SemverErrorKind::Context(ctx)
                    } else {
                        SemverErrorKind::Other
                    },
                },
                Err::Incomplete(_) => SemverError {
                    input: input.into(),
                    offset: input.len() - 1,
                    kind: SemverErrorKind::IncompleteInput,
                },
            }),
        }
    }

    pub fn any() -> Self {
        Self {
            predicates: vec![Range::new(Bound::lower(), Bound::upper()).unwrap()],
        }
    }

    pub fn satisfies(&self, version: &Version) -> bool {
        for range in &self.predicates {
            if range.satisfies(version) {
                return true;
            }
        }

        false
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
        let mut predicates = Vec::new();

        for lefty in &self.predicates {
            for righty in &other.predicates {
                if let Some(range) = lefty.intersect(righty) {
                    predicates.push(range)
                }
            }
        }

        if predicates.is_empty() {
            None
        } else {
            Some(Self { predicates })
        }
    }

    pub fn difference(&self, other: &Self) -> Option<Self> {
        let mut predicates = Vec::new();

        for lefty in &self.predicates {
            for righty in &other.predicates {
                if let Some(mut range) = lefty.difference(righty) {
                    predicates.append(&mut range)
                }
            }
        }

        if predicates.is_empty() {
            None
        } else {
            Some(Self { predicates })
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

fn many_predicates<'a>(input: &'a str) -> IResult<&'a str, Vec<Range>, SemverParseError<&'a str>> {
    context("many predicates", separated_list1(tag(" || "), predicates))(input)
}

fn predicates<'a>(input: &'a str) -> IResult<&'a str, Range, SemverParseError<&'a str>> {
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

fn wildcard<'a>(input: &'a str) -> IResult<&'a str, Range, SemverParseError<&'a str>> {
    context(
        "wildcard",
        map_opt(x_or_asterisk, |_| {
            Range::at_least(Predicate::Including((0, 0, 0).into()))
        }),
    )(input)
}

fn x_or_asterisk<'a>(input: &'a str) -> IResult<&'a str, (), SemverParseError<&'a str>> {
    map(alt((tag("x"), tag("*"))), |_| ())(input)
}

type PartialVersion = (
    u64,
    Option<u64>,
    Option<u64>,
    Vec<Identifier>,
    Vec<Identifier>,
);

fn partial_version<'a>(
    input: &'a str,
) -> IResult<&'a str, PartialVersion, SemverParseError<&'a str>> {
    map(
        tuple((number, maybe_dot_number, maybe_dot_number, extras)),
        |(major, minor, patch, (pre_release, build))| (major, minor, patch, pre_release, build),
    )(input)
}

fn maybe_dot_number<'a>(
    input: &'a str,
) -> IResult<&'a str, Option<u64>, SemverParseError<&'a str>> {
    opt(preceded(tag("."), number))(input)
}

fn any_operation_followed_by_version<'a>(
    input: &'a str,
) -> IResult<&'a str, Range, SemverParseError<&'a str>> {
    use Operation::*;
    context(
        "operation followed by version",
        map_opt(
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

fn x_and_asterisk_version<'a>(
    input: &'a str,
) -> IResult<&'a str, Range, SemverParseError<&'a str>> {
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

fn lower_bound(major: u64, maybe_minor: Option<u64>) -> Bound {
    Bound::Lower(Predicate::Including(
        (major, maybe_minor.unwrap_or(0), 0).into(),
    ))
}

fn upper_bound(major: u64, maybe_minor: Option<u64>) -> Bound {
    if let Some(minor) = maybe_minor {
        Bound::Upper(Predicate::Excluding((major, minor + 1, 0, 0).into()))
    } else {
        Bound::Upper(Predicate::Excluding((major + 1, 0, 0, 0).into()))
    }
}

fn caret<'a>(input: &'a str) -> IResult<&'a str, Range, SemverParseError<&'a str>> {
    context(
        "caret",
        map_opt(
            preceded(tuple((tag("^"), space0)), partial_version),
            |parsed| match parsed {
                (0, None, None, _, _) => Range::at_most(Predicate::Excluding((1, 0, 0, 0).into())),
                (0, Some(minor), None, _, _) => Range::new(
                    Bound::Lower(Predicate::Including((0, minor, 0).into())),
                    Bound::Upper(Predicate::Excluding((0, minor + 1, 0, 0).into())),
                ),
                // TODO: can be compressed?
                (major, None, None, _, _) => Range::new(
                    Bound::Lower(Predicate::Including((major, 0, 0).into())),
                    Bound::Upper(Predicate::Excluding((major + 1, 0, 0, 0).into())),
                ),
                (major, Some(minor), None, _, _) => Range::new(
                    Bound::Lower(Predicate::Including((major, minor, 0).into())),
                    Bound::Upper(Predicate::Excluding((major + 1, 0, 0, 0).into())),
                ),
                (major, Some(minor), Some(patch), pre_release, _) => Range::new(
                    Bound::Lower(Predicate::Including(Version {
                        major,
                        minor,
                        patch,
                        pre_release,
                        build: vec![],
                    })),
                    Bound::Upper(Predicate::Excluding(match (major, minor, patch) {
                        (0, 0, n) => Version::from((0, 0, n + 1, 0)),
                        (0, n, _) => Version::from((0, n + 1, 0, 0)),
                        (n, _, _) => Version::from((n + 1, 0, 0, 0)),
                    })),
                ),
                _ => unreachable!(),
            },
        ),
    )(input)
}

fn tilde_gt<'a>(input: &'a str) -> IResult<&'a str, Option<&'a str>, SemverParseError<&'a str>> {
    map(
        tuple((tag("~"), space0, opt(tag(">")), space0)),
        |(_, _, gt, _)| gt,
    )(input)
}

fn tilde<'a>(input: &'a str) -> IResult<&'a str, Range, SemverParseError<&'a str>> {
    context(
        "tilde",
        map_opt(tuple((tilde_gt, partial_version)), |parsed| match parsed {
            (Some(_gt), (major, None, None, _, _)) => Range::new(
                Bound::Lower(Predicate::Including((major, 0, 0).into())),
                Bound::Upper(Predicate::Excluding((major + 1, 0, 0, 0).into())),
            ),
            (Some(_gt), (major, Some(minor), Some(patch), _, _)) => Range::new(
                Bound::Lower(Predicate::Including((major, minor, patch).into())),
                Bound::Upper(Predicate::Excluding((major, minor + 1, 0, 0).into())),
            ),
            (None, (major, Some(minor), Some(patch), _, _)) => Range::new(
                Bound::Lower(Predicate::Including((major, minor, patch).into())),
                Bound::Upper(Predicate::Excluding((major, minor + 1, 0, 0).into())),
            ),
            (None, (major, Some(minor), None, _, _)) => Range::new(
                Bound::Lower(Predicate::Including((major, minor, 0).into())),
                Bound::Upper(Predicate::Excluding((major, minor + 1, 0, 0).into())),
            ),
            (None, (major, None, None, _, _)) => Range::new(
                Bound::Lower(Predicate::Including((major, 0, 0).into())),
                Bound::Upper(Predicate::Excluding((major + 1, 0, 0, 0).into())),
            ),
            _ => unreachable!("Should not have gotten here"),
        }),
    )(input)
}

fn hyphenated<'a, F, G, S, T>(
    left: F,
    right: G,
) -> impl Fn(&'a str) -> IResult<&'a str, (S, T), SemverParseError<&'a str>>
where
    F: Fn(&'a str) -> IResult<&'a str, S, SemverParseError<&'a str>>,
    G: Fn(&'a str) -> IResult<&'a str, T, SemverParseError<&'a str>>,
{
    move |input: &'a str| {
        context(
            "hyphenated",
            map(tuple((&left, spaced_hypen, &right)), |(l, _, r)| (l, r)),
        )(input)
    }
}

fn hyphenated_range<'a>(input: &'a str) -> IResult<&'a str, Range, SemverParseError<&'a str>> {
    context(
        "hyphenated with major and minor",
        map_opt(
            hyphenated(partial_version, partial_version),
            |((left, maybe_l_minor, maybe_l_patch, pre_release, _), upper)| {
                Range::new(
                    Bound::Lower(Predicate::Including(Version {
                        major: left,
                        minor: maybe_l_minor.unwrap_or(0),
                        patch: maybe_l_patch.unwrap_or(0),
                        pre_release,
                        build: vec![],
                    })),
                    Bound::Upper(match upper {
                        (major, None, None, _, _) => {
                            Predicate::Excluding((major + 1, 0, 0, 0).into())
                        }
                        (major, Some(minor), None, _, _) => {
                            Predicate::Excluding((major, minor + 1, 0, 0).into())
                        }
                        (major, Some(minor), Some(patch), pre_release, _) => {
                            Predicate::Including(Version {
                                major,
                                minor,
                                patch,
                                pre_release,
                                build: vec![],
                            })
                        }
                        _ => unreachable!("No way to a have a patch wtihout a minor"),
                    }),
                )
            },
        ),
    )(input)
}

fn no_operation_followed_by_version<'a>(
    input: &'a str,
) -> IResult<&'a str, Range, SemverParseError<&'a str>> {
    context(
        "major and minor",
        map_opt(partial_version, |parsed| match parsed {
            (major, Some(minor), Some(patch), _, _) => Range::exact((major, minor, patch).into()),
            (major, maybe_minor, _, _, _) => Range::new(
                lower_bound(major, maybe_minor),
                upper_bound(major, maybe_minor),
            ),
        }),
    )(input)
}

fn spaced_hypen<'a>(input: &'a str) -> IResult<&'a str, (), SemverParseError<&'a str>> {
    map(tuple((space0, tag("-"), space0)), |_| ())(input)
}

fn operation<'a>(input: &'a str) -> IResult<&'a str, Operation, SemverParseError<&'a str>> {
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

    #[test]
    fn multiple() {
        let base_range = v("<1 || 3-4");

        let samples = vec![("0.5 - 3.5.0", Some(">=0.5.0 <1.0.0||>=3.0.0 <=3.5.0"))];

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
                resulting_range.unwrap_or_else(|| "⊗".into())
            );
        }
    }
}

#[cfg(test)]
mod difference {
    use super::*;

    fn v(range: &'static str) -> VersionReq {
        range.parse().unwrap()
    }

    #[test]
    fn gt_eq_123() {
        let base_range = v(">=1.2.3");

        let samples = vec![
            ("<=2.0.0", Some(">2.0.0")),
            ("<2.0.0", Some(">=2.0.0")),
            (">=2.0.0", Some(">=1.2.3 <2.0.0")),
            (">2.0.0", Some(">=1.2.3 <=2.0.0")),
            (">1.0.0", None),
            (">1.2.3", Some("1.2.3")),
            ("<=1.2.3", Some(">1.2.3")),
            ("1.1.1", Some(">=1.2.3")),
            ("<1.0.0", Some(">=1.2.3")),
            ("2.0.0", Some(">=1.2.3 <2.0.0||>2.0.0")),
        ];

        assert_ranges_match(base_range, samples);
    }

    #[test]
    fn gt_123() {
        let base_range = v(">1.2.3");

        let samples = vec![
            ("<=2.0.0", Some(">2.0.0")),
            ("<2.0.0", Some(">=2.0.0")),
            (">=2.0.0", Some(">1.2.3 <2.0.0")),
            (">2.0.0", Some(">1.2.3 <=2.0.0")),
            (">1.0.0", None),
            (">1.2.3", None),
            ("<=1.2.3", Some(">1.2.3")),
            ("1.1.1", Some(">1.2.3")),
            ("<1.0.0", Some(">1.2.3")),
            ("2.0.0", Some(">1.2.3 <2.0.0||>2.0.0")),
        ];

        assert_ranges_match(base_range, samples);
    }

    #[test]
    fn eq_123() {
        let base_range = v("1.2.3");

        let samples = vec![
            ("<=2.0.0", None),
            ("<2.0.0", None),
            (">=2.0.0", Some("1.2.3")),
            (">2.0.0", Some("1.2.3")),
            (">1.0.0", None),
            (">1.2.3", Some("1.2.3")),
            ("1.2.3", None),
            ("<=1.2.3", None),
            ("1.1.1", Some("1.2.3")),
            ("<1.0.0", Some("1.2.3")),
            ("2.0.0", Some("1.2.3")),
        ];

        assert_ranges_match(base_range, samples);
    }

    #[test]
    fn lt_123() {
        let base_range = v("<1.2.3");

        let samples = vec![
            ("<=2.0.0", None),
            ("<2.0.0", None),
            (">=2.0.0", Some("<1.2.3")),
            (">2.0.0", Some("<1.2.3")),
            (">1.0.0", Some("<=1.0.0")),
            (">1.2.3", Some("<1.2.3")),
            ("<=1.2.3", None),
            ("1.1.1", Some("<1.1.1||>1.1.1 <1.2.3")),
            ("<1.0.0", Some(">=1.0.0 <1.2.3")),
            ("2.0.0", Some("<1.2.3")),
        ];

        assert_ranges_match(base_range, samples);
    }

    #[test]
    fn lt_eq_123() {
        let base_range = v("<=1.2.3");

        let samples = vec![
            ("<=2.0.0", None),
            ("<2.0.0", None),
            (">=2.0.0", Some("<=1.2.3")),
            (">2.0.0", Some("<=1.2.3")),
            (">1.0.0", Some("<=1.0.0")),
            (">1.2.3", Some("<=1.2.3")),
            ("<=1.2.3", None),
            ("1.1.1", Some("<1.1.1||>1.1.1 <=1.2.3")),
            ("<1.0.0", Some(">=1.0.0 <=1.2.3")),
            ("2.0.0", Some("<=1.2.3")),
        ];

        assert_ranges_match(base_range, samples);
    }

    #[test]
    fn multiple() {
        let base_range = v("<1 || 3-4");

        let samples = vec![("0.5 - 3.5.0", Some("<0.5.0||>3.5.0 <4.0.0-0"))];

        assert_ranges_match(base_range, samples);
    }

    fn assert_ranges_match(base: VersionReq, samples: Vec<(&'static str, Option<&'static str>)>) {
        for (other, expected) in samples {
            let other = v(other);
            let resulting_range = base.difference(&other).map(|v| v.to_string());
            assert_eq!(
                resulting_range.clone(),
                expected.map(|e| e.to_string()),
                "{} \\ {} := {}",
                base,
                other,
                resulting_range.unwrap_or_else(|| "⊗".into())
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
        beta          => ["^0.0.1-beta", ">=0.0.1-beta <0.0.2-0"],
        beta_4        => ["^1.2.3-beta.4", ">=1.2.3-beta.4 <2.0.0-0"],
        pre_release_on_both => ["1.0.0-alpha - 2.0.0-beta", ">=1.0.0-alpha <=2.0.0-beta"],
        single_sided_lower_bound_with_pre_release => [">1.0.0-alpha", ">1.0.0-alpha"],
    ];

    /*
    // From here onwards we might have to deal with pre-release tags to?
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
                predicates: vec![
                    Range::at_most(Predicate::Excluding("1.2.3".parse().unwrap())).unwrap(),
                ],
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
            Bound::Lower(Predicate::Including((1, 2, 0).into())),
            Bound::Upper(Predicate::Excluding((3, 3, 4).into())),
        )
        .unwrap();

        assert_eq!(r.to_string(), ">=1.2.0 <3.3.4")
    }
}
