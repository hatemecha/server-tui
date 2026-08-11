# ADR 0003 — Single-writer persistence

## Status

Accepted

## Context

Multiple writers to `state.toml` (notably SMART probes) race and blur ownership.

## Decision

Providers emit observations only. The reducer accepts current-generation observations and emits a typed save side effect; runtime performs the only atomic save (`0600` file in an explicitly app-owned `0700` directory).

## Consequences

Probe code must not call `AppPersistState::save_atomic`. Diagnostics tests pass previous CRC via snapshot fields set by the runtime.
