# 3. Checklists and Runbooks for Processes

Date: 2021-02-08

## Status

Proposed

## Context

This describes the use of Checklists and Runbooks which can be used in README.md to describe processes related to the
software. The core use cases are release and deployment, but they are useful in other scenarios.

### Checklists

- Checklists are simple lists of sentences which describe requirements and steps necessary to perform a task.
- Checklists should be short and useful to someone unfamiliar with the task at hand.

#### Example Checklist
```
# Release a Crate
1. Fetch and rebase the latest develop branch.
2. Create a release branch. I do something like `$author/release/$crate_name_$newvers`.
3. Update `CHANGELOG.md` to reflect incoming changes.
4. Bump version numbers in `Cargo.toml`, `Cargo.lock`, and `README.md`
...
```


- Example Checklists we can use:
  1. PR Readiness
  1. Release Processes (Crates, Hex, integrations)
  1. Documentation best practices
  1. Testing

### Runbooks

- Runbooks are rich text documents (Markdown) which provide at-a-glance information about a service, system, or process.
In practice, each service has one runbook used for maintenance. Multi-stage processes like code artifact publishing also benefit
  from this documentation.

Runbooks have the following properties:

  1. Comprised of Checklists and information tables.
  1. Contain or link to a Deployment Checklist.
  1. Contain or link to a Start/Restart Checklist.
  1. Contain emergency contact information for responsible team, manager, and/or technical expert.
  1. Contain deployment status information for each environment. Where is it running? What is its health?

Runbooks are primarily useful for services or other deployed software that can fail. They should be considered the first,
best place to look for information regarding a service during an emergency or getting new contributors up to speed.

Runbooks are also useful to describe biz-dev integration maintenance like Slack and Salesforce.

Example Runbooks we can use:
  1. Crate Release
  1. Hex Release
  1. Several GitHub processes to provision new users
  1. Ockam Hub Maintenance (Soon!)

## Decision


## Consequences
