// Enhanced discovery features for finding interesting repositories
use chrono::{Duration, Utc};

/// Build a search query for "New & Notable" - recently created repos gaining traction
pub fn new_and_notable_query(language: Option<&str>, days_back: i64) -> String {
    let date_threshold = (Utc::now() - Duration::days(days_back))
        .format("%Y-%m-%d")
        .to_string();

    let mut parts = vec![
        format!("created:>={}", date_threshold),
        "stars:>10".to_string(), // At least 10 stars to show traction
    ];

    if let Some(lang) = language {
        parts.push(format!("language:{}", lang));
    }

    parts.join(" ")
}

/// Build a search query for "Hidden Gems" - quality repos with low stars
pub fn hidden_gems_query(language: Option<&str>, max_stars: u32) -> String {
    let mut parts = vec![
        format!("stars:{}..{}", 10, max_stars), // Between 10 and max_stars
        "pushed:>2024-01-01".to_string(), // Recently updated
        "forks:>2".to_string(), // Some community engagement
    ];

    if let Some(lang) = language {
        parts.push(format!("language:{}", lang));
    }

    parts.join(" ")
}

/// Build a search query for a specific topic
pub fn topic_query(topic: &str, min_stars: u32) -> String {
    format!("topic:{} stars:>={}", topic, min_stars)
}

/// Popular topics for discovery
pub fn popular_topics() -> Vec<(&'static str, &'static str)> {
    vec![
        ("web", "Web Development"),
        ("mobile", "Mobile Development"),
        ("cli", "Command Line Tools"),
        ("machine-learning", "Machine Learning"),
        ("ai", "Artificial Intelligence"),
        ("blockchain", "Blockchain"),
        ("devops", "DevOps"),
        ("security", "Security"),
        ("game-development", "Game Development"),
        ("data-science", "Data Science"),
        ("frontend", "Frontend"),
        ("backend", "Backend"),
        ("database", "Databases"),
        ("kubernetes", "Kubernetes"),
        ("docker", "Docker"),
        ("automation", "Automation"),
        ("api", "APIs"),
        ("framework", "Frameworks"),
        ("library", "Libraries"),
        ("tool", "Developer Tools"),
    ]
}

/// Popular awesome lists
pub fn awesome_lists() -> Vec<(&'static str, &'static str)> {
    vec![
        ("sindresorhus/awesome", "Awesome Lists"),
        ("awesome-selfhosted/awesome-selfhosted", "Self-hosted"),
        ("avelino/awesome-go", "Awesome Go"),
        ("rust-unofficial/awesome-rust", "Awesome Rust"),
        ("sorrycc/awesome-javascript", "Awesome JavaScript"),
        ("vinta/awesome-python", "Awesome Python"),
        ("akullpp/awesome-java", "Awesome Java"),
        ("enaqx/awesome-react", "Awesome React"),
        ("vuejs/awesome-vue", "Awesome Vue"),
        ("awesome-foss/awesome-sysadmin", "Awesome Sysadmin"),
        ("k4m4/movies-for-hackers", "Movies for Hackers"),
        ("sdmg15/Best-websites-a-programmer-should-visit", "Best Websites"),
        ("EbookFoundation/free-programming-books", "Free Programming Books"),
        ("awesome-lists/awesome-bash", "Awesome Bash"),
        ("veggiemonk/awesome-docker", "Awesome Docker"),
    ]
}

/// Calculate "traction score" for new repos (stars per day)
pub fn calculate_traction_score(stars: u32, created_days_ago: i64) -> f64 {
    if created_days_ago <= 0 {
        return 0.0;
    }
    stars as f64 / created_days_ago as f64
}

/// Calculate "gem score" for hidden gems (activity vs popularity)
pub fn calculate_gem_score(stars: u32, forks: u32, open_issues: u32, days_since_update: i64) -> f64 {
    // Higher score for recent activity and engagement relative to stars
    let recency_multiplier = if days_since_update < 7 {
        2.0
    } else if days_since_update < 30 {
        1.5
    } else {
        1.0
    };

    let engagement_ratio = (forks + open_issues) as f64 / stars.max(1) as f64;
    engagement_ratio * recency_multiplier
}
