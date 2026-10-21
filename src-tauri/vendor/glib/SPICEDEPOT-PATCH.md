# GLib 0.18.5 security backport

Source: official crates.io glib 0.18.5, retaining upstream MIT license.
Fix: https://github.com/gtk-rs/gtk-rs-core/pull/1343
Advisory: RUSTSEC-2024-0429 / GHSA-wrw7-89jp-8q8g.

Only src/variant_iter.rs is changed: the C out-pointer is mutable and passed as &mut p. GTK3/Tauri requires this API family; glib 0.20 is not a drop-in replacement. Keep the original version; do not disguise this backport as an upstream release. Remove the override when the GTK dependency chain supports a patched upstream version. Linux runtime testing is still required.
