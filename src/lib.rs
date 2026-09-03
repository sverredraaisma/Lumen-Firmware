//! The host-testable half of the firmware shell.
//!
//! The binary in `main.rs` owns the sockets, the timers, the flash and the LED
//! peripheral, and none of that can run on a build machine. Everything that
//! *can* lives here, so it is covered by ordinary tests rather than by plugging
//! a board in.
//!
//! Keeping `main` to a single line is the rule that makes that true. Logic that
//! lands in `main` is logic no test will ever reach, and on a device shell that
//! is exactly the code that strands a room in the dark.

pub mod boards;
mod toml;

/// The build's version, as published in `CAPS` and printed on start-up.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The optional capabilities this binary was compiled with.
///
/// Cargo features decide what a board image can do, and the device advertises
/// the result to the mesh. A build that renders but reports otherwise is not a
/// cosmetic bug: peers route work by what a device claims, so an image whose
/// claims and abilities disagree gets handed jobs it silently cannot perform.
///
/// The order is fixed and alphabetical, so two builds of the same image produce
/// the same string and banners diff cleanly across firmware revisions.
pub fn capabilities() -> &'static [&'static str] {
    &[
        #[cfg(feature = "audio")]
        "audio",
        #[cfg(feature = "compile")]
        "compile",
        #[cfg(feature = "render")]
        "render",
        #[cfg(feature = "zigbee")]
        "zigbee",
    ]
}

/// The line the binary prints on start-up.
///
/// It is the only thing a person sees when they run a host build, so it says
/// what the build is and what it can do rather than merely that it started.
pub fn banner() -> String {
    banner_for(version(), capabilities())
}

/// `banner` with its two inputs supplied.
///
/// Split out because the capability list is decided by Cargo features, so the
/// assembled banner is only ever exercised for whichever feature set the test
/// run happened to be compiled with. A pure function takes the combination as
/// an argument and can therefore be checked for all of them at once — including
/// the empty case, which is what a default build actually ships.
fn banner_for(version: &str, caps: &[&str]) -> String {
    let caps = if caps.is_empty() {
        "none".to_string()
    } else {
        caps.join(",")
    };
    format!("lumen-firmware {version} — host stub; capabilities: {caps}; no HAL implementation yet")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every flag this crate may ever report, in the order `capabilities` must
    /// return them.
    const KNOWN: [&str; 4] = ["audio", "compile", "render", "zigbee"];

    #[test]
    fn version_is_the_package_version() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
        // A version that does not parse as three dotted fields would break the
        // OTA compatibility check that lands with W9.
        assert_eq!(version().split('.').count(), 3, "version must be x.y.z");
    }

    #[test]
    fn capabilities_are_the_known_flags_in_canonical_order() {
        // Filtering the canonical list by what is enabled reproduces exactly
        // what `capabilities` should have returned, so one comparison pins
        // membership, ordering and uniqueness together. It also does the same
        // work whichever features are on, which matters because a test whose
        // body is skipped in the default build is a test that never runs.
        let expected: Vec<&str> = KNOWN
            .iter()
            .copied()
            .filter(|k| capabilities().contains(k))
            .collect();
        assert_eq!(
            capabilities(),
            &expected[..],
            "capabilities must be the known flags, in canonical order, without duplicates"
        );
    }

    #[test]
    fn capabilities_reflect_the_enabled_features() {
        let caps = capabilities();
        assert_eq!(caps.contains(&"audio"), cfg!(feature = "audio"));
        assert_eq!(caps.contains(&"compile"), cfg!(feature = "compile"));
        assert_eq!(caps.contains(&"render"), cfg!(feature = "render"));
        assert_eq!(caps.contains(&"zigbee"), cfg!(feature = "zigbee"));
    }

    #[test]
    fn banner_lists_every_capability_it_is_given() {
        assert_eq!(
            banner_for("9.9.9", &["audio", "render"]),
            "lumen-firmware 9.9.9 — host stub; capabilities: audio,render; no HAL implementation yet"
        );
    }

    #[test]
    fn banner_says_none_rather_than_leaving_a_gap() {
        // The empty case is what a default build ships, and "capabilities: ;"
        // reads like a truncated line rather than a deliberate answer.
        assert_eq!(
            banner_for("9.9.9", &[]),
            "lumen-firmware 9.9.9 — host stub; capabilities: none; no HAL implementation yet"
        );
    }

    #[test]
    fn banner_uses_the_real_version_and_capabilities() {
        assert_eq!(banner(), banner_for(version(), capabilities()));
        assert!(banner().contains(version()), "banner: {}", banner());
        assert!(
            banner().contains("no HAL implementation yet"),
            "the stub must say it is a stub: {}",
            banner()
        );
    }

    #[test]
    fn banner_is_a_single_line() {
        // It is printed to the serial console on boot, where a stray newline
        // costs a line of a short scrollback.
        assert!(!banner().contains('\n'), "banner must not wrap");
    }
}
