# ADR 0002: Privileges: restart as admin + per-action elevation; helper service later

- **Status:** accepted (2026-11-04)
- **Decision ref:** Q2 · details in [docs/v1/README.md](../../docs/v1/README.md#43-privileges-getting-adminroot-decided-q2)

## Context
Sensors, FPS (ETW) and system actions need admin/root, but the app must start unprivileged (P3) and never run the GUI as root on Linux.

## Decision
1.0 ships **A** (a "Restart as administrator" button) and **B** (each privileged action runs in an elevated `cpuzeez` subprocess that accepts only allow-listed, serialized actions). The read-only helper service **C** comes after 1.0.

## Alternatives considered
Always elevated (violates P3); a helper service in 1.0 (more attack surface before the core is stable).

## Consequences
One UAC/polkit prompt per action or per elevated session; the executor's allow-list is a security boundary and is fuzzed.
