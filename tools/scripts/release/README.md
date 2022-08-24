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

We can also indicate crates to follow a different release version, ignoring `OCKAM_BUMP_RELEASE_VERSION`. To bump a crate to a different version, we indicate crates and the bumped version in `OCKAM_BUMP_MODIFIED_RELEASE`
```bash
OCKAM_BUMP_MODIFIED_RELEASE="signature_core:patch ockam_entity:major" OCKAM_BUMP_RELEASE_VERSION=minor tools/scripts/release/crate-bump.sh
```
this bumps `signature_core` as a `patch`, `ockam_entity` as `major` and every other crate as `minor`.

If we indicate `OCKAM_BUMP_RELEASE_VERSION` as a [release](https://github.com/crate-ci/cargo-release/blob/master/docs/reference.md#bump-level)
```bash
OCKAM_BUMP_MODIFIED_RELEASE="signature_core:patch ockam_entity:major" OCKAM_BUMP_RELEASE_VERSION=release tools/scripts/release/crate-bump.sh
```
only signature_core and ockam_entity crates are bumped.

Crates whose transitive dependencies were `only` bumped can be version-bumped with a specified version using the `OCKAM_BUMP_BUMPED_DEP_CRATES_VERSION` definition so as to follow a different release version
```bash
OCKAM_BUMP_BUMPED_DEP_CRATES_VERSION=patch RELEASE_VERSION=minor tools/scripts/release/crate-bump.sh
```
If `OCKAM_BUMP_BUMPED_DEP_CRATES_VERSION` is not defined then transitive dependent crates are bumped as `minor`.

## Changelog Generation (Requires zsh)

Changelogs are generated using [git-cliff](https://github.com/orhun/git-cliff). To generate changelogs, we call the [changelog.sh script](https://github.com/build-trust/ockam/blob/develop/tools/scripts/release/changelog.sh) which will generate changelogs and append to their CHANGELOG.md file.
To run changelog generator, from the Ockam root path, call
```bash
tools/scripts/release/changelog.sh
```
Generated changelogs should be called after `crate-bump` so we can log crates whose dependencies was only bumped.
We can also generate changelog from a referenced `git tag`, changelog should be reviewed before commit.

## Crate Publish

Crates are published to `crates.io` using [cargo-release](https://github.com/crate-ci/cargo-release) right after bump. Only crates that have been updated (comparing `git diff` with last git tag) are published. Crates can also be excluded from being published using the `OCKAM_PUBLISH_EXCLUDE_CRATES` variable, to exclude crates, we can optionally specify crates that are to be excluded `OCKAM_PUBLISH_EXCLUDE_CRATES="signature_core ockam_core"`, where `signature_core` and `ockam_core` are excluded. Publish script can also be rerun after a recent fail, recently successfully published crates will automatically be detected and excluded. To indicate a script rerun we set the `OCKAM_PUBLISH_RECENT_FAILURE` env to a `true`.

`OCKAM_PUBLISH_RECENT_FAILURE="true"`.

To publish crates
```bash
OCKAM_PUBLISH_PUBLISH_TOKEN=my_crates.io_token OCKAM_PUBLISH_EXCLUDE_CRATES="signature_core ockam_core" tools/scripts/release/crate-publish.sh
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

There are two steps to final release
- Draft release
- Production release

To create a release, we first create a draft, which will later on be reviewed and pull requests merged before running the script to create a final release.

To start the release in draft mode we call from the ockam home

```bash
IS_DRAFT_RELEASE=true ./tools/scripts/release/release.sh
```

On a successful run will,
- Bump Ockam crates and create a pull request in the /ockam repo
- Create Github release as draft with built binaries and NIFs
- Release Ockam docker image with a draft tag
- Bump Homebrew version and create a pull request for review in /homebrew-ockam repository
- Bump Terraform version and create a pull request for review in /terraform-provider-ockam repository
- Create Terraform draft release in /terraform-provider-ockam repository

After draft release is created, release is to be vetted and pull requests created in /ockam, /homebrew-ockam approved and merged before final release is started.

To start final release, from ockam home, call

```bash
IS_DRAFT_RELEASE=false ./tools/scripts/release/release.sh
```
This will
- Release Ockam docker image as latest
- Make Ockam Github release non-draft and latest
- Make Terraform Github release non-draft and latest
- Push our crates to crates.io

The release script also allows for modifications provided by the `bump` and `publish` scripts, for example to create a release that uses a `RELEASE_VERSION` different from the default (minor)

```bash
RELEASE_VERSION=major GITHUB_USERNAME=metaclips release.sh
```

We can skip steps during a release by defining variable below as `true`
- SKIP_OCKAM_BUMP - Skips Ockam bump
- SKIP_OCKAM_PACKAGE_RELEASE - Skips Ockam Docker package release
- SKIP_CRATES_IO_PUBLISH - Skips crates.io publish
- SKIP_OCKAM_BINARY_RELEASE - Skips binary release
- SKIP_HOMEBREW_BUMP - Skips Homebrew version bump
- SKIP_TERRAFORM_BUMP - Skips Terraform version bump
- SKIP_TERRAFORM_BINARY_RELEASE - Skips Terraform binary release

To skip Ockam bump
```bash
SKIP_OCKAM_BUMP=true ./tools/scripts/release/release.sh
```

The release script can be called from any path.

We also have a script to delete draft release, to delete draft

```bash
TAG_NAME=ockam_v0.71.0 ./delete_draft.sh
```

Where TAG_NAME is the tag of the draft release.

## Acceptance Test

After a release, we can test all generated assets to ensure they work accurately, the acceptance script checks

- Test build our latest published Ockam crate from crates.io
- Run Docker image
- Build Homebrew
- Build Terraform
- Run our multi architechture binaries
- Esure all creates are published to crates.io
