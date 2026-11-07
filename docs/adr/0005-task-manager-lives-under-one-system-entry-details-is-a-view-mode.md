# ADR 0005: Task Manager lives under one "System" entry; Details is a view mode

- **Status:** accepted (2026-11-04)
- **Decision ref:** Q5 · details in [docs/v1/task-manager.md](../../docs/v1/task-manager.md#2-navigation-decided-q5)

## Context
Seven Task Manager tabs would overload an 11-entry sidebar.

## Decision
One "System" entry with tabs (Processes, Performance, App history, Startup, Users, Services). Details is the Detailed view mode of Processes, keeping every column and action.

## Alternatives considered
Seven sidebar entries; a separate Details tab duplicating filters and actions.

## Consequences
One process table, one set of filters and actions.
