# ADR 0001: ZeeZ Live is the brand of the monitoring feature

- **Status:** accepted (2026-11-04)
- **Decision ref:** Q1 · details in [docs/v1/zeez-live.md](../../docs/v1/zeez-live.md#1-the-brand-and-its-parts)

## Context
The overlay, capture and reports needed a name users grasp in seconds and that can carry a brand.

## Decision
The feature is **ZeeZ Live**. Its parts keep plain, universal names: Overlay, Capture, Reports. The brand name is never translated; the parts' names are. ● LIVE / ● REC indicators look the same everywhere.

## Alternatives considered
"Performance Monitor" (generic, no brand), "Game Monitor" (excludes professionals), "HUD" (names only the surface).

## Consequences
One sidebar entry "ZeeZ Live" with Live / Overlay / Sessions tabs; internal module `live`.
