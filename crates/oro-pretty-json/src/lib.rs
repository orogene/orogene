//! Utility for pretty printing JSON while preserving the order of keys and
//! the original indentation and line endings from a JSON source.

use serde_json::{Error, Value};

#[derive(Debug, PartialEq, Eq)]
pub struct Formatted {
    pub value: Value,
    pub character: char,
    pub count: usize,
    pub line_end: String,
    pub trailing_line_end: bool,
}

pub fn from_str(json: impl AsRef<str>) -> Result<Formatted, Error> {
    let json = json.as_ref();
    let value = serde_json::from_str(json)?;
    let (character, count) = detect_indentation(json).unwrap_or((' ', 2));
    let (line_end, trailing_line_end) = detect_line_end(json).unwrap_or(("\n".into(), false));
    Ok(Formatted {
        value,
        character,
        count,
        line_end,
        trailing_line_end,
    })
}

pub fn to_string_pretty(formatted: &Formatted) -> Result<String, Error> {
    let json = serde_json::to_string_pretty(&formatted.value)?;
    let mut ret = String::new();
    let mut past_first_line = false;
    for line in json.lines() {
        if past_first_line {
            ret.push_str(&formatted.line_end);
        } else {
            past_first_line = true;
        }
        let indent_chars = line.find(|c: char| !is_json_whitespace(c)).unwrap_or(0);
        ret.push_str(
            &formatted
                .character
                .to_string()
                .repeat(formatted.count * (indent_chars / 2)),
        );
        ret.push_str(&line[indent_chars..]);
    }
    if formatted.trailing_line_end {
        ret.push_str(&formatted.line_end);
    }
    Ok(ret)
}

fn detect_indentation(json: &str) -> Option<(char, usize)> {
    let mut lines = json.lines();
    lines.next()?;
    let second_line = lines.next()?;
    let mut indent = 0;
    let mut character = None;
    let mut last_whitespace_char = None;
    for c in second_line.chars() {
        if is_json_whitespace(c) {
            indent += 1;
            last_whitespace_char = Some(c);
        } else {
            character = last_whitespace_char;
            break;
        }
    }
    character.map(|c| (c, indent))
}

fn detect_line_end(json: &str) -> Option<(String, bool)> {
    json.find(['\r', '\n'])
        .map(|idx| {
            let c = json
                .get(idx..idx + 1)
                .expect("we already know there's a char there");
            if c == "\r" && json.get(idx..idx + 2) == Some("\r\n") {
                return "\r\n".into();
            }
            c.into()
        })
        .map(|end| (end, matches!(json.chars().last(), Some('\n' | '\r'))))
}

fn is_json_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}

#[cfg(test)]
mod tests {
    use super::Formatted;

    #[test]
    fn basic() -> Result<(), serde_json::Error> {
        let json = "{\n      \"a\": 1,\n      \"b\": 2\n}";
        let ind = super::from_str(json)?;

        assert_eq!(
            ind,
            Formatted {
                value: serde_json::json!({
                    "a": 1,
                    "b": 2
                }),
                character: ' ',
                count: 6,
                line_end: "\n".into(),
                trailing_line_end: false,
            }
        );

        assert_eq!(super::to_string_pretty(&ind)?, json);

        let json = "{\r\n\t\"a\": 1,\r\n\t\"b\": 2\r\n}\r\n";
        let ind = super::from_str(json)?;

        assert_eq!(
            ind,
            Formatted {
                value: serde_json::json!({
                    "a": 1,
                    "b": 2
                }),
                character: '\t',
                count: 1,
                line_end: "\r\n".into(),
                trailing_line_end: true,
            }
        );

        Ok(())
    }
}
