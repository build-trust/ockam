# Ockam Scripts

This folder contains scripts to release Ockam Rust crates. Note, to run these scripts you need to run Bash version 4 upwards. All commands should be called from the Ockam root path.
To perform release, release scripts automatically check for updated crates using `recently created git tags`, we can override the default setting if want to track updated crates with a more recent tag. To specify a `git tag`, we can define a variable `GIT_TAG` to any of the scripts. For example to generate changelog using a more recent `git tag` we can call the following command below
```bash
GIT_TAG="a_more_recent_git_tag_v0.0.0" tools/scripts/release/changelog.sh
```
This is same for crate bump, crate publish and tagging scripts.

## Crate Bump

Crates versions are bumped using [cargo-release](https://github.com/crate-ci/cargo-release/issues) >= v0.18.6. While bumping crates, CHANGELOG.md and README.md files are also updated with the bumped version.
To run crate bump, from the Ockam root path, call
```bash
RELEASE_VERSION=minor tools/scripts/release/crate-bump.sh
```
where RELEASE_VERSION is the [version](https://github.com/crate-ci/cargo-release/blob/master/docs/reference.md#bump-level) all crates are to be bumped to.

We can also indicate crates to follow a different release version, ignoring `RELEASE_VERSION`. To bump a crate to a different version, we indicate crates and the bumped version in `MODIFIED_RELEASE`
```bash
MODIFIED_RELEASE="signature_core:patch ockam_entity:major" RELEASE_VERSION=minor tools/scripts/release/crate-bump.sh
```
this bumps `signature_core` as a `patch`, `ockam_entity` as `major` and every other crate as `minor`.

If we indicate `RELEASE_VERSION` as a [release](https://github.com/crate-ci/cargo-release/blob/master/docs/reference.md#bump-level)
```bash
MODIFIED_RELEASE="signature_core:patch ockam_entity:major" RELEASE_VERSION=release tools/scripts/release/crate-bump.sh
```
only signature_core and ockam_entity crates are bumped.

Crates whose transitive dependencies were `only` bumped can be version-bumped with a specified version using the `BUMPED_DEP_CRATES_VERSION` definition so as to follow a different release version
```bash
BUMPED_DEP_CRATES_VERSION=patch RELEASE_VERSION=minor tools/scripts/release/crate-bump.sh
```
If `BUMPED_DEP_CRATES_VERSION` is not defined then transitive dependent crates are bumped as `minor`.

## Changelog Generation (Requires zsh)

Changelogs are generated using [git-cliff](https://github.com/orhun/git-cliff). To generate changelogs, we call the [changelog.sh script](https://github.com/build-trust/ockam/blob/develop/tools/scripts/release/changelog.sh) which will generate changelogs and append to their CHANGELOG.md file.
To run changelog generator, from the Ockam root path, call
```bash
tools/scripts/release/changelog.sh
```
Generated changelogs should be called after `crate-bump` so we can log crates whose dependencies was only bumped.
We can also generate changelog from a referenced `git tag`, changelog should be reviewed before commit.

## Crate Publish

Crates are published to `crates.io` using [cargo-release](https://github.com/crate-ci/cargo-release) right after bump. Only crates that have been updated (comparing `git diff` with last git tag) are published. Crates can also be excluded from being published using the `EXCLUDE_CRATES` variable, to exclude crates, we can optionally specify crates that are to be excluded `EXCLUDE_CRATES="signature_core ockam_core"`, where `signature_core` and `ockam_core` are excluded. Publish script can also be rerun after a recent fail, recently successfully published crates will automatically be detected and excluded. To indicate a script rerun we set the `RECENT_FAILURE` env to a `true`.

`RECENT_FAILURE="true"`.

To publish crates
```bash
PUBLISH_TOKEN=my_crates.io_token EXCLUDE_CRATES="signature_core ockam_core" tools/scripts/release/crate-publish.sh
```
Note: Require cargo-release >= version 0.18.6

## Tagging

We automate Git tagging and binary release over CI

## Manual Release

For a manual release to be done, we should

- Bump Crates
- Generate Changelogs
- Publish Crates
- Tag Crates

## CI Release

Ockam release can also be done over CI either manually using the provided workflows, or automatically using the `release.sh` script. Release consists of

- Crate Bump
- Crates IO Release
- Binary Release
- Homebrew Repo Bump
- Terraform Repo Bump
- Terraform Binary Release

To release, we call the script also indicating the Github username of the executor

```bash
GITHUB_USERNAME=metaclips release.sh
```

Indicating username ensures we only watch workflows that are created by the executor. The release script also allows for modifications provided by the `bump` and `publish` scripts, for example to create a release that uses a `RELEASE_VERSION` different from the default (minor)

```bash
RELEASE_VERSION=major GITHUB_USERNAME=metaclips release.sh
```

We can skip steps during a release by defining variable below as `true`
- SKIP_OCKAM_BUMP - Skips Ockam bump
- SKIP_CRATES_IO_PUBLISH - Skips crates.io publish
- SKIP_OCKAM_BINARY_RELEASE - Skips binary release
- SKIP_HOMEBREW_BUMP - Skips Homebrew version bump
- SKIP_TERRAFORM_BUMP - Skips Terraform version bump
- SKIP_TERRAFORM_BINARY_RELEASE - Skips Terraform binary release

To skip Ockam bump
```bash
SKIP_OCKAM_BUMP=true GITHUB_USERNAME=metaclips release.sh
```

The release script can be called from any path.
