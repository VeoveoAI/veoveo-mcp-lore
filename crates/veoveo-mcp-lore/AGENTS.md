# Lore MCP Server — Agent Manual

Delta over the repository root `AGENTS.md`. [`DESIGN.md`](DESIGN.md) is the design and
operational contract.

## Purpose

Lore for agents and people: `list_files`, `thumbnail` and the App `ui://lore-api/repositories`. It starts with seeing what is
there, and is where Lore's API grows.

## Invariants

- Owns the `lore-api://` URI scheme. It has no tables. Identity: slug `lore-api`, MCP `/lore-api/mcp`, port 8808.
- Every read goes through the Hub's routes, as the caller, so Lore's rules decide it. Never read Lore another way.
- It makes no pictures: `thumbnail` asks the Thumb server, as the same person.
- The App's look and its gallery come from `veoveo-hub-ui`. `assets/repositories.js` holds only what is Lore's.
- A tool argument never names a tenant or a person: those come from the forwarded identity.
- Ready `Routes` bind typed tool requests and readers to their output, permission and cache policy.
- Addresses come from `LoreResource` through the shared route declaration. Do not build or split a URI by hand.
- `skills/browse-repositories/` is the server's skill.

## Build And Test

- `cargo test -p veoveo-mcp-lore`, as the `dev` image provides it.
- `tests/mcp.rs` calls the server through the SDK's test Hub.
- `docker build --target check .` runs the gate.
