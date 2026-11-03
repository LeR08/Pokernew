# Fork notes

Vendored from https://github.com/b-inary/postflop-solver (commit in `UPSTREAM_COMMIT`),
AGPL-3.0-or-later, by Wataru Inariba. Upstream development is paused, so Local Poker
Suite owns this fork.

Changes from upstream:
- `Cargo.toml`: `publish = false`, `zstd` enabled by default (compressed solution cache).
- `Cargo.toml`: `bincode` pinned to `=2.0.0-rc.3` and `bincode_derive` too (2.0 final changed the `Decode` trait).
- `action_tree.rs`: explicit `&(*node).children` (newer compilers reject the implicit autoref through a raw pointer).
- `lib.rs`, `tests/*.rs`, `examples/*.rs`: `#![allow(warnings, clippy::all)]` — upstream code is kept as is, our lints apply to our crates.
