# ADR 0012: No code injection, ever

- **Status:** accepted (2026-11-04)
- **Decision ref:** P4 · details in [docs/v1/zeez-live.md](../../docs/v1/zeez-live.md#32-drawing-on-top-of-the-game)

## Context
In-game overlays are usually DLL hooks, which anti-cheats ban and which crash games.

## Decision
ZeeZ Live draws in its own transparent, click-through, always-on-top window. Game processes are only opened with PROCESS_QUERY_LIMITED_INFORMATION.

## Alternatives considered
Present hooks (RTSS-style).

## Consequences
Not visible in true exclusive fullscreen; the app detects it and suggests borderless.
