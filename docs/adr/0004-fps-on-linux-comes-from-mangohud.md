# ADR 0004: FPS on Linux comes from MangoHud

- **Status:** accepted (2026-11-04)
- **Decision ref:** Q4 · details in [docs/v1/zeez-live.md](../../docs/v1/zeez-live.md#31-measuring-the-games-frames)

## Context
Linux has no system-wide present tracing; frame timing needs a Vulkan layer / GL preload in the game.

## Decision
Integrate MangoHud: detect it, one-click install (distro package or Flatpak Steam extension), opt-in "enable for all games" via environment.d, generate its config from our layout, ingest its logs.

## Alternatives considered
Our own Vulkan layer now (later), external capture-based FPS (displayed frames only).

## Consequences
Works on X11 and Wayland, including GNOME where an external overlay can't stay on top.
