# Continuous Deployment Pipeline (CDP)

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

### Activate

Input: Triggering event such as a commit, merge, or manual invocation.
Output: Release Fork

Activate is the entry point to the pipeline. 