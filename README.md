# veoveo-mcp-repo

An MCP server for Veoveo: Lore repositories for agents and people. It shows what people and their agents did in the repositories a person can read, and what
those repositories hold, at `/apps/repo/activity`. It reads as the person who asked, through the
Hub, so Lore's rules decide every read. See [crates/veoveo-mcp-repo/DESIGN.md](crates/veoveo-mcp-repo/DESIGN.md).

```sh
docker build --target check .
```

## License

Apache-2.0.
