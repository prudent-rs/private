# private

This will be a set of macros to enable other crates to export macros (either declarative/by
example/using `macro_rules`, or procedural) with private-like variables or constants.

## Blockers and related issues

Please give thumbs up (and contribute, if you can) to

- [SergioBenitez/proc-macro2-diagnostics#13](https://github.com/SergioBenitez/proc-macro2-diagnostics/issues/13)
  defect: Error message and details missing, when macro fails to generate main() on STABLE

## NOT watt-compatible

NOT compatible with [dtolnay/watt](https://github.com/dtolnay/watt) (because of side effects of
build.rs).
