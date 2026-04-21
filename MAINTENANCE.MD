# Soroban Target Maintenance Policy

This document defines the maintenance and compatibility expectations for the Soroban target.

## Status

The Soroban target is currently considered **experimental**.

This means:

- the target is actively maintained
- core functionality is expected to work for supported features
- some language features and platform integrations are not supported or may still change
- breaking changes may occur when required by correctness, upstream Soroban changes, or architectural improvements

Experimental does **not** mean unmaintained. It means the support surface is explicitly scoped and evolves with the platform.

---

## Goals

The Soroban backend aims to:

- make Soroban more accessible to Solidity developers
- document all meaningful deviations from `solc` and from EVM behavior
- evolve alongside the Soroban platform in a predictable way

---

## Non-Goals

The Soroban backend does not currently guarantee:

- full Solidity language parity with `solc`
- identical runtime semantics to the EVM

---

## Support Levels

Each Soroban-related feature falls into one of three categories.

### Supported

A feature marked **Supported**:

- is covered by automated tests
- is documented
- is expected to remain stable across patch releases

### Experimental

A feature marked **Experimental**:

- is available for use
- may have incomplete edge-case coverage
- may be removed or redesigned if needed

### Unsupported

A feature marked **Unsupported**:

- is not intended to work on Soroban
- must not silently compile into incorrect behavior

---

## Compatibility Policy

This policy defines maintenance expectations. The detailed compatibility behavior is documented in the published Solang docs.

Soroban compatibility expectations are divided into two scopes:

- Solidity language compatibility
- Solang and Soroban Rust SDK differences

This section does not define guarantees for Soroban protocol-version compatibility or tooling-version compatibility.

The detailed compatibility documentation lives in the published Solang docs:

- [Soroban Solidity Language Compatibility](https://solang.readthedocs.io/en/latest/targets/soroban_language_compatibility.html)
- [Solang and Soroban Rust SDK Differences](https://solang.readthedocs.io/en/latest/targets/soroban_rust_sdk_differences.html)

### Solidity Language Compatibility

Authoritative docs: [Soroban Solidity Language Compatibility](https://solang.readthedocs.io/en/latest/targets/soroban_language_compatibility.html)

Solang aims for source-level Solidity familiarity, not EVM equivalence.

This includes language features and semantics such as:

- types
- structs
- arrays
- mappings
- function behavior
- authentication-related language differences from standard Solidity usage

Examples include differences such as Soroban host-based authorization flows, where authentication is performed through Soroban auth mechanisms rather than typical Solidity or EVM patterns like direct `msg.sender` checks.

Where Soroban differs from normal Solidity or from EVM assumptions, Solang will prefer one of the following:

- document the difference
- emit a warning
- reject the construct

Silent semantic mismatches should be treated as bugs.

### Solang and Soroban Rust SDK Differences

Authoritative docs: [Solang and Soroban Rust SDK Differences](https://solang.readthedocs.io/en/latest/targets/soroban_rust_sdk_differences.html)

Solang also documents where its Soroban implementation differs from common patterns used by the Rust SDK.

This includes differences such as:

- storage layout and key structure for contract state
- representation of arrays and vectors in storage, including `VecObject`-backed layouts
- representation of structs and nested values in storage
- host object and handle conventions needed to interact with Soroban contracts written in Rust

Examples include differences or compatibility requirements around storage representation, such as arrays of native types being stored through `VecObject` handles, and structs or arrays of structs being represented through composite storage keys rather than EVM-style slot packing.

This scope is about documented differences in representation and implementation choices.
It is not a general guarantee of raw storage compatibility.

When Solang uses a different layout or access pattern than a Rust SDK contract would typically use, that difference should be documented clearly.
Undocumented mismatches in documented areas should be treated as bugs.

---

## Release Policy

### Patch Releases

Patch releases may include:

- bug fixes
- diagnostic improvements
- documentation improvements
- non-breaking test and tooling updates
- correctness fixes for clearly incorrect existing behavior

Patch releases should not intentionally introduce new breaking Soroban semantics unless the current behavior is wrong and keeping it would be more harmful.

### Minor Releases

Minor releases may include:

- new Soroban features
- Soroban backend refactors
- changes required by upstream protocol or tooling evolution
- breaking changes, with migration notes

Every Soroban-breaking change in a minor release should include:

- a short description of the change
- the reason for the change
- who is affected
- migration guidance where applicable

---

## Upstream Change Handling

When Soroban platform changes occur, maintainers should classify them as one of:

- no action needed
- tooling-only update
- compiler/runtime behavior update required
- breaking change requiring user migration

Protocol upgrades and upstream breaking changes take priority over new feature work.

---

## Testing Requirements

A Soroban feature should not be considered Supported unless it is covered by tests appropriate to its risk.

The Soroban CI suite should include, where applicable:

- compile-time tests
- negative tests for unsupported constructs
- runtime tests against a local Soroban environment
- regression tests for prior bugs
- integration tests for common contract patterns

Priority test areas include:

- storage behavior
- contract initialization
- authorization/auth-related behavior
- cross-contract calls
- ABI/spec generation and decoding expectations
- integer and data-layout correctness
- failure/revert/error behavior where relevant

Every confirmed Soroban bug should add a regression test when feasible.

---

## Bug Triage Priorities

### P0 — Critical

Examples:

- miscompilation
- security-sensitive incorrect behavior
- incorrect authorization behavior
- incorrect storage behavior
- breakage caused by Soroban protocol/tooling upgrades

These issues should take precedence over feature development.

### P1 — High

Examples:

- valid Soroban-target contracts rejected unexpectedly
- major supported feature broken
- deployment/invocation workflow broken

### P2 — Normal

Examples:

- diagnostics quality
- missing warnings
- documentation gaps
- edge-case issues in experimental areas

---

## Documentation Requirements

Soroban behavior should be documented in a way users can rely on.

At minimum, the project should maintain:

- a Soroban compatibility matrix
- a list of unsupported features
- a list of known differences from `solc` / EVM expectations
- migration notes for breaking changes
- examples for supported Soroban workflows

If a feature is not documented and not tested, it should not be treated as stable.

---

## Deprecation Policy

When support is planned to be removed for:

- a protocol version
- a tooling version
- a Soroban-specific feature shape
- an experimental interface

the project should communicate this in advance whenever feasible through:

- release notes
- migration notes
- warnings in docs and/or compiler diagnostics where appropriate

Immediate removals may still occur if required by correctness or upstream incompatibility.

---

## Definition of Production Readiness

The Soroban target should only be described as production-ready when all of the following are true:

- a compatibility matrix exists and is maintained
- core supported features have stable regression coverage
- at least one full upstream protocol transition has been handled cleanly
- no known critical miscompilation bugs remain open in supported areas
- major behavioral differences are documented
- users have a clear upgrade and migration path

Until then, users should treat the Soroban target as suitable for experimentation, prototyping, evaluation, and controlled pilots.

---

## Maintainer Expectations

The Soroban backend should have:

- at least one named primary maintainer
- at least one backup reviewer familiar with the target
- a release checklist for Soroban-affecting changes

Ownership is required for the policy to be meaningful.

---

## Summary

In short, the Soroban target is maintained with the following principles:

- correctness over feature count
- explicit support levels
- clear compatibility boundaries
- fast reaction to upstream Soroban changes
- no silent incorrect behavior
- documented breaking changes
