# RepoScout Testing Guide

Quick reference for building and testing RepoScout features.

## Build Commands

```bash
# Release build (optimized)
cargo build --release

# Debug build
cargo build

# Run tests
cargo test

# Check formatting and linting
cargo fmt --all -- --check
cargo clippy --all-targets --all-features
```

## Platform Support

RepoScout searches across **multiple platforms simultaneously**:
- **GitHub** - Default, works without token (rate-limited)
- **GitLab** - Searches GitLab.com projects

### Setting up Tokens (Optional but Recommended)

For higher rate limits and private repository access:

```bash
# GitHub token
export GITHUB_TOKEN="your_github_token"

# GitLab token
export GITLAB_TOKEN="your_gitlab_token"

# Or use command-line flags
./target/release/reposcout --github-token "token1" --gitlab-token "token2" search "rust"
```

## Feature Testing

### 1. Multi-Platform Search

RepoScout automatically searches **both GitHub and GitLab** in parallel:

```bash
# Searches GitHub + GitLab simultaneously
./target/release/reposcout search "rust cli" -n 10

# You'll see results from both platforms mixed together
# Look for (GitHub) and (GitLab) labels in results
```

**Note**: Without tokens, you get ~30 results from GitHub + ~30 from GitLab = ~60 total results!

### 2. Repository Details

Works for both GitHub and GitLab repositories:

```bash
# GitHub repository
./target/release/reposcout show "ratatui/ratatui"

# GitLab repository
./target/release/reposcout show "gitlab-org/gitlab"
```

### 3. Cache Performance Test
```bash
# First run (API call)
time ./target/release/reposcout search "rust tui" -n 5

# Second run (cache hit - ~73x faster)
time ./target/release/reposcout search "rust tui" -n 5

# Cache management
./target/release/reposcout cache stats
./target/release/reposcout cache clear
./target/release/reposcout cache cleanup
```

### 4. Search Filters

**By Language:**
```bash
./target/release/reposcout search "terminal" --language rust -n 5
./target/release/reposcout search "web framework" -l python -n 5
```

**By Stars:**
```bash
# Min stars
./target/release/reposcout search "cli" --min-stars 10000 -n 5

# Stars range
./target/release/reposcout search "template" --min-stars 100 --max-stars 5000
```

**By Date:**
```bash
./target/release/reposcout search "ai" --pushed ">2024-01-01" -n 5
```

**Sort Options:**
```bash
./target/release/reposcout search "cli" --sort stars -n 5
./target/release/reposcout search "cli" --sort forks -n 5
./target/release/reposcout search "cli" --sort updated -n 5
```

**Combined:**
```bash
./target/release/reposcout search "terminal" \
  --language rust \
  --min-stars 5000 \
  --pushed ">2024-01-01" \
  --sort stars \
  -n 10
```

### 5. Interactive TUI (Multi-Platform)

The TUI searches both GitHub and GitLab simultaneously:

```bash
./target/release/reposcout tui
```

**Note**: Results from both platforms appear in the same list, sorted by stars.

**TUI Controls:**

**Search Mode:**
- Type to search, press `Enter`
- Press `Esc` to switch to navigation mode

**Navigation Mode:**
- `j` / `k` or arrow keys to navigate results
- `/` to enter search mode
- `F` to toggle filters panel
- `R` to fetch and display README (fetches automatically on first press, cached afterward)
- `Enter` to open repository in browser
- `q` to quit

**Filter Mode (press `F` to toggle):**
- `j` / `k` or arrow keys to navigate between filters
- `Tab` to move to next filter
- `Enter` to edit selected filter (see live typing with cursor █)
- `d` or `Delete` to clear selected filter
- `s` to cycle sort options (when on Sort By field)
- `Esc` to close filters and return to navigation

**Editing Filter (when typing):**
- Type your value - see it appear in real-time
- `Backspace` to delete characters
- `Enter` to save the value
- `Esc` to cancel without saving

**Available Filters:**
- **Language**: Filter by programming language (e.g., rust, python, go)
- **Min Stars**: Minimum star count
- **Max Stars**: Maximum star count
- **Pushed**: Date filter (e.g., >2024-01-01, <2023-12-31)
- **Sort By**: Sort results by stars, forks, or updated

**Test Scenarios:**
1. **Basic search with filters**:
   - Launch TUI: `./target/release/reposcout tui`
   - Type "terminal" and press Enter
   - Press `F` to open filters
   - Navigate to Language with `j`
   - Press Enter, type "rust", press Enter
   - Navigate to Min Stars
   - Press Enter, type "5000", press Enter
   - Press `Esc` and then `/` to search again
   - Results should now show only Rust terminal projects with 5k+ stars

2. **Change sorting**:
   - Open filters with `F`
   - Navigate to Sort By
   - Press `s` to cycle through: stars → forks → updated
   - Press `Esc` to apply sorting

3. **Clear filters**:
   - Open filters with `F`
   - Navigate to any filter
   - Press `d` to clear it

4. **README preview**:
   - Launch TUI and search for a repository (e.g., "ratatui")
   - Navigate to a repository with `j`/`k`
   - Press `R` to fetch and view the README
   - README is fetched from the API and cached automatically
   - Press `R` again to toggle back to stats view
   - Supports both GitHub and GitLab repositories
   - Basic markdown rendering: headers, code blocks, lists

## GitHub Token (Optional)

For higher rate limits:

```bash
export GITHUB_TOKEN="your_github_token"
./target/release/reposcout search "rust"

# Or use flag
./target/release/reposcout --github-token "your_token" search "rust"
```

## Cache Location

- **macOS/Linux**: `~/.cache/reposcout/reposcout.db`
- **Windows**: `%LOCALAPPDATA%\reposcout\reposcout.db`

## Verification Checklist

- [ ] Build completes: `cargo build --release`
- [ ] Tests pass: `cargo test`
- [ ] Basic search returns results
- [ ] Cache works (second search faster)
- [ ] Filters work (language, stars, date)
- [ ] TUI launches and keyboard works
- [ ] Browser opens from TUI

## Performance Expectations

- **First search**: ~1 second (API call)
- **Cached search**: ~13ms (73x faster)
- **TUI responsiveness**: Instant navigation
