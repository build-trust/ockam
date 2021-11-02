
# Current Process

Steps:

- Create a release branch
- Enumerate per-crate incoming Changelog entries
- Create a release plan (list of crates to be published, in dependency order)
- Increment version number for changed crates and crates with changed dependencies.
- Update READMEs
- Update dependency versions in Cargo.toml
- Integrate incoming Changelog entries into existing Changelogs
- Merge to develop
- Merge to main
- `cargo publish` crates in dependency order
- `gh release` crates. Release `ockam` crate last.

Details:

### Incoming Changes

To find what has changed between releases for each crate, I use a script that parses `git log` output.

New files named `Changelog-INCOMING.md` are created for each crate. This file has a header that has the correct version, date, and header structure. Below the header is a list of commit messages that affected the crate.

I review the list of commits and copy/paste the `feat`, `fix` and other relevant changes into the appropriate section (Added, Changed, Removed). We do not include doc or test entries, only important changes.

After the new Changelog entry is ready, it is cut and paste into the real `Changelog.md` for the crate.

### Version increments

Prior to workspaces, I was able to use the `cargo bump` utility to manage version increments, but that it doesn't work with workspaces.

Now I manually increase the version number in `Cargo.toml` for each crate that requires a release.

The version needs to be increased IF:

- There is any change listed in the `Changelog-INCOMING.md`
- There is a version bump of an internal crate dependency.

When we begin using patch releases, this is an opportunity to determine the 'size' of the bump..minor vs patch, informed by the entries in the Changelog.

### Update READMEs and Cargo.toml dependencies

A script performs a find and replace in `README`s and and dependants `Cargo.toml` to bump the crate version to the new version.

### Merge to Develop and Main

A PR is created for the release branch, passes CI and is merged to `develop`.

A PR is created for the release branch, passes CI and is merged to `main`.

### cargo publish

After the release PR has been merged to `main`, A `cargo publish` is done for each crate, in dependency order.

The timing of this process can be tricky since there is a delay between a successful `cargo publish` and the time the artifact is available in the crate index. Attempting to publish a crate too quickly before its dependencies are available will cause an error.

### gh release

I run `gh release` for each crate. The command creates tags, pushes them, then creates a GitHub release using the standard template for the description.

The `ockam` crate is released last, so that it is the last visible release listed on the GitHub website.

# New Process

Steps:

- Create a release branch
- Add `Unreleased` section to each crate `Changelog.md` which lists incoming changes.
- Set the `TOKEN` env var to a crates.io API token with publish rights
- Set the `UPSTREAM` env var to a branch to diff against
- Run `crate-publisher.sh` script


### crate-publisher steps

Steps that the `crate-publisher.sh` script  performs:

- Ensure upstream branch exists
- Install `cargo release` if needed
- Determine what crates have changed by using `git diff` against `UPSTREAM`
- For each changed crate:
  - Run `cargo release minor --skip-tag --skip-push --skip-publish --no-dev-version --execute`
  - Run `git reset` to `HEAD`
  - Run `git add --all`
  - Run `git commit -m "committed all crates"`
- Run `git reset HEAD` again
  - Prints "cargo release commits squashed" ?
- Pause and prompt user to verify `git diff`
- Run `git add --all` again
- Run `git commit` again
- FORCE PUSH to current branch: `git push --set-upstream origin "$current_branch" -f`
- Pause and print `Commits pushed upstream, please merge and press enter to start git tagging`
- For each changed crate:
  - Get current crate version via the changelog (?)
  - Run `gh release create`
- For each changed crate:
  - Get current crate version via the changelog (again)
  - Run `cargo publish`
  - Run `cargo search` until published crate appears.


# Concerns

## Logical concerns

### Too many git operations
- The critical steps that this process needs to accomplish are:
  - Version increment crate `Cargo.toml` for changed crates
  - Find and replace new version in `README`s
  - Update interdependencies in crates' `Cargo.toml`

These are all simple text manipulation tasks. None of this should require so many git calls.

### Using diff from main instead of finding last tags

- This process relies on the fact that main and develop diverge between releases, and compares them to find the differences.
- It is more accurate to find the commit hash of the last crate tagging. Since currently all tags happen at the same time, this is a single commit.
- If for some reason `main` diverges for non-release reason, such as maintenance or bug fixes, this could cause problems

### Using the Changelog version as the source for version info

We should not rely on Changelog for the accuracy of the crate version. The crate version should come from `Cargo.toml`, where we _know_ it is accurate. If the version is wrong in `Cargo.toml`, it is apparent and things will break. If the version is wrong in the Changelog, the script will just use it and believe it.

The Changelog is metadata for humans, tooling should not rely on it.

### Publishing to GitHub before crates.io

It is more liklely that things will go wrong when performing `cargo publish` than to releasing GitHub. For this reason, it's important that we successfully publish to crates.io before we publish to GitHub, or we risk publishing a bad release.

I have many times only discovered problems with a crate after it has been rejected by `cargo publish`.

### Static list of crates

- This release process requires a static list of crates embedded in the script, which has already become outdated.
  - Should not be necessary: we know exactly what our list of crates is from our repo directory structure.
  - Instead we should have a way to omit crates that we do not want to publish (if any) for example the `ockam_examples`
  - Further more, this static list must be ordered by internal dependencies, and is currently incorrect.
  - Internal dependencies can and do change often. This is hard to maintain.

## General script concerns

- Relative directories are used too much. See the existing scripts strategies of utilizing an `$OCKAM_HOME` variable, which can be used as the base to define other locations.
- Use `pushd` and `popd` to manage directory state instead of `cd`
- Remove all `$(tput setaf 2)` and `$(tput sgr0)` from message lines. They add too much noise and obscure what is going on.
  - Consider making a function named something like `print_message` which injects these colors, if necessary.
- The presence of `gh` is assumed but not checked

# Suggestions
- Split the script into two scripts:
  - One script to prepare the release branch
  - One script to publish to GitHub and crates.io

## Release branch script
- Remove all git operations other than those used to build Changelogs.
- Remove ordered list of crates and use the directory structure of the repo to determine crates. For this script, dependency order is not important.
- Use existing `crate-changes.sh` script to automatically populate Changelogs
- Increment version numbers
- Update READMEs and dependant `Cargo.toml`.
- The end result should be changes staged to a release branch that includes all version increment work.

## Creating PRs and Merging branches

- For this initial work, we should keep the PR and merging processes manual until we are 100% confident that the workflow is accurate and safe.

## Publish script
- This script will need to be aware of crate dependency order, which is difficult. For now we should maybe consider my existing strategy of using a 'release plan' (list of crates to be published)
  - This release plan can differ from release to release depending on what needs to be published.
- Switch to main branch (after release PR has been merged)
- For each crate in the release plan
  - `cargo publish`
  - Wait for crate to appear in crates.io index
  - `gh release`

