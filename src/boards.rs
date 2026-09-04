//! Board definitions: reading them, and refusing the ones that cannot work.
//!
//! A board definition is one checked-in file, and it is the intended first
//! contribution to this project — someone who has read nothing else here should
//! be able to add support for their hardware by copying a file and editing it.
//! Two things follow from that, and both are load-bearing.
//!
//! **Adding a board must not require touching a `.rs` file.** So this reads the
//! file rather than a table compiled into the binary. If supporting a board ever
//! needs code, that is the bug.
//!
//! **The diagnostics are the interface.** A contributor's first change is a
//! board file, and the second thing they see is whatever this says about it. So
//! every refusal names the field, the file's line where it can, and what the
//! rule is for — never just that something is wrong.
//!
//! # The rule that is not about convenience
//!
//! A board that declares no LED output, no button and no printed code has no way
//! for a person to physically confirm a pairing, which the security model
//! requires: pairing is confirmed at the device, precisely so that reaching the
//! network is not enough to join the mesh. Such a definition is refused here
//! rather than shipped, because the alternative is hardware that cannot be
//! paired safely and no way to tell until someone tries.

use std::collections::BTreeSet;
use std::fmt;

use crate::toml::{self, Value};

/// Chips this firmware knows how to build for, and the Rust target each needs.
///
/// RISC-V parts first and by preference: they build on upstream Rust, while
/// Xtensa needs a forked toolchain. Setup friction is a tax the project pays
/// forever, so the default examples are RISC-V.
const CHIPS: &[(&str, &str, u8)] = &[
    ("esp32c3", "riscv32imc-unknown-none-elf", 1),
    ("esp32c6", "riscv32imac-unknown-none-elf", 1),
    ("esp32s3", "xtensa-esp32s3-none-elf", 2),
];

/// Cargo features a board variant may enable.
///
/// Only genuinely optional capabilities are features: pins and LED counts are
/// runtime configuration, because every feature doubles a dimension of the CI
/// build matrix.
const FEATURES: &[&str] = &["audio", "compile", "render", "zigbee"];

/// LED protocols an output may declare.
const OUTPUT_KINDS: &[&str] = &["ws2812", "sk6812", "apa102", "pwm", "dmx"];

/// One LED output on a board.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Output {
    pub id: String,
    pub kind: String,
    pub pin: u8,
    pub max_pixels: u16,
    /// Milliamps at full white. The runtime derates against this rather than
    /// browning out the board, so a definition without it would let a strip
    /// pull more than the supply can give.
    pub max_current_ma: u32,
}

/// How a person can confirm a pairing at the device itself.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct Identify {
    /// A button they can press.
    pub button: bool,
    /// An indicator they can watch blink.
    pub indicator: bool,
    /// A code printed on the board.
    pub qr: bool,
}

impl Identify {
    /// Whether a person has any way to confirm a pairing physically.
    ///
    /// An LED output counts on its own: the device can blink the strip. That is
    /// why this takes the outputs rather than standing alone.
    pub fn is_confirmable(&self, outputs: &[Output]) -> bool {
        self.button || self.indicator || self.qr || !outputs.is_empty()
    }
}

/// A board definition that has been read and checked.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Board {
    pub name: String,
    pub chip: String,
    pub target: String,
    pub description: String,
    /// Cores this board renders on. See `render_cores` in `parse`.
    pub render_cores: u8,
    pub features: Vec<String>,
    pub outputs: Vec<Output>,
    pub identify: Identify,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Error {
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

fn bad<T>(message: impl Into<String>) -> Result<T, Error> {
    Err(Error {
        message: message.into(),
    })
}

/// A comma-separated list, for a diagnostic that offers the alternatives.
fn one_of(options: &[&str]) -> String {
    options.join(", ")
}

/// Read and check a board definition.
pub fn parse(text: &str) -> Result<Board, Error> {
    let doc = toml::parse(text).map_err(|e| Error {
        message: format!("{e}"),
    })?;

    let Some(board) = doc.table("board") else {
        return bad("no `[board]` table; every definition starts with one");
    };

    let name = require_str(board, "board", "name")?;
    if name.is_empty() {
        return bad("`board.name` is empty");
    }
    // Kebab-case, because the name becomes a filename, a build-matrix entry and
    // a published artefact. Deciding that once here is kinder than three
    // different opinions later.
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return bad(format!(
            "`board.name` is `{name}`; use lowercase letters, digits and dashes — it becomes a filename and a published artefact name"
        ));
    }

    let chip = require_str(board, "board", "chip")?;
    let Some((_, expected_target, chip_cores)) = CHIPS.iter().find(|(c, _, _)| *c == chip) else {
        return bad(format!(
            "`board.chip` is `{chip}`, which this firmware has no target for; known chips are {}",
            one_of(&CHIPS.iter().map(|(c, _, _)| *c).collect::<Vec<_>>())
        ));
    };
    let target = require_str(board, "board", "target")?;
    if target != *expected_target {
        // Caught here rather than by the compiler, because the compiler's
        // version of this is a linker error a contributor cannot read.
        return bad(format!(
            "`board.target` is `{target}` but `{chip}` builds for `{expected_target}`"
        ));
    }

    let description = board
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    // How many cores render. The pixels of a frame are independent, so a
    // dual-core chip can split the strip - `lumen_device::Shard` is the seam and
    // Spike S4 measured 1.97x on an S3 with the output byte-identical to one
    // core, which is the part that has to hold: a two-core device rendering a
    // different show from a one-core device would break the mesh's agreement
    // with itself.
    //
    // Defaults to the chip's core count, so a dual-core board gets the
    // speed-up without anyone remembering to ask. Set it to 1 on a board whose
    // second core has another job.
    let render_cores = match board.get("render_cores") {
        None => *chip_cores,
        Some(v) => {
            let Some(n) = v.as_int() else {
                return bad("`board.render_cores` is a number of cores");
            };
            if n < 1 {
                return bad(
                    "`board.render_cores` is less than 1; a device renders on at least one core",
                );
            }
            if n > *chip_cores as i64 {
                return bad(format!(
                    "`board.render_cores` is {n} but `{chip}` has {chip_cores} core(s)"
                ));
            }
            n as u8
        }
    };

    let mut features = Vec::new();
    if let Some(t) = doc.table("features") {
        let Some(list) = t.get("enable") else {
            return bad(
                "`[features]` is present but has no `enable` list; remove the table or give it one",
            );
        };
        let Some(list) = list.as_list() else {
            return bad("`features.enable` is a list of feature names");
        };
        for f in list {
            if !FEATURES.contains(&f.as_str()) {
                return bad(format!(
                    "`features.enable` names `{f}`, which is not a feature of this firmware; the options are {}",
                    one_of(FEATURES)
                ));
            }
            if features.contains(f) {
                return bad(format!("`features.enable` names `{f}` twice"));
            }
            features.push(f.clone());
        }
    }

    let mut outputs = Vec::new();
    let mut ids = BTreeSet::new();
    let mut pins = BTreeSet::new();
    for (i, t) in doc.array("outputs").iter().enumerate() {
        let where_ = format!("outputs[{i}]");
        let id = require_str(t, &where_, "id")?.to_string();
        if !ids.insert(id.clone()) {
            return bad(format!("two outputs share the id `{id}`"));
        }
        let kind = require_str(t, &where_, "kind")?.to_string();
        if !OUTPUT_KINDS.contains(&kind.as_str()) {
            return bad(format!(
                "output `{id}` is of kind `{kind}`, which this firmware cannot drive; the options are {}",
                one_of(OUTPUT_KINDS)
            ));
        }

        let pin = require_int(t, &where_, "pin")?;
        // GPIO numbers on these parts fit in a byte with room to spare, and a
        // negative pin is a typo rather than a choice.
        let pin: u8 = match u8::try_from(pin) {
            Ok(p) => p,
            Err(_) => {
                return bad(format!(
                    "output `{id}` has pin {pin}, which is not a GPIO number"
                ))
            }
        };
        if !pins.insert(pin) {
            return bad(format!(
                "two outputs are on pin {pin}; a pin drives one output"
            ));
        }

        let max_pixels = require_int(t, &where_, "max_pixels")?;
        if max_pixels <= 0 {
            return bad(format!(
                "output `{id}` declares {max_pixels} pixels; an output with none is not an output"
            ));
        }
        let max_pixels: u16 = match u16::try_from(max_pixels) {
            Ok(v) => v,
            Err(_) => {
                return bad(format!(
                    "output `{id}` declares {max_pixels} pixels, more than a segment can address"
                ))
            }
        };

        let max_current_ma = require_int(t, &where_, "max_current_ma")?;
        if max_current_ma <= 0 {
            // Absent or zero would mean "no limit", and the runtime derates
            // against this number. A board that declared none would be allowed
            // to pull whatever the strip asks for.
            return bad(format!(
                "output `{id}` declares {max_current_ma} mA; the runtime derates against this figure, so it has to be a real one"
            ));
        }

        outputs.push(Output {
            id,
            kind,
            pin,
            max_pixels,
            max_current_ma: max_current_ma as u32,
        });
    }

    let identify = match doc.table("identify") {
        Some(t) => Identify {
            button: t.get("button").and_then(Value::as_bool).unwrap_or(false),
            indicator: t.get("indicator").and_then(Value::as_bool).unwrap_or(false),
            qr: t.get("qr").and_then(Value::as_bool).unwrap_or(false),
        },
        None => Identify::default(),
    };

    if !identify.is_confirmable(&outputs) {
        return bad(
            "this board has no LED output, no button, no indicator and no printed code, so nobody can confirm a pairing at the device. \
             Pairing is confirmed physically on purpose: reaching the network must not be enough to join the mesh. \
             Give it `[identify] qr = true` if it carries a printed code, or a button or indicator if it has one",
        );
    }

    // Rendering with nothing to render on is a build that cannot do its job.
    if features.iter().any(|f| f == "render") && outputs.is_empty() {
        return bad(
            "`render` is enabled but the board declares no outputs; either add one or drop the feature",
        );
    }

    Ok(Board {
        name: name.to_string(),
        chip: chip.to_string(),
        target: target.to_string(),
        description,
        render_cores,
        features,
        outputs,
        identify,
    })
}

type Table = std::collections::BTreeMap<String, Value>;

fn require_str<'a>(t: &'a Table, where_: &str, key: &str) -> Result<&'a str, Error> {
    match t.get(key) {
        Some(Value::Str(s)) => Ok(s),
        Some(_) => bad(format!("`{where_}.{key}` is a string")),
        None => bad(format!("`{where_}.{key}` is missing")),
    }
}

fn require_int(t: &Table, where_: &str, key: &str) -> Result<i64, Error> {
    match t.get(key) {
        Some(v) => match v.as_int() {
            Some(i) => Ok(i),
            None => bad(format!("`{where_}.{key}` is a number")),
        },
        None => bad(format!("`{where_}.{key}` is missing")),
    }
}

#[cfg(test)]
#[path = "boards_tests.rs"]
mod tests;
