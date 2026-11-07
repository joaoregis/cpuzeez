# ADR 0007: One 1.0.0 release, built in gated stages

- **Status:** accepted (2026-11-04)
- **Decision ref:** Q7 · details in [docs/v1/README.md](../../docs/v1/README.md#6-roadmap-decided-q7)

## Context
The 1.0 scope is large and must ship stable.

## Decision
Stages S0 foundation → S1 existing features → S2 new features → S3 hardening; everything ships together in 1.0.0. A stage closes only when its automated quality gate passes.

## Alternatives considered
Shipping features as they land (0.4, 0.5…).

## Consequences
Pre-releases (ADR 0010) give testers each closed stage.
