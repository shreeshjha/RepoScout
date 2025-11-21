# RepoScout

> Terminal-based Git repository discovery platform. Because clicking through GitHub's web UI is so 2015.

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

## What is this?

RepoScout lets you search, discover, and manage repositories across GitHub, GitLab, and Bitbucket without leaving your terminal. Think of it as a souped-up version of GitHub CLI with semantic search, offline caching, and a slick TUI.

## Features

- 🔍 **Multi-platform search** - Search GitHub, GitLab, Bitbucket simultaneously
- 📝 **Code search** - Search for code snippets across millions of repositories
- 🎨 **Beautiful TUI** - Terminal UI that doesn't look like it's from 1985
- 💾 **Smart caching** - Works offline with intelligent SQLite + FTS5 caching
- 📚 **Bookmarks** - Save and organize your favorite repositories
- 🔬 **Dependency analysis** - View project dependencies (Cargo, npm, pip)
- ⚡ **Fast** - Async I/O and parallel API requests
- 🔧 **Flexible** - Use as CLI or interactive TUI

## Installation

### From Source (for now)

```bash
git clone https://github.com/shreeshjha/RepoScout.git
cd RepoScout
cargo build --release
```

Binary will be at `target/release/reposcout`

## Quick Start

```bash
# Search for repositories
reposcout search "rust tui"

# Search for code within repositories (requires GitHub token)
export GITHUB_TOKEN="your_token_here"
reposcout code "fn main" --language rust --limit 10

# Search code in specific repository
reposcout code "authentication" --repo "rust-lang/rust"

# Search by file extension
reposcout code "error handling" --extension rs

# Show repository details
reposcout show "ratatui/ratatui"

# Launch interactive TUI
reposcout tui
```

## Configuration

RepoScout looks for config at `~/.config/reposcout/config.toml`

```toml
[platforms.github]
token = "ghp_your_token_here"

[platforms.gitlab]
token = "your_gitlab_token"
url = "https://gitlab.com"  # or your self-hosted instance

[cache]
ttl_hours = 24
max_size_mb = 500

[ui]
theme = "dark"
```

## Project Structure

This is a Rust workspace with multiple crates:

- `reposcout-cli` - Command-line interface
- `reposcout-core` - Core business logic and search engine
- `reposcout-tui` - Terminal UI using ratatui
- `reposcout-api` - API clients for various platforms
- `reposcout-cache` - SQLite-based caching layer

## Development

```bash
# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy

# Run locally
cargo run -- search "your query"
```

## Why Another Git Tool?

Good question. Here's why RepoScout exists:

1. **GitHub CLI is great but limited** - Only works with GitHub
2. **Web interfaces are slow** - Context switching between terminal and browser sucks
3. **Offline mode matters** - Airplane coding, slow connections, API rate limits
4. **Discovery is hard** - Finding quality repos matching your needs shouldn't require 20 browser tabs
5. **I wanted to build something cool in Rust** - Most honest reason

## Roadmap

- [x] Project structure and core architecture
- [x] Basic CLI with argument parsing
- [x] Error handling framework
- [x] Logging setup
- [x] GitHub API client
- [x] GitLab API client
- [x] SQLite caching with FTS5
- [x] Search engine core
- [x] Interactive TUI
- [x] Semantic search with embeddings
- [x] Local repository management
- [x] Collections and watchlists
- [x] Configuration system
- [x] Tests (lots of them)

## Contributing

PRs welcome! Just make sure:
- Code is formatted (`cargo fmt`)
- Lints pass (`cargo clippy`)
- Tests pass (`cargo test`)
- Commit messages are descriptive

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Acknowledgments

Built with:
- [clap](https://github.com/clap-rs/clap) - CLI parsing
- [ratatui](https://github.com/ratatui-org/ratatui) - TUI framework
- [tokio](https://github.com/tokio-rs/tokio) - Async runtime
- [reqwest](https://github.com/seanmonstar/reqwest) - HTTP client
- [rusqlite](https://github.com/rusqlite/rusqlite) - SQLite bindings

---

Made with ☕ and late-night coding sessions
