use std::cmp::{self, Ordering};
use std::fmt;
use std::num::ParseIntError;

use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::bytes::complete::take_while;
use nom::character::complete::digit1;
use nom::character::is_alphanumeric;
use nom::combinator::{all_consuming, map, map_res, opt, recognize};
use nom::error::{context, ContextError, ErrorKind, FromExternalError, ParseError};
use nom::multi::separated_list1;
use nom::sequence::{preceded, tuple};
use nom::{Err, IResult};
use oro_diagnostics::{Diagnostic, DiagnosticCategory, Explain, Meta};
use oro_diagnostics_derive::Diagnostic;
use serde::de::{self, Deserialize, Deserializer, Visitor};
use serde::ser::{Serialize, Serializer};
use thiserror::Error;

pub use version_req::VersionReq;

pub mod version_req;

// from JavaScript: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Number/MAX_SAFE_INTEGER
const MAX_SAFE_INTEGER: u64 = 900_719_925_474_099;
const MAX_LENGTH: usize = 256;

#[derive(Debug, Error, Eq, PartialEq, Diagnostic)]
#[error("Error parsing semver string. {kind}")]
#[label("semver::no_parse")]
#[category(Parse)]
pub struct SemverError {
    input: String,
    offset: usize,
    kind: SemverErrorKind,
}

impl SemverError {
    pub fn location(&self) -> (usize, usize) {
        // Taken partially from nom.
        let prefix = &self.input.as_bytes()[..self.offset];

        // Count the number of newlines in the first `offset` bytes of input
        let line_number = bytecount::count(prefix, b'\n');

        // Find the line that includes the subslice:
        // Find the *last* newline before the substring starts
        let line_begin = prefix
            .iter()
            .rev()
            .position(|&b| b == b'\n')
            .map(|pos| self.offset - pos)
            .unwrap_or(0);

        // Find the full line after that newline
        let line = self.input[line_begin..]
            .lines()
            .next()
            .unwrap_or(&self.input[line_begin..])
            .trim_end();

        // The (1-indexed) column number is the offset of our substring into that line
        let column_number = self.input[self.offset..].as_ptr() as usize - line.as_ptr() as usize;

        (line_number, column_number)
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum SemverErrorKind {
    #[error("Semver string can't be longer than {} characters.", MAX_LENGTH)]
    MaxLengthError,
    #[error("Incomplete input to semver parser.")]
    IncompleteInput,
    #[error("Failed to parse an integer component of a semver string: {0}")]
    ParseIntError(ParseIntError),
    #[error("Integer component of semver string is larger than JavaScript's Number.MAX_SAFE_INTEGER: {0}")]
    MaxIntError(u64),
    #[error("Failed to parse {0} component of semver string.")]
    Context(&'static str),
    #[error("An unspecified error occurred.")]
    Other,
}

#[derive(Debug)]
struct SemverParseError<I> {
    input: I,
    context: Option<&'static str>,
    kind: Option<SemverErrorKind>,
}

impl Explain for SemverError {
    fn meta(&self) -> Option<Meta> {
        let (row, col) = self.location();
        Some(Meta::Parse {
            input: self.input.clone(),
            path: None,
            row,
            col,
        })
    }
}

impl<I> ParseError<I> for SemverParseError<I> {
    fn from_error_kind(input: I, _kind: nom::error::ErrorKind) -> Self {
        Self {
            input,
            context: None,
            kind: None,
        }
    }

    fn append(_input: I, _kind: nom::error::ErrorKind, other: Self) -> Self {
        other
    }
}

impl<I> ContextError<I> for SemverParseError<I> {
    fn add_context(_input: I, ctx: &'static str, mut other: Self) -> Self {
        other.context = Some(ctx);
        other
    }
}

impl<'a> FromExternalError<&'a str, SemverParseError<&'a str>> for SemverParseError<&'a str> {
    fn from_external_error(
        _input: &'a str,
        _kind: ErrorKind,
        e: SemverParseError<&'a str>,
    ) -> Self {
        e
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Identifier {
    /// An identifier that's solely numbers.
    Numeric(u64),
    /// An identifier with letters and numbers.
    AlphaNumeric(String),
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Identifier::Numeric(n) => write!(f, "{}", n),
            Identifier::AlphaNumeric(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
    build: Vec<Identifier>,
    pre_release: Vec<Identifier>,
}

impl Version {
    pub fn parse<S: AsRef<str>>(input: S) -> Result<Version, SemverError> {
        let input = &input.as_ref()[..];

        if input.len() > MAX_LENGTH {
            return Err(SemverError {
                input: input.into(),
                offset: 0,
                kind: SemverErrorKind::MaxLengthError,
            });
        }

        match all_consuming(version)(input) {
            Ok((_, arg)) => Ok(arg),
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
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.major == other.major
            && self.minor == other.minor
            && self.patch == other.patch
            && self.pre_release == other.pre_release
    }
}

impl Eq for Version {}

impl std::hash::Hash for Version {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.major.hash(state);
        self.minor.hash(state);
        self.patch.hash(state);
        self.pre_release.hash(state);
    }
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct IntegrityVisitor;

        impl<'de> Visitor<'de> for IntegrityVisitor {
            type Value = Version;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a version string")
            }

            fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Version::parse(v).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_str(IntegrityVisitor)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;

        for (i, ident) in self.pre_release.iter().enumerate() {
            if i == 0 {
                write!(f, "-")?;
            } else {
                write!(f, ".")?;
            }
            write!(f, "{}", ident)?;
        }

        for (i, ident) in self.build.iter().enumerate() {
            if i == 0 {
                write!(f, "+")?;
            } else {
                write!(f, ".")?;
            }
            write!(f, "{}", ident)?;
        }

        Ok(())
    }
}

impl std::convert::From<(u64, u64, u64)> for Version {
    fn from((major, minor, patch): (u64, u64, u64)) -> Self {
        Version {
            major,
            minor,
            patch,
            build: Vec::new(),
            pre_release: Vec::new(),
        }
    }
}

impl std::str::FromStr for Version {
    type Err = SemverError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Version::parse(s)
    }
}

impl std::convert::From<(u64, u64, u64, u64)> for Version {
    fn from((major, minor, patch, pre_release): (u64, u64, u64, u64)) -> Self {
        Version {
            major,
            minor,
            patch,
            build: Vec::new(),
            pre_release: vec![Identifier::Numeric(pre_release)],
        }
    }
}

impl cmp::PartialOrd for Version {
    fn partial_cmp(&self, other: &Version) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl cmp::Ord for Version {
    fn cmp(&self, other: &Version) -> cmp::Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => {}
            //if difference in major version, just return result
            order_result => return order_result,
        }

        match self.minor.cmp(&other.minor) {
            Ordering::Equal => {}
            //if difference in minor version, just return result
            order_result => return order_result,
        }

        match self.patch.cmp(&other.patch) {
            Ordering::Equal => {}
            //if difference in patch version, just return result
            order_result => return order_result,
        }

        match (self.pre_release.len(), other.pre_release.len()) {
            //if no pre_release string, they're equal
            (0, 0) => Ordering::Equal,
            //if other has a pre-release string, but this doesn't, this one is greater
            (0, _) => Ordering::Greater,
            //if this one has a pre-release string, but other doesn't this one is less than
            (_, 0) => Ordering::Less,
            // if both have pre_release strings, compare the strings and return the result
            (_, _) => self.pre_release.cmp(&other.pre_release),
        }
    }
}

enum Extras {
    Build(Vec<Identifier>),
    Release(Vec<Identifier>),
    ReleaseAndBuild((Vec<Identifier>, Vec<Identifier>)),
}

impl Extras {
    fn values(self) -> (Vec<Identifier>, Vec<Identifier>) {
        use Extras::*;
        match self {
            Release(ident) => (ident, Vec::new()),
            Build(ident) => (Vec::new(), ident),
            ReleaseAndBuild(ident) => ident,
        }
    }
}

/// <valid semver> ::= <version core>
///                 | <version core> "-" <pre-release>
///                 | <version core> "+" <build>
///                 | <version core> "-" <pre-release> "+" <build>
fn version<'a>(input: &'a str) -> IResult<&'a str, Version, SemverParseError<&'a str>> {
    context(
        "version",
        map(
            tuple((version_core, extras)),
            |((major, minor, patch), (pre_release, build))| Version {
                major,
                minor,
                patch,
                pre_release,
                build,
            },
        ),
    )(input)
}

fn extras<'a>(
    input: &'a str,
) -> IResult<&'a str, (Vec<Identifier>, Vec<Identifier>), SemverParseError<&'a str>> {
    map(
        opt(alt((
            map(tuple((pre_release, build)), Extras::ReleaseAndBuild),
            map(pre_release, Extras::Release),
            map(build, Extras::Build),
        ))),
        |extras| match extras {
            Some(extras) => extras.values(),
            _ => Default::default(),
        },
    )(input)
}

/// <version core> ::= <major> "." <minor> "." <patch>
fn version_core<'a>(
    input: &'a str,
) -> IResult<&'a str, (u64, u64, u64), SemverParseError<&'a str>> {
    context(
        "version core",
        map(
            tuple((number, tag("."), number, tag("."), number)),
            |(major, _, minor, _, patch)| (major, minor, patch),
        ),
    )(input)
}

// I believe build, pre_release, and identifier are not 100% spec compliant.
fn build<'a>(input: &'a str) -> IResult<&'a str, Vec<Identifier>, SemverParseError<&'a str>> {
    context(
        "build version",
        preceded(tag("+"), separated_list1(tag("."), identifier)),
    )(input)
}

fn pre_release<'a>(input: &'a str) -> IResult<&'a str, Vec<Identifier>, SemverParseError<&'a str>> {
    context(
        "pre_release version",
        preceded(tag("-"), separated_list1(tag("."), identifier)),
    )(input)
}

fn identifier<'a>(input: &'a str) -> IResult<&'a str, Identifier, SemverParseError<&'a str>> {
    context(
        "identifier",
        map(
            take_while(|x: char| is_alphanumeric(x as u8) || x == '-'),
            |s: &str| {
                str::parse::<u64>(s)
                    .map(Identifier::Numeric)
                    .unwrap_or_else(|_err| Identifier::AlphaNumeric(s.to_string()))
            },
        ),
    )(input)
}

pub(crate) fn number<'a>(input: &'a str) -> IResult<&'a str, u64, SemverParseError<&'a str>> {
    context(
        "number component",
        map_res(recognize(digit1), |raw| {
            let value = str::parse(raw).map_err(|e| SemverParseError {
                input,
                context: None,
                kind: Some(SemverErrorKind::ParseIntError(e)),
            })?;

            if value > MAX_SAFE_INTEGER {
                return Err(SemverParseError {
                    input,
                    context: None,
                    kind: Some(SemverErrorKind::MaxIntError(value)),
                });
            }

            Ok(value)
        }),
    )(input)
}

#[cfg(test)]
mod tests {
    use super::Identifier::*;
    use super::*;

    use pretty_assertions::assert_eq;
    use serde_derive::{Deserialize, Serialize};

    #[test]
    fn trivial_version_number() {
        let v = Version::parse("1.2.34").unwrap();

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
        let v = Version::parse("1.2.34+123.456").unwrap();

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
        let v = Version::parse("1.2.34-abc.123").unwrap();

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
        let v = Version::parse("1.2.34-abc.123+1").unwrap();

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

    #[test]
    fn pre_release_that_could_look_numeric_at_first() {
        let v = Version::parse("1.0.0-rc.2-migration").unwrap();

        assert_eq!(
            v,
            Version {
                major: 1,
                minor: 0,
                patch: 0,
                pre_release: vec![
                    Identifier::AlphaNumeric("rc".into()),
                    Identifier::AlphaNumeric("2-migration".into())
                ],
                build: vec![],
            }
        );
    }

    #[test]
    fn comparison_with_different_major_version() {
        let lesser_version = Version {
            major: 1,
            minor: 2,
            patch: 34,
            pre_release: vec![AlphaNumeric("abc".into()), Numeric(123)],
            build: vec![],
        };
        let greater_version = Version {
            major: 2,
            minor: 2,
            patch: 34,
            pre_release: vec![AlphaNumeric("abc".into()), Numeric(123)],
            build: vec![],
        };
        assert_eq!(lesser_version.cmp(&greater_version), Ordering::Less);
        assert_eq!(greater_version.cmp(&lesser_version), Ordering::Greater);
    }
    #[test]
    fn comparison_with_different_minor_version() {
        let lesser_version = Version {
            major: 1,
            minor: 2,
            patch: 34,
            pre_release: vec![AlphaNumeric("abc".into()), Numeric(123)],
            build: vec![],
        };
        let greater_version = Version {
            major: 1,
            minor: 3,
            patch: 34,
            pre_release: vec![AlphaNumeric("abc".into()), Numeric(123)],
            build: vec![],
        };
        assert_eq!(lesser_version.cmp(&greater_version), Ordering::Less);
        assert_eq!(greater_version.cmp(&lesser_version), Ordering::Greater);
    }

    #[test]
    fn comparison_with_different_patch_version() {
        let lesser_version = Version {
            major: 1,
            minor: 2,
            patch: 34,
            pre_release: vec![AlphaNumeric("abc".into()), Numeric(123)],
            build: vec![],
        };
        let greater_version = Version {
            major: 1,
            minor: 2,
            patch: 56,
            pre_release: vec![AlphaNumeric("abc".into()), Numeric(123)],
            build: vec![],
        };
        assert_eq!(lesser_version.cmp(&greater_version), Ordering::Less);
        assert_eq!(greater_version.cmp(&lesser_version), Ordering::Greater);
    }

    #[test]
    //confirms the comparison matches the pre-release comparison example in the SemVer spec.
    //ie checks that 1.0.0-alpha < 1.0.0-alpha.1 < 1.0.0-alpha.beta < 1.0.0-beta < 1.0.0-beta.2 < 1.0.0-beta.11 < 1.0.0-rc.1 < 1.0.0.
    //for simplicity just checks them in order. Assumes that the transitive property holds. So if a < b & b < c then a < c.
    fn comparison_with_different_pre_release_version() {
        let v1_alpha = Version {
            major: 1,
            minor: 0,
            patch: 0,
            pre_release: vec![AlphaNumeric("alpha".into())],
            build: vec![],
        };
        let v1_alpha1 = Version {
            major: 1,
            minor: 0,
            patch: 0,
            pre_release: vec![AlphaNumeric("alpha".into()), Numeric(1)],
            build: vec![],
        };
        assert_eq!(v1_alpha.cmp(&v1_alpha1), Ordering::Less);
        let v1_alpha_beta = Version {
            major: 1,
            minor: 0,
            patch: 0,
            pre_release: vec![AlphaNumeric("alpha".into()), AlphaNumeric("beta".into())],
            build: vec![],
        };
        assert_eq!(v1_alpha1.cmp(&v1_alpha_beta), Ordering::Less);
        let v1_beta = Version {
            major: 1,
            minor: 0,
            patch: 0,
            pre_release: vec![AlphaNumeric("beta".into())],
            build: vec![],
        };
        assert_eq!(v1_alpha_beta.cmp(&v1_beta), Ordering::Less);
        let v1_beta2 = Version {
            major: 1,
            minor: 0,
            patch: 0,
            pre_release: vec![AlphaNumeric("beta".into()), Numeric(2)],
            build: vec![],
        };
        assert_eq!(v1_beta.cmp(&v1_beta2), Ordering::Less);
        let v1_beta11 = Version {
            major: 1,
            minor: 0,
            patch: 0,
            pre_release: vec![AlphaNumeric("beta".into()), Numeric(11)],
            build: vec![],
        };
        assert_eq!(v1_beta2.cmp(&v1_beta11), Ordering::Less);
        let v1_rc1 = Version {
            major: 1,
            minor: 0,
            patch: 0,
            pre_release: vec![AlphaNumeric("rc".into()), Numeric(1)],
            build: vec![],
        };
        assert_eq!(v1_beta11.cmp(&v1_rc1), Ordering::Less);
        let v1 = Version {
            major: 1,
            minor: 0,
            patch: 0,
            pre_release: vec![],
            build: vec![],
        };
        assert_eq!(v1_rc1.cmp(&v1), Ordering::Less);
    }

    #[test]
    fn individual_version_component_has_an_upper_bound() {
        let out_of_range = MAX_SAFE_INTEGER + 1;
        let v = Version::parse(format!("1.2.{}", out_of_range));
        assert_eq!(v.err().expect("Parse should have failed.").to_string(), "Error parsing semver string. Integer component of semver string is larger than JavaScript's Number.MAX_SAFE_INTEGER: 900719925474100");
    }

    #[test]
    fn version_string_limited_to_256_characters() {
        let prebuild = (0..257).map(|_| "X").collect::<Vec<_>>().join("");
        let version_string = format!("1.1.1-{}", prebuild);
        let v = Version::parse(version_string.clone());

        assert_eq!(
            v.err().expect("Parse should have failed").to_string(),
            "Error parsing semver string. Semver string can't be longer than 256 characters."
        );

        let ok_version = version_string[0..255].to_string();
        let v = Version::parse(ok_version);
        assert!(v.is_ok());
    }

    #[derive(Serialize, Deserialize, Eq, PartialEq)]
    struct Versioned {
        version: Version,
    }

    #[test]
    fn read_version_from_string() {
        let v: Versioned = serde_json::from_str(r#"{"version":"1.2.34-abc.213+2"}"#).unwrap();

        assert_eq!(
            v.version,
            Version {
                major: 1,
                minor: 2,
                patch: 34,
                pre_release: vec![
                    Identifier::AlphaNumeric("abc".into()),
                    Identifier::Numeric(213)
                ],
                build: vec![Identifier::Numeric(2)],
            }
        );
    }

    #[test]
    fn serialize_a_version_to_string() {
        let output = serde_json::to_string(&Versioned {
            version: Version {
                major: 1,
                minor: 2,
                patch: 34,
                pre_release: vec![
                    Identifier::AlphaNumeric("abc".into()),
                    Identifier::Numeric(213),
                ],
                build: vec![Identifier::Numeric(2)],
            },
        })
        .unwrap();
        let expected: String = r#"{"version":"1.2.34-abc.213+2"}"#.into();

        assert_eq!(output, expected);
    }
}
