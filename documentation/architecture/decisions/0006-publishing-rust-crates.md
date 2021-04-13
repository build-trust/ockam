# 6. Rust Crate Publishing Checklist

Date: 2021-04-06

## Status

Proposed

## Context

Publishing a crate is a multi-step process requiring a variety of changes. A publishing checklist helps maintain consistency and quality in releases.


## Decision

### Checklist

- [ ] **New crate?** - Crate has an entry in `directories` in `implementations/rust/build.gradle`
- [ ] Cargo.toml - Correct/Next release version according to semver.
- [ ] Cargo.toml - Categories and keywords are correct.
- [ ] Cargo.lock - Present and updated. (Running `cargo build` does not generate changes to `Cargo.lock`)
- [ ] Build - `cargo test` succeeds.
- [ ] Examples - Any examples in `examples` run via `cargo run --example $name`.
- [ ] Docs - `cargo doc --open` - Docs are correct, clear and useful.
- [ ] README - Version matches release version.
- [ ] README - Header is correct.
- [ ] README - Have any features changed? Verify any updates.
- [ ] CHANGELOG - Entry present for release version. Date is correct.
- [ ] CHANGELOG - Added section is correct.
- [ ] CHANGELOG - Changed section is correct.
- [ ] CHANGELOG - Deleted section is correct.
- [ ] Publish - `cargo publish --dry-run` succeeds.


Correctness of the Added, Changed and Deleted sections can be verified by running `git log $crate_name_v$current_vers..HEAD .` from the crate root directory. The commit
titles should all be accounted for in the CHANGELOG in some manner. Commits containing the `feat` or `fix` tag must be accounted for.

For example, `git log ockam_node_v0.5.0..HEAD .` will show the difference between that node release tag and the current develop branch, restricted to the crate directory.

## Consequences


