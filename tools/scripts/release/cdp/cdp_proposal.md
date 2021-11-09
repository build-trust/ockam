# Continuous Deployment Pipeline (CDP)

Miro board: https://miro.com/app/board/o9J_lkvpyIY=/

## Goals

- A build pipeline comprised of decoupled, idempotent stages.
- Automated creation of releasable artifacts.
- Maintenance of Changelogs derived from git history.
- Real time status notification and error alerting.

## Overview

The CDP should be thought of as a process consisting of logical stages - not as a collection of tools. The tools that are
used and the order in which they are run can and will change over time.

We should use tools that are:

1. Right for a particular stage
2. Integrate well with other stages and tools
3. Allow flexible configuration
4. Allow for evolution and improvement

The design focus of the CDP should be on the inputs and outputs of the stages, _not_ the tools used to accomplish it. 
Inputs and outputs should be specified, and the behavior of a stage evaluated depending on these factors. The implementation
of a stage should be a black box relative to the whole pipeline.

The stages of the CDP should be idempotent and expected to fail. By idempotent it is meant that the same stage can be
run multiple times with no side effects, producing the same output.

The rollout of the CDP should be a phased, gradual approach. This allows us to check assumptions, change behavior and test
as we build the pipeline:

- Phase 0: Human in the Loop verifying process steps. Several intervention steps may be necessary. Step processes scrutinized carefully.
- Phase 1: Human in the Loop verifying outputs. Confidence should be high that processes are correct, and focus can shift to outputs.
- Phase 2: No human in the loop, until final Publish stage.
- Phase 3: Fully automated luxury deployment.

We may want to consider going back to using pre-release semver tags such as `-dev`:
- `cargo-release` has excellent support for managing pre-release versions
- We recently learned that we can safely publish pre-release versions to crates.io
- Mitigates risk of bad releases impacting examples and dependant projects, especially while CDP is developed.

## Stages

The CDP stages are listed below. All stages have _health checks_ which upon failure will fail the entire pipeline.

1. Activate
2. Validate
3. Test
4. Version
5. Package
6. Tag
7. Merge
8. Publish

The stages are described in terms of their inputs and outputs. Several fork/branch names are used. These branches
could either be distinct branches that are created through the pipeline, or a single branch that is progressively
updated. There are tradeoff to both strategies: multiple intermediate branches allows for better debugging and auditing,
whereas a single branch will probably use less space and be less complex.

### Activate

**Input**: Triggering event such as a commit, merge, or manual invocation.

**Output**: Release Fork

`Activate` is the entry point to the pipeline. Multiple triggering conditions can be configured to start the stage.

The output of `Activate` is a _Release Fork_. Release Fork is a private fork of the repository at a particular commit.
Using a fork allows us to more securely and safely run source modifying operations on a branch.

### Validate

**Input**: Release Fork

**Output**: Release Fork

The `Validate` stage runs initial checks and verification on the Release Fork. The individual checks in this stage
can be enhanced over time. This stage should not modify the Release Fork.

### Test

**Input**: Release Fork

**Output**: Release Fork

The `Test` stage runs multiple test scenarios, including but not limited to:

- Unit tests
- Integration tests
- Static analysis / Coverage
- Performance testing

This stage should not modify the Release Fork.

### Version

**Input**: Release Fork, Git History

**Output**: Versioned Fork

The `Version` stage is responsible for:

- gathering a change set/delta from previous release.
- creating a Release Plan from the change set.
- incrementing crate versions according to the Release Plan.

A Release Plan is a set of (crate, version) tuples that specify the necessary version increments for each crate. This
allows crates to have differ version bumps depending on content. A simplified version could be used initially, for
example a Release Plan that simply performs a minor bump for each changed crate.

The result of the `Version` stage is a _Versioned Fork_. A Versioned Fork is constructed by performing bumps for
every crate in the Release Plan on the Release Fork.

**Tooling**: `cargo release` utilizing _only_ version bump functionality. Other features of this tool such as managing
tags and changelogs should _not be used_ at this stage. Post release hooks can be used to record version change
information (such as old and new versions).

### Package

**Input**: Versioned Fork, Git History

**Output**: Packaged Fork

The `Package` stage updates all metadata files that contain version information, and build `.crate` files.

In this stage, `README.md` and `Changelog.md` for each crate in the Release Plan are modified.

For each crate in the Release Plan, a `.crate` file is generated, but not yet published.

**Tooling**: git-cliff or some other purpose built system for managing Changelogs. The Changelog updating features
of `cargo release` are awkward and less powerful than tools like `cliff`.

### Tag

**Input**: Packaged Fork

**Output**: Packaged Fork, Git Tags

The `Tag` stage is responsible for tagging commits and pushing them to GitHub. It does not modify the source fork, but
creates new tags.

**Tooling**: `git` and `gh`

### Merge

**Input**: Packaged Fork

**Output**: PR of Packaged Fork to Repo

The `Merge` stage takes the Package Fork and creates a PR back to the primary repository.

**Tooling**: `git` and `gh`

### Publish

**Input**: Packaged Fork

**Output**: External Publish

The `Publish` stage is the last stage, and performs the actual publishing of the Packaged Fork to all external
parties such as crates.io and GitHub.

**Tooling**: `cargo-publish` or `crates.io API` and `gh`