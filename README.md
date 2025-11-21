# RepoScout

> Terminal-based Git repository discovery platform. Because clicking through GitHub's web UI is so 2015.

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

## What is this?

RepoScout lets you search, discover, and manage repositories across GitHub, GitLab, and Bitbucket without leaving your terminal. Think of it as GitHub CLI on steroids - with semantic search, trending discovery, health scoring, dependency analysis, and a TUI that doesn't look like it escaped from the 80s.

## Features

### Search & Discovery
- **Multi-platform search** - Search GitHub, GitLab, and Bitbucket simultaneously
- **Code search** - Search code snippets with syntax highlighting
- **Semantic search** - Natural language queries using AI embeddings (finally, search that understands what you actually want)
- **Trending repos** - Discover daily/weekly/monthly trending repositories
- **Discovery mode** - Browse New & Notable, Hidden Gems, Topics, and Awesome Lists

### Terminal UI
- **Beautiful TUI** - Modern terminal interface with ratatui
- **10+ themes** - Customizable color themes with full RGB support
- **Preview modes** - Stats, README, Activity, Dependencies, Package info
- **Fuzzy filtering** - Filter results in real-time
- **Keybindings help** - Press `?` for comprehensive help

### Data & Analysis
- **Smart caching** - SQLite + FTS5 for offline access and fast searches
- **Health scoring** - Repository quality metrics (0-100 score)
- **Dependency analysis** - View dependencies for 13 package managers
- **Package detection** - Auto-detect package managers with install commands
- **Bookmarks** - Save repos with tags and notes
- **Portfolio/Watchlist** - Organize repos into custom collections
- **Export** - JSON, CSV, and Markdown export

### Platform Features
- **GitHub notifications** - View and manage notifications
- **Token management** - Secure storage for API tokens
- **Search history** - Track and replay past searches

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
# Launch interactive TUI
reposcout tui

# Search for repositories
reposcout search "rust tui" --language rust --min-stars 100

# Search for code
reposcout code "async fn main" --language rust

# Semantic search (natural language)
reposcout semantic "machine learning image classification"

# Find trending repositories
reposcout trending --period weekly --language python

# Show repository details
reposcout show "ratatui/ratatui"

# Manage bookmarks
reposcout bookmark add "tokio-rs/tokio" --tags "async,runtime"
reposcout bookmark list
```

## TUI Usage

Launch with `reposcout tui`, then:

- **`/`** - Enter search mode
- **`M`** - Cycle search modes (Repository/Code/Trending/Semantic/Discovery)
- **`j/k`** - Navigate up/down
- **`TAB`** - Cycle preview tabs
- **`b`** - Bookmark repository
- **`R`** - Fetch README
- **`d`** - Fetch dependencies
- **`T`** - Open theme selector
- **`?`** - Show all keybindings
- **`q`** - Quit

### Search Modes

1. **Repository** - Search repos by name, description, topics
2. **Code** - Search code content across repositories
3. **Trending** - Browse trending repos by time period
4. **Semantic** - Natural language search using AI
5. **Discovery** - Explore curated categories
6. **Portfolio** - View your watchlists
7. **Notifications** - GitHub notifications

## CLI Commands

```bash
# Repository search with filters
reposcout search <query> [OPTIONS]
  -n, --limit <N>           # Results to show (default: 10)
  -l, --language <LANG>     # Filter by language
  --min-stars <N>           # Minimum stars
  --max-stars <N>           # Maximum stars
  --pushed <DATE>           # Filter by push date
  -s, --sort <BY>           # Sort: stars, forks, updated
  -o, --export <FILE>       # Export to .json/.csv/.md

# Code search
reposcout code <query> [OPTIONS]
  -l, --language <LANG>     # Filter by language
  -r, --repo <OWNER/REPO>   # Search in specific repo
  -p, --path <PATH>         # Filter by path
  -e, --extension <EXT>     # Filter by extension

# Semantic search
reposcout semantic <query> [OPTIONS]
  --hybrid                  # Combine semantic + keyword
  --min-similarity <0-1>    # Similarity threshold

# Trending repositories
reposcout trending [OPTIONS]
  -p, --period <P>          # daily, weekly, monthly
  -v, --velocity            # Sort by star velocity

# Bookmark management
reposcout bookmark list|add|remove|export|import|clear

# Cache management
reposcout cache stats|clear|cleanup

# Search history
reposcout history list|search|clear

# Notifications (GitHub)
reposcout notifications list|mark-read|mark-all-read
```

## Configuration

### API Tokens

Set tokens via environment variables or CLI flags:

```bash
export GITHUB_TOKEN="ghp_your_token"
export GITLAB_TOKEN="your_gitlab_token"
export BITBUCKET_USERNAME="username"
export BITBUCKET_APP_PASSWORD="app_password"
```

Or configure in TUI with `Ctrl+S`.

### Config File

Config at `~/.config/reposcout/config.toml`:

```toml
[platforms.github]
token = "ghp_your_token"

[platforms.gitlab]
token = "your_gitlab_token"
url = "https://gitlab.com"

[platforms.bitbucket]
username = "your_username"
app_password = "your_app_password"

[cache]
ttl_hours = 24

[ui]
theme = "Default Dark"
```

## Project Structure

```
reposcout/
├── crates/
│   ├── reposcout-cli/      # Command-line interface
│   ├── reposcout-core/     # Core logic, search, health scoring
│   ├── reposcout-tui/      # Terminal UI (ratatui)
│   ├── reposcout-api/      # API clients (GitHub, GitLab, Bitbucket)
│   ├── reposcout-cache/    # SQLite caching layer
│   ├── reposcout-semantic/ # Semantic search with embeddings
│   └── reposcout-deps/     # Dependency parsing
```

## Supported Package Managers

RepoScout can detect and analyze dependencies for:

- **Cargo** (Rust) - crates.io
- **npm** (JavaScript) - npmjs.com
- **PyPI** (Python) - pypi.org
- **Go** - pkg.go.dev
- **Maven/Gradle** (Java) - maven.org
- **RubyGems** (Ruby) - rubygems.org
- **Composer** (PHP) - packagist.org
- **NuGet** (.NET) - nuget.org
- **Pub** (Dart/Flutter) - pub.dev
- **CocoaPods** (iOS) - cocoapods.org
- **Swift PM** - swift.org
- **Hex** (Elixir) - hex.pm

## Development

```bash
# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy

# Run locally
cargo run -- tui
cargo run -- search "your query"
```

## Why Another Git Tool?

Good question. Here's why RepoScout exists:

1. **GitHub CLI is great but limited** - Only works with GitHub
2. **Web interfaces are slow** - Context switching between terminal and browser sucks
3. **Offline mode matters** - Airplane coding, slow connections, API rate limits
4. **Discovery is hard** - Finding quality repos matching your needs shouldn't require 20 browser tabs
5. **Semantic search changes everything** - Search by what you want to do, not just keywords
6. **I wanted to build something cool in Rust** - Most honest reason

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
- [ratatui](https://github.com/ratatui-org/ratatui) - TUI framework
- [tokio](https://github.com/tokio-rs/tokio) - Async runtime
- [reqwest](https://github.com/seanmonstar/reqwest) - HTTP client
- [rusqlite](https://github.com/rusqlite/rusqlite) - SQLite bindings
- [fastembed](https://github.com/Anush008/fastembed-rs) - Embeddings
- [clap](https://github.com/clap-rs/clap) - CLI parsing
- [syntect](https://github.com/trishume/syntect) - Syntax highlighting

---

Made with ☕ and late-night coding sessions
