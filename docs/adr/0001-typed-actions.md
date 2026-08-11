# ADR 0001 — Typed AppAction / SideEffect flow

## Status

Accepted

## Context

A TUI that can signal processes and control systemd must not accept free-form privileged strings from the UI.

## Decision

Input maps to typed `AppAction`. The reducer updates state and returns typed `SideEffect` values. The runtime executes side effects via providers / `AdministrativeExecutor`. UI never calls OS APIs.

## Consequences

New admin capabilities require new enum variants and confirmations. Exports and FS I/O must also leave the reducer as side effects.
