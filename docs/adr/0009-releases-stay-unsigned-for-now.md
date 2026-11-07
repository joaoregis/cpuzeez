# ADR 0009: Releases stay unsigned for now

- **Status:** accepted (2026-11-04)
- **Decision ref:** Q9 · details in [docs/v1/dependencies.md](../../docs/v1/dependencies.md#31-the-install-friction-that-isnt-a-dependency)

## Context
Authenticode certificates and Apple notarization cost money every year; the project has no budget.

## Decision
No paid code signing yet. Zero-cost mitigations: an explicit README notice, SHA256SUMS with verification steps, public CI builds, and evaluating free OSS signing (SignPath Foundation).

## Alternatives considered
Paid certificate / Apple Developer Program.

## Consequences
SmartScreen and Gatekeeper warnings remain; the README explains them.
