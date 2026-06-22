# restricted

Prevent consumers of your Rust macros from accidents. Enable your crates to export private-like types,
traits and functions that are to be used only by its own `macro_rules` (from the same crate).

## Blockers and related issues

Please give thumbs up (and contribute, if you can) to

- [SergioBenitez/proc-macro2-diagnostics#13](https://github.com/SergioBenitez/proc-macro2-diagnostics/issues/13)
  defect: Error message and details missing, when macro fails to generate main() on STABLE
- [SergioBenitez/proc-macro2-diagnostics#12](https://github.com/SergioBenitez/proc-macro2-diagnostics/issues/12)
  tests/stable/errors.rs fails with Rust 1.89.0 and also stable 1.95.0

## Requiring UTF-8

Source file paths have be in UTF-8.

## Normally non-ASCII Unicode-friendly

<!-- @TODO
`restricted` _does_ allow non-ASCII identifiers, as per [Rust RFC
2457](https://rust-lang.github.io/rfcs/2457-non-ascii-idents.html). You can have non-ASCII characters in either/both

- `path` right of `@` (optional; like `$crate` or `$crate::module::submodule` if used from a consumer/3rd party macro),
  and
- the `name` (given by you or the developer of the consumer/3rd party macro).
-->
## NOT watt-compatible

NOT compatible with [dtolnay/watt](https://github.com/dtolnay/watt) (because of side effects of
build.rs).
