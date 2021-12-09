# Ockam Scripts

## Changelog Generation

Changelogs are automatically generated using [git-cliff](https://github.com/orhun/git-cliff). To generate changelogs, we call the [changelog.sh script](https://github.com/ockam-network/ockam/blob/develop/tools/scripts/release/changelog.sh) which will generate and append to individual CHANGELOG.md all Ockam crates that have been updated, comparing them to the last `git tag`.
To run changelog generator, from the Ockam root path, call
```bash
tools/scripts/release/changelog.sh
```


## Crate Bump

Crates versions are bumped using [cargo-release](https://github.com/crate-ci/cargo-release/issues). While bumping crates CHANGELOG.md and README.md files are also updated with the latest bumped version.
To run crate bump, from the Ockam root path, call
```bash
RELEASE_VERSION=minor tools/scripts/release/crate-bump.sh
```
where RELEASE_VERSION is the [version](https://github.com/crate-ci/cargo-release/blob/master/docs/reference.md#bump-level) all crates to be bumped to.
Crates can also be bumped to a different level, ignoring `RELEASE_VERSION`. To bump a crate to a different level, we indicate crates and the bumped level in `MODIFIED_RELEASE`
```bash
MODIFIED_RELEASE="signature_core:patch ockam_entity:major" RELEASE_VERSION=minor tools/scripts/release/crate-bump.sh
```
this bumps `signature_core` as a `patch`, `ockam_entity` as `major` and every other crate as `minor`.

We can also release a `-dev` version of all crates right after a bump. To release crates,
```bash
DEV_VERSION=true tools/scripts/release/crate-bump.sh
```
This should be called before a new `git tag` is done so that only crates that are bumped and updated to a `-dev` version.


## Crate Publish

Crates are published to `crates.io` using [cargo-release](https://github.com/crate-ci/cargo-release/issues) right after bump. Only crates that have been updated (comparing `git diff` with last git tag) are published.
To publish crates
```bash
PUBLISH_TOKEN=my_crates.io_token tools/scripts/release/crate-publish.sh
```

## Tagging

We perform tag release using [gh cli](https://cli.github.com), a commit SHA is provided which all bumped crates are git tagged.
To perform `git tag`
```bash
COMMIT_SHA=000000000 tools/scripts/release/tagging.sh
```
