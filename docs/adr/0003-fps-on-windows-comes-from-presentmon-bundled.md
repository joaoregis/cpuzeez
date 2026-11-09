# ADR 0003: FPS on Windows comes from PresentMon, bundled

- **Status:** accepted (2026-11-04)
- **Decision ref:** Q3 · details in [docs/v1/zeez-live.md](../../docs/v1/zeez-live.md#31-measuring-the-games-frames)

## Context
Frame timing must work for every graphics API without touching the game process (anti-cheat).

## Decision
Bundle Intel's PresentMon console executable (MIT, Intel-signed) as a sidecar and read its stream. The user installs nothing.

## Alternatives considered
Present hooks / injection (ban risk, rejected by P4); a native Rust ETW consumer now (more work, re-evaluated after 1.0).

## Consequences
Needs admin (ETW). One third-party binary pinned and hash-checked in CI.
