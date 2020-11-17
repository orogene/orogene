use nom::error::{ContextError, ErrorKind, FromExternalError, ParseError};
use oro_diagnostics::{Diagnostic, DiagnosticCategory, Explain, Meta};
use oro_node_semver::SemverError;
use thiserror::Error;
use url::ParseError as UrlParseError;

#[derive(Debug, Error)]
#[error("Error parsing package spec. {kind}")]
pub struct PackageSpecError {
    pub input: String,
    pub offset: usize,
    pub kind: SpecErrorKind,
}

impl PackageSpecError {
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

#[derive(Debug, Error)]
pub enum SpecErrorKind {
    #[error("Found invalid characters: `{0}`")]
    InvalidCharacters(String),
    #[error("Drive letters on Windows can only be alphabetical. Got `{0}`.")]
    InvalidDriveLetter(char),
    #[error("Invalid git host `{0}`. Only github:, gitlab:, gist:, and bitbucket: are supported in shorthands.")]
    InvalidGitHost(String),
    #[error(transparent)]
    SemverParseError(SemverError),
    #[error(transparent)]
    UrlParseError(UrlParseError),
    #[error(transparent)]
    GitHostParseError(Box<PackageSpecError>),
    #[error("Failed to parse {0} component of semver string.")]
    Context(&'static str),
    #[error("Incomplete input to semver parser.")]
    IncompleteInput,
    #[error("An unspecified error occurred.")]
    Other,
}

impl Explain for PackageSpecError {
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

impl Diagnostic for PackageSpecError {
    fn category(&self) -> DiagnosticCategory {
        DiagnosticCategory::Parse
    }

    fn label(&self) -> String {
        // TODO: add more detail
        "package_spec::no_parse".into()
    }

    fn advice(&self) -> Option<String> {
        // TODO: please fix this
        Some("Please fix your spec. Go look up wherever they're documented idk.".into())
    }
}

#[derive(Debug)]
pub(crate) struct SpecParseError<I> {
    pub(crate) input: I,
    pub(crate) context: Option<&'static str>,
    pub(crate) kind: Option<SpecErrorKind>,
}

impl<I> ParseError<I> for SpecParseError<I> {
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

impl<I> ContextError<I> for SpecParseError<I> {
    fn add_context(_input: I, ctx: &'static str, mut other: Self) -> Self {
        other.context = Some(ctx);
        other
    }
}

// There's a few parsers that just... manually return SpecParseError in a
// map_res, so this absurd thing is actually needed. Curious? Just comment it
// out and look at all the red.
impl<'a> FromExternalError<&'a str, SpecParseError<&'a str>> for SpecParseError<&'a str> {
    fn from_external_error(_input: &'a str, _kind: ErrorKind, e: SpecParseError<&'a str>) -> Self {
        e
    }
}

impl<'a> FromExternalError<&'a str, SemverError> for SpecParseError<&'a str> {
    fn from_external_error(input: &'a str, _kind: ErrorKind, e: SemverError) -> Self {
        SpecParseError {
            input,
            context: None,
            kind: Some(SpecErrorKind::SemverParseError(e)),
        }
    }
}

impl<'a> FromExternalError<&'a str, UrlParseError> for SpecParseError<&'a str> {
    fn from_external_error(input: &'a str, _kind: ErrorKind, e: UrlParseError) -> Self {
        SpecParseError {
            input,
            context: None,
            kind: Some(SpecErrorKind::UrlParseError(e)),
        }
    }
}

impl<'a> FromExternalError<&'a str, PackageSpecError> for SpecParseError<&'a str> {
    fn from_external_error(input: &'a str, _kind: ErrorKind, e: PackageSpecError) -> Self {
        SpecParseError {
            input,
            context: None,
            kind: Some(SpecErrorKind::GitHostParseError(Box::new(e))),
        }
    }
}
