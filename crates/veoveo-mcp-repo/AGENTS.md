# Repo MCP Server — Agent Manual

Delta over the repository root `AGENTS.md`. [`DESIGN.md`](DESIGN.md) is the design and
operational contract.

## Purpose

Lore repositories for agents and people: what people did (`activity`, `changes`, `file_diff`), what they hold (`list_files`, `thumbnail`), and the App
`ui://repo/activity`. It only views. It is where Lore's API grows.

## Invariants

- Owns the `repo://` URI scheme. It has no tables. Identity: slug `repo`, MCP `/repo/mcp`, port 8808.
- Every read goes through the Hub's routes, as the caller, so Lore's rules decide it. Never read Lore another way.
- It makes no pictures: `thumbnail` asks the Thumb server, as the same person.
- The App's look and its gallery come from `veoveo-hub-ui`. `assets/viewer.js` and `viewer.css` are the viewer, and `assets/repositories.js` holds only what is Lore's of the gallery.
- It reads, and has no tool that changes anything in Lore: agents do the editing.
- Nothing reaches a person from memory until that person's own read of their repositories says they may see it.
- A tool argument never names a tenant or a person: those come from the forwarded identity.
- Ready `Routes` bind typed tool requests and readers to their output, permission and cache policy.
- Addresses come from `RepoResource` through the shared route declaration. Do not build or split a URI by hand.
- `skills/browse-repositories/` is the server's skill.

## Build And Test

- `cargo test -p veoveo-mcp-repo`, as the `dev` image provides it.
- `tests/mcp.rs` calls the server through the SDK's test Hub.
- `docker build --target check .` runs the gate.
