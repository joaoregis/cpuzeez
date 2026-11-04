# ADR 0010: Pre-releases for testers

- **Status:** accepted (2026-11-04)
- **Decision ref:** Q10 · details in [docs/v1/README.md](../../docs/v1/README.md#61-beta-releases-decided-q10)

## Context
CI can't cover the real variety of boards, GPUs and drivers.

## Decision
Publish `X.0.0-beta.N` after each closed stage and `-rc.N` during hardening, as GitHub pre-releases (never "latest"). An opt-in beta channel in Settings. Package versions are mapped per format by `.github/scripts/pkgver.py`.

## Alternatives considered
Only stable releases; a separate beta repository.

## Consequences
Stable users never see betas; `update::newer` orders pre-releases per SemVer.
