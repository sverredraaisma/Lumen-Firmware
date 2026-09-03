# Board definitions

**This is the intended first contribution.** Adding support for your own board
is a single file here — no understanding of the internals required, and a
genuinely useful change.

One TOML file per board, describing the chip, the pins, the LED outputs, the
power limits and which Cargo features the build variant enables. Pins and LED
counts are *runtime* configuration; features are build-time. CI builds every
published board and feature variant, and the prebuilt matrix is what means most
users never compile anything.

Prefer RISC-V parts (C3, C6) where the choice is free: they use upstream Rust
with no forked toolchain, which removes a whole class of setup friction — and
contributor friction is a tax an open-source project pays forever.

See `esp32c6-generic.toml` and copy it.

The filename has to match `board.name`: the name becomes a build-matrix entry
and a published artefact, and the two disagreeing would leave nobody able to say
what a board is called.

Every definition here is parsed and checked when `cargo test` runs, discovered
from this directory rather than listed anywhere — so a new file is validated the
moment it is added and no `.rs` file needs touching. If yours is refused, the
message names the field and says what the rule is for.

One rule is worth knowing before you write the file. A board that declares no
LED output, no button, no indicator and no printed code is **refused**, because
nobody could confirm a pairing at the device — and pairing is confirmed
physically on purpose, so that reaching the network is not enough to join a
mesh. A board with an LED output needs nothing extra: it can blink the strip.
