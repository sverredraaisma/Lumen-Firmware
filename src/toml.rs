//! Just enough TOML to read a board definition.
//!
//! Hand-written rather than a dependency, for the same reason the rest of this
//! repo has none: the device binary should pull in nothing it does not need, and
//! a board definition is a boring file by design — tables, arrays of tables,
//! strings, integers, booleans and arrays of strings. Nothing here supports
//! anything else, and it says so rather than guessing.
//!
//! Refusing what it does not understand is the whole point. A parser that
//! silently skipped a line it could not read would let a board ship with a pin
//! nobody configured, and the failure would appear as one dark output on
//! somebody else's desk.

use std::collections::BTreeMap;
use std::fmt;

/// A scalar. Board definitions have no use for anything richer.
#[derive(Clone, PartialEq, Debug)]
pub enum Value {
    Str(String),
    Int(i64),
    Bool(bool),
    List(Vec<String>),
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[String]> {
        match self {
            Value::List(v) => Some(v),
            _ => None,
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Value::Str(_) => "a string",
            Value::Int(_) => "an integer",
            Value::Bool(_) => "a boolean",
            Value::List(_) => "a list",
        }
    }
}

/// A parsed document: named tables, plus repeated tables in file order.
#[derive(Default, Debug)]
pub struct Document {
    tables: BTreeMap<String, BTreeMap<String, Value>>,
    arrays: BTreeMap<String, Vec<BTreeMap<String, Value>>>,
}

impl Document {
    pub fn table(&self, name: &str) -> Option<&BTreeMap<String, Value>> {
        self.tables.get(name)
    }

    /// Every `[[name]]` in the order they appeared.
    pub fn array(&self, name: &str) -> &[BTreeMap<String, Value>] {
        self.arrays.get(name).map(Vec::as_slice).unwrap_or(&[])
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

fn err<T>(line: usize, message: impl Into<String>) -> Result<T, ParseError> {
    Err(ParseError {
        line,
        message: message.into(),
    })
}

/// Parse a board definition.
pub fn parse(text: &str) -> Result<Document, ParseError> {
    let mut doc = Document::default();
    // Where the next `key = value` goes.
    let mut current: Option<(String, bool)> = None;

    for (n, raw) in text.lines().enumerate() {
        let line = n + 1;
        let s = strip_comment(raw).trim();
        if s.is_empty() {
            continue;
        }

        if let Some(rest) = s.strip_prefix("[[") {
            let name = match rest.strip_suffix("]]") {
                Some(v) => v.trim(),
                None => return err(line, "an array-of-table header ends with `]]`"),
            };
            check_name(line, name)?;
            doc.arrays
                .entry(name.to_string())
                .or_default()
                .push(BTreeMap::new());
            current = Some((name.to_string(), true));
            continue;
        }
        if let Some(rest) = s.strip_prefix('[') {
            let name = match rest.strip_suffix(']') {
                Some(v) => v.trim(),
                None => return err(line, "a table header ends with `]`"),
            };
            check_name(line, name)?;
            if doc.tables.contains_key(name) {
                return err(line, format!("`[{name}]` appears twice"));
            }
            doc.tables.insert(name.to_string(), BTreeMap::new());
            current = Some((name.to_string(), false));
            continue;
        }

        let Some((key, value)) = s.split_once('=') else {
            return err(
                line,
                "expected `key = value`, a `[table]` or an `[[array]]`",
            );
        };
        let key = key.trim();
        check_name(line, key)?;
        let value = parse_value(line, value.trim())?;

        let Some((table, is_array)) = current.as_ref() else {
            // A key before any header. Board definitions always open with
            // `[board]`, and accepting a stray key would put it somewhere
            // nobody looks.
            return err(line, format!("`{key}` is outside any table"));
        };
        let slot = if *is_array {
            doc.arrays
                .get_mut(table)
                .and_then(|v| v.last_mut())
                .expect("pushed when the header was read")
        } else {
            doc.tables.get_mut(table).expect("inserted with the header")
        };
        if slot.insert(key.to_string(), value).is_some() {
            return err(line, format!("`{key}` is set twice in `{table}`"));
        }
    }
    Ok(doc)
}

/// Everything after an unquoted `#`.
fn strip_comment(line: &str) -> &str {
    let mut in_quotes = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => in_quotes = !in_quotes,
            '#' if !in_quotes => return &line[..i],
            _ => {}
        }
    }
    line
}

/// Keys and table names are bare words. Anything else is a schema this parser
/// does not implement, and saying so beats accepting half of it.
fn check_name(line: usize, name: &str) -> Result<(), ParseError> {
    if name.is_empty() {
        return err(line, "an empty name");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return err(
            line,
            format!("`{name}` is not a bare name; quoted and dotted keys are not supported here"),
        );
    }
    Ok(())
}

fn parse_value(line: usize, text: &str) -> Result<Value, ParseError> {
    if text.is_empty() {
        return err(line, "a value is missing");
    }
    if let Some(rest) = text.strip_prefix('"') {
        let Some(body) = rest.strip_suffix('"') else {
            return err(line, "an unterminated string");
        };
        if body.contains('"') {
            return err(line, "escapes are not supported in a board definition");
        }
        return Ok(Value::Str(body.to_string()));
    }
    if text == "true" {
        return Ok(Value::Bool(true));
    }
    if text == "false" {
        return Ok(Value::Bool(false));
    }
    if let Some(rest) = text.strip_prefix('[') {
        let Some(body) = rest.strip_suffix(']') else {
            return err(line, "an unterminated list");
        };
        let body = body.trim();
        if body.is_empty() {
            return Ok(Value::List(Vec::new()));
        }
        let mut out = Vec::new();
        for part in body.split(',') {
            let part = part.trim();
            if part.is_empty() {
                return err(line, "an empty entry in a list");
            }
            match parse_value(line, part)? {
                Value::Str(s) => out.push(s),
                other => {
                    return err(
                        line,
                        format!("a list holds strings, found {}", other.kind()),
                    )
                }
            }
        }
        return Ok(Value::List(out));
    }
    // Integers only: a pin, a pixel count and a current limit are all whole
    // numbers, and accepting `2.5` for a pin would be accepting nonsense.
    match text.parse::<i64>() {
        Ok(i) => Ok(Value::Int(i)),
        Err(_) => err(
            line,
            format!("`{text}` is not a string, an integer, a boolean or a list"),
        ),
    }
}

#[cfg(test)]
#[path = "toml_tests.rs"]
mod tests;
