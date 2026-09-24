# rss-mcp

An [MCP](https://modelcontextprotocol.io) server, written in Rust, that lets an
AI assistant (such as Claude Code) read RSS and Atom feeds listed in a TOML file.

> Learning project — the code favours clarity over features.

## Features

| Tool         | Description                                       | Status  |
|--------------|---------------------------------------------------|---------|
| `list_feeds` | List configured feeds (name, URL, tags)           | ✅      |
| `get_feed`   | Fetch a feed and return its latest items          | ✅      |
| `get_latest` | Merge several feeds (optionally by tag), by date  | planned |

RSS 0.9x/1.0/2.0, Atom and JSON Feed are supported through
[`feed-rs`](https://crates.io/crates/feed-rs).

## Requirements

- Rust (stable). The toolchain is pinned in `rust-toolchain.toml`; run
  `rustup toolchain install` once in this directory.

## Configuration

Copy the example and edit the list:

```sh
cp feeds.example.toml feeds.toml
```

```toml
[settings]            # optional
timeout_secs = 10     # HTTP timeout per feed
max_items = 20        # default number of items per feed

[[feeds]]
name = "rust-blog"    # unique identifier used by the tools
url = "https://blog.rust-lang.org/feed.xml"
tags = ["rust"]       # optional
```

The configuration path is given with `--config <path>` or the
`RSS_MCP_CONFIG` environment variable. The file is validated at startup
(at least one feed, unique names, `http`/`https` URLs, no unknown keys).

## Running

```sh
cargo build --release
./target/release/rss-mcp --config feeds.toml
```

The server speaks JSON-RPC over **stdio**. Logs are written to **stderr**
(stdout is reserved for the protocol); adjust verbosity with `RUST_LOG`,
e.g. `RUST_LOG=rss_mcp=debug`.

## Using with Claude Code

### Inside this repository

The project ships a `.mcp.json` that starts the server with `cargo run`, so
Claude Code always uses the current code. Start `claude` in this directory,
approve the `rss` server when prompted, then check it with `/mcp`.

After changing the code, reconnect the server from `/mcp` to rebuild it.

### From anywhere

Register the release binary at user scope:

```sh
claude mcp add rss --scope user -- /absolute/path/to/rss-mcp/target/release/rss-mcp --config /absolute/path/to/feeds.toml
```

Example prompts:

- "Which RSS feeds are available?"
- "Summarize the latest 3 posts from rust-blog."

## Development

```sh
cargo test                  # unit + integration tests (no network needed)
cargo clippy --all-targets
cargo fmt
```

To call the tools by hand, use the
[MCP Inspector](https://github.com/modelcontextprotocol/inspector):

```sh
npx @modelcontextprotocol/inspector ./target/debug/rss-mcp --config feeds.toml
```

### Layout

```
src/
├── main.rs     # CLI arguments, logging, stdio transport
├── lib.rs      # module declarations (lets tests/ use the crate)
├── config.rs   # TOML loading and validation
├── feed.rs     # HTTP fetching and RSS/Atom parsing — no MCP code
└── server.rs   # MCP tools: thin adapter over config + feed
tests/
├── feed_tests.rs   # parsing fixtures, HTTP via a wiremock server
├── server_tests.rs # MCP tools of RssServer
└── fixtures/       # sample RSS 2.0 and Atom documents
```

`feed.rs` has no dependency on MCP, so the logic can be tested without a
server; `server.rs` only maps it to tools.

## License

Copyright (C) 2026 Alexandre MATHIEU

This program is free software: you can redistribute it and/or modify it under
the terms of the GNU General Public License as published by the Free Software
Foundation, either version 3 of the License, or (at your option) any later
version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY
WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A
PARTICULAR PURPOSE. See the [LICENSE](LICENSE) file for details.
