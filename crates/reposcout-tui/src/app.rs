// TUI application state and event handling
use reposcout_core::models::{Repository, CodeSearchResult};
use reposcout_deps::DependencyInfo;
use reposcout_cache::SearchHistoryEntry;
use ratatui::widgets::ListState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    Repository,     // Searching for repositories (default)
    Code,           // Searching for code
    Trending,       // Browsing trending repositories
    Notifications,  // Viewing GitHub notifications
    Semantic,       // Semantic search with natural language
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,        // Navigating results
    Searching,     // Typing in search box
    Filtering,     // Navigating filters
    EditingFilter, // Actively typing in a filter field
    FuzzySearch,   // Fuzzy filtering current results
    HistoryPopup,  // Browsing search history
    Settings,      // Settings/token management popup
    TokenInput,    // Entering API token
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewMode {
    Stats,         // Show repository statistics
    Readme,        // Show README content
    Activity,      // Show repository activity/commits
    Dependencies,  // Show dependency analysis
    Package,       // Show package manager info and install commands
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodePreviewMode {
    Code,       // Show highlighted code with context
    Raw,        // Show raw text
    FileInfo,   // Show file metadata and repository info
}

#[derive(Debug, Clone)]
pub struct SearchFilters {
    pub language: Option<String>,
    pub min_stars: Option<u32>,
    pub max_stars: Option<u32>,
    pub pushed: Option<String>,
    pub sort_by: String,
}

impl Default for SearchFilters {
    fn default() -> Self {
        Self {
            language: None,
            min_stars: None,
            max_stars: None,
            pushed: None,
            sort_by: "stars".to_string(),
        }
    }
}

impl SearchFilters {
    pub fn build_query(&self, base_query: &str) -> String {
        let mut parts = vec![base_query.to_string()];

        if let Some(lang) = &self.language {
            if !lang.is_empty() {
                parts.push(format!("language:{}", lang));
            }
        }

        match (self.min_stars, self.max_stars) {
            (Some(min), Some(max)) => parts.push(format!("stars:{}..{}", min, max)),
            (Some(min), None) => parts.push(format!("stars:>={}", min)),
            (None, Some(max)) => parts.push(format!("stars:<={}", max)),
            (None, None) => {}
        }

        if let Some(pushed) = &self.pushed {
            if !pushed.is_empty() {
                parts.push(format!("pushed:{}", pushed));
            }
        }

        parts.join(" ")
    }

    pub fn sort_results(&self, results: &mut [Repository]) {
        match self.sort_by.as_str() {
            "stars" => results.sort_by(|a, b| b.stars.cmp(&a.stars)),
            "forks" => results.sort_by(|a, b| b.forks.cmp(&a.forks)),
            "updated" => results.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
            _ => {}
        }
    }
}

#[derive(Debug, Clone)]
pub struct CodeSearchFilters {
    pub language: Option<String>,
    pub repo: Option<String>,
    pub path: Option<String>,
    pub extension: Option<String>,
}

impl Default for CodeSearchFilters {
    fn default() -> Self {
        Self {
            language: None,
            repo: None,
            path: None,
            extension: None,
        }
    }
}

impl CodeSearchFilters {
    pub fn build_query(&self, base_query: &str) -> String {
        let mut parts = vec![base_query.to_string()];

        if let Some(lang) = &self.language {
            if !lang.is_empty() {
                parts.push(format!("language:{}", lang));
            }
        }

        if let Some(repository) = &self.repo {
            if !repository.is_empty() {
                parts.push(format!("repo:{}", repository));
            }
        }

        if let Some(path_filter) = &self.path {
            if !path_filter.is_empty() {
                parts.push(format!("path:{}", path_filter));
            }
        }

        if let Some(ext) = &self.extension {
            if !ext.is_empty() {
                parts.push(format!("extension:{}", ext));
            }
        }

        parts.join(" ")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendingPeriod {
    Daily,
    Weekly,
    Monthly,
}

impl TrendingPeriod {
    pub fn display_name(&self) -> &'static str {
        match self {
            TrendingPeriod::Daily => "Daily",
            TrendingPeriod::Weekly => "Weekly",
            TrendingPeriod::Monthly => "Monthly",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            TrendingPeriod::Daily => TrendingPeriod::Weekly,
            TrendingPeriod::Weekly => TrendingPeriod::Monthly,
            TrendingPeriod::Monthly => TrendingPeriod::Daily,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TrendingFilters {
    pub period: TrendingPeriod,
    pub language: Option<String>,
    pub min_stars: u32,
    pub topic: Option<String>,
    pub sort_by_velocity: bool,
}

impl Default for TrendingFilters {
    fn default() -> Self {
        Self {
            period: TrendingPeriod::Weekly,
            language: None,
            min_stars: 100,
            topic: None,
            sort_by_velocity: false,
        }
    }
}

pub struct App {
    pub should_quit: bool,
    pub input_mode: InputMode,
    pub search_mode: SearchMode,
    pub search_input: String,
    pub results: Vec<Repository>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub loading: bool,
    pub error_message: Option<String>,
    pub error_timestamp: Option<std::time::SystemTime>,
    pub filters: SearchFilters,
    pub show_filters: bool,
    pub filter_cursor: usize,
    pub filter_edit_buffer: String,
    pub list_state: ListState,
    pub preview_mode: PreviewMode,
    pub readme_content: Option<String>,
    pub readme_loading: bool,
    // Cache README content per repository to avoid re-fetching
    pub readme_cache: std::collections::HashMap<String, String>,
    // Scroll position for README view
    pub readme_scroll: u16,
    // Track bookmarked repositories (platform + full_name)
    pub bookmarked: std::collections::HashSet<String>,
    // Show bookmarks only
    pub show_bookmarks_only: bool,
    // Fuzzy search state
    pub fuzzy_input: String,
    pub all_results: Vec<Repository>, // Store original results before fuzzy filtering
    pub fuzzy_match_count: usize,
    // Dependency analysis state
    pub dependencies_cache: std::collections::HashMap<String, Option<DependencyInfo>>,
    pub dependencies_loading: bool,
    // Package manager integration
    pub package_info_cache: std::collections::HashMap<String, Vec<reposcout_core::PackageInfo>>,
    pub package_loading: bool,
    // Code search state
    pub code_results: Vec<CodeSearchResult>,
    pub code_filters: CodeSearchFilters,
    pub code_selected_index: usize,
    pub code_scroll: u16,
    pub code_preview_mode: CodePreviewMode,
    pub show_code_filters: bool,
    pub code_filter_cursor: usize,
    pub code_filter_edit_buffer: String,
    pub code_match_index: usize, // Which match within a file to highlight
    // Full file content cache for code preview
    pub code_content_cache: std::collections::HashMap<String, String>,
    // Platform status tracking
    pub platform_status: PlatformStatus,
    // Search history popup state
    pub search_history: Vec<SearchHistoryEntry>,
    pub history_selected_index: usize,
    // Trending state
    pub trending_filters: TrendingFilters,
    pub show_trending_options: bool,
    pub trending_option_cursor: usize,
    // Settings/Token management state
    pub show_settings: bool,
    pub settings_cursor: usize,
    pub token_input_buffer: String,
    pub token_input_platform: String, // "github", "gitlab", or "bitbucket"
    pub token_status_message: Option<String>,
    // Notification state
    pub notifications: Vec<reposcout_core::Notification>,
    pub notifications_selected_index: usize,
    pub notifications_loading: bool,
    pub notifications_show_all: bool, // false = unread only, true = all
    pub notifications_participating: bool, // filter to participating only
}

#[derive(Debug, Clone)]
pub struct PlatformStatus {
    pub github_configured: bool,
    pub gitlab_configured: bool,
    pub bitbucket_configured: bool,
}

impl App {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            should_quit: false,
            input_mode: InputMode::Searching,
            search_mode: SearchMode::Repository,
            search_input: String::new(),
            results: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            loading: false,
            error_message: None,
            error_timestamp: None,
            filters: SearchFilters::default(),
            show_filters: false,
            filter_cursor: 0,
            filter_edit_buffer: String::new(),
            list_state,
            preview_mode: PreviewMode::Stats,
            readme_content: None,
            readme_loading: false,
            readme_cache: std::collections::HashMap::new(),
            readme_scroll: 0,
            bookmarked: std::collections::HashSet::new(),
            show_bookmarks_only: false,
            fuzzy_input: String::new(),
            all_results: Vec::new(),
            fuzzy_match_count: 0,
            dependencies_cache: std::collections::HashMap::new(),
            dependencies_loading: false,
            package_info_cache: std::collections::HashMap::new(),
            package_loading: false,
            code_results: Vec::new(),
            code_filters: CodeSearchFilters::default(),
            code_selected_index: 0,
            code_scroll: 0,
            code_preview_mode: CodePreviewMode::Code,
            show_code_filters: false,
            code_filter_cursor: 0,
            code_filter_edit_buffer: String::new(),
            code_match_index: 0,
            code_content_cache: std::collections::HashMap::new(),
            platform_status: PlatformStatus {
                github_configured: true,  // Always available (public repos don't need auth)
                gitlab_configured: true,  // Always available (public repos don't need auth)
                bitbucket_configured: false,
            },
            search_history: Vec::new(),
            history_selected_index: 0,
            trending_filters: TrendingFilters::default(),
            show_trending_options: false,
            trending_option_cursor: 0,
            show_settings: false,
            settings_cursor: 0,
            token_input_buffer: String::new(),
            token_input_platform: String::new(),
            token_status_message: None,
            notifications: Vec::new(),
            notifications_selected_index: 0,
            notifications_loading: false,
            notifications_show_all: false,
            notifications_participating: true,
        }
    }

    pub fn set_platform_status(&mut self, github: bool, gitlab: bool, bitbucket: bool) {
        self.platform_status = PlatformStatus {
            github_configured: github,
            gitlab_configured: gitlab,
            bitbucket_configured: bitbucket,
        };
    }

    /// Enter fuzzy search mode
    pub fn enter_fuzzy_mode(&mut self) {
        self.input_mode = InputMode::FuzzySearch;
        self.fuzzy_input.clear();
        // Store all current results
        self.all_results = self.results.clone();
        self.fuzzy_match_count = self.results.len();
    }

    /// Exit fuzzy search mode
    pub fn exit_fuzzy_mode(&mut self) {
        self.input_mode = InputMode::Normal;
        self.fuzzy_input.clear();
        // Restore all results
        if !self.all_results.is_empty() {
            self.results = self.all_results.clone();
            self.all_results.clear();
        }
        self.selected_index = 0;
        self.list_state.select(Some(0));
    }

    /// Apply fuzzy filter to results
    pub fn apply_fuzzy_filter(&mut self) {
        use fuzzy_matcher::FuzzyMatcher;
        use fuzzy_matcher::skim::SkimMatcherV2;

        if self.fuzzy_input.is_empty() {
            // No filter, show all results
            self.results = self.all_results.clone();
            self.fuzzy_match_count = self.results.len();
        } else {
            let matcher = SkimMatcherV2::default();
            let query = self.fuzzy_input.to_lowercase();

            // Filter and score results
            let mut scored_results: Vec<(Repository, i64)> = self
                .all_results
                .iter()
                .filter_map(|repo| {
                    // Match against repo name and description
                    let name_score = matcher.fuzzy_match(&repo.full_name.to_lowercase(), &query);
                    let desc_score = repo
                        .description
                        .as_ref()
                        .and_then(|d| matcher.fuzzy_match(&d.to_lowercase(), &query));

                    // Take the best score
                    let score = name_score.or(desc_score)?;
                    Some((repo.clone(), score))
                })
                .collect();

            // Sort by score (highest first)
            scored_results.sort_by(|a, b| b.1.cmp(&a.1));

            self.results = scored_results.into_iter().map(|(repo, _)| repo).collect();
            self.fuzzy_match_count = self.results.len();
        }

        // Reset selection
        self.selected_index = 0;
        self.list_state.select(Some(0));
    }

    /// Get bookmark key for a repository
    pub fn bookmark_key(platform: &str, full_name: &str) -> String {
        format!("{}:{}", platform, full_name)
    }

    /// Check if current repository is bookmarked
    pub fn is_current_bookmarked(&self) -> bool {
        if let Some(repo) = self.selected_repository() {
            let key = Self::bookmark_key(&repo.platform.to_string().to_lowercase(), &repo.full_name);
            self.bookmarked.contains(&key)
        } else {
            false
        }
    }

    /// Add/remove current repository from bookmarks
    pub fn toggle_current_bookmark(&mut self) {
        if let Some(repo) = self.selected_repository() {
            let key = Self::bookmark_key(&repo.platform.to_string().to_lowercase(), &repo.full_name);
            if self.bookmarked.contains(&key) {
                self.bookmarked.remove(&key);
            } else {
                self.bookmarked.insert(key);
            }
        }
    }

    /// Toggle showing bookmarks only
    pub fn toggle_bookmarks_view(&mut self) {
        self.show_bookmarks_only = !self.show_bookmarks_only;
    }

    pub fn toggle_preview_mode(&mut self) {
        self.preview_mode = match self.preview_mode {
            PreviewMode::Stats => PreviewMode::Readme,
            PreviewMode::Readme => PreviewMode::Activity,
            PreviewMode::Activity => PreviewMode::Dependencies,
            PreviewMode::Dependencies => PreviewMode::Package,
            PreviewMode::Package => PreviewMode::Stats,
        };
    }

    pub fn next_preview_tab(&mut self) {
        self.preview_mode = match self.preview_mode {
            PreviewMode::Stats => PreviewMode::Readme,
            PreviewMode::Readme => PreviewMode::Activity,
            PreviewMode::Activity => PreviewMode::Dependencies,
            PreviewMode::Dependencies => PreviewMode::Package,
            PreviewMode::Package => PreviewMode::Stats,
        };
        self.reset_readme_scroll();

        // Auto-detect package info when switching to Package tab
        if self.preview_mode == PreviewMode::Package {
            if self.get_cached_package_info().is_none() {
                self.detect_package_info();
            }
        }
    }

    pub fn previous_preview_tab(&mut self) {
        self.preview_mode = match self.preview_mode {
            PreviewMode::Stats => PreviewMode::Package,
            PreviewMode::Package => PreviewMode::Dependencies,
            PreviewMode::Dependencies => PreviewMode::Activity,
            PreviewMode::Activity => PreviewMode::Readme,
            PreviewMode::Readme => PreviewMode::Stats,
        };
        self.reset_readme_scroll();
    }

    pub fn set_readme(&mut self, content: String) {
        self.readme_content = Some(content);
        self.readme_loading = false;
    }

    pub fn clear_readme(&mut self) {
        self.readme_content = None;
        self.readme_loading = false;
    }

    /// Check if README is cached for the currently selected repository
    pub fn get_cached_readme(&self) -> Option<&String> {
        if let Some(repo) = self.selected_repository() {
            self.readme_cache.get(&repo.full_name)
        } else {
            None
        }
    }

    /// Cache README content for a repository
    pub fn cache_readme(&mut self, repo_name: String, content: String) {
        self.readme_cache.insert(repo_name, content);
    }

    /// Start README loading for current repository
    pub fn start_readme_loading(&mut self) {
        self.readme_loading = true;
        self.readme_content = None;
    }

    /// Set README from cache or fetched content
    pub fn load_readme_for_current(&mut self) {
        if let Some(repo) = self.selected_repository() {
            if let Some(cached) = self.readme_cache.get(&repo.full_name) {
                self.readme_content = Some(cached.clone());
                self.readme_loading = false;
            } else {
                // Mark as loading - will be fetched async
                self.start_readme_loading();
            }
        }
    }

    /// Scroll README down
    pub fn scroll_readme_down(&mut self) {
        self.readme_scroll = self.readme_scroll.saturating_add(1);
    }

    /// Scroll README up
    pub fn scroll_readme_up(&mut self) {
        self.readme_scroll = self.readme_scroll.saturating_sub(1);
    }

    /// Reset README scroll position
    pub fn reset_readme_scroll(&mut self) {
        self.readme_scroll = 0;
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn enter_search_mode(&mut self) {
        self.input_mode = InputMode::Searching;
    }

    pub fn enter_normal_mode(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    pub fn enter_filter_mode(&mut self) {
        self.input_mode = InputMode::Filtering;
    }

    pub fn enter_editing_filter_mode(&mut self) {
        self.input_mode = InputMode::EditingFilter;
        // Load current filter value into edit buffer
        self.filter_edit_buffer = match self.filter_cursor {
            0 => self.filters.language.clone().unwrap_or_default(),
            1 => self.filters.min_stars.map(|s| s.to_string()).unwrap_or_default(),
            2 => self.filters.max_stars.map(|s| s.to_string()).unwrap_or_default(),
            3 => self.filters.pushed.clone().unwrap_or_default(),
            4 => self.filters.sort_by.clone(),
            _ => String::new(),
        };
    }

    pub fn save_filter_edit(&mut self) {
        // Save the edit buffer to the actual filter
        match self.filter_cursor {
            0 => {
                self.filters.language = if self.filter_edit_buffer.is_empty() {
                    None
                } else {
                    Some(self.filter_edit_buffer.clone())
                };
            }
            1 => {
                self.filters.min_stars = self.filter_edit_buffer.parse().ok();
            }
            2 => {
                self.filters.max_stars = self.filter_edit_buffer.parse().ok();
            }
            3 => {
                self.filters.pushed = if self.filter_edit_buffer.is_empty() {
                    None
                } else {
                    Some(self.filter_edit_buffer.clone())
                };
            }
            4 => {
                if !self.filter_edit_buffer.is_empty() {
                    self.filters.sort_by = self.filter_edit_buffer.clone();
                }
            }
            _ => {}
        }
        self.filter_edit_buffer.clear();
        self.input_mode = InputMode::Filtering;
    }

    pub fn cancel_filter_edit(&mut self) {
        self.filter_edit_buffer.clear();
        self.input_mode = InputMode::Filtering;
    }

    pub fn toggle_filters(&mut self) {
        self.show_filters = !self.show_filters;
    }

    pub fn next_filter(&mut self) {
        self.filter_cursor = (self.filter_cursor + 1).min(4); // 5 filter fields
    }

    pub fn previous_filter(&mut self) {
        if self.filter_cursor > 0 {
            self.filter_cursor -= 1;
        }
    }

    pub fn cycle_sort(&mut self) {
        self.filters.sort_by = match self.filters.sort_by.as_str() {
            "stars" => "forks".to_string(),
            "forks" => "updated".to_string(),
            _ => "stars".to_string(),
        };
    }

    pub fn clear_current_filter(&mut self) {
        match self.filter_cursor {
            0 => self.filters.language = None,
            1 => self.filters.min_stars = None,
            2 => self.filters.max_stars = None,
            3 => self.filters.pushed = None,
            4 => self.filters.sort_by = "stars".to_string(),
            _ => {}
        }
    }

    pub fn next_result(&mut self) {
        if !self.results.is_empty() {
            self.selected_index = (self.selected_index + 1).min(self.results.len() - 1);
            self.list_state.select(Some(self.selected_index));
        }
    }

    pub fn previous_result(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.list_state.select(Some(self.selected_index));
        }
    }

    pub fn selected_repository(&self) -> Option<&Repository> {
        self.results.get(self.selected_index)
    }

    pub fn set_results(&mut self, mut results: Vec<Repository>) {
        // Apply sorting based on filters
        self.filters.sort_results(&mut results);
        self.results = results;
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.list_state.select(Some(0));
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
        self.error_timestamp = None;
    }

    /// Set a temporary error message that will auto-clear after 5 seconds
    pub fn set_temp_error(&mut self, message: String) {
        self.error_message = Some(message);
        self.error_timestamp = Some(std::time::SystemTime::now());
    }

    /// Set a permanent error message that won't auto-clear
    pub fn set_error(&mut self, message: String) {
        self.error_message = Some(message);
        self.error_timestamp = None;
    }

    /// Clear error if it has been shown for more than 5 seconds
    pub fn clear_expired_error(&mut self) {
        if let Some(timestamp) = self.error_timestamp {
            if let Ok(elapsed) = timestamp.elapsed() {
                if elapsed.as_secs() >= 5 {
                    self.clear_error();
                }
            }
        }
    }

    pub fn get_search_query(&self) -> String {
        self.filters.build_query(&self.search_input)
    }

    /// Get cached dependencies for current repository
    pub fn get_cached_dependencies(&self) -> Option<&Option<DependencyInfo>> {
        if let Some(repo) = self.selected_repository() {
            self.dependencies_cache.get(&repo.full_name)
        } else {
            None
        }
    }

    /// Cache dependencies for a repository
    pub fn cache_dependencies(&mut self, repo_name: String, deps: Option<DependencyInfo>) {
        self.dependencies_cache.insert(repo_name, deps);
    }

    /// Start dependency loading
    pub fn start_dependencies_loading(&mut self) {
        self.dependencies_loading = true;
    }

    /// Stop dependency loading
    pub fn stop_dependencies_loading(&mut self) {
        self.dependencies_loading = false;
    }

    /// Get cached package info for current repository
    pub fn get_cached_package_info(&self) -> Option<&Vec<reposcout_core::PackageInfo>> {
        if let Some(repo) = self.selected_repository() {
            self.package_info_cache.get(&repo.full_name)
        } else {
            None
        }
    }

    /// Cache package info for a repository
    pub fn cache_package_info(&mut self, repo_name: String, packages: Vec<reposcout_core::PackageInfo>) {
        self.package_info_cache.insert(repo_name, packages);
    }

    /// Start package loading
    pub fn start_package_loading(&mut self) {
        self.package_loading = true;
    }

    /// Stop package loading
    pub fn stop_package_loading(&mut self) {
        self.package_loading = false;
    }

    /// Detect and cache package info for current repository
    pub fn detect_package_info(&mut self) {
        if let Some(repo) = self.selected_repository() {
            let managers = reposcout_core::PackageDetector::detect(repo);

            let mut packages = Vec::new();
            for manager in managers {
                if let Some(pkg_name) = reposcout_core::PackageDetector::extract_package_name(repo, manager) {
                    let pkg_info = reposcout_core::PackageInfo::new(manager, pkg_name);
                    packages.push(pkg_info);
                }
            }

            if !packages.is_empty() {
                self.cache_package_info(repo.full_name.clone(), packages);
            }
        }
    }

    /// Toggle between repository, code, trending, notifications, and semantic search modes
    pub fn toggle_search_mode(&mut self) {
        self.search_mode = match self.search_mode {
            SearchMode::Repository => SearchMode::Code,
            SearchMode::Code => SearchMode::Trending,
            SearchMode::Trending => SearchMode::Notifications,
            SearchMode::Notifications => SearchMode::Semantic,
            SearchMode::Semantic => SearchMode::Repository,
        };
        // Clear results and errors when switching modes
        self.code_results.clear();
        self.results.clear();
        self.notifications.clear();
        self.code_selected_index = 0;
        self.selected_index = 0;
        self.notifications_selected_index = 0;
        self.error_message = None;
        self.loading = false;
    }

    /// Get the currently selected code search result
    pub fn selected_code_result(&self) -> Option<&CodeSearchResult> {
        self.code_results.get(self.code_selected_index)
    }

    /// Navigate to next code search result
    pub fn next_code_result(&mut self) {
        if !self.code_results.is_empty() {
            self.code_selected_index = (self.code_selected_index + 1).min(self.code_results.len() - 1);
        }
    }

    /// Navigate to previous code search result
    pub fn previous_code_result(&mut self) {
        if self.code_selected_index > 0 {
            self.code_selected_index -= 1;
        }
    }

    /// Set code search results
    pub fn set_code_results(&mut self, results: Vec<CodeSearchResult>) {
        self.code_results = results;
        self.code_selected_index = 0;
    }

    /// Scroll code preview down
    pub fn scroll_code_down(&mut self) {
        self.code_scroll = self.code_scroll.saturating_add(1);
    }

    /// Scroll code preview up
    pub fn scroll_code_up(&mut self) {
        self.code_scroll = self.code_scroll.saturating_sub(1);
    }

    /// Reset code scroll position
    pub fn reset_code_scroll(&mut self) {
        self.code_scroll = 0;
    }

    /// Get code search query with filters
    pub fn get_code_search_query(&self) -> String {
        self.code_filters.build_query(&self.search_input)
    }

    /// Toggle code filter panel visibility
    pub fn toggle_code_filters(&mut self) {
        self.show_code_filters = !self.show_code_filters;
        if self.show_code_filters {
            self.code_filter_cursor = 0;
        }
    }

    /// Navigate to next code filter field
    pub fn next_code_filter(&mut self) {
        self.code_filter_cursor = (self.code_filter_cursor + 1).min(3); // 4 filter fields
    }

    /// Navigate to previous code filter field
    pub fn previous_code_filter(&mut self) {
        if self.code_filter_cursor > 0 {
            self.code_filter_cursor -= 1;
        }
    }

    /// Clear current code filter
    pub fn clear_current_code_filter(&mut self) {
        match self.code_filter_cursor {
            0 => self.code_filters.language = None,
            1 => self.code_filters.repo = None,
            2 => self.code_filters.path = None,
            3 => self.code_filters.extension = None,
            _ => {}
        }
    }

    /// Toggle code preview mode (Code/Raw/FileInfo)
    pub fn toggle_code_preview_mode(&mut self) {
        self.code_preview_mode = match self.code_preview_mode {
            CodePreviewMode::Code => CodePreviewMode::Raw,
            CodePreviewMode::Raw => CodePreviewMode::FileInfo,
            CodePreviewMode::FileInfo => CodePreviewMode::Code,
        };
        // Reset scroll when switching modes
        self.code_scroll = 0;
    }

    /// Navigate to next match within the current code result
    pub fn next_code_match(&mut self) {
        if let Some(result) = self.selected_code_result() {
            let max_matches = result.matches.len().saturating_sub(1);
            self.code_match_index = (self.code_match_index + 1).min(max_matches);
        }
    }

    /// Navigate to previous match within the current code result
    pub fn previous_code_match(&mut self) {
        if self.code_match_index > 0 {
            self.code_match_index -= 1;
        }
    }

    /// Reset match index when navigating to a different result
    pub fn reset_code_match_index(&mut self) {
        self.code_match_index = 0;
    }

    // ===== Search History Methods =====

    /// Enter history popup mode
    pub fn enter_history_popup(&mut self) {
        self.input_mode = InputMode::HistoryPopup;
        self.history_selected_index = 0;
    }

    /// Exit history popup mode
    pub fn exit_history_popup(&mut self) {
        self.input_mode = InputMode::Normal;
        self.search_history.clear();
        self.history_selected_index = 0;
    }

    /// Load search history for display
    pub fn load_search_history(&mut self, history: Vec<SearchHistoryEntry>) {
        self.search_history = history;
        self.history_selected_index = 0;
    }

    /// Navigate to next history entry
    pub fn next_history_entry(&mut self) {
        if !self.search_history.is_empty() {
            self.history_selected_index = (self.history_selected_index + 1).min(self.search_history.len() - 1);
        }
    }

    /// Navigate to previous history entry
    pub fn previous_history_entry(&mut self) {
        if self.history_selected_index > 0 {
            self.history_selected_index -= 1;
        }
    }

    /// Get the currently selected history entry
    pub fn selected_history_entry(&self) -> Option<&SearchHistoryEntry> {
        self.search_history.get(self.history_selected_index)
    }

    /// Apply selected history entry to search
    pub fn apply_selected_history(&mut self) -> Option<String> {
        // Clone the query first to avoid borrowing issues
        let query = self.selected_history_entry().map(|e| e.query.clone())?;
        // Set search input to the query from history
        self.search_input = query.clone();
        // Return the query so caller can trigger a search
        Some(query)
    }

    // ===== Trending Methods =====

    /// Toggle trending options panel
    pub fn toggle_trending_options(&mut self) {
        self.show_trending_options = !self.show_trending_options;
        if self.show_trending_options {
            self.trending_option_cursor = 0;
        }
    }

    /// Navigate trending options
    pub fn next_trending_option(&mut self) {
        // Options: 0=Period, 1=Language, 2=MinStars, 3=Topic, 4=SortByVelocity
        self.trending_option_cursor = (self.trending_option_cursor + 1).min(4);
    }

    pub fn previous_trending_option(&mut self) {
        if self.trending_option_cursor > 0 {
            self.trending_option_cursor -= 1;
        }
    }

    /// Toggle trending period
    pub fn toggle_trending_period(&mut self) {
        self.trending_filters.period = self.trending_filters.period.next();
    }

    /// Toggle sort by velocity
    pub fn toggle_trending_velocity(&mut self) {
        self.trending_filters.sort_by_velocity = !self.trending_filters.sort_by_velocity;
    }

    /// Adjust min stars for trending
    pub fn increase_trending_min_stars(&mut self) {
        self.trending_filters.min_stars = (self.trending_filters.min_stars + 50).min(10000);
    }

    pub fn decrease_trending_min_stars(&mut self) {
        self.trending_filters.min_stars = self.trending_filters.min_stars.saturating_sub(50);
    }

    // Settings/Token management methods

    /// Toggle settings popup
    pub fn toggle_settings(&mut self) {
        self.show_settings = !self.show_settings;
        if self.show_settings {
            self.input_mode = InputMode::Settings;
            self.settings_cursor = 0;
            self.token_status_message = None;
        } else {
            self.input_mode = InputMode::Normal;
        }
    }

    /// Open token input for a platform
    pub fn start_token_input(&mut self, platform: &str) {
        self.token_input_platform = platform.to_string();
        self.token_input_buffer.clear();
        self.input_mode = InputMode::TokenInput;
        self.token_status_message = None;
    }

    /// Save the entered token
    pub fn save_token(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use reposcout_core::TokenStore;

        if self.token_input_buffer.is_empty() {
            self.token_status_message = Some("Token cannot be empty".to_string());
            return Ok(());
        }

        // Load or create token store
        let mut store = TokenStore::load().unwrap_or_else(|_| TokenStore::new());

        // Store token with 30 days validity
        store.set_token(&self.token_input_platform, &self.token_input_buffer, 30);

        // Save to disk
        store.save()?;

        self.token_status_message = Some(format!(
            "{} token saved successfully! (Valid for 30 days)",
            self.token_input_platform.to_uppercase()
        ));

        // Clear input
        self.token_input_buffer.clear();
        self.input_mode = InputMode::Settings;

        Ok(())
    }

    /// Cancel token input
    pub fn cancel_token_input(&mut self) {
        self.token_input_buffer.clear();
        self.token_input_platform.clear();
        self.input_mode = InputMode::Settings;
    }

    /// Navigate settings options
    pub fn next_setting(&mut self) {
        self.settings_cursor = (self.settings_cursor + 1) % 4; // 4 options: GitHub, GitLab, Bitbucket, Close
    }

    pub fn previous_setting(&mut self) {
        if self.settings_cursor == 0 {
            self.settings_cursor = 3;
        } else {
            self.settings_cursor -= 1;
        }
    }

    /// Get current token status for a platform
    pub fn get_token_status(&self, platform: &str) -> String {
        use reposcout_core::TokenStore;

        match TokenStore::load() {
            Ok(store) => {
                if let Some(days) = store.get_token_days_remaining(platform) {
                    if days == 0 {
                        "Token expired".to_string()
                    } else if days == 1 {
                        "Expires in 1 day".to_string()
                    } else {
                        format!("Expires in {} days", days)
                    }
                } else {
                    "No token set".to_string()
                }
            }
            Err(_) => "No token set".to_string(),
        }
    }

    /// Navigate to next notification
    pub fn next_notification(&mut self) {
        if !self.notifications.is_empty() {
            self.notifications_selected_index = (self.notifications_selected_index + 1) % self.notifications.len();
        }
    }

    /// Navigate to previous notification
    pub fn previous_notification(&mut self) {
        if !self.notifications.is_empty() {
            if self.notifications_selected_index > 0 {
                self.notifications_selected_index -= 1;
            } else {
                self.notifications_selected_index = self.notifications.len() - 1;
            }
        }
    }

    /// Toggle showing all vs unread-only notifications
    pub fn toggle_notification_filter(&mut self) {
        self.notifications_show_all = !self.notifications_show_all;
    }

    /// Toggle participating filter
    pub fn toggle_participating_filter(&mut self) {
        self.notifications_participating = !self.notifications_participating;
    }

    /// Get currently selected notification
    pub fn get_selected_notification(&self) -> Option<&reposcout_core::Notification> {
        self.notifications.get(self.notifications_selected_index)
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
