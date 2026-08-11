# ADR 0003 — Single-writer persistence

## Status

Accepted

## Context

Multiple writers to `state.toml` (notably SMART probes) race and blur ownership.

## Decision

Providers emit observations only. The runtime/app merges into `AppPersistState` and performs the only atomic save (0600 / 0700).

## Consequences

Probe code must not call `AppPersistState::save_atomic`. Diagnostics tests pass previous CRC via snapshot fields set by the runtime.
