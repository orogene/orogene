use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};

use crate::error::PackageSpecError;

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

pub fn no_url_encode(tag: &str) -> Result<&str, PackageSpecError> {
    if format!("{}", utf8_percent_encode(tag, JS_ENCODED)) == tag {
        Ok(tag)
    } else {
        Err(PackageSpecError::InvalidCharacters(tag.into()))
    }
}
