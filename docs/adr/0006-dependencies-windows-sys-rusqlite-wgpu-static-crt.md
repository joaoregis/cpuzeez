# ADR 0006: Dependencies: windows-sys, rusqlite (bundled), wgpu; static CRT

- **Status:** accepted (2026-11-04)
- **Decision ref:** Q6 · details in [docs/v1/dependencies.md](../../docs/v1/dependencies.md)

## Context
The Task Manager needs 100+ Win32 APIs, histories need queries, the GPU benchmark needs compute, and the release exe depended on an unshipped VC++ runtime.

## Decision
Adopt `windows-sys` (platform/windows only), `rusqlite` with SQLite compiled in, and `wgpu` behind the `gpu-bench` feature (on in releases). Link the MSVC CRT statically. Rule: pick what solves the problem best while removing friction for the user.

## Alternatives considered
Hand-written FFI; JSON/CSV histories; OpenGL compute (absent on macOS); shipping the VC++ redistributable.

## Consequences
No runtime installs for users; a C compiler is needed at build time (CI has one).
