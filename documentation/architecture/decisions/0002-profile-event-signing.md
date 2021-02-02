# 2. Profile Event Signing

Date: 2021-02-02

## Status

Proposed

## Context
Profile events can include multiple keys so the profile can prove the keys are valid. The question centers on
when adding new keys, should a proof of possession be required? This involves the new key signing the current
event data, then the current profile root key signing the event data with the proof into the event.

## Decision

It has been decided that a proof of possession will be required for now when adding a new key to the profile.

## Consequences

If a proof of possession is not required, an attacker could add arbirary data to the event log.
