# ADR 0002 — No shell, typed argv only

## Status

Accepted

## Context

Shell wrappers (`sh -c`, `bash -c`, `eval`) invite injection when unit names or paths are interpolated.

## Decision

All external processes use `Command::arg` / `args` with allowlisted action verbs and validated unit names. Sudo and systemctl resolve under trusted absolute directories (`/usr/bin`, `/bin`, `/usr/sbin`, `/sbin`).

## Consequences

Helpers such as `TrustedCommand` / `ExecutionPolicy` (enforced timeouts and output caps) and `FakeCommandRunner` keep tests free of real elevation. Streaming `journalctl --follow` uses trusted resolution + `kill_on_drop` / cancel without a oneshot timeout.
