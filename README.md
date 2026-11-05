# veoveo-mcp-lore

An MCP server for Veoveo: Lore for agents and people. It starts with seeing what is there: what the repositories a person can read
hold, and a gallery of them at `/apps/lore/repositories`, with a picture of each file. It reads as the person who asked, through the
Hub, so Lore's rules decide every read. See [crates/veoveo-mcp-lore/DESIGN.md](crates/veoveo-mcp-lore/DESIGN.md).

```sh
docker build --target check .
```

## License

Apache-2.0.
