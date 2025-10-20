# RepoScout Roadmap

Feature backlog and implementation plan for future releases.

## 🎯 Next Up

### 1. GitLab Integration (In Progress)
Multi-platform support starting with GitLab API integration.

**Why:** Expand beyond GitHub to cover more of the open-source ecosystem
**Effort:** Medium
**Priority:** High

## 📋 Planned Features

### Phase 1: Platform Expansion

#### 2. Bitbucket Support
- Add Bitbucket Cloud API integration
- Support Bitbucket Server/Data Center
- Unified search across GitHub + GitLab + Bitbucket

**Why:** Complete the "big 3" Git platforms
**Effort:** Medium
**Priority:** Medium

### Phase 2: Core Enhancements

#### 3. Bookmarking/Favorites System
- Press `b` in TUI to bookmark repositories
- Local storage of favorite repos
- Quick access with `/bookmarks` or filter
- Export bookmarks to JSON/CSV
- Import bookmarks from file

**Why:** Users often want to save and return to interesting repos
**Effort:** Easy
**Priority:** High

#### 4. README Preview
- Fetch repository README in TUI
- Render markdown in preview pane
- Toggle between stats view and README view
- Cache README content for offline viewing

**Why:** Evaluate repos without leaving the terminal
**Effort:** Medium
**Priority:** High

#### 5. Clone Integration
- Press `c` in TUI to clone selected repo
- Configure default clone directory
- Choose SSH vs HTTPS
- Show clone progress
- Open cloned repo in editor (optional)

**Why:** Complete the workflow from search to local development
**Effort:** Easy
**Priority:** High

### Phase 3: Advanced Features

#### 6. Trending Repositories
- `reposcout trending` command
- Daily/weekly/monthly trending views
- Filter by language, date range
- Trending in TUI mode

**Why:** Discover popular new projects
**Effort:** Easy
**Priority:** Medium

#### 7. Fuzzy Search in Results
- Press `f` in TUI to fuzzy filter current results
- Typo-tolerant matching
- Incremental filtering as you type
- Clear with Esc

**Why:** Quick refinement of large result sets
**Effort:** Easy
**Priority:** Medium

#### 8. Export Results
- Export to CSV, JSON, Markdown formats
- `--export` flag in CLI
- Export from TUI with `e` key
- Include configurable fields

**Why:** Research, documentation, sharing findings
**Effort:** Easy
**Priority:** Low

#### 9. Search History
- Store previous search queries
- `Ctrl+R` to search history in TUI
- Auto-complete from history
- Clear history command

**Why:** Re-run common searches quickly
**Effort:** Easy
**Priority:** Medium

### Phase 4: Advanced Workflows

#### 10. Repository Comparison
- Select multiple repos with `Space`
- Press `Ctrl+C` to compare selected
- Side-by-side stats comparison
- Highlight differences

**Why:** Choose between similar projects
**Effort:** Medium
**Priority:** Low

#### 11. Statistics Dashboard
- Detailed repo statistics view
- Contributor count and top contributors
- Commit frequency graphs (ASCII art)
- Issue/PR statistics
- Language breakdown

**Why:** Deep dive into repository health
**Effort:** Medium
**Priority:** Low

#### 12. Custom Themes
- User-configurable color schemes
- Config file for theme settings
- Built-in themes: dark, light, solarized, gruvbox, nord
- Theme preview in TUI

**Why:** Personalization and accessibility
**Effort:** Easy
**Priority:** Low

### Phase 5: Background Features

#### 13. Watch & Notifications
- Watch repos for changes
- Notify on new releases
- Alert on significant star/fork changes
- Background daemon mode
- Desktop notifications

**Why:** Stay updated on interesting projects
**Effort:** Hard
**Priority:** Low

## 🚀 Future Ideas

- **AI-powered recommendations**: Suggest repos based on search history
- **Code search**: Search within repository code
- **Dependency analysis**: Show dependencies and dependents
- **License compatibility checker**: Check license conflicts
- **Contribution opportunities**: Find good first issues
- **Team collaboration**: Share searches and bookmarks
- **GitHub/GitLab sync**: Sync with platform stars/watches

## ✅ Completed

- [x] Initial project setup
- [x] GitHub API integration
- [x] SQLite caching with FTS5
- [x] Retry logic with exponential backoff
- [x] CLI search filters
- [x] Interactive TUI mode
- [x] TUI filters with panel
- [x] Live filter editing feedback

---

**Last Updated:** 2025-10-20
