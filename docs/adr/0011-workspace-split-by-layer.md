# ADR 0011: Workspace split by layer

- **Status:** accepted (2026-11-04)
- **Decision ref:** engineering · details in [docs/v1/engineering.md](../../docs/v1/engineering.md#2-layers-and-dependency-direction)

## Context
One crate lets collection code import egui and the GUI call OS APIs; `unsafe` is scattered.

## Decision
Cargo workspace: `core` (pure logic, no OS/GUI), `platform` (per-OS adapters, all `unsafe`), `gui` (no OS calls), the `cpuzeez` binary (composition). Migrated incrementally, moving code without changing behaviour.

## Alternatives considered
Keep one crate with conventions; one crate per feature.

## Consequences
The compiler enforces boundaries; `core` is testable anywhere with fakes.
