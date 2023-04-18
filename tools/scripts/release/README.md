# Ockam Scripts

This folder contains scripts to release Ockam Rust infrastructures. Note, to run these scripts you need to run Bash version 4 upwards. All commands should be called from the Ockam root path.

To perform release on the Ockam repository, we can release crates automatically using the [release.sh](https://github.com/build-trust/ockam/blob/develop/tools/scripts/release/release.sh) script

## Automatic Ockam Release

Ockam release can be done automatically using the `release.sh` script. The release script interfaces with workflows that performs release over GitHub CI, this helps to ensure we perform release using one architechture (ubuntu-20.04).

- [Crate Bump](https://github.com/build-trust/ockam/blob/develop/.github/workflows/release-crate-version-bump-pr.yml)
- [Crates IO Release](https://github.com/build-trust/ockam/blob/develop/.github/workflows/release-publish-crates.yml)
- [Binary Release](https://github.com/build-trust/ockam/blob/develop/.github/workflows/release-draft-binaries.yml)
- [Ockam Command Docker Image](https://github.com/build-trust/ockam/blob/develop/.github/workflows/release-ockam-package.yml)
- [Homebrew Repo Bump](https://github.com/build-trust/homebrew-ockam/blob/main/.github/workflows/release-version-bump-pr.yml)

There are two steps to a release
- Draft release
- Production release

To create a release, we first create a draft, which will later on be reviewed and pull requests created merged before running the script to create a production release.

To start the release in draft mode we call from the ockam home

```bash
IS_DRAFT_RELEASE=true ./tools/scripts/release/release.sh
```

A successful draft release run will,
- Bump Ockam crates and create a pull request in the /ockam repo
- Create Github release as draft with built binaries and NIFs
- Release Ockam docker image with a draft tag
- Bump Homebrew version and create a pull request for review in /homebrew-ockam repository

After draft release is created, release is to be vetted and pull requests created in /ockam approved and merged before production release is started.

To start production release, `from ockam home`, call

```bash
IS_DRAFT_RELEASE=false ./tools/scripts/release/release.sh
```

A successful production release run will
- Release Ockam docker image as latest
- Make Ockam Github release non-draft and latest
- Push our crates to crates.io

### Indicating Release Version

By default, we release crates with a `minor version` bump, to bump version of crate to major or patch version, you can pass the `OCKAM_BUMP_RELEASE_VERSION` with either `major`, `minor` or `patch`.
```bash
OCKAM_BUMP_RELEASE_VERSION=major ./tools/scripts/release/release.sh
```

### Skipping Release Steps

We follow defined steps when releasing, we can indicate to the script to skip steps by setting any of the below variables as `true`

#### For Draft
- SKIP_OCKAM_BUMP - Skips Bumping Crates Version
- SKIP_OCKAM_BINARY_RELEASE - Skips Ockam Command Binary Draft Release And NIFs
- SKIP_OCKAM_PACKAGE_DRAFT_RELEASE - Skips Ockam Command Package Draft Release
- SKIP_HOMEBREW_BUMP: Skip Homebrew Ockam Version Bump PR
- SKIP_TERRAFORM_BUMP: Skipe Terraform Ockam Version Bump PR
- SKIP_TERRAFORM_BINARY_RELEASE: Skip Terraform Ockam Binary Draft Release

#### For Production
- SKIP_OCKAM_PRODUCTION_RELEASE - Skips Ockam Binary and NIFs Release to Production
- SKIP_OCKAM_PACKAGE_PRODUCTION_RELEASE: Skips Ockam Command Docker Package Production Release
- SKIP_TERRAFORM_PRODUCTION_RELEASE" Skips Terraform Production Release
- SKIP_CRATES_IO_PUBLISH - Skips crates.io Publish

For example,to skip Ockam bump in Draft release
```bash
SKIP_OCKAM_BUMP=true IS_DRAFT_RELEASE=true ./tools/scripts/release/release.sh
```

For a success release to be done
- A draft release should be called `IS_DRAFT_RELEASE=true ./tools/scripts/release/release.sh`
- PRs in /ockam, /homebrew-ockam, and /terraform-provider-ockam should be reviewed
- /ockam PR that bumps crates should be merged
- A production release should be called with `IS_DRAFT_RELEASE=false ./tools/scripts/release/release.sh`
- PRs in /homebrew-ockam and /terraform-provider-ockam should be merged


### Draft Release Rollback

We have a script to rollback draft release, to rollback a release

```bash
TAG_NAME=ockam_v0.71.0 ./tools/scripts/release/release-rollback.sh
```

Where `TAG_NAME` is the tag of the draft release.

Rollback consists of

- Deleting GitHub release for indicated `TAG_NAME`
- Deleting tag that was created during release
- Closing /ockam PR
- Closing Homebrew PR
- Closing Terraform PR
- Closing Terraform GitHub release
- Deleting Ockam command package draft release


## <u>More Details On Individual Scripts That Enables Release</u>

To enable a successful release, we create subset of scripts that enables a specific function, e.g. bump all Rust crate version, CI inherits each script variables which enables us interact with other scripts (changelog.sh, crate-bump.sh, etc) directly from `release.sh` for example, we can omit a crate (ockam_abac) from being bumped by calling

```bash
OCKAM_BUMP_MODIFIED_RELEASE="ockam_abac:release" IS_DRAFT_RELEASE=true tools/scripts/release/release.sh
```

`OCKAM_BUMP_MODIFIED_RELEASE` is used directly in the tools/scripts/release/crate-bump.sh file.

### Crate Bump

Crates versions are bumped using [cargo-release](https://github.com/crate-ci/cargo-release) >= v0.18.6. While bumping crates, CHANGELOG.md and README.md files are also updated with the bumped version.
To run crate bump, from the Ockam root path, call
```bash
OCKAM_BUMP_RELEASE_VERSION=minor tools/scripts/release/crate-bump.sh
```
where RELEASE_VERSION is the [version](https://github.com/crate-ci/cargo-release/blob/master/docs/reference.md#bump-level) all crates are to be bumped to.


#### OCKAM_BUMP_MODIFIED_RELEASE

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


#### OCKAM_BUMP_BUMPED_DEP_CRATES_VERSION

Crates whose `dependencies were only bumped` and do not have Rust code updates can be version-bumped with a specified version using the `OCKAM_BUMP_BUMPED_DEP_CRATES_VERSION` definition so as to follow a different release version
```bash
OCKAM_BUMP_BUMPED_DEP_CRATES_VERSION=patch RELEASE_VERSION=minor tools/scripts/release/crate-bump.sh
```
If `OCKAM_BUMP_BUMPED_DEP_CRATES_VERSION` is not defined it is automatically set to `minor`.


### Changelog Generation

Changelogs are generated using [git-cliff](https://github.com/orhun/git-cliff). To generate changelogs, we call the [changelog.sh script](https://github.com/build-trust/ockam/blob/develop/tools/scripts/release/changelog.sh) which will generate changelogs and append to their CHANGELOG.md file.
To run changelog generator, from the Ockam root path, call
```bash
tools/scripts/release/changelog.sh
```


### Crate Publish

Crates are published to `crates.io` using [cargo-release](https://github.com/crate-ci/cargo-release) right after bump. Only crates that have been updated (comparing `git diff` with last git tag) are published. Crates can also be excluded from being published using the `OCKAM_PUBLISH_EXCLUDE_CRATES` variable, to exclude crates, we can optionally specify crates that are to be excluded `OCKAM_PUBLISH_EXCLUDE_CRATES="signature_core ockam_core"`, where `signature_core` and `ockam_core` are excluded. Publish script can also be rerun after a recent fail, recently successfully published crates will automatically be detected and excluded. To indicate a script rerun we set the `OCKAM_PUBLISH_RECENT_FAILURE` env to `true`.

`OCKAM_PUBLISH_RECENT_FAILURE="true"`.

To publish crates
```bash
OCKAM_PUBLISH_PUBLISH_TOKEN=my_crates.io_token OCKAM_PUBLISH_EXCLUDE_CRATES="signature_core ockam_core" tools/scripts/release/crate-publish.sh
```
Note: Require cargo-release >= version 0.18.6


### Binaries and NIF Release

We create draft binaries over GitHub CI. To create draft binaries, we initiate the [Binary Release Workflow](https://github.com/build-trust/ockam/blob/develop/.github/workflows/release-draft-binaries.yml) by indicating the below

- release_branch: Release branch is the branch that all binaries and NIFs will be built from (this is normally indicated as the PR that bumps crates)


### Docker Package Release

We release our Docker image [using a workflow on the /ockam repository](https://github.com/build-trust/ockam/blob/develop/.github/workflows/release-ockam-package.yml). To create a docker release, we pull ockam command assets from GitHub release then embed the executables in the docker image. To initiate the workflow, we need to set the below inputs

- tag: Ockam Git tag to build image on
- binaries_sha: A string that consists of Ockam command binary name and their SHA for different architechture. This is usually gotten from [GitHub release](https://github.com/build-trust/ockam/releases)
- is_release: Indicates if we are building docker packages as draft or production

