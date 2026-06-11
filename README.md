# osTicket Modernisation

This repository is the **osTicket modernisation project**: a ground-up reimplementation
of the osTicket 1.7 helpdesk / support-ticket system on a modern stack, driven by
functional specifications reverse-engineered from the original source.

## Repository layout

- **`legacy/`** — the original **osTicket 1.7** PHP source, archived verbatim. It is the
  reference implementation we are modernising away from. This source is **frozen** — it
  must not be modified. The exact pre-modernisation state is captured by the annotated git
  tag **`php-1.7-final`**.
- **`specs/`** — technology-agnostic **functional specifications** (FS-XXX docs with
  embedded BS-XXX business rules, EC-XXX edge cases, KL-XXX known limitations)
  reverse-engineered from the legacy PHP source. Start with
  [`specs/CLAUDE.md`](./specs/CLAUDE.md) for the index and conventions. These specs are the
  contract that the new implementation must satisfy.
- **`work/`** — process artifacts from the spec reverse-engineering run (plan, trackers,
  gap / coverage / dedupe reports).

## What we are building

A clean reimplementation of osTicket's functionality:

- **Backend** — **Rust** (a Cargo workspace, to be created at the repo root).
- **Frontend** — **React + TypeScript**.

The functional specifications in `specs/` define the behaviour to be reproduced. The legacy
PHP under `legacy/` is the authoritative reference whenever a spec is ambiguous.

## History

The original PHP source was first reverse-engineered into the 22 functional specifications
under `specs/` (with `@implements` tags inserted into the PHP source linking each
declaration back to the requirements it satisfies). That run is complete; the repository has
now entered its **modernisation phase**, building the Rust + React/TypeScript replacement in
this same repo while preserving the legacy source for reference.

## Conventions

- Planning for the modernisation lives under `kanban/` (created by planning).
- **All git operations stay local. Never push to any remote.**
