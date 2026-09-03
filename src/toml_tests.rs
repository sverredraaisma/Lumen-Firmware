//! Tests for the board-definition parser.
//!
//! Weighted toward what it must refuse. A parser that quietly skipped a line it
//! could not read would let a board ship with an output nobody configured, and
//! that surfaces as one dark strip on somebody else's desk with nothing to look
//! at.

use super::*;

fn doc(text: &str) -> Document {
    parse(text).expect("should parse")
}

fn reject(text: &str) -> String {
    parse(text).expect_err("should be refused").message
}

#[test]
fn a_table_of_scalars_round_trips() {
    let d =
        doc("[board]\nname = \"a\"\npin = 8\nready = true\nfeatures = [\"render\", \"audio\"]\n");
    let t = d.table("board").expect("board");
    assert_eq!(t.get("name").and_then(Value::as_str), Some("a"));
    assert_eq!(t.get("pin").and_then(Value::as_int), Some(8));
    assert_eq!(t.get("ready").and_then(Value::as_bool), Some(true));
    assert_eq!(
        t.get("features").and_then(Value::as_list),
        Some(&["render".to_string(), "audio".to_string()][..])
    );
}

#[test]
fn arrays_of_tables_keep_their_order() {
    // Outputs are addressed by position in error messages, so the order they
    // were written in has to survive.
    let d = doc("[[outputs]]\nid = \"a\"\n\n[[outputs]]\nid = \"b\"\n\n[[outputs]]\nid = \"c\"\n");
    let ids: Vec<&str> = d
        .array("outputs")
        .iter()
        .map(|t| t.get("id").and_then(Value::as_str).expect("id"))
        .collect();
    assert_eq!(ids, ["a", "b", "c"]);
}

#[test]
fn an_absent_array_is_empty_rather_than_missing() {
    // The validator asks for outputs before it knows whether there are any.
    assert!(doc("[board]\nname = \"a\"\n").array("outputs").is_empty());
}

#[test]
fn comments_and_blank_lines_are_ignored() {
    let d = doc("# a board\n\n[board]  # trailing\nname = \"a\" # and here\n\n");
    assert_eq!(
        d.table("board")
            .and_then(|t| t.get("name"))
            .and_then(Value::as_str),
        Some("a")
    );
}

#[test]
fn a_hash_inside_a_string_is_not_a_comment() {
    // Colours and descriptions contain them, and truncating the value there
    // would silently shorten a name.
    let d = doc("[board]\nname = \"rev#2\"\n");
    assert_eq!(
        d.table("board")
            .and_then(|t| t.get("name"))
            .and_then(Value::as_str),
        Some("rev#2")
    );
}

#[test]
fn a_negative_integer_parses() {
    // Not useful for a pin, but a validator complaining about a negative pin is
    // a better error than the parser refusing to read the file at all.
    let d = doc("[board]\npin = -1\n");
    assert_eq!(
        d.table("board")
            .and_then(|t| t.get("pin"))
            .and_then(Value::as_int),
        Some(-1)
    );
}

// ---- what it refuses -------------------------------------------------------

#[test]
fn a_key_outside_any_table_is_refused() {
    // A stray key would land somewhere nobody looks.
    assert!(reject("name = \"a\"\n").contains("outside any table"));
}

#[test]
fn a_duplicate_key_is_refused_rather_than_last_wins() {
    // Last-wins is how a board ends up with a pin the author did not intend,
    // having edited the wrong one of two lines.
    assert!(reject("[board]\npin = 1\npin = 2\n").contains("set twice"));
}

#[test]
fn a_duplicate_table_is_refused() {
    assert!(reject("[board]\nname = \"a\"\n[board]\npin = 1\n").contains("appears twice"));
}

#[test]
fn an_unterminated_header_is_refused() {
    assert!(reject("[board\nname = \"a\"\n").contains("ends with `]`"));
    assert!(reject("[[outputs\nid = \"a\"\n").contains("ends with `]]`"));
}

#[test]
fn an_unterminated_string_is_refused() {
    assert!(reject("[board]\nname = \"a\n").contains("unterminated string"));
}

#[test]
fn a_line_that_is_not_a_key_or_a_header_is_refused() {
    assert!(reject("[board]\nnonsense\n").contains("expected `key = value`"));
}

#[test]
fn a_value_this_parser_does_not_understand_is_refused_by_name() {
    // Floats, dates and inline tables are all real TOML and none of them mean
    // anything in a board definition. Refusing them by name is what stops
    // somebody assuming support that is not there.
    for text in [
        "[board]\nx = 2.5\n",
        "[board]\nx = 1979-05-27\n",
        "[board]\nx = {a=1}\n",
    ] {
        let e = reject(text);
        assert!(
            e.contains("is not a string, an integer, a boolean or a list"),
            "{text}: {e}"
        );
    }
}

#[test]
fn a_dotted_or_quoted_key_is_refused_by_name() {
    // Both are valid TOML that this parser does not implement, and half-reading
    // one would put the value under the wrong key.
    assert!(reject("[board]\na.b = 1\n").contains("not a bare name"));
    assert!(reject("[\"board\"]\na = 1\n").contains("not a bare name"));
}

#[test]
fn a_list_of_anything_but_strings_is_refused() {
    assert!(reject("[board]\nx = [1, 2]\n").contains("a list holds strings"));
}

#[test]
fn an_empty_entry_in_a_list_is_refused() {
    assert!(reject("[board]\nx = [\"a\", ]\n").contains("empty entry"));
}

#[test]
fn a_missing_value_is_refused() {
    assert!(reject("[board]\nname =\n").contains("value is missing"));
}

#[test]
fn an_error_names_the_line_it_was_on() {
    // A contributor's first change to this project is a board file, so the
    // error has to point at the line rather than at the file.
    let e = parse("[board]\nname = \"a\"\nbroken\n").expect_err("refused");
    assert_eq!(e.line, 3);
    assert!(e.to_string().starts_with("line 3: "), "{e}");
}

#[test]
fn asking_a_value_for_the_wrong_type_answers_none() {
    // The validator asks by type and reports "is a number" when the answer is
    // `None`. If a wrong-type accessor returned something, that diagnostic
    // would never fire and the field would be read as whatever it coerced to.
    let d = doc("[board]
s = \"x\"
i = 1
b = true
l = [\"a\"]
");
    let t = d.table("board").expect("board");
    let (s_, i_, b_, l_) = (
        t.get("s").expect("s"),
        t.get("i").expect("i"),
        t.get("b").expect("b"),
        t.get("l").expect("l"),
    );

    assert_eq!(s_.as_str(), Some("x"));
    assert_eq!(i_.as_str(), None);
    assert_eq!(b_.as_str(), None);
    assert_eq!(l_.as_str(), None);

    assert_eq!(i_.as_int(), Some(1));
    assert_eq!(s_.as_int(), None);
    assert_eq!(b_.as_int(), None);

    assert_eq!(b_.as_bool(), Some(true));
    assert_eq!(s_.as_bool(), None);
    assert_eq!(i_.as_bool(), None);

    assert_eq!(l_.as_list().map(|v| v.len()), Some(1));
    assert_eq!(s_.as_list(), None);
    assert_eq!(i_.as_list(), None);
}

#[test]
fn an_empty_list_parses_as_an_empty_list() {
    // Not as a missing key. `enable = []` is a board that deliberately turns
    // every optional capability off, which is a real thing to want.
    let d = doc("[features]
enable = []
");
    assert_eq!(
        d.table("features")
            .and_then(|t| t.get("enable"))
            .and_then(Value::as_list),
        Some(&[][..])
    );
}

#[test]
fn an_empty_table_or_key_name_is_refused() {
    assert!(reject(
        "[]
"
    )
    .contains("an empty name"));
    assert!(reject(
        "[board]
 = 1
"
    )
    .contains("an empty name"));
}

#[test]
fn an_escape_inside_a_string_is_refused_by_name() {
    // Not supported, and half-reading one would silently change a name. The
    // message says so rather than leaving somebody to wonder. Built with
    // `format!` because writing the quotes inline is unreadable.
    let quoted = "\"a\"b\"";
    let e = reject(&format!(
        "[board]
name = {quoted}
"
    ));
    assert!(e.contains("escapes are not supported"), "{e}");
}

#[test]
fn an_unterminated_list_is_refused() {
    assert!(reject(
        "[features]
enable = [\"render\"
"
    )
    .contains("unterminated list"));
}

#[test]
fn every_value_can_describe_its_own_type() {
    // The wrong-type diagnostics quote these, so a missing arm would read as an
    // empty phrase in the middle of a sentence.
    let d = doc("[board]
s = \"x\"
i = 1
b = true
l = [\"a\"]
");
    let t = d.table("board").expect("board");
    let mut kinds: Vec<&str> = ["s", "i", "b", "l"]
        .iter()
        .map(|k| t.get(*k).expect("present").kind())
        .collect();
    kinds.sort_unstable();
    assert_eq!(kinds, ["a boolean", "a list", "a string", "an integer"]);
}
