//! Board definition tests.
//!
//! Every refusal is asserted by its message, not merely by failing. A board
//! file is a contributor's first change to this project and the diagnostic is
//! the whole of their experience of it, so a wrong-but-refused board has to say
//! which field and why.

use super::*;

/// A definition that passes, to vary one field at a time from.
fn good() -> String {
    String::from(
        "[board]\n\
         name = \"esp32c6-generic\"\n\
         chip = \"esp32c6\"\n\
         target = \"riscv32imac-unknown-none-elf\"\n\
         description = \"a devkit\"\n\
         \n\
         [features]\n\
         enable = [\"render\"]\n\
         \n\
         [[outputs]]\n\
         id = \"strip0\"\n\
         kind = \"ws2812\"\n\
         pin = 8\n\
         max_pixels = 300\n\
         max_current_ma = 2000\n",
    )
}

fn refuse(text: &str) -> String {
    parse(text).expect_err("should be refused").message
}

#[test]
fn a_complete_definition_parses() {
    let b = parse(&good()).expect("should parse");
    assert_eq!(b.name, "esp32c6-generic");
    assert_eq!(b.chip, "esp32c6");
    assert_eq!(b.target, "riscv32imac-unknown-none-elf");
    assert_eq!(b.description, "a devkit");
    assert_eq!(b.features, ["render"]);
    assert_eq!(b.outputs.len(), 1);
    let o = &b.outputs[0];
    assert_eq!((o.id.as_str(), o.kind.as_str()), ("strip0", "ws2812"));
    assert_eq!((o.pin, o.max_pixels, o.max_current_ma), (8, 300, 2000));
}

#[test]
fn every_board_shipped_in_this_repo_parses() {
    // Discovered rather than listed, because "adding a board must not require
    // touching any `.rs` file" has to include this test. A new definition is
    // checked the moment it is added, and nobody has to remember to come here.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("boards");
    let mut seen = 0;
    for entry in std::fs::read_dir(&dir).expect("boards/ should exist") {
        let path = entry.expect("readable").path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("readable");
        let board = parse(&text)
            .unwrap_or_else(|e| panic!("{}: {e}", path.file_name().unwrap().to_string_lossy()));

        // The filename and the declared name have to agree, or the build matrix
        // and the published artefacts disagree about what a board is called.
        let stem = path.file_stem().expect("a name").to_string_lossy();
        assert_eq!(
            board.name,
            stem,
            "{}: `board.name` and the filename must match",
            path.display()
        );
        seen += 1;
    }
    assert!(
        seen > 0,
        "boards/ held no definitions; the example is missing"
    );
}

#[test]
fn a_description_is_optional() {
    let text = good().replace("description = \"a devkit\"\n", "");
    assert_eq!(parse(&text).expect("should parse").description, "");
}

#[test]
fn a_board_with_no_features_parses() {
    let text = good().replace("[features]\nenable = [\"render\"]\n", "");
    assert!(parse(&text).expect("should parse").features.is_empty());
}

// ---- the chip and its target ------------------------------------------------

#[test]
fn an_unknown_chip_is_refused_and_the_known_ones_offered() {
    let text = good().replace("esp32c6\"", "esp32h9\"");
    let e = refuse(&text);
    assert!(e.contains("esp32h9"), "{e}");
    assert!(
        e.contains("esp32c3"),
        "the message should offer the known chips: {e}"
    );
}

#[test]
fn a_target_that_does_not_match_the_chip_is_refused() {
    // The compiler's version of this is a linker error a contributor cannot
    // read, so it is caught on the way in.
    let text = good().replace(
        "riscv32imac-unknown-none-elf",
        "riscv32imc-unknown-none-elf",
    );
    let e = refuse(&text);
    assert!(
        e.contains("builds for `riscv32imac-unknown-none-elf`"),
        "{e}"
    );
}

#[test]
fn every_known_chip_has_a_target_and_they_are_distinct() {
    let mut targets = std::collections::BTreeSet::new();
    for (chip, target) in CHIPS {
        assert!(!chip.is_empty() && !target.is_empty());
        assert!(targets.insert(*target), "{chip} shares a target");
    }
}

// ---- names ------------------------------------------------------------------

#[test]
fn a_name_that_is_not_kebab_case_is_refused() {
    // It becomes a filename, a build-matrix entry and a published artefact.
    for bad_name in ["My Lamp", "my_lamp", "MyLamp", "my.lamp"] {
        let text = good().replace("esp32c6-generic", bad_name);
        let e = refuse(&text);
        assert!(
            e.contains("lowercase letters, digits and dashes"),
            "{bad_name}: {e}"
        );
    }
}

#[test]
fn a_missing_board_table_is_refused_by_name() {
    let e = refuse("[features]\nenable = [\"render\"]\n");
    assert!(e.contains("no `[board]` table"), "{e}");
}

#[test]
fn a_missing_required_field_names_the_field() {
    for (line, field) in [
        ("name = \"esp32c6-generic\"\n", "board.name"),
        ("chip = \"esp32c6\"\n", "board.chip"),
        (
            "target = \"riscv32imac-unknown-none-elf\"\n",
            "board.target",
        ),
    ] {
        let text = good().replace(line, "");
        let e = refuse(&text);
        assert!(e.contains(field) && e.contains("missing"), "{field}: {e}");
    }
}

// ---- features ---------------------------------------------------------------

#[test]
fn an_unknown_feature_is_refused_and_the_real_ones_offered() {
    let text = good().replace("\"render\"", "\"rendering\"");
    let e = refuse(&text);
    assert!(e.contains("rendering"), "{e}");
    assert!(
        e.contains("audio"),
        "the message should list the options: {e}"
    );
}

#[test]
fn a_feature_named_twice_is_refused() {
    let text = good().replace("[\"render\"]", "[\"render\", \"render\"]");
    assert!(refuse(&text).contains("twice"));
}

#[test]
fn an_empty_features_table_is_refused_rather_than_ignored() {
    // Silence here would read as "no features", which may be what was meant or
    // may be a deleted line. Saying so costs nothing.
    let text = good().replace("enable = [\"render\"]\n", "");
    assert!(refuse(&text).contains("no `enable` list"));
}

#[test]
fn render_without_an_output_is_refused() {
    let text = good()
        .replace("\n[[outputs]]\nid = \"strip0\"\nkind = \"ws2812\"\npin = 8\nmax_pixels = 300\nmax_current_ma = 2000\n", "\n[identify]\nbutton = true\n");
    let e = refuse(&text);
    assert!(
        e.contains("`render` is enabled but the board declares no outputs"),
        "{e}"
    );
}

// ---- outputs ----------------------------------------------------------------

#[test]
fn two_outputs_on_one_pin_are_refused() {
    let text = good() + "\n[[outputs]]\nid = \"strip1\"\nkind = \"ws2812\"\npin = 8\nmax_pixels = 60\nmax_current_ma = 500\n";
    assert!(refuse(&text).contains("pin 8"));
}

#[test]
fn two_outputs_sharing_an_id_are_refused() {
    let text = good() + "\n[[outputs]]\nid = \"strip0\"\nkind = \"ws2812\"\npin = 9\nmax_pixels = 60\nmax_current_ma = 500\n";
    assert!(refuse(&text).contains("share the id `strip0`"));
}

#[test]
fn an_output_kind_this_firmware_cannot_drive_is_refused() {
    let text = good().replace("ws2812", "ws2811b");
    let e = refuse(&text);
    assert!(e.contains("ws2811b") && e.contains("sk6812"), "{e}");
}

#[test]
fn a_pin_that_is_not_a_gpio_number_is_refused() {
    for pin in ["-1", "300"] {
        let text = good().replace("pin = 8", &format!("pin = {pin}"));
        assert!(refuse(&text).contains("not a GPIO number"), "pin {pin}");
    }
}

#[test]
fn an_output_with_no_pixels_is_refused() {
    let text = good().replace("max_pixels = 300", "max_pixels = 0");
    assert!(refuse(&text).contains("not an output"));
}

#[test]
fn an_output_with_more_pixels_than_a_segment_can_address_is_refused() {
    let text = good().replace("max_pixels = 300", "max_pixels = 70000");
    assert!(refuse(&text).contains("more than a segment can address"));
}

#[test]
fn an_output_without_a_real_current_limit_is_refused() {
    // The runtime derates against this figure. A board that declared none would
    // be allowed to pull whatever the strip asks for.
    for value in ["0", "-100"] {
        let text = good().replace(
            "max_current_ma = 2000",
            &format!("max_current_ma = {value}"),
        );
        let e = refuse(&text);
        assert!(e.contains("derates against this figure"), "{value}: {e}");
    }
    let text = good().replace("max_current_ma = 2000\n", "");
    assert!(refuse(&text).contains("max_current_ma"));
}

#[test]
fn a_board_may_declare_several_outputs() {
    let text = good() + "\n[[outputs]]\nid = \"strip1\"\nkind = \"sk6812\"\npin = 9\nmax_pixels = 60\nmax_current_ma = 500\n";
    let b = parse(&text).expect("should parse");
    assert_eq!(b.outputs.len(), 2);
    assert_eq!(b.outputs[1].id, "strip1");
}

// ---- the pairing rule -------------------------------------------------------

#[test]
fn a_board_with_no_way_to_confirm_a_pairing_is_refused() {
    // The rule that is not about convenience. Pairing is confirmed at the
    // device precisely so that reaching the network is not enough to join the
    // mesh, and hardware that cannot do it would fail at somebody's desk with
    // nothing to explain why.
    let text = "[board]\nname = \"headless\"\nchip = \"esp32c3\"\ntarget = \"riscv32imc-unknown-none-elf\"\n";
    let e = refuse(text);
    assert!(e.contains("confirm a pairing at the device"), "{e}");
    assert!(
        e.contains("qr = true"),
        "the message must offer the way out: {e}"
    );
}

#[test]
fn any_one_way_to_confirm_a_pairing_is_enough() {
    let base = "[board]\nname = \"headless\"\nchip = \"esp32c3\"\ntarget = \"riscv32imc-unknown-none-elf\"\n";
    for line in ["button = true", "indicator = true", "qr = true"] {
        let text = format!("{base}\n[identify]\n{line}\n");
        assert!(parse(&text).is_ok(), "{line} should be enough");
    }
}

#[test]
fn an_led_output_is_itself_a_way_to_confirm_a_pairing() {
    // The device can blink the strip, so a board with an output needs no
    // separate indicator. Requiring one anyway would refuse most real hardware.
    let b = parse(&good()).expect("should parse");
    assert_eq!(b.identify, Identify::default(), "it declares none");
    assert!(b.identify.is_confirmable(&b.outputs));
}

#[test]
fn declaring_identify_false_everywhere_is_the_same_as_declaring_nothing() {
    let text = "[board]\nname = \"headless\"\nchip = \"esp32c3\"\ntarget = \"riscv32imc-unknown-none-elf\"\n\n[identify]\nbutton = false\nindicator = false\nqr = false\n";
    assert!(refuse(text).contains("confirm a pairing at the device"));
}

// ---- diagnostics ------------------------------------------------------------

#[test]
fn a_syntax_error_is_reported_with_its_line() {
    let text = good().replace("chip = \"esp32c6\"", "chip");
    let e = refuse(&text);
    assert!(e.starts_with("line "), "a parse error keeps its line: {e}");
}

#[test]
fn a_field_of_the_wrong_type_says_which_type_it_wants() {
    let text = good().replace("pin = 8", "pin = \"eight\"");
    assert!(refuse(&text).contains("is a number"));
    let text = good().replace("name = \"esp32c6-generic\"", "name = 5");
    assert!(refuse(&text).contains("is a string"));
}

#[test]
fn an_empty_name_is_refused_before_the_character_check() {
    // `name = ""` passes the kebab-case test vacuously, so it needs its own
    // refusal or a board could ship with no name at all.
    let text = good().replace("esp32c6-generic", "");
    assert!(refuse(&text).contains("is empty"));
}

#[test]
fn features_enable_must_be_a_list_not_a_single_name() {
    // The natural mistake, and one that would otherwise be read as "no
    // features" — a build variant quietly missing the capability it was for.
    let text = good().replace("enable = [\"render\"]", "enable = \"render\"");
    assert!(refuse(&text).contains("is a list of feature names"));
}

#[test]
fn a_refusal_prints_as_its_message() {
    // It is shown to a contributor as-is, so `Display` has to be the message
    // and not a wrapper around it.
    let e = parse(
        "[features]
enable = []
",
    )
    .expect_err("refused");
    assert_eq!(e.to_string(), e.message);
    assert!(e.to_string().contains("no `[board]` table"));
}
