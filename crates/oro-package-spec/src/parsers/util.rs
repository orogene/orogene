use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

use crate::error::{SpecErrorKind, SpecParseError};

const JS_ENCODED: &AsciiSet = {
    &NON_ALPHANUMERIC
        .remove(b'-')
        .remove(b'_')
        .remove(b'.')
        .remove(b'!')
        .remove(b'~')
        .remove(b'*')
        .remove(b'\'')
        .remove(b'(')
        .remove(b')')
};

pub(crate) fn no_url_encode(tag: &str) -> Result<&str, SpecParseError<&str>> {
    if format!("{}", utf8_percent_encode(tag, JS_ENCODED)) == tag {
        Ok(tag)
    } else {
        Err(SpecParseError {
            input: tag,
            context: None,
            kind: Some(SpecErrorKind::InvalidCharacters(tag.into())),
        })
    }
}
